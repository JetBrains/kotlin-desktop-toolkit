use std::mem::ManuallyDrop;

use desktop_common::{
    ffi_utils::{BorrowedStrPtr, RustAllocatedRcPtr},
    logger::{PanicDefault, ffi_boundary},
};

use windows::Win32::{
    Foundation::INVALID_HANDLE_VALUE,
    Graphics::Dwm::{DWM_SYSTEMBACKDROP_TYPE, DWMSBT_AUTO, DWMSBT_MAINWINDOW, DWMSBT_NONE, DWMSBT_TABBEDWINDOW, DWMSBT_TRANSIENTWINDOW},
    UI::WindowsAndMessaging::{WINDOW_STYLE, WS_CAPTION, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_OVERLAPPEDWINDOW, WS_THICKFRAME},
};

use super::{
    application::Application,
    application_api::AppPtr,
    geometry::{LogicalPoint, LogicalSize, PhysicalPoint, PhysicalSize},
    window::Window,
};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct WindowId(pub isize);

impl PanicDefault for WindowId {
    fn default() -> Self {
        Self(INVALID_HANDLE_VALUE.0 as isize)
    }
}

pub type WindowPtr<'a> = RustAllocatedRcPtr<'a>;

#[repr(C)]
pub struct WindowParams<'a> {
    pub origin: LogicalPoint,
    pub size: LogicalSize,
    pub title: BorrowedStrPtr<'a>,
    pub style: WindowStyle,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WindowStyle {
    pub title_bar_kind: WindowTitleBarKind,

    pub is_resizable: bool,
    pub is_minimizable: bool,
    pub is_maximizable: bool,

    pub system_backdrop_type: WindowSystemBackdropType,
}

impl WindowStyle {
    #[must_use]
    pub const fn to_system(&self) -> WINDOW_STYLE {
        let mut style = WS_OVERLAPPEDWINDOW.0;
        if matches!(self.title_bar_kind, WindowTitleBarKind::None) {
            style &= !WS_CAPTION.0;
        }
        if !self.is_resizable {
            style &= !WS_THICKFRAME.0;
        }
        if !self.is_minimizable {
            style &= !WS_MINIMIZEBOX.0;
        }
        if !self.is_maximizable {
            style &= !WS_MAXIMIZEBOX.0;
        }
        WINDOW_STYLE(style)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum WindowTitleBarKind {
    System,
    Custom,
    None,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum WindowSystemBackdropType {
    Auto,
    None,
    Mica,
    DesktopAcrylic,
    MicaAlt,
}

impl WindowSystemBackdropType {
    #[must_use]
    pub const fn to_system(&self) -> DWM_SYSTEMBACKDROP_TYPE {
        match self {
            Self::Auto => DWMSBT_AUTO,
            Self::None => DWMSBT_NONE,
            Self::Mica => DWMSBT_MAINWINDOW,
            Self::DesktopAcrylic => DWMSBT_TRANSIENTWINDOW,
            Self::MicaAlt => DWMSBT_TABBEDWINDOW,
        }
    }
}

pub(crate) fn with_window<R: PanicDefault>(window_ptr: &WindowPtr, name: &str, f: impl FnOnce(&Window) -> anyhow::Result<R>) -> R {
    ffi_boundary(name, || {
        let w = ManuallyDrop::new(unsafe { window_ptr.to_rc::<Window>() });
        f(&w)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_create(app_ptr: AppPtr, params: WindowParams) -> WindowPtr<'static> {
    let window = ffi_boundary("window_create", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        let window = app.create_window(&params)?;
        Ok(Some(window))
    });
    WindowPtr::from_rc(window)
}

#[unsafe(no_mangle)]
pub extern "C" fn window_get_window_id(window_ptr: WindowPtr) -> WindowId {
    with_window(&window_ptr, "window_get_window_id", |window| Ok(window.id()))
}

#[unsafe(no_mangle)]
pub extern "C" fn window_get_scale_factor(window_ptr: WindowPtr) -> f32 {
    with_window(&window_ptr, "window_get_scale_factor", |window| Ok(window.get_scale()))
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_min_size(window_ptr: WindowPtr, size: LogicalSize) {
    with_window(&window_ptr, "window_set_min_size", |window| {
        window.set_min_size(size);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_show(window_ptr: WindowPtr) {
    with_window(&window_ptr, "window_show", |window| {
        window.show();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_rect(window_ptr: WindowPtr, origin: PhysicalPoint, size: PhysicalSize) {
    with_window(&window_ptr, "window_set_rect", |window| {
        window.set_position(origin, size)?;
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_request_redraw(window_ptr: WindowPtr) {
    with_window(&window_ptr, "window_request_redraw", |window| {
        window.request_redraw()?;
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_drop(window_ptr: WindowPtr) {
    ffi_boundary("window_drop", || {
        let _window = unsafe { window_ptr.to_rc::<Window>() };
        Ok(())
    });
}
