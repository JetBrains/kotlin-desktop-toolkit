use core::slice;
use std::ffi::CString;

use log::warn;
pub type StrPtr = *mut std::ffi::c_char;
pub type ConstStrPtr = *const std::ffi::c_char;

#[repr(transparent)]
pub struct AutoDropStrPtr(pub(crate) *const std::ffi::c_char);

impl Drop for AutoDropStrPtr {
    fn drop(&mut self) {
        let _s = unsafe { CString::from_raw(self.0.cast_mut()) };
    }
}

pub type ArraySize = usize;

#[repr(C)]
pub struct AutoDropArray<T> {
    pub ptr: *const T,
    pub len: ArraySize,
}

impl<T> AutoDropArray<T> {
    pub(crate) fn new(array: Box<[T]>) -> Self {
        let array = Box::leak(array);
        Self {
            ptr: array.as_ptr(),
            len: array.len(),
        }
    }
}

impl<T> Drop for AutoDropArray<T> {
    fn drop(&mut self) {
        if self.ptr.is_null() {
            warn!("Got null pointer in AutoDropArray");
        } else {
            let array = unsafe {
                let s = slice::from_raw_parts_mut(self.ptr.cast_mut(), self.len);
                Box::from_raw(s)
            };
            std::mem::drop(array);
        }
    }
}

// ffi ready analog of &[T]
//#[repr(C)]
//pub struct Array<T> {
//    pub arr: *mut T,
//    pub len: ArraySize,
//}

pub type PhysicalPixels = f64;
pub type LogicalPixels = f64;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PhysicalSize {
    pub width: PhysicalPixels,
    pub height: PhysicalPixels,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PhysicalPoint {
    pub x: PhysicalPixels,
    pub y: PhysicalPixels,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LogicalSize {
    pub width: LogicalPixels,
    pub height: LogicalPixels,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LogicalPoint {
    pub x: LogicalPixels,
    pub y: LogicalPixels,
}

#[derive(Debug)]
pub struct LogicalRect {
    // the point closest to coordinates origin
    pub(crate) origin: LogicalPoint,
    pub(crate) size: LogicalSize,
}

impl LogicalRect {
    pub(crate) const fn new(origin: LogicalPoint, size: LogicalSize) -> Self {
        Self { origin, size }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

#[macro_export]
macro_rules! define_objc_ref {
    ($name:ident, $otype:ty) => {
        #[allow(dead_code)]
        impl $name {
            pub(crate) fn new(obj: Retained<$otype>) -> Self {
                return Self {
                    ptr: Retained::into_raw(obj).cast::<c_void>(),
                };
            }

            pub(crate) unsafe fn retain(&self) -> Retained<$otype> {
                return unsafe { Retained::retain(self.ptr.cast::<$otype>()) }.unwrap();
            }

            pub(crate) unsafe fn consume(self) -> Retained<$otype> {
                return unsafe { Retained::from_raw(self.ptr.cast::<$otype>()) }.unwrap();
            }
        }
    };
}

//#[repr(transparent)]
//pub struct SomeRef<T> {
//    ptr: *mut c_void,
//    p: PhantomData<T>
//}
//
//impl <T> SomeRef<T> {
//    fn new(obj: Arc<T>) -> Self {
//        SomeRef {
//            ptr: Arc::into_raw(obj) as *mut c_void,
//            p: PhantomData,
//        }
//    }
//
//    unsafe fn retain(&self) -> Arc<T> { // never be unique => no &mut
//        let arc = Arc::from_raw(self.ptr as *mut T);
//        let local_copy = arc.clone();
//        let _ = Arc::into_raw(arc);
//        local_copy
//    }
//
//    unsafe fn consume(self) -> Arc<T> {
//        Arc::from_raw(self.ptr as *mut T)
//    }
//}
