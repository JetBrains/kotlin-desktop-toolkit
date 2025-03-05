use core::slice;
use std::{
    ffi::{CStr, CString, NulError},
    marker::PhantomData,
    ptr::NonNull,
};

use anyhow::Context;
use log::warn;

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
struct GenericRawPtr<'a, T> {
    ptr: *const T,
    phantom: PhantomData<&'a T>,
}

#[repr(transparent)]
pub struct RustAllocatedRawPtr<'a, T>(GenericRawPtr<'a, T>);

impl<T> RustAllocatedRawPtr<'_, T> {
    pub(crate) fn from_value<R>(value: Option<R>) -> Self {
        Self(GenericRawPtr {
            ptr: value.map_or(std::ptr::null(), |v| Box::into_raw(Box::new(v)).cast_const()).cast(),
            phantom: PhantomData,
        })
    }

    #[allow(clippy::unnecessary_box_returns)]
    pub(crate) unsafe fn to_owned<R>(&self) -> Box<R> {
        assert!(!self.0.ptr.is_null());
        let ptr = self.0.ptr.cast_mut().cast::<R>();
        unsafe { Box::from_raw(ptr) }
    }

    pub(crate) unsafe fn borrow<R>(&self) -> &R {
        Box::leak(unsafe { self.to_owned() })
    }

    pub(crate) unsafe fn borrow_mut<R>(&mut self) -> &mut R {
        Box::leak(unsafe { self.to_owned() })
    }
}

#[repr(transparent)]
pub struct BorrowedStrPtr<'a>(GenericRawPtr<'a, std::ffi::c_char>);

impl BorrowedStrPtr<'_> {
    pub(crate) const fn new(s: &CStr) -> Self {
        Self(GenericRawPtr {
            ptr: s.as_ptr(),
            phantom: PhantomData,
        })
    }

    pub(crate) const fn as_non_null(&self) -> Option<NonNull<std::ffi::c_char>> {
        NonNull::new(self.0.ptr.cast_mut())
    }

    pub(crate) fn as_str(&self) -> anyhow::Result<&str> {
        assert!(!self.0.ptr.is_null());
        let c_str = unsafe { CStr::from_ptr(self.0.ptr) };
        c_str.to_str().with_context(|| format!("Invalid unicode in {c_str:?}"))
    }
}

impl<'a> std::fmt::Debug for BorrowedStrPtr<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct RustAllocatedStrPtr(GenericRawPtr<'static, std::ffi::c_char>);

impl RustAllocatedStrPtr {
    pub(crate) const fn null() -> Self {
        Self(GenericRawPtr {
            ptr: std::ptr::null(),
            phantom: PhantomData,
        })
    }

    pub(crate) fn allocate(data: &[u8]) -> Result<Self, NulError> {
        Ok(Self(GenericRawPtr {
            ptr: CString::new(data)?.into_raw(),
            phantom: PhantomData,
        }))
    }

    pub(crate) fn deallocate(&mut self) {
        assert!(!self.0.ptr.is_null());
        let _s = unsafe { CString::from_raw(self.0.ptr.cast_mut()) };
        self.0.ptr = std::ptr::null();
    }

    pub(crate) const fn to_auto_drop(self) -> AutoDropStrPtr {
        AutoDropStrPtr(self)
    }
}

#[repr(transparent)]
pub struct AutoDropStrPtr(RustAllocatedStrPtr);

impl Drop for AutoDropStrPtr {
    fn drop(&mut self) {
        self.0.deallocate();
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
