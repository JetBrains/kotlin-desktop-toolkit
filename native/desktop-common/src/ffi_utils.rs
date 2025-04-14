#![allow(clippy::missing_safety_doc)]

use core::slice;
use std::{
    ffi::{CStr, CString, NulError},
    marker::PhantomData,
    ptr::NonNull,
};

use anyhow::{Context, bail};
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
    pub fn from_value<R>(value: Option<R>) -> Self {
        Self(GenericRawPtr {
            ptr: value.map_or(std::ptr::null(), |v| Box::into_raw(Box::new(v)).cast_const()).cast(),
            phantom: PhantomData,
        })
    }

    #[allow(clippy::unnecessary_box_returns)]
    #[must_use]
    pub unsafe fn to_owned<R>(&self) -> Box<R> {
        assert!(!self.0.ptr.is_null());
        let ptr = self.0.ptr.cast_mut().cast::<R>();
        unsafe { Box::from_raw(ptr) }
    }

    #[must_use]
    pub unsafe fn borrow<R>(&self) -> &R {
        Box::leak(unsafe { self.to_owned() })
    }

    #[must_use]
    pub unsafe fn borrow_mut<R>(&mut self) -> &mut R {
        Box::leak(unsafe { self.to_owned() })
    }
}

#[repr(transparent)]
pub struct BorrowedStrPtr<'a>(GenericRawPtr<'a, std::ffi::c_char>);

impl<'a> BorrowedStrPtr<'a> {
    #[must_use]
    pub const fn new(s: &'a CStr) -> Self {
        Self(GenericRawPtr {
            ptr: s.as_ptr(),
            phantom: PhantomData,
        })
    }

    #[must_use]
    pub const fn as_non_null(&self) -> Option<NonNull<std::ffi::c_char>> {
        NonNull::new(self.0.ptr.cast_mut())
    }

    #[must_use]
    pub const fn is_not_null(&self) -> bool {
        !self.0.ptr.is_null()
    }

    pub fn as_str(&self) -> anyhow::Result<&str> {
        assert!(!self.0.ptr.is_null());
        let c_str = unsafe { CStr::from_ptr(self.0.ptr) };
        c_str.to_str().with_context(|| format!("Invalid unicode in {c_str:?}"))
    }
}

impl std::fmt::Debug for BorrowedStrPtr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.as_str() {
            Ok(s) => {
                if s.is_ascii() {
                    f.write_str(s)
                } else {
                    write!(f, "{}", s.escape_unicode())
                }
            }
            Err(e) => write!(f, "{e}"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct RustAllocatedStrPtr(GenericRawPtr<'static, std::ffi::c_char>);

impl RustAllocatedStrPtr {
    #[must_use]
    pub const fn null() -> Self {
        Self(GenericRawPtr {
            ptr: std::ptr::null(),
            phantom: PhantomData,
        })
    }

    pub fn allocate(data: &[u8]) -> Result<Self, NulError> {
        Ok(Self(GenericRawPtr {
            ptr: CString::new(data)?.into_raw(),
            phantom: PhantomData,
        }))
    }

    pub fn deallocate(&mut self) {
        assert!(!self.0.ptr.is_null());
        let _s = unsafe { CString::from_raw(self.0.ptr.cast_mut()) };
        self.0.ptr = std::ptr::null();
    }

    #[must_use]
    pub const fn to_auto_drop(self) -> AutoDropStrPtr {
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
    #[must_use]
    pub fn new(array: Box<[T]>) -> Self {
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

#[repr(C)]
#[derive(Debug)]
pub struct BorrowedArray<'a, T> {
    pub ptr: *const T,
    pub len: ArraySize,
    phantom: PhantomData<&'a T>,
}

impl<'a, T> BorrowedArray<'a, T> {
    pub fn as_slice(&'a self) -> anyhow::Result<&'a [T]> {
        if self.ptr.is_null() {
            bail!("Null pointer!")
        }
        let slice = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        Ok(slice)
    }
}
