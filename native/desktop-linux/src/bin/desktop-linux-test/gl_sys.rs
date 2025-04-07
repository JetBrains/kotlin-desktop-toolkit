#![allow(dead_code, non_snake_case)]

use std::{
    ffi::{CStr, c_char, c_int, c_uchar, c_uint, c_void},
    mem::ManuallyDrop,
};

use anyhow::bail;
use desktop_common::ffi_utils::BorrowedStrPtr;
use desktop_linux::linux::application_api::GetEglProcFuncData;

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
pub struct OpenGlFuncs {
    pub glGetString: unsafe extern "C" fn(name: GLenum) -> *const u8,
    pub glGenRenderbuffers: unsafe fn(n: GLsizei, renderbuffers: *mut GLuint),
    pub glRenderbufferStorage: unsafe fn(target: GLenum, format: GLenum, width: GLsizei, height: GLsizei),
    pub glDeleteRenderbuffers: unsafe fn(n: GLsizei, renderbuffers: *const GLuint),
    pub glBindRenderbuffer: unsafe fn(target: GLenum, renderbuffer: GLuint),
    pub glGenFramebuffers: unsafe fn(n: GLsizei, framebuffers: *mut GLuint),
    pub glDeleteFramebuffers: unsafe fn(n: GLsizei, framebuffers: *const GLuint),
    pub glBindFramebuffer: unsafe fn(target: GLenum, framebuffer: GLuint),
    pub glFramebufferRenderbuffer: unsafe fn(target: GLenum, attachment: GLenum, renderbuffertarget: GLenum, renderbuffer: GLuint),
    pub glCheckFramebufferStatus: unsafe fn(target: GLenum) -> GLenum,
    pub glClear: unsafe fn(mask: GLbitfield),
    pub glBlendFunc: unsafe fn(sfactor: GLenum, dfactor: GLenum),
    pub glClearColor: unsafe fn(red: GLfloat, green: GLfloat, blue: GLfloat, alpha: GLfloat),
    pub glFinish: unsafe fn(),

    pub glReadnPixels:
        unsafe fn(x: GLint, y: GLint, width: GLsizei, height: GLsizei, format: GLenum, ty: GLenum, buf_size: GLsizei, data: *mut c_void),

    pub glGenTextures: unsafe fn(n: GLsizei, textures: *mut GLuint),
    pub glDeleteTextures: unsafe fn(n: GLsizei, textures: *const GLuint),
    pub glBindTexture: unsafe fn(target: GLenum, texture: GLuint),
    pub glTexParameteri: unsafe fn(target: GLenum, pname: GLenum, param: GLint),
    pub glPixelStorei: unsafe fn(pname: GLenum, param: GLint),

    pub glTexImage2D: unsafe fn(
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

    pub glEnable: unsafe fn(cap: GLenum),
    pub glDisable: unsafe fn(cap: GLenum),
    pub glViewport: unsafe fn(x: GLint, y: GLint, width: GLsizei, height: GLsizei),

    pub glCreateShader: unsafe fn(ty: GLenum) -> GLuint,
    pub glDeleteShader: unsafe fn(shader: GLuint),
    pub glShaderSource: unsafe fn(shader: GLuint, count: GLsizei, string: *const *const GLchar, length: *const GLint),
    pub glCompileShader: unsafe fn(shader: GLuint),
    pub glGetShaderiv: unsafe fn(shader: GLuint, pname: GLenum, params: *mut GLint),
    pub glCreateProgram: unsafe fn() -> GLuint,
    pub glDeleteProgram: unsafe fn(prog: GLuint),
    pub glAttachShader: unsafe fn(prog: GLuint, shader: GLuint),
    pub glDetachShader: unsafe fn(prog: GLuint, shader: GLuint),
    pub glLinkProgram: unsafe fn(prog: GLuint),
    pub glGetProgramiv: unsafe fn(program: GLuint, pname: GLenum, params: *mut GLint),
    pub glUseProgram: unsafe fn(program: GLuint),
    pub glGetUniformLocation: unsafe fn(prog: GLuint, name: *const GLchar) -> GLint,
    pub glGetAttribLocation: unsafe fn(prog: GLuint, name: *const GLchar) -> GLint,
    pub glUniform1i: unsafe fn(location: GLint, v0: GLint),
    pub glUniform1f: unsafe fn(location: GLint, v0: GLfloat),
    pub glUniform4f: unsafe fn(location: GLint, v0: GLfloat, v1: GLfloat, v2: GLfloat, v3: GLfloat),

    pub glVertexAttribPointer:
        unsafe fn(index: GLuint, size: GLint, ty: GLenum, normalized: GLboolean, stride: GLsizei, pointer: *const u8),

    pub glActiveTexture: unsafe fn(texture: GLuint),
    pub glEnableVertexAttribArray: unsafe fn(idx: GLuint),
    pub glDisableVertexAttribArray: unsafe fn(idx: GLuint),
    pub glDrawArrays: unsafe fn(mode: GLenum, first: GLint, count: GLsizei),
    pub glGenVertexArrays: unsafe fn(n: GLsizei, arrays: *const GLuint),
    pub glBindVertexArray: unsafe fn(array: GLuint),
    pub glGenBuffers: unsafe fn(n: GLsizei, buffers: *const GLuint),
    pub glBindBuffer: unsafe fn(target: GLenum, buffer: GLuint),
    pub glBufferData: unsafe fn(target: GLenum, size: GLsizeiptr, data: *const c_void, usage: GLenum),
    pub glBindAttribLocation: unsafe fn(program: GLuint, index: GLuint, name: *const GLchar),
}

const unsafe fn cast_f<T, S>(t: T) -> S {
    unsafe { std::mem::transmute_copy::<ManuallyDrop<T>, S>(&ManuallyDrop::new(t)) }
}

fn get<T>(lib: &GetEglProcFuncData, name: &CStr) -> anyhow::Result<T> {
    if let Some(f_raw) = (lib.f)(lib.ctx.clone(), BorrowedStrPtr::new(name)) {
        let f = unsafe { cast_f(f_raw) };
        Ok(f)
    } else {
        bail!(format!("{name:?}"))
    }
}

impl OpenGlFuncs {
    pub fn new(lib: &GetEglProcFuncData) -> anyhow::Result<Self> {
        Ok(Self {
            glGetString: get(lib, c"glGetString")?,
            glGenRenderbuffers: get(lib, c"glGenRenderbuffers")?,
            glRenderbufferStorage: get(lib, c"glRenderbufferStorage")?,
            glDeleteRenderbuffers: get(lib, c"glDeleteRenderbuffers")?,
            glBindRenderbuffer: get(lib, c"glBindRenderbuffer")?,
            glGenFramebuffers: get(lib, c"glGenFramebuffers")?,
            glDeleteFramebuffers: get(lib, c"glDeleteFramebuffers")?,
            glBindFramebuffer: get(lib, c"glBindFramebuffer")?,
            glFramebufferRenderbuffer: get(lib, c"glFramebufferRenderbuffer")?,
            glCheckFramebufferStatus: get(lib, c"glCheckFramebufferStatus")?,
            glClear: get(lib, c"glClear")?,
            glBlendFunc: get(lib, c"glBlendFunc")?,
            glClearColor: get(lib, c"glClearColor")?,
            glFinish: get(lib, c"glFinish")?,
            glReadnPixels: get(lib, c"glReadnPixels")?,
            glGenTextures: get(lib, c"glGenTextures")?,
            glDeleteTextures: get(lib, c"glDeleteTextures")?,
            glBindTexture: get(lib, c"glBindTexture")?,
            glTexParameteri: get(lib, c"glTexParameteri")?,
            glPixelStorei: get(lib, c"glPixelStorei")?,
            glTexImage2D: get(lib, c"glTexImage2D")?,
            glEnable: get(lib, c"glEnable")?,
            glDisable: get(lib, c"glDisable")?,
            glViewport: get(lib, c"glViewport")?,
            glCreateShader: get(lib, c"glCreateShader")?,
            glDeleteShader: get(lib, c"glDeleteShader")?,
            glShaderSource: get(lib, c"glShaderSource")?,
            glCompileShader: get(lib, c"glCompileShader")?,
            glGetShaderiv: get(lib, c"glGetShaderiv")?,
            glCreateProgram: get(lib, c"glCreateProgram")?,
            glDeleteProgram: get(lib, c"glDeleteProgram")?,
            glAttachShader: get(lib, c"glAttachShader")?,
            glDetachShader: get(lib, c"glDetachShader")?,
            glLinkProgram: get(lib, c"glLinkProgram")?,
            glGetProgramiv: get(lib, c"glGetProgramiv")?,
            glUseProgram: get(lib, c"glUseProgram")?,
            glGetUniformLocation: get(lib, c"glGetUniformLocation")?,
            glGetAttribLocation: get(lib, c"glGetAttribLocation")?,
            glUniform1i: get(lib, c"glUniform1i")?,
            glUniform1f: get(lib, c"glUniform1f")?,
            glUniform4f: get(lib, c"glUniform4f")?,
            glVertexAttribPointer: get(lib, c"glVertexAttribPointer")?,
            glActiveTexture: get(lib, c"glActiveTexture")?,
            glEnableVertexAttribArray: get(lib, c"glEnableVertexAttribArray")?,
            glDisableVertexAttribArray: get(lib, c"glDisableVertexAttribArray")?,
            glDrawArrays: get(lib, c"glDrawArrays")?,
            glGenVertexArrays: get(lib, c"glGenVertexArrays")?,
            glBindVertexArray: get(lib, c"glBindVertexArray")?,
            glGenBuffers: get(lib, c"glGenBuffers")?,
            glBindBuffer: get(lib, c"glBindBuffer")?,
            glBufferData: get(lib, c"glBufferData")?,
            glBindAttribLocation: get(lib, c"glBindAttribLocation")?,
        })
    }
}
