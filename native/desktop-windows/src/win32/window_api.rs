use desktop_common::{
    ffi_utils::{BorrowedStrPtr, RustAllocatedRawPtr},
    logger::{PanicDefault, ffi_boundary},
};
use log::debug;
use windows::Win32::{
    Foundation::{HWND, INVALID_HANDLE_VALUE},
    Graphics::Dwm::{DWM_SYSTEMBACKDROP_TYPE, DWMSBT_AUTO, DWMSBT_MAINWINDOW, DWMSBT_NONE, DWMSBT_TABBEDWINDOW, DWMSBT_TRANSIENTWINDOW},
};

use super::{
    application::Application,
    application_api::AppPtr,
    geometry::{PhysicalPoint, PhysicalSize},
    window::Window,
};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct WindowId(pub isize);

impl PanicDefault for WindowId {
    fn default() -> Self {
        WindowId(INVALID_HANDLE_VALUE.0 as isize)
    }
}

impl From<WindowId> for HWND {
    fn from(value: WindowId) -> Self {
        HWND(value.0 as _)
    }
}

impl From<HWND> for WindowId {
    fn from(value: HWND) -> Self {
        WindowId(value.0 as _)
    }
}

pub type WindowPtr<'a> = RustAllocatedRawPtr<'a>;

#[repr(C)]
pub struct WindowParams<'a> {
    pub origin: PhysicalPoint,
    pub size: PhysicalSize,
    pub title: BorrowedStrPtr<'a>,

    pub is_resizable: bool,
    pub is_closable: bool,
    pub is_minimizable: bool,
}

#[repr(C)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum WindowSystemBackdropType {
    Auto,
    None,
    Mica,
    DesktopAcrylic,
    MicaAlt,
}

impl WindowSystemBackdropType {
    pub const fn to_system(&self) -> DWM_SYSTEMBACKDROP_TYPE {
        match self {
            WindowSystemBackdropType::Auto => DWMSBT_AUTO,
            WindowSystemBackdropType::None => DWMSBT_NONE,
            WindowSystemBackdropType::Mica => DWMSBT_MAINWINDOW,
            WindowSystemBackdropType::DesktopAcrylic => DWMSBT_TRANSIENTWINDOW,
            WindowSystemBackdropType::MicaAlt => DWMSBT_TABBEDWINDOW,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn window_create(app_ptr: AppPtr, params: WindowParams) -> WindowPtr<'static> {
    let window = ffi_boundary("window_create", || {
        debug!("window_create");

        let app = unsafe { app_ptr.borrow::<Application>() };
        let window = Window::new(&params, app)?;

        Ok(Some(window))
    });
    WindowPtr::from_value(window)
}

#[unsafe(no_mangle)]
pub extern "C" fn window_get_window_id(window_ptr: WindowPtr) -> WindowId {
    ffi_boundary("window_get_window_id", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        Ok(window.id())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_extend_content_into_titlebar(window_ptr: WindowPtr) {
    ffi_boundary("window_extend_content_into_titlebar", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        Ok(window.extend_content_into_titlebar()?)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_apply_system_backdrop(window_ptr: WindowPtr, backdrop_type: WindowSystemBackdropType) {
    ffi_boundary("window_apply_system_backdrop", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        Ok(window.apply_system_backdrop(backdrop_type)?)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_show(window_ptr: WindowPtr) {
    ffi_boundary("window_show", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        window.show();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_drop(window_ptr: WindowPtr) {
    ffi_boundary("window_drop", || {
        let _window = unsafe { window_ptr.to_owned::<Window>() };
        Ok(())
    });
}
