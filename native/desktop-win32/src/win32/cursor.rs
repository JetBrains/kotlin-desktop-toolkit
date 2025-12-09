use std::path::Path;

use windows::{
    Win32::UI::WindowsAndMessaging::{
        HCURSOR, IDC_APPSTARTING, IDC_ARROW, IDC_CROSS, IDC_HAND, IDC_HELP, IDC_IBEAM, IDC_NO, IDC_PERSON, IDC_PIN, IDC_SIZEALL,
        IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, IDC_UPARROW, IDC_WAIT, IMAGE_CURSOR, LR_DEFAULTSIZE, LR_LOADFROMFILE,
        LR_SHARED, LoadImageW,
    },
    core::{Free, HSTRING, PCWSTR, Result as WinResult},
};

// see https://learn.microsoft.com/en-us/windows/win32/menurc/about-cursors
#[repr(C)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum CursorIcon {
    Unknown,

    Arrow,
    IBeam,
    Wait,
    Crosshair,
    UpArrow,

    SizeNWSE,
    SizeNESW,
    SizeWE,
    SizeNS,

    SizeAll,
    NotAllowed,
    Hand,
    AppStarting,
    Help,
    Pin,
    Person,
}

impl CursorIcon {
    pub(crate) fn to_native(self) -> PCWSTR {
        match self {
            Self::Arrow => IDC_ARROW,
            Self::IBeam => IDC_IBEAM,
            Self::Wait => IDC_WAIT,
            Self::Crosshair => IDC_CROSS,
            Self::UpArrow => IDC_UPARROW,
            Self::SizeNWSE => IDC_SIZENWSE,
            Self::SizeNESW => IDC_SIZENESW,
            Self::SizeWE => IDC_SIZEWE,
            Self::SizeNS => IDC_SIZENS,
            Self::SizeAll => IDC_SIZEALL,
            Self::NotAllowed => IDC_NO,
            Self::Hand => IDC_HAND,
            Self::AppStarting => IDC_APPSTARTING,
            Self::Help => IDC_HELP,
            Self::Pin => IDC_PIN,
            Self::Person => IDC_PERSON,
            Self::Unknown => panic!("Can't create Unknown cursor"),
        }
    }
}

pub struct Cursor {
    handle: HCURSOR,
    is_system: bool,
}

impl Cursor {
    pub(crate) fn load_from_system(cursor_icon: CursorIcon) -> WinResult<Self> {
        let cursor_resource = cursor_icon.to_native();
        unsafe { LoadImageW(None, cursor_resource, IMAGE_CURSOR, 0, 0, LR_DEFAULTSIZE | LR_SHARED) }.map(|handle| Self {
            handle: HCURSOR(handle.0),
            is_system: true,
        })
    }

    pub(crate) fn load_from_file<T: AsRef<Path>>(file_path: T) -> WinResult<Self> {
        let path_str = HSTRING::from(file_path.as_ref());
        unsafe { LoadImageW(None, &path_str, IMAGE_CURSOR, 0, 0, LR_DEFAULTSIZE | LR_LOADFROMFILE) }.map(|handle| Self {
            handle: HCURSOR(handle.0),
            is_system: false,
        })
    }

    pub(crate) const fn as_native(&self) -> HCURSOR {
        self.handle
    }
}

impl Drop for Cursor {
    fn drop(&mut self) {
        if !self.is_system {
            unsafe { self.handle.free() };
        }
    }
}
