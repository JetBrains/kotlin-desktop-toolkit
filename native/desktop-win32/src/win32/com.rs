//! FFI handle for COM interfaces shared across the Java/Kotlin boundary.
//!
//! Each [`ComInterfaceRawPtr`] owns one reference on a COM interface that
//! travels by value across the FFI: Kotlin stores the raw bits and
//! reconstructs a wrapper for every native call.
//!
//! # Lifecycle
//!
//! 1. [`ComInterfaceRawPtr::from_interface`] or
//!    [`ComInterfaceRawPtr::from_object`] takes ownership of one reference.
//! 2. [`ComInterfaceRawPtr::cast`] reads the handle as a typed interface.
//! 3. [`ComInterfaceRawPtr::release`] releases the owned reference.

use desktop_common::logger::PanicDefault;

use windows::Win32::Foundation::E_POINTER;
use windows_core::{ComObject, ComObjectInner, ComObjectInterface, IUnknown, Interface, Result as WinResult};

/// FFI-safe handle to a COM interface that owns one reference.
#[repr(transparent)]
#[must_use = "ComInterfaceRawPtr owns a COM reference; pass to `release` or hand back across the FFI boundary"]
pub struct ComInterfaceRawPtr {
    ptr: *mut core::ffi::c_void,
}

impl ComInterfaceRawPtr {
    /// Takes ownership of one reference from `com_interface`.
    ///
    /// # Errors
    /// Propagates `QueryInterface` failure.
    pub fn from_interface<T: Interface>(com_interface: &T) -> WinResult<Self> {
        let unknown = com_interface.cast::<IUnknown>()?;
        Ok(Self { ptr: unknown.into_raw() })
    }

    /// Takes ownership of one reference from `com_object`.
    ///
    /// # Errors
    /// Propagates `QueryInterface` failure.
    pub fn from_object<T>(com_object: &ComObject<T>) -> WinResult<Self>
    where
        T: ComObjectInner,
        <T as ComObjectInner>::Outer: ComObjectInterface<IUnknown>,
    {
        let unknown = com_object.cast::<IUnknown>()?;
        Ok(Self { ptr: unknown.into_raw() })
    }

    /// Returns the handle as interface `T` via `QueryInterface`.
    ///
    /// # Errors
    /// Returns `E_POINTER` for a null handle; propagates `QueryInterface`
    /// failure for incompatible `T`.
    pub fn cast<T: Interface>(&self) -> WinResult<T> {
        // SAFETY: `from_raw_borrowed` returns a reference without taking
        // ownership of the underlying reference count.
        let unknown = unsafe { IUnknown::from_raw_borrowed(&self.ptr) }.ok_or(E_POINTER)?;
        unknown.cast()
    }

    /// Releases the owned reference.
    pub fn release(self) {
        if !self.ptr.is_null() {
            // SAFETY: reconstructing the owned `IUnknown` and dropping it
            // issues `Release` on the owed reference.
            drop(unsafe { IUnknown::from_raw(self.ptr) });
        }
    }
}

impl PanicDefault for ComInterfaceRawPtr {
    fn default() -> Self {
        Self {
            ptr: core::ptr::null_mut(),
        }
    }
}
