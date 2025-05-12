use core::str;
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::{CStr, CString},
    str::FromStr,
};

use desktop_common::{
    ffi_utils::{ArraySize, BorrowedArray, BorrowedStrPtr},
    logger_api::{LogLevel, LoggerConfiguration, logger_init_impl},
};
use desktop_linux::linux::{
    application_api::{
        AppPtr, ApplicationCallbacks, DataSource, DragAction, DragAndDropQueryData, application_clipboard_put,
        application_get_egl_proc_func, application_init, application_run_event_loop, application_set_cursor_theme, application_shutdown,
        application_start_drag_and_drop, application_stop_event_loop, application_text_input_disable, application_text_input_enable,
        application_text_input_update,
    },
    events::{Event, KeyModifiers, SoftwareDrawData, WindowDrawEvent, WindowId},
    geometry::{LogicalPixels, LogicalPoint, LogicalRect, LogicalSize, PhysicalSize},
    text_input_api::{TextInputContentPurpose, TextInputContext},
    window_api::{WindowParams, window_clipboard_paste, window_close, window_create},
    xdg_desktop_settings_api::XdgDesktopSetting,
};
use log::{debug, info};

use crate::gl_sys::{
    GL_COLOR_BUFFER_BIT, GL_COMPILE_STATUS, GL_DEPTH_BUFFER_BIT, GL_FALSE, GL_FLOAT, GL_FRAGMENT_SHADER, GL_LINK_STATUS, GL_TRIANGLES,
    GL_VERTEX_SHADER, GLchar, GLenum, GLint, GLuint, OpenGlFuncs,
};

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

pub const TEXT_MIME_TYPE: &CStr = c"text/plain;charset=utf-8";
pub const URI_LIST_MIME_TYPE: &CStr = c"text/uri-list";

pub const ALL_MIMES: &CStr = c"text/uri-list,text/plain;charset=utf-8";

#[derive(Debug)]
struct OpenglState {
    funcs: OpenGlFuncs,
    programs: HashMap<WindowId, GLuint>,
}

#[derive(Debug, Default)]
struct Settings {
    cursor_theme_name: Option<CString>,
    cursor_theme_size: Option<u32>,
}

#[derive(Debug, Default)]
struct WindowState {
    text_input_available: bool,
    composed_text: String,
    text: String,
    key_modifiers: KeyModifiers,
}

#[derive(Debug)]
struct State {
    app_ptr: AppPtr<'static>,
    windows: HashMap<WindowId, WindowState>,
    opengl: Option<OpenglState>,
    settings: Settings,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
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
        (gl.glViewport)(0, 0, data.physical_size.width.0, data.physical_size.height.0);
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

fn draw_opengl_triangle_with_init(data: &WindowDrawEvent, window_id: WindowId) {
    STATE.with(|c| {
        let mut state = c.borrow_mut();
        let state = state.as_mut().unwrap();
        let opengl_state = state.opengl.get_or_insert_with(|| {
            let egl_lib = application_get_egl_proc_func(state.app_ptr.clone());
            let funcs = OpenGlFuncs::new(&egl_lib).unwrap();
            let program = create_opengl_program(&funcs).unwrap();
            debug!("draw_opengl_triangle_with_init, program = {program}, event = {data:?}");
            let mut programs = HashMap::new();
            programs.insert(window_id, program);
            OpenglState { funcs, programs }
        });
        let program = opengl_state
            .programs
            .entry(window_id)
            .or_insert_with(|| create_opengl_program(&opengl_state.funcs).unwrap());

        draw_opengl_triangle(&opengl_state.funcs, *program, data);
    });
}

#[allow(clippy::many_single_char_names)]
fn draw_software(data: &SoftwareDrawData, physical_size: PhysicalSize, scale: f64) {
    const BYTES_PER_PIXEL: u8 = 4;
    let canvas = unsafe { std::slice::from_raw_parts_mut(data.canvas, usize::try_from(physical_size.height.0 * data.stride).unwrap()) };
    let w = f64::from(physical_size.width.0);
    let h = f64::from(physical_size.height.0);
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

#[allow(clippy::many_single_char_names)]
fn draw(event: &WindowDrawEvent, window_id: WindowId) {
    if event.software_draw_data.canvas.is_null() {
        draw_opengl_triangle_with_init(event, window_id);
    } else {
        draw_software(&event.software_draw_data, event.physical_size, event.scale);
    }
}

fn log_event(event: &Event, window_id: WindowId) {
    match event {
        Event::WindowDraw(_) | Event::MouseMoved(_) => {}
        _ => {
            debug!("{window_id:?} : {event:?}");
        }
    }
}

fn create_text_input_context<'a>(text: &str, text_cstring: &'a CString, change_caused_by_input_method: bool) -> TextInputContext<'a> {
    let mut codepoints_count = 0;
    for _ in text.chars() {
        codepoints_count += 1;
    }
    TextInputContext {
        surrounding_text: BorrowedStrPtr::new(text_cstring),
        cursor_codepoint_offset: codepoints_count,
        selection_start_codepoint_offset: codepoints_count,
        is_multiline: true,
        content_purpose: TextInputContentPurpose::Normal,
        cursor_rectangle: LogicalRect {
            origin: LogicalPoint {
                x: LogicalPixels(f64::from(codepoints_count) * 10.0),
                y: LogicalPixels(100.0),
            },
            size: LogicalSize {
                width: LogicalPixels(5.0),
                height: LogicalPixels(10.0),
            },
        },
        change_caused_by_input_method,
    }
}

fn update_text_input_context(app_ptr: AppPtr<'_>, text: &str, change_caused_by_input_method: bool) {
    let surrounding_text_cstring = CString::from_str(text).unwrap();
    application_text_input_update(
        app_ptr,
        create_text_input_context(text, &surrounding_text_cstring, change_caused_by_input_method),
    );
}

#[allow(clippy::too_many_lines)]
extern "C" fn event_handler(event: &Event, window_id: WindowId) -> bool {
    log_event(event, window_id);
    match event {
        Event::WindowDraw(data) => {
            draw(data, window_id);
            return true;
        }
        Event::WindowCloseRequest => STATE.with(|c| {
            let mut state = c.borrow_mut();
            let state = state.as_mut().unwrap();
            state.windows.retain(|&k, _v| k != window_id);
            window_close(state.app_ptr.clone(), window_id);
            if state.windows.is_empty() {
                application_stop_event_loop(state.app_ptr.clone());
            }
        }),
        Event::MouseDown(_) => STATE.with(|c| {
            let mut state = c.borrow_mut();
            let state = state.as_mut().unwrap();
            application_start_drag_and_drop(state.app_ptr.clone(), window_id, BorrowedStrPtr::new(ALL_MIMES), DragAction::Copy);
        }),
        Event::ModifiersChanged(data) => STATE.with(|c| {
            let mut state = c.borrow_mut();
            let state = state.as_mut().unwrap();
            let window_state = state.windows.get_mut(&window_id).unwrap();
            window_state.key_modifiers = data.modifiers;
        }),
        Event::KeyDown(data) => STATE.with(|c| {
            let mut state = c.borrow_mut();
            let state = state.as_mut().unwrap();
            let window_state = state.windows.get_mut(&window_id).unwrap();
            match data.code.0 {
                14 => {
                    window_state.text.pop();
                    if window_state.text_input_available {
                        update_text_input_context(state.app_ptr.clone(), &window_state.text, false);
                    }
                }
                _ => {
                    if data.code.0 == 47 && window_state.key_modifiers.ctrl {
                        window_clipboard_paste(state.app_ptr.clone(), window_id, 0, BorrowedStrPtr::new(TEXT_MIME_TYPE));
                    } else if data.code.0 == 46 && window_state.key_modifiers.ctrl {
                        application_clipboard_put(state.app_ptr.clone(), BorrowedStrPtr::new(ALL_MIMES));
                    } else if let Some(event_chars) = data.characters.as_optional_str().unwrap() {
                        window_state.text += event_chars;
                        if window_state.text_input_available {
                            update_text_input_context(state.app_ptr.clone(), &window_state.text, false);
                        }
                    }
                }
            }

            debug!("{window_id:?} : {} : {}", window_state.text.len(), window_state.text);
        }),
        Event::DataTransfer(data) => {
            if data
                .mime_types
                .as_str()
                .unwrap()
                .split(',')
                .any(|s| s == URI_LIST_MIME_TYPE.to_str().unwrap())
            {
                let list_str = str::from_utf8(data.data.as_slice().unwrap()).unwrap();
                let list = list_str.split('\n').collect::<Vec<_>>();
                info!("Pasted file list: {list:?}");
            } else if data
                .mime_types
                .as_str()
                .unwrap()
                .split(',')
                .any(|s| s == TEXT_MIME_TYPE.to_str().unwrap())
            {
                STATE.with(|c| {
                    let mut state = c.borrow_mut();
                    let state = state.as_mut().unwrap();
                    let window_state = state.windows.get_mut(&window_id).unwrap();
                    let data_str = str::from_utf8(data.data.as_slice().unwrap()).unwrap();
                    window_state.text += data_str;
                });
            }
        }
        Event::TextInputAvailability(data) => STATE.with(|c| {
            let mut state = c.borrow_mut();
            let state = state.as_mut().unwrap();
            let window_state = state.windows.get_mut(&window_id).unwrap();
            if data.available {
                let surrounding_text_cstring = CString::from_str(&window_state.text).unwrap();
                application_text_input_enable(
                    state.app_ptr.clone(),
                    create_text_input_context(&window_state.text, &surrounding_text_cstring, false),
                );
                window_state.text_input_available = true;
            } else {
                application_text_input_disable(state.app_ptr.clone());
                window_state.text_input_available = false;
            }
        }),
        Event::TextInput(data) => STATE.with(|c| {
            let mut state = c.borrow_mut();
            let state = state.as_mut().unwrap();
            let window_state = state.windows.get_mut(&window_id).unwrap();
            window_state.composed_text.clear();
            if data.has_delete_surrounding_text {
                let cursor_pos = window_state.text.len();
                let range = (cursor_pos - data.delete_surrounding_text.before_length_in_bytes as usize)
                    ..(cursor_pos + data.delete_surrounding_text.after_length_in_bytes as usize);
                window_state.text.drain(range);
            }
            if data.has_commit_string {
                if let Some(commit_string) = data.commit_string.as_optional_str().unwrap() {
                    debug!("{window_id:?} commit_string: {commit_string}");
                    window_state.text += commit_string;
                }
            }
            if data.has_delete_surrounding_text || data.has_commit_string {
                update_text_input_context(state.app_ptr.clone(), &window_state.text, true);
            }

            if data.has_preedit_string {
                if data.preedit_string.cursor_begin_byte_pos == -1 && data.preedit_string.cursor_end_byte_pos == -1 {
                    // TODO: hide cursor
                } else if let Some(preedit_string) = data.preedit_string.text.as_optional_str().unwrap() {
                    window_state.composed_text.push_str(preedit_string);
                }
            }

            debug!("{window_id:?} : {} : {:?}", window_state.text.len(), window_state);
        }),
        _ => {}
    }
    true
}

extern "C" fn on_xdg_desktop_settings_change(s: &XdgDesktopSetting) {
    debug!("{s:?}");
    STATE.with(|c| {
        let mut state = c.borrow_mut();
        let state = state.as_mut().unwrap();
        match s {
            XdgDesktopSetting::CursorSize(v) => {
                let size = (*v).try_into().unwrap();
                if let Some(name) = &state.settings.cursor_theme_name {
                    application_set_cursor_theme(state.app_ptr.clone(), BorrowedStrPtr::new(name), size);
                }
                state.settings.cursor_theme_size = Some(size);
            }
            XdgDesktopSetting::CursorTheme(v) => {
                let name = CString::new(v.as_str().unwrap()).unwrap();
                if let Some(size) = state.settings.cursor_theme_size {
                    application_set_cursor_theme(state.app_ptr.clone(), BorrowedStrPtr::new(&name), size);
                }
                state.settings.cursor_theme_name = Some(name);
            }
            _ => {}
        };
    });
}

extern "C" fn on_application_started() {
    const APP_ID: &CStr = c"org.jetbrains.desktop.linux.native.sample1";
    STATE.with(|c| {
        let mut state = c.borrow_mut();
        let state = state.as_mut().unwrap();
        let window_1_id = WindowId(1);
        window_create(
            state.app_ptr.clone(),
            WindowParams {
                window_id: window_1_id,
                size: LogicalSize {
                    width: LogicalPixels(200.),
                    height: LogicalPixels(300.),
                },
                title: BorrowedStrPtr::new(c"Window 1"),
                app_id: BorrowedStrPtr::new(APP_ID),
                force_client_side_decoration: false,
                force_software_rendering: true,
            },
        );
        state.windows.insert(window_1_id, WindowState::default());

        let window_2_id = WindowId(2);
        window_create(
            state.app_ptr.clone(),
            WindowParams {
                window_id: window_2_id,
                size: LogicalSize {
                    width: LogicalPixels(300.),
                    height: LogicalPixels(200.),
                },
                title: BorrowedStrPtr::new(c"Window 2"),
                app_id: BorrowedStrPtr::new(APP_ID),
                force_client_side_decoration: true,
                force_software_rendering: false,
            },
        );
        state.windows.insert(window_2_id, WindowState::default());
    });
}

extern "C" fn drag_and_drop_query_handler(data: &DragAndDropQueryData) -> BorrowedStrPtr<'static> {
    if data.point.x.0 < 100. {
        BorrowedStrPtr::new(ALL_MIMES)
    } else {
        BorrowedStrPtr::new(c"")
    }
}

extern "C" fn deinit_u8_vec(ptr: *const u8, len: ArraySize) {
    let _ = unsafe {
        let s = std::slice::from_raw_parts_mut(ptr.cast_mut(), len);
        Box::from_raw(s)
    };
}

fn leaked_string_data(s: &str) -> &'static [u8] {
    Box::leak(s.to_owned().into_boxed_str().into_boxed_bytes())
}

extern "C" fn get_data_source_data(source: DataSource, mime_type: BorrowedStrPtr) -> BorrowedArray<'static, u8> {
    let mime_type_cstr = mime_type.as_optional_cstr().unwrap().unwrap();
    let v = if mime_type_cstr == URI_LIST_MIME_TYPE {
        match source {
            DataSource::Clipboard => leaked_string_data("file:///etc/hosts"),
            DataSource::DragAndDrop => leaked_string_data("file:///boot/efi/"),
        }
    } else if mime_type_cstr == TEXT_MIME_TYPE {
        match source {
            DataSource::Clipboard => leaked_string_data("/etc/hosts (from clipboard)"),
            DataSource::DragAndDrop => leaked_string_data("/boot/efi/ (from d&d)"),
        }
    } else {
        leaked_string_data(mime_type_cstr.to_str().unwrap())
    };

    let mut a = BorrowedArray::from_slice(v);
    a.deinit = Some(deinit_u8_vec);
    a
}

pub fn main() {
    logger_init_impl(&LoggerConfiguration {
        file_path: BorrowedStrPtr::new(c"/tmp/a"),
        console_level: LogLevel::Debug,
        file_level: LogLevel::Error,
    });
    let app_ptr = application_init(ApplicationCallbacks {
        on_application_started,
        on_should_terminate,
        on_will_terminate,
        on_display_configuration_change,
        on_xdg_desktop_settings_change,
        event_handler,
        get_drag_and_drop_supported_mime_types: drag_and_drop_query_handler,
        get_data_transfer_data: get_data_source_data,
    });
    STATE.with(|c| {
        c.replace(Some(State {
            app_ptr: app_ptr.clone(),
            windows: HashMap::new(),
            opengl: None,
            settings: Settings::default(),
        }));
    });
    application_run_event_loop(app_ptr.clone());
    application_shutdown(app_ptr);
}
