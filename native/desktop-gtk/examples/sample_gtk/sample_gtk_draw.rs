use desktop_common::ffi_utils::BorrowedStrPtr;
use desktop_gtk::gtk::application_api::{application_get_egl_proc_func, application_init_gl};
use desktop_gtk::gtk::geometry::PhysicalSize;
use gles30::{
    GL_ARRAY_BUFFER, GL_COLOR_BUFFER_BIT, GL_COMPILE_STATUS, GL_DEPTH_BUFFER_BIT, GL_FLOAT, GL_FRAGMENT_SHADER, GL_LINK_STATUS,
    GL_STATIC_DRAW, GL_TRIANGLES, GL_VERTEX_SHADER, GLchar, GLenum, GLint, GLsizeiptr, GLuint, GlFns,
};
use log::warn;
use std::f32::consts::PI;
use std::ffi::CStr;

const V_POSITION: GLuint = 0;

pub struct OpenglState {
    program: GLuint,
    position_buffer: GLuint,
    mvp_location: GLint,
}

impl OpenglState {
    pub fn new(gl: &GlFns, is_es: bool) -> Self {
        const V_VERTICES: [f32; 12] = [0.0, 0.5, 0.0, 1.0, 0.5, -0.366, 0.0, 1.0, -0.5, -0.366, 0.0, 1.0];
        let (program, mvp_location) = create_opengl_program(gl, is_es).unwrap();
        let mut vao = 0;
        let mut position_buffer = 0;
        unsafe {
            gl.GenVertexArrays(1, &raw mut vao);
            gl.BindVertexArray(vao);

            gl.GenBuffers(1, &raw mut position_buffer);
            gl.BindBuffer(GL_ARRAY_BUFFER, position_buffer);
            gl.BufferData(
                GL_ARRAY_BUFFER,
                size_of::<[f32; 12]>() as GLsizeiptr,
                V_VERTICES.as_ptr().cast(),
                GL_STATIC_DRAW,
            );
            gl.BindBuffer(GL_ARRAY_BUFFER, 0);
        }
        // debug!("draw_opengl_triangle_with_init, program = {program}");
        Self {
            program,
            position_buffer,
            mvp_location,
        }
    }
}

pub fn init_gl() -> Option<GlFns> {
    let gl_lib = {
        let egl_lib = application_get_egl_proc_func();

        if egl_lib.ctx.is_null() {
            application_init_gl(BorrowedStrPtr::new(
                c"/System/Library/Frameworks/OpenGL.framework/Versions/A/Libraries/libGL.dylib",
            ))
        } else {
            egl_lib
        }
    };

    if gl_lib.ctx.is_null() {
        warn!("OpenGL library not available");
        None
    } else {
        let gl = unsafe { GlFns::load_with(|c| (gl_lib.f)(gl_lib.ctx.clone(), BorrowedStrPtr::from_ptr(c))) };
        Some(gl)
    }
}

fn load_shader(gl: &GlFns, shader_type: GLenum, shader_src: *const GLchar) -> Option<GLuint> {
    // Create the shader object
    let shader = unsafe { gl.CreateShader(shader_type) };
    if shader == 0 {
        warn!("glCreateShader failed");
        return None;
    }
    // Load the shader source
    unsafe { gl.ShaderSource(shader, 1, &raw const shader_src, std::ptr::null()) };
    // Compile the shader
    unsafe { gl.CompileShader(shader) };
    // Check the compilation status
    {
        let mut compiled = 0;
        unsafe { gl.GetShaderiv(shader, GL_COMPILE_STATUS, &raw mut compiled) };
        if compiled == 0 {
            warn!("glCompileShader failed for shader {shader} (type {shader_type:x}");
            unsafe { gl.DeleteShader(shader) };
            return None;
        }
    }
    Some(shader)
}

/// Initialize the shader and program object
fn create_opengl_program(gl: &GlFns, is_es: bool) -> Option<(GLuint, GLint)> {
    const V_SHADER_STR: &CStr = c"#version 330

in vec4 in_position;
uniform mat4 mvp;

void main() {
  gl_Position = mvp * in_position;
}
";

    const V_SHADER_ES_STR: &CStr = c"attribute vec4 in_position;
uniform mat4 mvp;

void main() {
  gl_Position = mvp * in_position;
}";

    const F_SHADER_STR: &CStr = c"#version 330

out vec4 outputColor;

void main()
{
  outputColor = vec4(1.0, 0.0, 0.0, 1.0);
}
";

    const F_SHADER_ES_STR: &CStr = c"precision highp float;

void main() {
  gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
}
";
    // Load the vertex/fragment shaders
    let vertex_shader = load_shader(gl, GL_VERTEX_SHADER, if is_es { V_SHADER_ES_STR } else { V_SHADER_STR }.as_ptr())?;
    let fragment_shader = load_shader(gl, GL_FRAGMENT_SHADER, if is_es { F_SHADER_ES_STR } else { F_SHADER_STR }.as_ptr())?;
    // Create the program object
    unsafe {
        let program = gl.CreateProgram();
        if program == 0 {
            return None;
        }
        gl.AttachShader(program, vertex_shader);
        gl.AttachShader(program, fragment_shader);
        // Bind in_position to attribute 0
        gl.BindAttribLocation(program, V_POSITION, c"in_position".as_ptr());
        gl.LinkProgram(program);
        // Check the link status
        {
            let mut linked = 0;
            gl.GetProgramiv(program, GL_LINK_STATUS, &raw mut linked);
            if linked == 0 {
                warn!("Error linking program; is_es={is_es}");
                gl.DeleteProgram(program);
                return None;
            }
        }

        let mvp = gl.GetUniformLocation(program, c"mvp".as_ptr());

        Some((program, mvp))
    }
}

/// Draw a triangle using the shader pair created in `Init()`
pub fn draw_opengl_triangle(gl: &GlFns, gl_state: &OpenglState, physical_size: PhysicalSize, _scale: f64, animation_progress: f32) {
    // let mut screen_fb = 0;
    // unsafe { (gl.glGetIntegerv)(GL_FRAMEBUFFER_BINDING, &raw mut screen_fb) };
    // dbg!(screen_fb);
    // debug!(
    //     "draw_opengl_triangle, program = {}, physical_size = {physical_size:?}",
    //     gl_state.program
    // );
    unsafe {
        gl.Viewport(0, 0, physical_size.width.0, physical_size.height.0);
        gl.ClearColor(0.0, 1.0, 0.0, 1.0);
        gl.Clear(GL_DEPTH_BUFFER_BIT | GL_COLOR_BUFFER_BIT);
        gl.UseProgram(gl_state.program);

        let mvp = compute_mvp(
            0.0,                // rotation_angles[X_AXIS],
            0.0,                // rotation_angles[Y_AXIS],
            animation_progress, // rotation_angles[Z_AXIS]);
        );

        gl.UniformMatrix4fv(gl_state.mvp_location, 1, 0, &raw const mvp[0]);

        gl.BindBuffer(GL_ARRAY_BUFFER, gl_state.position_buffer);
        gl.EnableVertexAttribArray(V_POSITION);
        gl.VertexAttribPointer(0, 4, GL_FLOAT, 0, 0, std::ptr::null());
        gl.DrawArrays(GL_TRIANGLES, 0, 3);

        /* We finished using the buffers and program */
        gl.DisableVertexAttribArray(0);
        gl.BindBuffer(GL_ARRAY_BUFFER, 0);
        gl.UseProgram(0);
    }
}

#[allow(clippy::similar_names)]
fn compute_mvp(phi: f32, theta: f32, psi: f32) -> [f32; 16] {
    let x = phi * (PI / 180.);
    let y = theta * (PI / 180.);
    let z = psi * (PI / 180.);
    let c1 = x.cos();
    let s1 = x.sin();
    let c2 = y.cos();
    let s2 = y.sin();
    let c3 = z.cos();
    let s3 = z.sin();
    let c3c2 = c3 * c2;
    let s3c1 = s3 * c1;
    let c3s2s1 = c3 * s2 * s1;
    let s3s1 = s3 * s1;
    let c3s2c1 = c3 * s2 * c1;
    let s3c2 = s3 * c2;
    let c3c1 = c3 * c1;
    let s3s2s1 = s3 * s2 * s1;
    let c3s1 = c3 * s1;
    let s3s2c1 = s3 * s2 * c1;
    let c2s1 = c2 * s1;
    let c2c1 = c2 * c1;

    /* initialize to the identity matrix */
    let mut res: [f32; 16] = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.];

    /* apply all three rotations using the three matrices:
     *
     * ⎡  c3 s3 0 ⎤ ⎡ c2  0 -s2 ⎤ ⎡ 1   0  0 ⎤
     * ⎢ -s3 c3 0 ⎥ ⎢  0  1   0 ⎥ ⎢ 0  c1 s1 ⎥
     * ⎣   0  0 1 ⎦ ⎣ s2  0  c2 ⎦ ⎣ 0 -s1 c1 ⎦
     */
    res[0] = c3c2;
    res[4] = s3c1 + c3s2s1;
    res[8] = s3s1 - c3s2c1;
    res[12] = 0.;
    res[1] = -s3c2;
    res[5] = c3c1 - s3s2s1;
    res[9] = c3s1 + s3s2c1;
    res[13] = 0.;
    res[2] = s2;
    res[6] = -c2s1;
    res[10] = c2c1;
    res[14] = 0.;
    res[3] = 0.;
    res[7] = 0.;
    res[11] = 0.;
    res[15] = 1.;

    res
}
