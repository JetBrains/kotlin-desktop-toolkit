use anyhow::Result;
use khronos_egl as egl;

/// cbindgen:ignore
pub(crate) type EglInstance = egl::DynamicInstance<egl::EGL1_5>;

type GrGLGetIntegervFn = extern "system" fn(pname: egl::Enum, params: *mut egl::Int) -> ();

pub(crate) type PostSubBufferNVFn = extern "system" fn(
    display: egl::EGLDisplay,
    surface: egl::EGLSurface,
    x: egl::Int,
    y: egl::Int,
    width: egl::Int,
    height: egl::Int,
) -> egl::Boolean;

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
    pub fGetIntegerv: GrGLGetIntegervFn,
}

impl GrGLFunctions {
    pub fn init(egl_instance: &EglInstance) -> Result<Self> {
        Ok(Self {
            fGetIntegerv: get_egl_proc!(egl_instance, "glGetIntegerv")?,
        })
    }
}
