use std::{collections::HashMap, ffi::CStr};

use crate::sample_linux::WindowState;
use desktop_linux::linux::application_api::{AppPtr, application_get_egl_proc_func};
use desktop_linux::linux::events::{SoftwareDrawData, WindowId};
use desktop_linux::linux::geometry::PhysicalSize;
use gles30::{
    GL_COLOR_BUFFER_BIT, GL_COMPILE_STATUS, GL_DEPTH_BUFFER_BIT, GL_FLOAT, GL_FRAGMENT_SHADER, GL_LINK_STATUS, GL_TRIANGLES,
    GL_VERTEX_SHADER, GLchar, GLenum, GLint, GLuint, GlFns,
};
use log::debug;

fn between(val: f64, min: f64, max: f64) -> bool {
    val > min && val < max
}

#[derive(Debug)]
pub struct OpenglState {
    gl: GlFns,
    programs: HashMap<WindowId, GLuint>,
}

const BYTES_PER_PIXEL: usize = 4;
const V_POSITION: GLuint = 0;
const DRAG_AND_DROP_LEFT_OF: f64 = 100.;

fn load_shader(gl: &GlFns, shader_type: GLenum, shader_src: *const GLchar) -> Option<GLuint> {
    // Create the shader object
    let shader = unsafe { gl.CreateShader(shader_type) };
    if shader == 0 {
        return None;
    }
    // Load the shader source
    unsafe { gl.ShaderSource(shader, 1, &raw const shader_src, std::ptr::null()) };
    // Compile the shader
    unsafe { gl.CompileShader(shader) };
    // Check the compile status
    {
        let mut compiled: GLint = 0;
        unsafe { gl.GetShaderiv(shader, GL_COMPILE_STATUS, &raw mut compiled) };
        if compiled == 0 {
            unsafe { gl.DeleteShader(shader) };
            return None;
        }
    }
    Some(shader)
}

/// Initialize the shader and program object
fn create_opengl_program(gl: &GlFns) -> Option<GLuint> {
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
        let program = gl.CreateProgram();
        if program == 0 {
            return None;
        }
        gl.AttachShader(program, vertex_shader);
        gl.AttachShader(program, fragment_shader);
        // Bind vPosition to attribute 0
        gl.BindAttribLocation(program, V_POSITION, c"vPosition".as_ptr());
        gl.LinkProgram(program);
        // Check the link status
        {
            let mut linked: GLint = 0;
            gl.GetProgramiv(program, GL_LINK_STATUS, &raw mut linked);
            if linked == 0 {
                gl.DeleteProgram(program);
                return None;
            }
        }
        gl.ClearColor(0.0, 1.0, 0.0, 1.0);
        Some(program)
    }
}

/// Draw a triangle using the shader pair created in `Init()`
fn draw_opengl_triangle(gl: &GlFns, program: GLuint, physical_size: PhysicalSize, animation_progress: f32) {
    //    debug!("draw_opengl_triangle, program = {program}, event = {data:?}");
    let v_vertices: [f32; 6] = [animation_progress, 1.0, -1.0, -1.0, 1.0, -1.0];
    unsafe {
        gl.Viewport(0, 0, physical_size.width.0, physical_size.height.0);
        gl.Clear(GL_DEPTH_BUFFER_BIT | GL_COLOR_BUFFER_BIT);
        gl.UseProgram(program);
        //let v_position = gl.GetAttribLocation)(program, c"vPosition".as_ptr());
        //assert!(v_position != -1);
        // Load the vertex data
        gl.VertexAttribPointer(V_POSITION, 2, GL_FLOAT, 0, 0, v_vertices.as_ptr().cast());
        gl.EnableVertexAttribArray(V_POSITION);
        gl.DrawArrays(GL_TRIANGLES, 0, 3);
    }
}

pub fn draw_opengl_triangle_with_init(app_ptr: AppPtr, physical_size: PhysicalSize, window_id: WindowId, window_state: &mut WindowState) {
    let opengl_state = window_state.opengl.get_or_insert_with(|| {
        let egl_lib = application_get_egl_proc_func(app_ptr);
        let gl = unsafe { GlFns::load_with(|name| (egl_lib.f)(egl_lib.ctx, name)) };
        let program = create_opengl_program(&gl).unwrap();
        debug!("draw_opengl_triangle_with_init, program = {program}");
        let mut programs = HashMap::new();
        programs.insert(window_id, program);
        OpenglState { gl, programs }
    });
    let program = opengl_state
        .programs
        .entry(window_id)
        .or_insert_with(|| create_opengl_program(&opengl_state.gl).unwrap());
    let animation_progress = if window_state.animation_progress < 100. {
        -1.0 + (window_state.animation_progress / 50.)
    } else {
        1.0 - ((window_state.animation_progress - 100.) / 50.)
    };

    draw_opengl_triangle(&opengl_state.gl, *program, physical_size, animation_progress);
}

#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
const fn pixel_bytes(v: f64) -> usize {
    v.round() as usize * BYTES_PER_PIXEL
}

fn stride_len(v: u32, stride: i32) -> usize {
    usize::try_from(v * stride.unsigned_abs()).unwrap()
}

fn stride_offset(v: i32, stride: i32) -> isize {
    isize::try_from(v * stride).unwrap()
}

#[allow(clippy::many_single_char_names)]
pub fn draw_software(data: &SoftwareDrawData, physical_size: PhysicalSize, window_state: &WindowState) {
    let scale = window_state.scale;
    let stride = data.stride;
    let w = f64::from(physical_size.width.0);
    let h = f64::from(physical_size.height.0);

    let line_thickness = 4. * scale;
    let drag_and_drop_left_of = DRAG_AND_DROP_LEFT_OF * scale;
    let drag_source_indicator_height = 100. * scale;

    let mut horizontal_line = vec![0; stride_len(1, stride)];

    // Order of bytes in `pixel` is [b, g, r, a] (for the Argb8888 format)
    for (pixel, i) in horizontal_line.as_chunks_mut::<BYTES_PER_PIXEL>().0.iter_mut().zip(0u32..) {
        let x = f64::from(i % physical_size.width.0.cast_unsigned());
        if between(x, drag_and_drop_left_of, drag_and_drop_left_of + line_thickness) {
            *pixel = [0, 0, 0, 255];
        } else if between(x, line_thickness, line_thickness * 2.) || between(x, w - (line_thickness * 2.), w - line_thickness) {
            // left and right border
            *pixel = [0, 0, 255, 255];
        } else if x < drag_and_drop_left_of && window_state.drag_and_drop_target {
            *pixel = [128, 0, 0, 255];
        } else if window_state.active {
            pixel.fill(255);
        } else {
            *pixel = [128, 128, 128, 255];
        }
    }

    let content_x_end_bytes = pixel_bytes(w);
    let half_line_thickness = line_thickness / 2.;

    for y in 0..physical_size.height.0 {
        let line = unsafe { std::slice::from_raw_parts_mut(data.canvas.offset(stride_offset(y, stride)), pixel_bytes(w)) };
        line.copy_from_slice(&horizontal_line);

        let y = f64::from(y);

        if between(y, line_thickness, line_thickness * 2.) || between(y, h - (line_thickness * 2.), h - line_thickness) {
            // top and bottom border
            line[..content_x_end_bytes]
                .as_chunks_mut::<BYTES_PER_PIXEL>()
                .0
                .fill([0, 0, 255, 255]);
        }

        if window_state.drag_and_drop_source && between(y, drag_source_indicator_height, drag_source_indicator_height + line_thickness) {
            line[..pixel_bytes(drag_and_drop_left_of)]
                .as_chunks_mut::<BYTES_PER_PIXEL>()
                .0
                .fill([255, 0, 0, 255]);
        }

        let diagonal_x = (y * w) / h;

        let diagonal_start_x_pixel = pixel_bytes((diagonal_x - half_line_thickness).max(0.));
        let diagonal_end_x_pixel = pixel_bytes(diagonal_x + half_line_thickness).min(content_x_end_bytes);
        line[diagonal_start_x_pixel..diagonal_end_x_pixel]
            .as_chunks_mut::<BYTES_PER_PIXEL>()
            .0
            .fill([0, 0, 255, 255]);
    }
}

#[allow(clippy::many_single_char_names)]
pub fn draw_software_drag_icon(data: &SoftwareDrawData, physical_size: PhysicalSize, scale: f64) {
    let stride = data.stride;
    let w = f64::from(physical_size.width.0);
    let h = f64::from(physical_size.height.0);

    let line_thickness = 4. * scale;

    let mut horizontal_line = vec![0; stride_len(1, stride)];

    // Order of bytes in `pixel` is [b, g, r, a] (for the Argb8888 format)
    for (pixel, i) in horizontal_line.as_chunks_mut::<BYTES_PER_PIXEL>().0.iter_mut().zip(0u32..) {
        let x = f64::from(i % physical_size.width.0.cast_unsigned());
        if between(x, line_thickness, line_thickness * 2.) || between(x, w - (line_thickness * 2.), w - line_thickness) {
            // left and right border
            *pixel = [0, 0, 255, 255];
        }
    }

    for y in 0..physical_size.height.0 {
        let line = unsafe { std::slice::from_raw_parts_mut(data.canvas.offset(stride_offset(y, stride)), pixel_bytes(w)) };
        let y = f64::from(y);

        if between(y, line_thickness, line_thickness * 2.) || between(y, h - (line_thickness * 2.), h - line_thickness) {
            // top and bottom border
            line.as_chunks_mut::<BYTES_PER_PIXEL>().0.fill([0, 0, 255, 255]);
        } else {
            line.copy_from_slice(&horizontal_line);
        }
    }
}
