#![allow(non_upper_case_globals)]

use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use khronos_egl as egl;
use windows::{
    Win32::{
        Foundation::{ERROR_PATH_NOT_FOUND, HWND},
        Graphics::Gdi::{GetDC, HDC},
        System::LibraryLoader::GetModuleFileNameW,
    },
    core::Error as WinError,
};

use super::{
    renderer_api::EglSurfaceData,
    renderer_egl_utils::{
        EglInstance, GR_GL_COLOR_BUFFER_BIT, GR_GL_FRAMEBUFFER_BINDING, GR_GL_STENCIL_BUFFER_BIT, GetPlatformDisplayEXTFn, GrGLFunctions,
        get_egl_proc,
    },
    window::Window,
};

pub type AngleDeviceDrawFun = extern "C" fn() -> ();

pub struct AngleDevice {
    egl_instance: EglInstance,
    window: HWND,
    display: egl::Display,
    context: egl::Context,
    surface: egl::Surface,
    surface_config: egl::Config,
    functions: GrGLFunctions,
}

impl AngleDevice {
    pub fn create_for_window(window: &Window) -> Result<Self> {
        let lib_egl = load_angle_libraries()?;

        let egl_instance = unsafe { EglInstance::load_required_from(lib_egl) }.context("Failed to load ANGLE library from libEGL.dll")?;

        let hdc = unsafe { GetDC(Some(window.hwnd())) };
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
        let functions = GrGLFunctions::init(&egl_instance)?;

        Ok(AngleDevice {
            egl_instance,
            window: window.hwnd(),
            display,
            context,
            surface: unsafe { egl::Surface::from_ptr(egl::NO_SURFACE) },
            surface_config,
            functions,
        })
    }

    pub fn make_surface(&mut self, width: egl::Int, height: egl::Int) -> Result<EglSurfaceData> {
        const EGL_FIXED_SIZE_ANGLE: egl::Int = 0x3201;

        #[rustfmt::skip]
        let surface_attribs = [
            EGL_FIXED_SIZE_ANGLE, egl::TRUE as _,
            egl::WIDTH, width,
            egl::HEIGHT, height,
            egl::NONE, egl::NONE,
        ];

        if self.surface.as_ptr() != egl::NO_SURFACE {
            self.egl_instance.destroy_surface(self.display, self.surface)?;
        }

        self.surface = unsafe {
            self.egl_instance
                .create_window_surface(self.display, self.surface_config, self.window.0 as _, Some(&surface_attribs))
        }?;

        self.egl_instance
            .make_current(self.display, Some(self.surface), Some(self.surface), Some(self.context))?;
        self.egl_instance.swap_interval(self.display, 1)?;

        (self.functions.fClearStencil)(0);
        (self.functions.fClearColor)(0_f32, 0_f32, 0_f32, 0_f32);
        (self.functions.fStencilMask)(0xffffffff);
        (self.functions.fClear)(GR_GL_STENCIL_BUFFER_BIT | GR_GL_COLOR_BUFFER_BIT);
        (self.functions.fViewport)(0, 0, width, height);

        let mut framebuffer_binding = 0;
        (self.functions.fGetIntegerv)(GR_GL_FRAMEBUFFER_BINDING, &mut framebuffer_binding);

        Ok(EglSurfaceData { framebuffer_binding })
    }

    pub fn draw(&self, wait_for_vsync: bool, draw_fun: AngleDeviceDrawFun) -> Result<()> {
        self.egl_instance
            .make_current(self.display, Some(self.surface), Some(self.surface), Some(self.context))?;

        draw_fun();

        self.egl_instance.swap_interval(self.display, if wait_for_vsync { 1 } else { 0 })?;
        self.egl_instance.swap_buffers(self.display, self.surface)?;

        Ok(())
    }

    #[inline]
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
    const EGL_PLATFORM_ANGLE_ANGLE: egl::Int = 0x3202;
    const EGL_PLATFORM_ANGLE_TYPE_ANGLE: egl::Int = 0x3203;

    //const EGL_PLATFORM_ANGLE_TYPE_D3D9_ANGLE: egl::Int = 0x3207;
    const EGL_PLATFORM_ANGLE_TYPE_D3D11_ANGLE: egl::Int = 0x3208;

    let fun: GetPlatformDisplayEXTFn = get_egl_proc!(egl_instance, "eglGetPlatformDisplayEXT")?;

    #[rustfmt::skip]
    let display_attribs = [
        EGL_PLATFORM_ANGLE_TYPE_ANGLE, EGL_PLATFORM_ANGLE_TYPE_D3D11_ANGLE,
        egl::NONE, egl::NONE,
    ];

    match fun(EGL_PLATFORM_ANGLE_ANGLE as _, hdc.0, display_attribs.as_ptr()) {
        egl::NO_DISPLAY => Err(egl_instance
            .get_error()
            .map_or_else(|| anyhow!("Could not get ANGLE platform display."), |err| err.into())),
        display => Ok(unsafe { egl::Display::from_ptr(display) }),
    }
}

fn load_angle_libraries() -> Result<libloading::Library> {
    let current_module_path: PathBuf = unsafe {
        let hmodule = crate::get_dll_instance().into();
        let mut filename = vec![0u16; 1024];
        match GetModuleFileNameW(Some(hmodule), &mut filename) {
            0 => Err(WinError::from_win32()),
            len => Ok(OsString::from_wide(&filename[..len as _]).into()),
        }?
    };
    let current_directory = current_module_path.parent().ok_or_else(|| WinError::from(ERROR_PATH_NOT_FOUND))?;
    let lib_egl = unsafe { libloading::Library::new(current_directory.join("libEGL.dll")) }?;
    Ok(lib_egl)
}
