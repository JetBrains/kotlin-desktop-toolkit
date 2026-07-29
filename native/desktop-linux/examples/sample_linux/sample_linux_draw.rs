use std::ffi::CStr;

use crate::sample_linux::{Drawable, WindowState};
use desktop_linux::linux::application_api::{AppPtr, application_get_egl_proc_func};
use desktop_linux::linux::geometry::PhysicalSize;
use gles30::{
    GL_COLOR_BUFFER_BIT, GL_COMPILE_STATUS, GL_DEPTH_BUFFER_BIT, GL_FLOAT, GL_FRAGMENT_SHADER, GL_LINK_STATUS, GL_TRIANGLES,
    GL_VERTEX_SHADER, GLchar, GLenum, GLint, GLuint, GlFns,
};
use log::debug;

#[derive(Debug)]
pub struct OpenglState {
    gl: GlFns,
    program: GLuint,
}

const V_POSITION: GLuint = 0;

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

impl Drawable for OpenglState {
    fn draw(&mut self, physical_size: PhysicalSize, window_state: &WindowState) {
        //    debug!("draw_opengl_triangle, program = {program}, event = {data:?}");
        let animation_progress = if window_state.animation_progress < 100. {
            -1.0 + (window_state.animation_progress / 50.)
        } else {
            1.0 - ((window_state.animation_progress - 100.) / 50.)
        };
        let v_vertices: [f32; 6] = [animation_progress, 1.0, -1.0, -1.0, 1.0, -1.0];
        let gl = &self.gl;
        unsafe {
            gl.Viewport(0, 0, physical_size.width.raw_physical(), physical_size.height.raw_physical());
            gl.Clear(GL_DEPTH_BUFFER_BIT | GL_COLOR_BUFFER_BIT);
            gl.UseProgram(self.program);
            //let v_position = gl.GetAttribLocation)(program, c"vPosition".as_ptr());
            //assert!(v_position != -1);
            // Load the vertex data
            gl.VertexAttribPointer(V_POSITION, 2, GL_FLOAT, 0, 0, v_vertices.as_ptr().cast());
            gl.EnableVertexAttribArray(V_POSITION);
            gl.DrawArrays(GL_TRIANGLES, 0, 3);
        }
    }
}

impl OpenglState {
    pub fn new(app_ptr: AppPtr) -> Self {
        let egl_lib = application_get_egl_proc_func(app_ptr);
        let gl = unsafe { GlFns::load_with(|name| (egl_lib.f)(egl_lib.ctx, name)) };
        let program = create_opengl_program(&gl).unwrap();
        debug!("draw_opengl_triangle_with_init, program = {program}");
        Self { gl, program }
    }
}
