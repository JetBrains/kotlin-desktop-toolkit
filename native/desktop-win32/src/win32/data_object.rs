#![allow(clippy::inline_always)]
#![allow(clippy::ref_as_ptr)]

use std::mem::ManuallyDrop;

use windows::Win32::{
    Foundation::{DV_E_FORMATETC, E_NOTIMPL, E_POINTER, HGLOBAL, OLE_E_ADVISENOTSUPPORTED, S_OK},
    System::Com::{
        DATADIR, DATADIR_GET, DVASPECT_CONTENT, FORMATETC, IAdviseSink, IDataObject, IDataObject_Impl, IEnumFORMATETC, IEnumSTATDATA,
        STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL,
    },
    UI::Shell::SHCreateStdEnumFmtEtc,
};
use windows_core::{BOOL, Error as WinError, HRESULT, Ref as WinRef, Result as WinResult, implement};

use super::global_data::HGlobalData;

#[implement(IDataObject)]
pub struct DataObject {
    format_etc: FORMATETC,
    data: HGlobalData,
}

impl DataObject {
    #[must_use]
    pub const fn create(format_id: u32, data: HGlobalData) -> Self {
        let format_etc = FORMATETC {
            #[allow(clippy::cast_possible_truncation)]
            cfFormat: format_id as u16,
            ptd: core::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0.cast_unsigned(),
        };
        Self { format_etc, data }
    }

    const fn is_format_supported(&self, format_etc: &FORMATETC) -> bool {
        self.format_etc.cfFormat == format_etc.cfFormat
            && self.format_etc.dwAspect == format_etc.dwAspect
            && (self.format_etc.tymed & format_etc.tymed) != 0
    }

    #[inline]
    fn get_content(&self) -> WinResult<HGLOBAL> {
        self.data.copied()
    }
}

#[allow(non_snake_case)]
impl IDataObject_Impl for DataObject_Impl {
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn GetData(&self, format_etc_in: *const FORMATETC) -> WinResult<STGMEDIUM> {
        let format_etc = unsafe { format_etc_in.as_ref() }.ok_or_else(|| WinError::from(E_POINTER))?;
        if !self.is_format_supported(format_etc) {
            return Err(DV_E_FORMATETC.into());
        }
        let content = self.get_content()?;
        let medium = STGMEDIUM {
            tymed: TYMED_HGLOBAL.0.cast_unsigned(),
            u: STGMEDIUM_0 { hGlobal: content },
            pUnkForRelease: ManuallyDrop::new(None),
        };
        Ok(medium)
    }

    fn GetDataHere(&self, _format_etc: *const FORMATETC, _medium: *mut STGMEDIUM) -> WinResult<()> {
        // We don't support putting data into a caller-allocated medium.
        Err(E_NOTIMPL.into())
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn QueryGetData(&self, format_etc_in: *const FORMATETC) -> HRESULT {
        match unsafe { format_etc_in.as_ref() } {
            Some(format_etc) if self.is_format_supported(format_etc) => S_OK,
            Some(_) => DV_E_FORMATETC,
            None => E_POINTER,
        }
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn GetCanonicalFormatEtc(&self, format_etc_in: *const FORMATETC, format_etc_out: *mut FORMATETC) -> HRESULT {
        if format_etc_in.is_null() || format_etc_out.is_null() {
            return E_POINTER;
        }
        // SAFETY: We have checked both pointers for null.
        unsafe {
            *format_etc_out = *format_etc_in;
            (*format_etc_out).ptd = core::ptr::null_mut();
        }
        windows::Win32::Foundation::DATA_S_SAMEFORMATETC
    }

    fn SetData(&self, _format_etc: *const FORMATETC, _medium: *const STGMEDIUM, _release: BOOL) -> WinResult<()> {
        // We don't support setting arbitrary data to the data object after its creation through the COM interface
        Err(E_NOTIMPL.into())
    }

    fn EnumFormatEtc(&self, direction: u32) -> WinResult<IEnumFORMATETC> {
        match DATADIR(direction.cast_signed()) {
            DATADIR_GET => unsafe { SHCreateStdEnumFmtEtc(&[self.format_etc]) },
            _ => Err(E_NOTIMPL.into()),
        }
    }

    fn DAdvise(&self, _format_etc: *const FORMATETC, _advf: u32, _adv_sink: WinRef<IAdviseSink>) -> WinResult<u32> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }

    fn DUnadvise(&self, _connection: u32) -> WinResult<()> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }

    fn EnumDAdvise(&self) -> WinResult<IEnumSTATDATA> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }
}
