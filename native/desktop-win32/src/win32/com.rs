use desktop_common::logger::PanicDefault;

use windows::Win32::Foundation::E_POINTER;
use windows_core::{IUnknown, Interface, Result as WinResult};

#[repr(transparent)]
pub struct ComInterfaceRawPtr {
    ptr: *mut core::ffi::c_void,
}

impl ComInterfaceRawPtr {
    pub fn new<T: Interface>(com_object: &T) -> WinResult<Self> {
        let unknown = com_object.cast::<IUnknown>()?;
        Ok(Self { ptr: unknown.into_raw() })
    }

    pub fn borrow<T: Interface>(&self) -> WinResult<T> {
        let unknown = unsafe { IUnknown::from_raw_borrowed(&self.ptr) }.ok_or(E_POINTER)?;
        unknown.cast()
    }
}

impl Drop for ComInterfaceRawPtr {
    fn drop(&mut self) {
        let _ = unsafe { IUnknown::from_raw(self.ptr) };
        self.ptr = core::ptr::null_mut();
    }
}

impl PanicDefault for ComInterfaceRawPtr {
    fn default() -> Self {
        Self {
            ptr: core::ptr::null_mut(),
        }
    }
}
