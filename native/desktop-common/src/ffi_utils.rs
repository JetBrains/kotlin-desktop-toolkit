#![allow(clippy::missing_safety_doc)]

use core::slice;
use std::{
    ffi::{CStr, CString, NulError},
    fmt::Write,
    marker::PhantomData,
    ptr::NonNull,
};

use anyhow::{Context, bail};
use log::debug;

#[derive(Debug, Copy)]
#[repr(transparent)]
struct GenericRawPtr<'a, T> {
    ptr: *const T,
    phantom: PhantomData<&'a T>,
}

impl<T> Clone for GenericRawPtr<'_, T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            phantom: PhantomData,
        }
    }
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct BorrowedOpaquePtr<'a>(GenericRawPtr<'a, std::ffi::c_void>);

impl<'a> BorrowedOpaquePtr<'a> {
    #[must_use]
    pub const fn new<T>(v: Option<&T>) -> Self {
        Self(GenericRawPtr {
            ptr: if let Some(r) = v {
                let ptr: *const T = r;
                ptr.cast()
            } else {
                std::ptr::null()
            },
            phantom: PhantomData,
        })
    }

    #[must_use]
    pub const unsafe fn borrow<R>(&self) -> Option<&'a R> {
        let p: *const R = self.0.ptr.cast();
        unsafe { p.as_ref() }
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct RustAllocatedRawPtr<'a>(GenericRawPtr<'a, std::ffi::c_void>);

impl Clone for RustAllocatedRawPtr<'_> {
    fn clone(&self) -> Self {
        Self(GenericRawPtr {
            ptr: self.0.ptr,
            phantom: PhantomData,
        })
    }
}

impl RustAllocatedRawPtr<'_> {
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
    pub fn new_optional(s: Option<&'a CString>) -> Self {
        if let Some(s) = s {
            BorrowedStrPtr::new(s.as_c_str())
        } else {
            Self(GenericRawPtr {
                ptr: std::ptr::null(),
                phantom: PhantomData,
            })
        }
    }

    #[must_use]
    pub const fn as_non_null(&self) -> Option<NonNull<std::ffi::c_char>> {
        NonNull::new(self.0.ptr.cast_mut())
    }

    pub fn as_str(&self) -> anyhow::Result<&str> {
        self.as_optional_str().transpose().expect("BorrowedStrPtr has null pointer")
    }

    pub const fn as_optional_cstr(&self) -> anyhow::Result<Option<&CStr>> {
        if self.0.ptr.is_null() {
            return Ok(None);
        }
        Ok(Some(unsafe { CStr::from_ptr(self.0.ptr) }))
    }

    pub fn as_optional_str(&self) -> anyhow::Result<Option<&str>> {
        self.as_optional_cstr()?
            .map(|cstr| cstr.to_str().with_context(|| format!("Invalid UTF-8 in {cstr:?}")))
            .transpose()
    }
}

impl std::fmt::Debug for BorrowedStrPtr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.as_optional_str() {
            Ok(None) => write!(f, "null"),
            Ok(Some(s)) => {
                f.write_char('"')?;
                for c in s.chars() {
                    if c.is_ascii() && !c.is_ascii_control() && !c.is_ascii_whitespace() {
                        f.write_char(c)?;
                    } else {
                        write!(f, "{}", c.escape_unicode())?;
                    }
                }
                f.write_char('"')?;
                Ok(())
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

    pub fn allocate<T: Into<Vec<u8>>>(data: T) -> Result<Self, NulError> {
        Ok(Self(GenericRawPtr {
            ptr: CString::new(data)?.into_raw(),
            phantom: PhantomData,
        }))
    }

    pub fn deallocate(&mut self) {
        if !self.0.ptr.is_null() {
            let _s = unsafe { CString::from_raw(self.0.ptr.cast_mut()) };
            self.0.ptr = std::ptr::null();
        }
    }

    #[must_use]
    pub const fn to_auto_drop(self) -> AutoDropStrPtr {
        AutoDropStrPtr(self)
    }

    pub fn as_str(&self) -> anyhow::Result<&str> {
        assert!(!self.0.ptr.is_null());
        let c_str = unsafe { CStr::from_ptr(self.0.ptr) };
        c_str.to_str().with_context(|| format!("Invalid unicode in {c_str:?}"))
    }
}

#[repr(transparent)]
pub struct AutoDropStrPtr(RustAllocatedStrPtr);

impl AutoDropStrPtr {
    #[must_use]
    pub const fn borrow(&self) -> BorrowedStrPtr {
        BorrowedStrPtr(self.0.0)
    }
}

impl Drop for AutoDropStrPtr {
    fn drop(&mut self) {
        self.0.deallocate();
    }
}

pub type ArraySize = usize;

#[repr(C)]
#[derive(Debug)]
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

    #[must_use]
    pub const fn null() -> Self {
        Self {
            ptr: std::ptr::null(),
            len: 0,
        }
    }
}

impl<T> Drop for AutoDropArray<T> {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
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
    ptr: *const T,
    len: ArraySize,
    pub deinit: Option<extern "C" fn(*const T, ArraySize)>,
    phantom: PhantomData<&'a T>,
}

impl<'a, T: std::fmt::Debug> BorrowedArray<'a, T> {
    pub fn from_slice(s: &'a [T]) -> Self {
        debug!("BorrowedArray::from_slice: {s:?}");
        Self {
            ptr: s.as_ptr(),
            len: s.len(),
            deinit: None,
            phantom: PhantomData,
        }
    }

    pub fn deinit(&self) {
        if let Some(d) = self.deinit {
            d(self.ptr, self.len);
        }
    }

    #[must_use]
    pub fn null() -> Self {
        Self {
            ptr: std::ptr::null(),
            len: 0,
            deinit: None,
            phantom: PhantomData,
        }
    }

    pub fn as_slice(&'a self) -> anyhow::Result<&'a [T]> {
        if self.ptr.is_null() {
            bail!("Null pointer!")
        }
        let slice = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        Ok(slice)
    }
}
