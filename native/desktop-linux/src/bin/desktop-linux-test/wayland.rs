use std::{cell::OnceCell, ffi::CStr, sync::atomic::AtomicI8};

use crate::gl_sys::{
    GL_COLOR_BUFFER_BIT, GL_COMPILE_STATUS, GL_DEPTH_BUFFER_BIT, GL_FALSE, GL_FLOAT, GL_FRAGMENT_SHADER, GL_LINK_STATUS, GL_TRIANGLES,
    GL_VERTEX_SHADER, GLchar, GLenum, GLint, GLuint, OpenGlFuncs,
};
use desktop_common::{
    ffi_utils::BorrowedStrPtr,
    logger_api::{LogLevel, LoggerConfiguration, logger_init_impl},
};
use desktop_linux::linux::{
    application_api::{
        AppPtr, ApplicationCallbacks, GetEglProcFuncData, application_get_egl_proc_func, application_init, application_run_event_loop,
    },
    events::{Event, WindowDrawEvent},
    window_api::{WindowParams, window_create},
    xdg_desktop_settings_api::XdgDesktopSetting,
};
use log::debug;

extern "C" fn on_should_terminate() -> bool {
    println!("on_should_terminate");
    true
}

extern "C" fn on_will_terminate() {
    println!("on_will_terminate");
}

extern "C" fn on_display_configuration_change() {
    println!("on_display_configuration_change");
}

fn between(val: f64, min: f64, max: f64) -> bool {
    val > min && val < max
}

static INSTANCE_COUNT: AtomicI8 = AtomicI8::new(0);

thread_local! {
    static APP_PTR: OnceCell<AppPtr<'static>> = const { OnceCell::new() };
    static LIB: OnceCell<GetEglProcFuncData<'static>> = const { OnceCell::new() };
    static OPENGL: OnceCell<OpenGlFuncs> = const { OnceCell::new() };
    static OPENGL_PROGRAM: OnceCell<GLuint> = const { OnceCell::new() };
}

const V_POSITION: GLuint = 0;

fn load_shader(gl: &OpenGlFuncs, shader_type: GLenum, shader_src: *const GLchar) -> Option<GLuint> {
    // Create the shader object
    let shader = unsafe { (gl.glCreateShader)(shader_type) };
    if shader == 0 {
        return None;
    }
    // Load the shader source
    unsafe { (gl.glShaderSource)(shader, 1, &shader_src, std::ptr::null()) };
    // Compile the shader
    unsafe { (gl.glCompileShader)(shader) };
    // Check the compile status
    {
        let mut compiled: GLint = 0;
        unsafe { (gl.glGetShaderiv)(shader, GL_COMPILE_STATUS, &mut compiled) };
        dbg!(compiled);
        if compiled == 0 {
            unsafe { (gl.glDeleteShader)(shader) };
            return None;
        }
    }
    Some(shader)
}

/// Initialize the shader and program object
fn create_opengl_program(gl: &OpenGlFuncs) -> Option<GLuint> {
    const V_SHADER_STR: &CStr = c"attribute vec4 vPosition;
void main()
{
  gl_Position = vPosition;
}
";
    const F_SHADER_STR: &CStr = c"precision mediump float;
void main()
{
  gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
}
";
    // Load the vertex/fragment shaders
    let vertex_shader = load_shader(gl, GL_VERTEX_SHADER, V_SHADER_STR.as_ptr()).unwrap();
    let fragment_shader = load_shader(gl, GL_FRAGMENT_SHADER, F_SHADER_STR.as_ptr()).unwrap();
    // Create the program object
    unsafe {
        let program = (gl.glCreateProgram)();
        if program == 0 {
            return None;
        }
        (gl.glAttachShader)(program, vertex_shader);
        (gl.glAttachShader)(program, fragment_shader);
        // Bind vPosition to attribute 0
        (gl.glBindAttribLocation)(program, V_POSITION, c"vPosition".as_ptr());
        (gl.glLinkProgram)(program);
        // Check the link status
        {
            let mut linked: GLint = 0;
            (gl.glGetProgramiv)(program, GL_LINK_STATUS, &mut linked);
            dbg!(linked);
            if linked == 0 {
                (gl.glDeleteProgram)(program);
                return None;
            }
        }
        (gl.glClearColor)(0.0, 1.0, 0.0, 1.0);
        Some(program)
    }
}

/// Draw a triangle using the shader pair created in `Init()`
fn draw_opengl_triangle(gl: &OpenGlFuncs, program: GLuint, data: &WindowDrawEvent) {
    //    debug!("draw_opengl_triangle, program = {program}, event = {data:?}");
    const V_VERTICES: [f32; 6] = [0.0f32, 1.0, -1.0, -1.0, 1.0, -1.0];
    unsafe {
        (gl.glViewport)(0, 0, data.physical_width, data.physical_height);
        (gl.glClear)(GL_DEPTH_BUFFER_BIT | GL_COLOR_BUFFER_BIT);
        (gl.glUseProgram)(program);
        //let v_position = (gl.glGetAttribLocation)(program, c"vPosition".as_ptr());
        //assert!(v_position != -1);
        // Load the vertex data
        (gl.glVertexAttribPointer)(V_POSITION, 2, GL_FLOAT, GL_FALSE, 0, V_VERTICES.as_ptr().cast());
        (gl.glEnableVertexAttribArray)(V_POSITION);
        (gl.glDrawArrays)(GL_TRIANGLES, 0, 3);
    }
}

fn draw_opengl_triangle_with_init(data: &WindowDrawEvent) {
    OPENGL.with(|c| {
        let gl = c.get_or_init(|| {
            LIB.with(|lib_c| {
                let lib = lib_c.get_or_init(|| {
                    assert!(
                        (INSTANCE_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0),
                        "Tried to instanciate more than once"
                    );
                    APP_PTR.with(|app_ptr_mutex| {
                        let app_ptr = app_ptr_mutex.get().unwrap();
                        application_get_egl_proc_func(app_ptr.clone())
                    })
                });

                OpenGlFuncs::new(lib).unwrap()
            })
        });

        OPENGL_PROGRAM.with(|c| {
            let program = c.get_or_init(|| {
                let program = create_opengl_program(gl).unwrap();
                debug!("draw_opengl_triangle_with_init, program = {program}, event = {data:?}");
                program
            });
            draw_opengl_triangle(gl, *program, data);
        });
    });
}

#[allow(clippy::many_single_char_names)]
fn draw(data: &WindowDrawEvent) {
    const BYTES_PER_PIXEL: u8 = 4;
    if data.buffer.is_null() {
        draw_opengl_triangle_with_init(data);
        return;
    }
    let canvas = unsafe { std::slice::from_raw_parts_mut(data.buffer, usize::try_from(data.physical_height * data.stride).unwrap()) };
    let w = f64::from(data.physical_width);
    let h = f64::from(data.physical_height);
    let scale = data.scale;
    let line_thickness = 5.0 * scale;

    // Order of bytes in `pixel` is [b, g, r, a] (for the Argb8888 format)
    for (pixel, i) in canvas.chunks_exact_mut(BYTES_PER_PIXEL.into()).zip(1u32..) {
        let i = f64::from(i);
        let x = i % w;
        let y = (i / f64::from(data.stride)) * f64::from(BYTES_PER_PIXEL);
        if between(x, line_thickness,  line_thickness * 2.0)  // left border
           || between(y, line_thickness,  line_thickness * 2.0)  // top border
           || between(x, line_thickness.mul_add(-2.0, w), w - line_thickness)  // right border
           || between(y, line_thickness.mul_add(-2.0, h), h - line_thickness)  // bottom border
           || between(x, (i / h) - (line_thickness / 2.0), (i / h) + (line_thickness / 2.0))
        {
            pixel[0] = 0;
            pixel[1] = 0;
        } else {
            pixel[0] = 255;
            pixel[1] = 255;
        }
        pixel[2] = 255;
        pixel[3] = 255;
    }
}

extern "C" fn event_handler_1(event: &Event) -> bool {
    match event {
        Event::WindowDraw(data) => {
            draw(data);
            return true;
        }
        Event::MouseMoved(_) => {}
        _ => {
            dbg!(event);
        }
    }
    true
}

extern "C" fn event_handler_2(event: &Event) -> bool {
    match event {
        Event::WindowDraw(data) => {
            draw(data);
            return true;
        }
        Event::MouseMoved(_) => {}
        _ => {
            dbg!(event);
        }
    }
    true
}

extern "C" fn on_xdg_desktop_settings_change(s: XdgDesktopSetting) {
    dbg!(s);
}

pub fn main() {
    const APP_ID: &CStr = c"org.jetbrains.desktop.linux.native.sample1";
    logger_init_impl(&LoggerConfiguration {
        file_path: BorrowedStrPtr::new(c"/tmp/a"),
        console_level: LogLevel::Debug,
        file_level: LogLevel::Error,
    });
    let app_ptr = application_init(ApplicationCallbacks {
        on_should_terminate,
        on_will_terminate,
        on_display_configuration_change,
        on_xdg_desktop_settings_change,
    });
    window_create(
        app_ptr.clone(),
        event_handler_1,
        WindowParams {
            width: 200,
            height: 300,
            title: BorrowedStrPtr::new(c"Window 1"),
            app_id: BorrowedStrPtr::new(APP_ID),
            force_client_side_decoration: false,
            force_software_rendering: true,
        },
    );
    window_create(
        app_ptr.clone(),
        event_handler_2,
        WindowParams {
            width: 300,
            height: 200,
            title: BorrowedStrPtr::new(c"Window 2"),
            app_id: BorrowedStrPtr::new(APP_ID),
            force_client_side_decoration: true,
            force_software_rendering: false,
        },
    );
    APP_PTR.with(|c| {
        c.set(application_init(ApplicationCallbacks {
            on_should_terminate,
            on_will_terminate,
            on_display_configuration_change,
            on_xdg_desktop_settings_change,
        }))
        .unwrap();
    });
    application_run_event_loop(app_ptr);
}
