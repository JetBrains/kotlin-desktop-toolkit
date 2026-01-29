use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Context;
use khronos_egl as egl;

/// cbindgen:ignore
pub(crate) type EglInstance = egl::DynamicInstance<egl::EGL1_5>;

/// cbindgen:ignore
pub(crate) type GrGLsizei = core::ffi::c_int;

/// cbindgen:ignore
pub(crate) type GrGLGetIntegervFn = unsafe extern "system" fn(pname: egl::Enum, params: *mut egl::Int) -> ();
/// cbindgen:ignore
pub(crate) type GrGLViewportFn = unsafe extern "system" fn(x: egl::Int, y: egl::Int, width: GrGLsizei, height: GrGLsizei) -> ();
/// cbindgen:ignore
pub(crate) type GrGLFinishFn = unsafe extern "system" fn() -> ();

/// cbindgen:ignore
pub(crate) type PostSubBufferNVFn = unsafe extern "system" fn(
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
            .with_context(|| format!("Could not load the {} function.", $name))
            .map(|f| unsafe { core::mem::transmute(f) })
    };
}
pub(crate) use get_egl_proc;

#[allow(non_snake_case)]
pub(crate) struct GrGLFunctions {
    pub fGetIntegerv: GrGLGetIntegervFn,
    pub fViewport: GrGLViewportFn,
    pub fFinish: GrGLFinishFn,
}

impl GrGLFunctions {
    pub fn init(egl_instance: &EglInstance) -> anyhow::Result<Self> {
        Ok(Self {
            fGetIntegerv: get_egl_proc!(egl_instance, "glGetIntegerv")?,
            fViewport: get_egl_proc!(egl_instance, "glViewport")?,
            fFinish: get_egl_proc!(egl_instance, "glFinish")?,
        })
    }
}

pub(crate) unsafe fn load_egl_function<T>(
    cache: &'static AtomicUsize,
    egl_instance: &EglInstance,
    name: &'static str,
) -> anyhow::Result<T> {
    let egl_func = {
        let cached_value = cache.load(Ordering::Relaxed);
        if cached_value == 0usize {
            let f = egl_instance
                .get_proc_address(name)
                .with_context(|| format!("Could not get address of the \"{name}\" function."))?;
            cache.store(f as usize, Ordering::Release);
            cache.load(Ordering::Acquire)
        } else {
            cached_value
        }
    };
    Ok(unsafe { core::mem::transmute_copy::<usize, T>(&egl_func) })
}

pub(crate) trait EGLOk {
    fn ok(&self, egl_instance: &EglInstance) -> anyhow::Result<()>;
}

impl EGLOk for egl::Boolean {
    fn ok(&self, egl_instance: &EglInstance) -> anyhow::Result<()> {
        match *self {
            egl::TRUE => Ok(()),
            egl::FALSE => Err(egl_instance.get_error().map_or_else(|| anyhow::Error::msg("EGL_FALSE"), Into::into)),
            _ => unreachable!("Boolean only has two values"),
        }
    }
}
