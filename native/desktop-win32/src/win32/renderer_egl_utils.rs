use anyhow::Result;
use khronos_egl as egl;

/// cbindgen:ignore
pub(crate) type EglInstance = egl::DynamicInstance<egl::EGL1_5>;

type GrGLclampf = core::ffi::c_float;
type GrGLbitfield = core::ffi::c_uint;
type GrGLuint = core::ffi::c_uint;
type GrGLsizei = core::ffi::c_int;

type GrGLClearFn = extern "system" fn(mask: GrGLbitfield) -> ();
type GrGLClearColorFn = extern "system" fn(red: GrGLclampf, green: GrGLclampf, blue: GrGLclampf, alpha: GrGLclampf) -> ();
type GrGLClearStencilFn = extern "system" fn(s: egl::Int) -> ();
type GrGLGetIntegervFn = extern "system" fn(pname: egl::Enum, params: *mut egl::Int) -> ();
type GrGLStencilMaskFn = extern "system" fn(mask: GrGLuint) -> ();
type GrGLViewportFn = extern "system" fn(x: egl::Int, y: egl::Int, width: GrGLsizei, height: GrGLsizei) -> ();

pub(crate) type GetPlatformDisplayEXTFn =
    extern "system" fn(platform: egl::Enum, native_display: *mut std::ffi::c_void, attrib_list: *const egl::Int) -> egl::EGLDisplay;

pub(crate) type PostSubBufferNVFn = extern "system" fn(
    display: egl::EGLDisplay,
    surface: egl::EGLSurface,
    x: egl::Int,
    y: egl::Int,
    width: egl::Int,
    height: egl::Int,
) -> egl::Boolean;

/// cbindgen:ignore
pub(crate) const GR_GL_STENCIL_BUFFER_BIT: GrGLbitfield = 0x0000_0400;

/// cbindgen:ignore
pub(crate) const GR_GL_COLOR_BUFFER_BIT: GrGLbitfield = 0x0000_4000;

/// cbindgen:ignore
pub(crate) const GR_GL_FRAMEBUFFER_BINDING: egl::Enum = 0x8CA6;

macro_rules! get_egl_proc {
    ($egl_instance:ident, $name:literal) => {
        $egl_instance
            .get_proc_address($name)
            .ok_or_else(|| anyhow::anyhow!("Could not load the {} function.", $name))
            .map(|f| unsafe { core::mem::transmute(f) })
    };
}
pub(crate) use get_egl_proc;

#[allow(non_snake_case)]
pub(crate) struct GrGLFunctions {
    pub fClear: GrGLClearFn,
    pub fClearColor: GrGLClearColorFn,
    pub fClearStencil: GrGLClearStencilFn,
    pub fGetIntegerv: GrGLGetIntegervFn,
    pub fStencilMask: GrGLStencilMaskFn,
    pub fViewport: GrGLViewportFn,
}

impl GrGLFunctions {
    pub fn init(egl_instance: &EglInstance) -> Result<Self> {
        Ok(Self {
            fClear: get_egl_proc!(egl_instance, "glClear")?,
            fClearColor: get_egl_proc!(egl_instance, "glClearColor")?,
            fClearStencil: get_egl_proc!(egl_instance, "glClearStencil")?,
            fGetIntegerv: get_egl_proc!(egl_instance, "glGetIntegerv")?,
            fStencilMask: get_egl_proc!(egl_instance, "glStencilMask")?,
            fViewport: get_egl_proc!(egl_instance, "glViewport")?,
        })
    }
}
