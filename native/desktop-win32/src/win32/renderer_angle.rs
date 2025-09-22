#![allow(non_upper_case_globals)]

use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf, sync::atomic::AtomicUsize};

use anyhow::Context;
use khronos_egl as egl;
use windows::{
    UI::Composition::SpriteVisual,
    Win32::{
        Foundation::ERROR_PATH_NOT_FOUND,
        Graphics::Gdi::{GetDC, ReleaseDC},
        System::LibraryLoader::GetModuleFileNameW,
    },
    core::{Error as WinError, Interface},
};
use windows_numerics::Vector2;

use super::{
    renderer_api::EglSurfaceData,
    renderer_egl_utils::{EGLOk, EglInstance, GR_GL_FRAMEBUFFER_BINDING, GrGLFunctions, PostSubBufferNVFn, load_egl_function},
    window::Window,
};

/// cbindgen:ignore
const EGL_PLATFORM_ANGLE_ANGLE: egl::Enum = 0x3202;
/// cbindgen:ignore
const EGL_PLATFORM_ANGLE_TYPE_ANGLE: egl::Attrib = 0x3203;
/// cbindgen:ignore
const EGL_PLATFORM_ANGLE_TYPE_D3D11_ANGLE: egl::Attrib = 0x3208;

pub struct AngleDevice {
    egl_instance: EglInstance,
    display: egl::Display,
    context: egl::Context,
    visual: SpriteVisual,
    surface: egl::Surface,
    functions: GrGLFunctions,
}

impl AngleDevice {
    #[allow(clippy::items_after_statements)]
    pub fn create_for_window(window: &Window) -> anyhow::Result<Self> {
        let egl_instance = load_angle_egl_instance()?;

        let display = {
            #[rustfmt::skip]
            let display_attribs = [
                EGL_PLATFORM_ANGLE_TYPE_ANGLE, EGL_PLATFORM_ANGLE_TYPE_D3D11_ANGLE,
                egl::ATTRIB_NONE, egl::ATTRIB_NONE,
            ];

            let hwnd = window.hwnd();
            let hdc = unsafe { GetDC(Some(hwnd)) };
            let display = unsafe { egl_instance.get_platform_display(EGL_PLATFORM_ANGLE_ANGLE, hdc.0, &display_attribs) }?;
            unsafe { ReleaseDC(Some(hwnd), hdc) };
            display
        };

        let (_major, _minor) = egl_instance.initialize(display)?;

        let surface_config = {
            #[rustfmt::skip]
            let config_attribs = [
                egl::RENDERABLE_TYPE, egl::OPENGL_ES2_BIT,
                egl::RED_SIZE, 8,
                egl::GREEN_SIZE, 8,
                egl::BLUE_SIZE, 8,
                egl::ALPHA_SIZE, 8,
                egl::NONE, egl::NONE,
            ];

            let mut configs = Vec::with_capacity(1);
            egl_instance.choose_config(display, &config_attribs, &mut configs)?;

            configs.pop().context("No configs were found.")?
        };

        let context = {
            #[rustfmt::skip]
            let context_attribs = [
                egl::CONTEXT_MAJOR_VERSION, 2,
                egl::CONTEXT_MINOR_VERSION, 0,
                egl::NONE, egl::NONE,
            ];
            egl_instance.create_context(display, surface_config, None, &context_attribs)?
        };

        let visual = window.get_visual()?;
        let surface = unsafe { egl_instance.create_window_surface(display, surface_config, visual.as_raw(), None) }?;

        let functions = GrGLFunctions::init(&egl_instance)?;

        Ok(Self {
            egl_instance,
            display,
            context,
            visual,
            surface,
            functions,
        })
    }

    pub fn resize_surface(&mut self, width: egl::Int, height: egl::Int) -> anyhow::Result<EglSurfaceData> {
        #[allow(clippy::cast_precision_loss)]
        self.visual.SetSize(Vector2 {
            X: width as f32,
            Y: height as f32,
        })?;

        self.egl_instance.swap_interval(self.display, 1)?;
        post_sub_buffer(&self.egl_instance, self.display, self.surface, 1, 1, width, height)?;

        let mut framebuffer_binding = 0;
        unsafe { (self.functions.fGetIntegerv)(GR_GL_FRAMEBUFFER_BINDING, &raw mut framebuffer_binding) };

        Ok(EglSurfaceData { framebuffer_binding })
    }

    pub fn make_current(&self) -> anyhow::Result<()> {
        self.egl_instance
            .make_current(self.display, Some(self.surface), Some(self.surface), Some(self.context))?;
        Ok(())
    }

    #[allow(clippy::bool_to_int_with_if)]
    pub fn swap_buffers(&self) -> anyhow::Result<()> {
        self.egl_instance.swap_interval(self.display, 1)?;
        self.egl_instance.swap_buffers(self.display, self.surface)?;
        Ok(())
    }

    #[inline]
    #[must_use]
    pub fn get_proc_address(&self, procname: &str) -> Option<extern "system" fn()> {
        self.egl_instance.get_proc_address(procname)
    }
}

impl Drop for AngleDevice {
    fn drop(&mut self) {
        let _ = self.egl_instance.make_current(self.display, None, None, None);
        if self.context.as_ptr() != egl::NO_CONTEXT {
            let _ = self.egl_instance.destroy_context(self.display, self.context);
        }
        if self.surface.as_ptr() != egl::NO_SURFACE {
            let _ = self.egl_instance.destroy_surface(self.display, self.surface);
        }
        if self.display.as_ptr() != egl::NO_DISPLAY {
            let _ = self.egl_instance.terminate(self.display);
        }
    }
}

fn post_sub_buffer(
    egl_instance: &EglInstance,
    display: egl::Display,
    surface: egl::Surface,
    x: egl::Int,
    y: egl::Int,
    width: egl::Int,
    height: egl::Int,
) -> anyhow::Result<()> {
    static POST_SUB_BUFFER_FN: AtomicUsize = AtomicUsize::new(0usize);

    let post_sub_buffer_fn: PostSubBufferNVFn = unsafe { load_egl_function(&POST_SUB_BUFFER_FN, egl_instance, "eglPostSubBufferNV")? };

    unsafe { post_sub_buffer_fn(display.as_ptr(), surface.as_ptr(), x, y, width, height) }
        .ok(egl_instance)
        .context("Could not post sub buffer.")
}

fn load_angle_egl_instance() -> anyhow::Result<EglInstance> {
    let current_module_path: PathBuf = unsafe {
        let hmodule = crate::get_dll_instance().into();
        let mut filename = vec![0u16; 1024];
        match GetModuleFileNameW(Some(hmodule), &mut filename) {
            0 => Err(WinError::from_thread()),
            len => Ok(OsString::from_wide(&filename[..len as _]).into()),
        }?
    };
    let current_directory = current_module_path.parent().ok_or_else(|| WinError::from(ERROR_PATH_NOT_FOUND))?;
    let egl_instance = unsafe { EglInstance::load_required_from_filename(current_directory.join("libEGL.dll")) }?;
    Ok(egl_instance)
}
