use std::ffi::c_void;

pub(crate) type StrPtr = *const std::ffi::c_char;
pub (crate) type ArraySize = i64;

#[repr(C)]
pub(crate) struct Array {
    arr: *const c_void,
    len: ArraySize,
}

#[macro_export]
macro_rules! define_ref {
    ($name:ident, $otype:ty) => {
        impl $name {
            pub(crate) fn new(obj: Retained<$otype>) -> Self {
                return Self {
                    ptr: Retained::into_raw(obj) as *mut c_void
                }
            }

            pub(crate) unsafe fn retain(&self) -> Retained<$otype> {
                return Retained::retain(self.ptr as *mut $otype).unwrap()
            }

            pub(crate) unsafe fn consume(self) -> Retained<$otype> {
                return Retained::from_raw(self.ptr as *mut $otype).unwrap()
            }
        }
    };
}