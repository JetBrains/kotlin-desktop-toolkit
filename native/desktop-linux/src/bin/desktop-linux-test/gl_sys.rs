#![allow(dead_code, non_snake_case)]

use std::ffi::{c_char, c_int, c_uchar, c_uint, c_void};

use libloading::{Library, Symbol};

pub type GLbitfield = c_uint;
pub type GLboolean = c_uchar;
pub type GLchar = c_char;
pub type GLenum = c_uint;
pub type GLfloat = f32;
pub type GLint = c_int;
pub type GLsizei = c_int;
pub type GLuint = c_uint;
pub type GLsizeiptr = isize;

pub const GL_RGBA: GLint = 0x1908;
pub const GL_RGBA8: GLenum = 0x8058;
pub const GL_BGRA_EXT: GLint = 0x80E1;
pub const GL_CLAMP_TO_EDGE: GLint = 0x812F;
pub const GL_COLOR_ATTACHMENT0: GLenum = 0x8CE0;
pub const GL_COLOR_BUFFER_BIT: GLbitfield = 0x0000_4000;
pub const GL_COMPILE_STATUS: GLenum = 0x8B81;
pub const GL_EXTENSIONS: GLenum = 0x1F03;
pub const GL_FALSE: GLboolean = 0;
pub const GL_FLOAT: GLenum = 0x1406;
pub const GL_FRAGMENT_SHADER: GLenum = 0x8B30;
pub const GL_FRAMEBUFFER_COMPLETE: GLenum = 0x8CD5;
pub const GL_FRAMEBUFFER: GLenum = 0x8D40;
pub const GL_LINEAR: GLint = 0x2601;
pub const GL_LINK_STATUS: GLenum = 0x8B82;
pub const GL_RENDERBUFFER: GLenum = 0x8D41;
pub const GL_TEXTURE0: GLenum = 0x84C0;
pub const GL_TEXTURE_2D: GLenum = 0x0DE1;
pub const GL_TEXTURE_EXTERNAL_OES: GLenum = 0x8D65;
pub const GL_TEXTURE_MAG_FILTER: GLenum = 0x2800;
pub const GL_TEXTURE_MIN_FILTER: GLenum = 0x2801;
pub const GL_TEXTURE_WRAP_S: GLenum = 0x2802;
pub const GL_TEXTURE_WRAP_T: GLenum = 0x2803;
pub const GL_TRIANGLE_STRIP: GLenum = 0x0005;
pub const GL_TRIANGLES: GLenum = 0x0004;
pub const GL_UNPACK_ROW_LENGTH_EXT: GLenum = 0x0CF2;
pub const GL_UNSIGNED_BYTE: GLint = 0x1401;
pub const GL_VERTEX_SHADER: GLenum = 0x8B31;
pub const GL_BLEND: GLenum = 0x0BE2;
pub const GL_ONE: GLenum = 1;
pub const GL_ONE_MINUS_SRC_ALPHA: GLenum = 0x0303;
pub const GL_ARRAY_BUFFER: GLenum = 0x8892;
pub const GL_STATIC_DRAW: GLenum = 0x88E4;
pub const GL_DEPTH_BUFFER_BIT: u32 = 0x0000_0100;

#[allow(clippy::type_complexity)]
#[derive(Debug)]
pub struct OpenGlFuncs<'lib> {
    pub glGetString: Symbol<'lib, unsafe fn(name: GLenum) -> *const u8>,
    pub glGenRenderbuffers: Symbol<'lib, unsafe fn(n: GLsizei, renderbuffers: *mut GLuint)>,
    pub glRenderbufferStorage: Symbol<'lib, unsafe fn(target: GLenum, format: GLenum, width: GLsizei, height: GLsizei)>,
    pub glDeleteRenderbuffers: Symbol<'lib, unsafe fn(n: GLsizei, renderbuffers: *const GLuint)>,
    pub glBindRenderbuffer: Symbol<'lib, unsafe fn(target: GLenum, renderbuffer: GLuint)>,
    pub glGenFramebuffers: Symbol<'lib, unsafe fn(n: GLsizei, framebuffers: *mut GLuint)>,
    pub glDeleteFramebuffers: Symbol<'lib, unsafe fn(n: GLsizei, framebuffers: *const GLuint)>,
    pub glBindFramebuffer: Symbol<'lib, unsafe fn(target: GLenum, framebuffer: GLuint)>,
    pub glFramebufferRenderbuffer:
        Symbol<'lib, unsafe fn(target: GLenum, attachment: GLenum, renderbuffertarget: GLenum, renderbuffer: GLuint)>,
    pub glCheckFramebufferStatus: Symbol<'lib, unsafe fn(target: GLenum) -> GLenum>,
    pub glClear: Symbol<'lib, unsafe fn(mask: GLbitfield)>,
    pub glBlendFunc: Symbol<'lib, unsafe fn(sfactor: GLenum, dfactor: GLenum)>,
    pub glClearColor: Symbol<'lib, unsafe fn(red: GLfloat, green: GLfloat, blue: GLfloat, alpha: GLfloat)>,
    pub glFinish: Symbol<'lib, unsafe fn()>,

    pub glReadnPixels: Symbol<
        'lib,
        unsafe fn(x: GLint, y: GLint, width: GLsizei, height: GLsizei, format: GLenum, ty: GLenum, buf_size: GLsizei, data: *mut c_void),
    >,

    pub glGenTextures: Symbol<'lib, unsafe fn(n: GLsizei, textures: *mut GLuint)>,
    pub glDeleteTextures: Symbol<'lib, unsafe fn(n: GLsizei, textures: *const GLuint)>,
    pub glBindTexture: Symbol<'lib, unsafe fn(target: GLenum, texture: GLuint)>,
    pub glTexParameteri: Symbol<'lib, unsafe fn(target: GLenum, pname: GLenum, param: GLint)>,
    pub glPixelStorei: Symbol<'lib, unsafe fn(pname: GLenum, param: GLint)>,

    pub glTexImage2D: Symbol<
        'lib,
        unsafe fn(
            target: GLenum,
            level: GLint,
            internalformat: GLint,
            width: GLsizei,
            height: GLsizei,
            border: GLint,
            format: GLenum,
            ty: GLenum,
            pixels: *const c_void,
        ),
    >,

    pub glEnable: Symbol<'lib, unsafe fn(cap: GLenum)>,
    pub glDisable: Symbol<'lib, unsafe fn(cap: GLenum)>,
    pub glViewport: Symbol<'lib, unsafe fn(x: GLint, y: GLint, width: GLsizei, height: GLsizei)>,

    pub glCreateShader: Symbol<'lib, unsafe fn(ty: GLenum) -> GLuint>,
    pub glDeleteShader: Symbol<'lib, unsafe fn(shader: GLuint)>,
    pub glShaderSource: Symbol<'lib, unsafe fn(shader: GLuint, count: GLsizei, string: *const *const GLchar, length: *const GLint)>,
    pub glCompileShader: Symbol<'lib, unsafe fn(shader: GLuint)>,
    pub glGetShaderiv: Symbol<'lib, unsafe fn(shader: GLuint, pname: GLenum, params: *mut GLint)>,
    pub glCreateProgram: Symbol<'lib, unsafe fn() -> GLuint>,
    pub glDeleteProgram: Symbol<'lib, unsafe fn(prog: GLuint)>,
    pub glAttachShader: Symbol<'lib, unsafe fn(prog: GLuint, shader: GLuint)>,
    pub glDetachShader: Symbol<'lib, unsafe fn(prog: GLuint, shader: GLuint)>,
    pub glLinkProgram: Symbol<'lib, unsafe fn(prog: GLuint)>,
    pub glGetProgramiv: Symbol<'lib, unsafe fn(program: GLuint, pname: GLenum, params: *mut GLint)>,
    pub glUseProgram: Symbol<'lib, unsafe fn(program: GLuint)>,
    pub glGetUniformLocation: Symbol<'lib, unsafe fn(prog: GLuint, name: *const GLchar) -> GLint>,
    pub glGetAttribLocation: Symbol<'lib, unsafe fn(prog: GLuint, name: *const GLchar) -> GLint>,
    pub glUniform1i: Symbol<'lib, unsafe fn(location: GLint, v0: GLint)>,
    pub glUniform1f: Symbol<'lib, unsafe fn(location: GLint, v0: GLfloat)>,
    pub glUniform4f: Symbol<'lib, unsafe fn(location: GLint, v0: GLfloat, v1: GLfloat, v2: GLfloat, v3: GLfloat)>,

    pub glVertexAttribPointer:
        Symbol<'lib, unsafe fn(index: GLuint, size: GLint, ty: GLenum, normalized: GLboolean, stride: GLsizei, pointer: *const u8)>,

    pub glActiveTexture: Symbol<'lib, unsafe fn(texture: GLuint)>,
    pub glEnableVertexAttribArray: Symbol<'lib, unsafe fn(idx: GLuint)>,
    pub glDisableVertexAttribArray: Symbol<'lib, unsafe fn(idx: GLuint)>,
    pub glDrawArrays: Symbol<'lib, unsafe fn(mode: GLenum, first: GLint, count: GLsizei)>,
    pub glGenVertexArrays: Symbol<'lib, unsafe fn(n: GLsizei, arrays: *const GLuint)>,
    pub glBindVertexArray: Symbol<'lib, unsafe fn(array: GLuint)>,
    pub glGenBuffers: Symbol<'lib, unsafe fn(n: GLsizei, buffers: *const GLuint)>,
    pub glBindBuffer: Symbol<'lib, unsafe fn(target: GLenum, buffer: GLuint)>,
    pub glBufferData: Symbol<'lib, unsafe fn(target: GLenum, size: GLsizeiptr, data: *const c_void, usage: GLenum)>,
    pub glBindAttribLocation: Symbol<'lib, unsafe fn(program: GLuint, index: GLuint, name: *const GLchar)>,
}

impl<'lib> OpenGlFuncs<'lib> {
    pub fn new(lib: &'lib Library) -> Result<Self, libloading::Error> {
        Ok(Self {
            glGetString: unsafe { lib.get(b"glGetString") }?,
            glGenRenderbuffers: unsafe { lib.get(b"glGenRenderbuffers") }?,
            glRenderbufferStorage: unsafe { lib.get(b"glRenderbufferStorage") }?,
            glDeleteRenderbuffers: unsafe { lib.get(b"glDeleteRenderbuffers") }?,
            glBindRenderbuffer: unsafe { lib.get(b"glBindRenderbuffer") }?,
            glGenFramebuffers: unsafe { lib.get(b"glGenFramebuffers") }?,
            glDeleteFramebuffers: unsafe { lib.get(b"glDeleteFramebuffers") }?,
            glBindFramebuffer: unsafe { lib.get(b"glBindFramebuffer") }?,
            glFramebufferRenderbuffer: unsafe { lib.get(b"glFramebufferRenderbuffer") }?,
            glCheckFramebufferStatus: unsafe { lib.get(b"glCheckFramebufferStatus") }?,
            glClear: unsafe { lib.get(b"glClear") }?,
            glBlendFunc: unsafe { lib.get(b"glBlendFunc") }?,
            glClearColor: unsafe { lib.get(b"glClearColor") }?,
            glFinish: unsafe { lib.get(b"glFinish") }?,
            glReadnPixels: unsafe { lib.get(b"glReadnPixels") }?,
            glGenTextures: unsafe { lib.get(b"glGenTextures") }?,
            glDeleteTextures: unsafe { lib.get(b"glDeleteTextures") }?,
            glBindTexture: unsafe { lib.get(b"glBindTexture") }?,
            glTexParameteri: unsafe { lib.get(b"glTexParameteri") }?,
            glPixelStorei: unsafe { lib.get(b"glPixelStorei") }?,
            glTexImage2D: unsafe { lib.get(b"glTexImage2D") }?,
            glEnable: unsafe { lib.get(b"glEnable") }?,
            glDisable: unsafe { lib.get(b"glDisable") }?,
            glViewport: unsafe { lib.get(b"glViewport") }?,
            glCreateShader: unsafe { lib.get(b"glCreateShader") }?,
            glDeleteShader: unsafe { lib.get(b"glDeleteShader") }?,
            glShaderSource: unsafe { lib.get(b"glShaderSource") }?,
            glCompileShader: unsafe { lib.get(b"glCompileShader") }?,
            glGetShaderiv: unsafe { lib.get(b"glGetShaderiv") }?,
            glCreateProgram: unsafe { lib.get(b"glCreateProgram") }?,
            glDeleteProgram: unsafe { lib.get(b"glDeleteProgram") }?,
            glAttachShader: unsafe { lib.get(b"glAttachShader") }?,
            glDetachShader: unsafe { lib.get(b"glDetachShader") }?,
            glLinkProgram: unsafe { lib.get(b"glLinkProgram") }?,
            glGetProgramiv: unsafe { lib.get(b"glGetProgramiv") }?,
            glUseProgram: unsafe { lib.get(b"glUseProgram") }?,
            glGetUniformLocation: unsafe { lib.get(b"glGetUniformLocation") }?,
            glGetAttribLocation: unsafe { lib.get(b"glGetAttribLocation") }?,
            glUniform1i: unsafe { lib.get(b"glUniform1i") }?,
            glUniform1f: unsafe { lib.get(b"glUniform1f") }?,
            glUniform4f: unsafe { lib.get(b"glUniform4f") }?,
            glVertexAttribPointer: unsafe { lib.get(b"glVertexAttribPointer") }?,
            glActiveTexture: unsafe { lib.get(b"glActiveTexture") }?,
            glEnableVertexAttribArray: unsafe { lib.get(b"glEnableVertexAttribArray") }?,
            glDisableVertexAttribArray: unsafe { lib.get(b"glDisableVertexAttribArray") }?,
            glDrawArrays: unsafe { lib.get(b"glDrawArrays") }?,
            glGenVertexArrays: unsafe { lib.get(b"glGenVertexArrays") }?,
            glBindVertexArray: unsafe { lib.get(b"glBindVertexArray") }?,
            glGenBuffers: unsafe { lib.get(b"glGenBuffers") }?,
            glBindBuffer: unsafe { lib.get(b"glBindBuffer") }?,
            glBufferData: unsafe { lib.get(b"glBufferData") }?,
            glBindAttribLocation: unsafe { lib.get(b"glBindAttribLocation") }?,
        })
    }
}
