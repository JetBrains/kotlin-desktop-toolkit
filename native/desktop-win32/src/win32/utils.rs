macro_rules! LOWORD {
    ($arg:expr) => {
        (($arg as usize & 0xffff) as u16)
    };
}

macro_rules! HIWORD {
    ($arg:expr) => {
        ((($arg as usize >> 16) & 0xffff) as u16)
    };
}

macro_rules! LOBYTE {
    ($arg:expr) => {
        (($arg as usize & 0xff) as u8)
    };
}

macro_rules! GET_X_LPARAM {
    ($arg:expr) => {
        ((($arg as usize & 0xffff) as i16) as i32)
    };
}

macro_rules! GET_Y_LPARAM {
    ($arg:expr) => {
        (((($arg as usize >> 16) & 0xffff) as i16) as i32)
    };
}

pub(crate) use {GET_X_LPARAM, GET_Y_LPARAM, HIWORD, LOBYTE, LOWORD};

pub(crate) fn is_windows_11_build_22000_or_higher() -> bool {
    unsafe {
        windows::Win32::System::WinRT::Metadata::RoIsApiContractPresent(windows::core::w!("Windows.Foundation.UniversalApiContract"), 14, 0)
    }
    .is_ok_and(windows::core::BOOL::as_bool)
}

pub(crate) fn is_windows_11_build_22621_or_higher() -> bool {
    unsafe {
        windows::Win32::System::WinRT::Metadata::RoIsApiContractPresent(windows::core::w!("Windows.Foundation.UniversalApiContract"), 15, 0)
    }
    .is_ok_and(windows::core::BOOL::as_bool)
}
