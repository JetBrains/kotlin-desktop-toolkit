#![allow(non_upper_case_globals)]

use std::{
    ffi::OsString,
    os::windows::ffi::OsStringExt,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use anyhow::{Result, anyhow};
use khronos_egl as egl;
use windows::{
    Win32::{
        Foundation::ERROR_PATH_NOT_FOUND,
        Graphics::Gdi::{GetDC, HDC},
        System::LibraryLoader::GetModuleFileNameW,
    },
    core::Error as WinError,
};

use super::{
    renderer_api::EglSurfaceData,
    renderer_egl_utils::{
        EglInstance, GR_GL_COLOR_BUFFER_BIT, GR_GL_FRAMEBUFFER_BINDING, GR_GL_STENCIL_BUFFER_BIT, GetPlatformDisplayEXTFn, GrGLFunctions,
        PostSubBufferNVFn, get_egl_proc,
    },
    window::Window,
};

/// cbindgen:ignore
const EGL_PLATFORM_ANGLE_ANGLE: egl::Int = 0x3202;
/// cbindgen:ignore
const EGL_PLATFORM_ANGLE_TYPE_ANGLE: egl::Int = 0x3203;
/// cbindgen:ignore
const EGL_PLATFORM_ANGLE_TYPE_D3D11_ANGLE: egl::Int = 0x3208;
/// cbindgen:ignore
const EGL_DIRECT_COMPOSITION_ANGLE: egl::Int = 0x33A5;

pub type AngleDeviceDrawFun = extern "C" fn() -> ();

pub struct AngleDevice {
    egl_instance: EglInstance,
    display: egl::Display,
    context: egl::Context,
    surface: egl::Surface,
    functions: GrGLFunctions,
}

impl AngleDevice {
    #[allow(clippy::items_after_statements)]
    pub fn create_for_window(window: &Window) -> Result<Self> {
        let egl_instance = load_angle_egl_instance()?;

        let hwnd = window.hwnd();

        let hdc = unsafe { GetDC(Some(hwnd)) };
        let display = get_angle_platform_display(&egl_instance, &hdc)?;

        let (_major, _minor) = egl_instance.initialize(display)?;

        const sample_count: egl::Int = 1;
        const sample_buffers: egl::Int = if sample_count > 1 { 1 } else { 0 };
        const egl_sample_count: egl::Int = if sample_count > 1 { sample_count } else { 0 };

        #[rustfmt::skip]
        let config_attribs = [
            // We currently only support ES3.
            egl::RENDERABLE_TYPE, egl::OPENGL_ES3_BIT,
            egl::RED_SIZE, 8,
            egl::GREEN_SIZE, 8,
            egl::BLUE_SIZE, 8,
            egl::ALPHA_SIZE, 8,
            egl::SAMPLE_BUFFERS, sample_buffers,
            egl::SAMPLES, egl_sample_count,
            egl::NONE, egl::NONE,
        ];

        let mut configs = Vec::with_capacity(1);
        egl_instance.choose_config(display, &config_attribs, &mut configs)?;

        let surface_config = configs.pop().ok_or_else(|| anyhow!("No configs were found."))?;

        // We currently only support ES3.
        #[rustfmt::skip]
        let context_attribs = [
            egl::CONTEXT_MAJOR_VERSION, 3,
            egl::CONTEXT_MINOR_VERSION, 0,
            egl::NONE, egl::NONE,
        ];
        let context = egl_instance.create_context(display, surface_config, None, &context_attribs)?;

        #[rustfmt::skip]
        let surface_attribs = [
            EGL_DIRECT_COMPOSITION_ANGLE, egl::TRUE as _,
            egl::NONE, egl::NONE,
        ];
        let surface = unsafe { egl_instance.create_window_surface(display, surface_config, hwnd.0.cast(), Some(&surface_attribs)) }?;

        let functions = GrGLFunctions::init(&egl_instance)?;

        Ok(Self {
            egl_instance,
            display,
            context,
            surface,
            functions,
        })
    }

    pub fn resize_surface(&mut self, width: egl::Int, height: egl::Int) -> Result<EglSurfaceData> {
        self.egl_instance
            .make_current(self.display, Some(self.surface), Some(self.surface), Some(self.context))?;
        self.egl_instance.swap_interval(self.display, 1)?;

        post_sub_buffer(&self.egl_instance, self.display, self.surface, 1, 1, width, height)?;

        (self.functions.fClearStencil)(0);
        (self.functions.fClearColor)(0_f32, 0_f32, 0_f32, 0_f32);
        (self.functions.fStencilMask)(0xffff_ffff);
        (self.functions.fClear)(GR_GL_STENCIL_BUFFER_BIT | GR_GL_COLOR_BUFFER_BIT);
        (self.functions.fViewport)(0, 0, width, height);

        let mut framebuffer_binding = 0;
        (self.functions.fGetIntegerv)(GR_GL_FRAMEBUFFER_BINDING, &raw mut framebuffer_binding);

        Ok(EglSurfaceData { framebuffer_binding })
    }

    #[allow(clippy::bool_to_int_with_if)]
    pub fn draw(&self, wait_for_vsync: bool, draw_fun: AngleDeviceDrawFun) -> Result<()> {
        self.egl_instance
            .make_current(self.display, Some(self.surface), Some(self.surface), Some(self.context))?;

        draw_fun();

        self.egl_instance.swap_interval(self.display, if wait_for_vsync { 1 } else { 0 })?;
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

fn get_angle_platform_display(egl_instance: &EglInstance, hdc: &HDC) -> Result<egl::Display> {
    let fun: GetPlatformDisplayEXTFn = get_egl_proc!(egl_instance, "eglGetPlatformDisplayEXT")?;

    #[rustfmt::skip]
    let display_attribs = [
        EGL_PLATFORM_ANGLE_TYPE_ANGLE, EGL_PLATFORM_ANGLE_TYPE_D3D11_ANGLE,
        egl::NONE, egl::NONE,
    ];

    match fun(EGL_PLATFORM_ANGLE_ANGLE as _, hdc.0, display_attribs.as_ptr()) {
        egl::NO_DISPLAY => Err(egl_instance
            .get_error()
            .map_or_else(|| anyhow!("Could not get ANGLE platform display."), Into::into)),
        display => Ok(unsafe { egl::Display::from_ptr(display) }),
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
) -> Result<()> {
    static FUN: AtomicUsize = AtomicUsize::new(0usize);

    let fun = {
        let temp = FUN.load(Ordering::Relaxed);
        if temp == 0usize {
            #[allow(clippy::transmutes_expressible_as_ptr_casts)]
            FUN.store(get_egl_proc!(egl_instance, "eglPostSubBufferNV")?, Ordering::Release);
            FUN.load(Ordering::Acquire)
        } else {
            temp
        }
    };
    let fun = unsafe { core::mem::transmute::<usize, PostSubBufferNVFn>(fun) };

    match fun(display.as_ptr(), surface.as_ptr(), x, y, width, height) {
        egl::TRUE => Ok(()),
        egl::FALSE => Err(egl_instance
            .get_error()
            .map_or_else(|| anyhow!("Could not post sub buffer."), Into::into)),
        _ => unreachable!("Boolean only has two values"),
    }
}

fn load_angle_egl_instance() -> Result<EglInstance> {
    let current_module_path: PathBuf = unsafe {
        let hmodule = crate::get_dll_instance().into();
        let mut filename = vec![0u16; 1024];
        match GetModuleFileNameW(Some(hmodule), &mut filename) {
            0 => Err(WinError::from_win32()),
            len => Ok(OsString::from_wide(&filename[..len as _]).into()),
        }?
    };
    let current_directory = current_module_path.parent().ok_or_else(|| WinError::from(ERROR_PATH_NOT_FOUND))?;
    let egl_instance = unsafe { EglInstance::load_required_from_filename(current_directory.join("libEGL.dll")) }?;
    Ok(egl_instance)
}
