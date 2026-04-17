#![allow(clippy::inline_always)]
#![allow(clippy::ref_as_ptr)]

use std::mem::ManuallyDrop;

use windows::Win32::{
    Foundation::{DV_E_FORMATETC, DV_E_TYMED, E_NOTIMPL, E_POINTER, HANDLE, HGLOBAL, OLE_E_ADVISENOTSUPPORTED, S_OK},
    System::Com::{
        DATADIR, DATADIR_GET, DVASPECT_CONTENT, FORMATETC, IAdviseSink, IDataObject, IDataObject_Impl, IEnumFORMATETC, IEnumSTATDATA,
        STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL,
    },
    UI::Shell::SHCreateStdEnumFmtEtc,
};
use windows_core::{BOOL, Error as WinError, HRESULT, Ref as WinRef, Result as WinResult, implement};

use super::{data_transfer::DataFormat, global_data::HGlobalData};

#[implement(IDataObject)]
pub struct DataObject {
    hash_map: papaya::HashMap<u32, HGlobalData>,
}

impl DataObject {
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            hash_map: papaya::HashMap::new(),
        }
    }

    pub fn add_format(&self, data_format: DataFormat, data: HGlobalData) -> bool {
        self.hash_map.pin().try_insert(data_format.id(), data).is_ok()
    }

    fn is_format_supported(&self, format_etc: &FORMATETC) -> bool {
        self.hash_map.pin().contains_key(&u32::from(format_etc.cfFormat))
            && format_etc.dwAspect == DVASPECT_CONTENT.0
            && (format_etc.tymed & TYMED_HGLOBAL.0.cast_unsigned()) != 0
    }

    #[inline]
    fn get_content(&self, format_id: u32) -> WinResult<HGLOBAL> {
        self.hash_map
            .pin()
            .get(&format_id)
            .ok_or_else(|| DV_E_FORMATETC.into())
            .and_then(HGlobalData::copied)
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
        let content = self.get_content(u32::from(format_etc.cfFormat))?;
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
            DATADIR_GET => {
                let formats = self
                    .hash_map
                    .pin()
                    .keys()
                    .map(|k| get_format_etc_for_hglobal(DataFormat::Other(*k)))
                    .collect::<Vec<_>>();
                unsafe { SHCreateStdEnumFmtEtc(&formats) }
            }
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

pub fn is_data_object_format_available(data_object: &IDataObject, data_format: DataFormat) -> anyhow::Result<bool> {
    let format_etc = get_format_etc_for_hglobal(data_format);
    match unsafe { data_object.QueryGetData(&raw const format_etc) } {
        S_OK => Ok(true),
        DV_E_FORMATETC | DV_E_TYMED => Ok(false),
        result => Err(WinError::from(result).into()),
    }
}

pub fn get_hglobal_from_data_object(data_object: &IDataObject, data_format: DataFormat) -> anyhow::Result<HGlobalData> {
    let format_etc = get_format_etc_for_hglobal(data_format);
    let medium = unsafe { data_object.GetData(&raw const format_etc)? };
    if medium.tymed & TYMED_HGLOBAL.0.cast_unsigned() == 0 {
        anyhow::bail!(windows_core::Error::from(DV_E_TYMED));
    }
    let hglobal = unsafe { medium.u.hGlobal };
    HGlobalData::copy_from(HANDLE(hglobal.0))
}

#[inline]
fn get_format_etc_for_hglobal(data_format: DataFormat) -> FORMATETC {
    FORMATETC {
        // Registered clipboard formats are identified by values in the range 0xC000 through 0xFFFF.
        // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerclipboardformatw
        #[allow(clippy::cast_possible_truncation)]
        cfFormat: data_format.id() as u16,
        ptd: core::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0.cast_unsigned(),
    }
}
