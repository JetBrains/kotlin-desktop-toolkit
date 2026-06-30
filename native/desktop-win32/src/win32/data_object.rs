#![allow(clippy::inline_always)]
#![allow(clippy::ref_as_ptr)]

use std::mem::ManuallyDrop;

use windows::Win32::{
    Foundation::{
        DV_E_FORMATETC, DV_E_TYMED, E_NOTIMPL, E_POINTER, E_UNEXPECTED, HANDLE, HGLOBAL, OLE_E_ADVISENOTSUPPORTED, S_FALSE, S_OK,
    },
    System::{
        Com::{
            DATADIR, DATADIR_GET, DVASPECT_CONTENT, FORMATETC, IAdviseSink, IDataObject, IDataObject_Impl, IEnumFORMATETC, IEnumSTATDATA,
            STGMEDIUM, STGMEDIUM_0, TYMED, TYMED_HGLOBAL, TYMED_ISTREAM,
        },
        Ole::ReleaseStgMedium,
    },
    UI::Shell::SHCreateStdEnumFmtEtc,
};
use windows_core::{BOOL, Error as WinError, HRESULT, Ref as WinRef, Result as WinResult, implement};

use super::{data_reader::istream_reader, data_transfer::DataFormat, global_data::HGlobalData};

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

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn SetData(&self, format_etc_in: *const FORMATETC, medium_in: *const STGMEDIUM, release: BOOL) -> WinResult<()> {
        let format_etc = unsafe { format_etc_in.as_ref() }.ok_or_else(|| WinError::from(E_POINTER))?;
        // Borrow the medium mutably so one reference both reads its fields and releases it in place
        // on fRelease. The SetData C ABI passes a non-const STGMEDIUM*, modeled here as *const.
        let medium = unsafe { medium_in.cast_mut().as_mut() }.ok_or_else(|| WinError::from(E_POINTER))?;
        // The drag-image helper stores its private formats as both HGLOBAL and ISTREAM. Normalize
        // either into an owned HGLOBAL copy so the rest of the data object only handles HGLOBAL; for
        // an ISTREAM the bytes are read out and copied, never the live stream retained.
        let medium_data: anyhow::Result<HGlobalData> = match TYMED(medium.tymed.cast_signed()) {
            TYMED_HGLOBAL => HGlobalData::copy_from(HANDLE(unsafe { medium.u.hGlobal }.0)),
            TYMED_ISTREAM => match unsafe { (*medium.u.pstm).as_ref() } {
                Some(stream) => istream_reader::get_bytes(stream).and_then(|bytes| HGlobalData::alloc_from(&bytes)),
                None => Err(anyhow::anyhow!("data object received a null IStream")),
            },
            _ => return Err(DV_E_TYMED.into()),
        };
        let data = medium_data.map_err(|err| {
            log::error!("DataObject::SetData failed: {err:?}");
            // Surface the original Win32 HRESULT when the failure chain carries one.
            err.chain()
                .find_map(|cause| cause.downcast_ref::<WinError>())
                .cloned()
                .unwrap_or_else(|| WinError::from(E_UNEXPECTED))
        })?;
        // SetData replaces any existing data for the format, per the IDataObject contract.
        self.hash_map.pin().insert(u32::from(format_etc.cfFormat), data);
        // fRelease == TRUE hands the medium's ownership to us; free it now that the bytes are copied.
        if release.as_bool() {
            unsafe { ReleaseStgMedium(medium) };
        }
        Ok(())
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
        S_FALSE | DV_E_FORMATETC | DV_E_TYMED => Ok(false),
        result => Err(WinError::from(result).into()),
    }
}

pub fn enum_data_object_format_ids(data_object: &IDataObject) -> anyhow::Result<Box<[u32]>> {
    let iter = unsafe { data_object.EnumFormatEtc(DATADIR_GET.0.cast_unsigned())? };
    let mut hash_set = std::collections::HashSet::new();
    let mut formats = [FORMATETC::default(); 1];
    while S_OK == unsafe { iter.Next(&mut formats, None) } {
        let format_etc = formats[0];
        if format_etc.cfFormat != 0
            && format_etc.dwAspect == DVASPECT_CONTENT.0
            && ((format_etc.tymed & TYMED_HGLOBAL.0.cast_unsigned()) != 0 || (format_etc.tymed & TYMED_ISTREAM.0.cast_unsigned()) != 0)
        {
            hash_set.insert(u32::from(format_etc.cfFormat));
        }
    }
    Ok(hash_set.into_iter().collect())
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

#[cfg(test)]
mod tests {
    use std::mem::ManuallyDrop;

    use windows::Win32::{
        Foundation::{DATA_S_SAMEFORMATETC, DV_E_FORMATETC, DV_E_TYMED, E_NOTIMPL, E_POINTER, HANDLE, OLE_E_ADVISENOTSUPPORTED, S_OK},
        System::Com::{
            DATADIR_SET, DVASPECT_DOCPRINT, FORMATETC, IAdviseSink, IDataObject, STGMEDIUM, STGMEDIUM_0, TYMED_GDI, TYMED_HGLOBAL,
            TYMED_ISTREAM,
        },
        UI::Shell::SHCreateMemStream,
    };
    use windows_core::ComObject;

    use super::{DataFormat, DataObject, get_format_etc_for_hglobal};
    use crate::win32::global_data::{HGlobalData, hglobal_reader, hglobal_writer};

    const CUSTOM_FORMAT: u32 = 0xC123;
    const UNKNOWN_FORMAT: u32 = 0xC456;

    fn hglobal_medium(tymed: i32, hglobal: windows::Win32::Foundation::HGLOBAL) -> STGMEDIUM {
        STGMEDIUM {
            tymed: tymed.cast_unsigned(),
            u: STGMEDIUM_0 { hGlobal: hglobal },
            pUnkForRelease: ManuallyDrop::new(None),
        }
    }

    fn read_get_data_bytes(data_object: &IDataObject, format_id: u32) -> Vec<u8> {
        let format_etc = get_format_etc_for_hglobal(DataFormat::Other(format_id));
        let medium = unsafe { data_object.GetData(&raw const format_etc) }.unwrap();
        let copied = HGlobalData::copy_from(HANDLE(unsafe { medium.u.hGlobal }.0)).unwrap();
        hglobal_reader::get_bytes(&copied).unwrap()
    }

    #[test]
    fn set_data_then_get_data_returns_same_bytes() {
        let source = hglobal_writer::new_bytes(b"drag-image-bits").unwrap();
        let medium = hglobal_medium(TYMED_HGLOBAL.0, source.as_raw());
        let data_object: IDataObject = DataObject::new().into();
        let format_etc = get_format_etc_for_hglobal(DataFormat::Other(CUSTOM_FORMAT));

        unsafe { data_object.SetData(&raw const format_etc, &raw const medium, false) }.unwrap();

        assert_eq!(read_get_data_bytes(&data_object, CUSTOM_FORMAT), b"drag-image-bits");
    }

    #[test]
    fn query_get_data_returns_s_ok_for_set_format() {
        let source = hglobal_writer::new_bytes(b"payload").unwrap();
        let medium = hglobal_medium(TYMED_HGLOBAL.0, source.as_raw());
        let data_object: IDataObject = DataObject::new().into();
        let format_etc = get_format_etc_for_hglobal(DataFormat::Other(CUSTOM_FORMAT));
        unsafe { data_object.SetData(&raw const format_etc, &raw const medium, false) }.unwrap();

        let result = unsafe { data_object.QueryGetData(&raw const format_etc) };

        assert_eq!(result, S_OK);
    }

    #[test]
    fn enum_format_etc_includes_set_format() {
        let source = hglobal_writer::new_bytes(b"payload").unwrap();
        let medium = hglobal_medium(TYMED_HGLOBAL.0, source.as_raw());
        let data_object: IDataObject = DataObject::new().into();
        let format_etc = get_format_etc_for_hglobal(DataFormat::Other(CUSTOM_FORMAT));
        unsafe { data_object.SetData(&raw const format_etc, &raw const medium, false) }.unwrap();

        let ids = super::enum_data_object_format_ids(&data_object).unwrap();

        assert!(ids.contains(&CUSTOM_FORMAT));
    }

    #[test]
    fn set_data_with_unsupported_tymed_returns_dv_e_tymed() {
        // TYMED_GDI is neither HGLOBAL nor ISTREAM, the two the data object accepts.
        let medium = hglobal_medium(TYMED_GDI.0, windows::Win32::Foundation::HGLOBAL::default());
        let data_object: IDataObject = DataObject::new().into();
        let format_etc = get_format_etc_for_hglobal(DataFormat::Other(CUSTOM_FORMAT));

        let err = unsafe { data_object.SetData(&raw const format_etc, &raw const medium, false) }.unwrap_err();

        assert_eq!(err.code(), DV_E_TYMED);
    }

    #[test]
    fn set_data_with_istream_copies_bytes_to_hglobal() {
        // The drag-image helper sets a TYMED_ISTREAM private format; the data object must read its
        // bytes and serve them back as HGLOBAL.
        let payload: &[u8] = b"istream-drag-context";
        let stream = unsafe { SHCreateMemStream(Some(payload)) }.expect("SHCreateMemStream returned null");
        let medium = STGMEDIUM {
            tymed: TYMED_ISTREAM.0.cast_unsigned(),
            u: STGMEDIUM_0 {
                pstm: ManuallyDrop::new(Some(stream)),
            },
            pUnkForRelease: ManuallyDrop::new(None),
        };
        let data_object: IDataObject = DataObject::new().into();
        let format_etc = get_format_etc_for_hglobal(DataFormat::Other(CUSTOM_FORMAT));

        // fRelease == TRUE: SetData reads the stream's bytes, then releases the stream.
        unsafe { data_object.SetData(&raw const format_etc, &raw const medium, true) }.unwrap();

        assert_eq!(read_get_data_bytes(&data_object, CUSTOM_FORMAT), payload.to_vec());
    }

    #[test]
    fn set_data_with_release_true_succeeds() {
        let mut source = hglobal_writer::new_bytes(b"owned-by-data-object").unwrap();
        let medium = hglobal_medium(TYMED_HGLOBAL.0, source.as_raw());
        // fRelease == TRUE transfers the medium to the data object, which frees it after copying;
        // detach so this owner does not also free the same handle.
        source.detach();
        let data_object: IDataObject = DataObject::new().into();
        let format_etc = get_format_etc_for_hglobal(DataFormat::Other(CUSTOM_FORMAT));

        let result = unsafe { data_object.SetData(&raw const format_etc, &raw const medium, true) };

        assert!(result.is_ok());
        assert_eq!(read_get_data_bytes(&data_object, CUSTOM_FORMAT), b"owned-by-data-object");
    }

    #[test]
    fn add_format_then_get_data_returns_same_bytes() {
        let data_object = ComObject::new(DataObject::new());
        let data = hglobal_writer::new_bytes(b"app-data-bits").unwrap();
        assert!(data_object.add_format(DataFormat::Other(CUSTOM_FORMAT), data));
        let iface: IDataObject = data_object.to_interface();

        assert_eq!(read_get_data_bytes(&iface, CUSTOM_FORMAT), b"app-data-bits");
    }

    #[test]
    fn get_data_for_unknown_format_returns_dv_e_formatetc() {
        let data_object: IDataObject = DataObject::new().into();
        let format_etc = get_format_etc_for_hglobal(DataFormat::Other(UNKNOWN_FORMAT));

        // STGMEDIUM is not Debug, so inspect the error without unwrap_err.
        let code = unsafe { data_object.GetData(&raw const format_etc) }.err().map(|err| err.code());

        assert_eq!(code, Some(DV_E_FORMATETC));
    }

    #[test]
    fn query_get_data_for_unknown_format_returns_dv_e_formatetc() {
        let data_object: IDataObject = DataObject::new().into();
        let format_etc = get_format_etc_for_hglobal(DataFormat::Other(UNKNOWN_FORMAT));

        let result = unsafe { data_object.QueryGetData(&raw const format_etc) };

        assert_eq!(result, DV_E_FORMATETC);
    }

    #[test]
    fn add_format_with_duplicate_id_returns_false_and_retains_first_value() {
        let data_object = ComObject::new(DataObject::new());
        let first = hglobal_writer::new_bytes(b"first").unwrap();
        let second = hglobal_writer::new_bytes(b"second").unwrap();
        assert!(data_object.add_format(DataFormat::Other(CUSTOM_FORMAT), first));

        // try_insert refuses the duplicate id, so the first value stays in place.
        assert!(!data_object.add_format(DataFormat::Other(CUSTOM_FORMAT), second));

        let iface: IDataObject = data_object.to_interface();
        assert_eq!(read_get_data_bytes(&iface, CUSTOM_FORMAT), b"first");
    }

    #[test]
    fn get_canonical_format_etc_reports_same_format_and_nulls_ptd() {
        let data_object: IDataObject = DataObject::new().into();
        let mut input = get_format_etc_for_hglobal(DataFormat::Other(CUSTOM_FORMAT));
        input.ptd = core::ptr::dangling_mut();
        let mut output = FORMATETC::default();

        let result = unsafe { data_object.GetCanonicalFormatEtc(&raw const input, &raw mut output) };

        assert_eq!(result, DATA_S_SAMEFORMATETC);
        assert_eq!(output.cfFormat, input.cfFormat);
        assert_eq!(output.dwAspect, input.dwAspect);
        assert!(output.ptd.is_null());
    }

    #[test]
    fn enum_format_etc_for_set_direction_returns_e_notimpl() {
        let data_object: IDataObject = DataObject::new().into();

        let err = unsafe { data_object.EnumFormatEtc(DATADIR_SET.0.cast_unsigned()) }.unwrap_err();

        assert_eq!(err.code(), E_NOTIMPL);
    }

    #[test]
    fn query_get_data_with_non_content_aspect_returns_dv_e_formatetc() {
        let data_object = ComObject::new(DataObject::new());
        let data = hglobal_writer::new_bytes(b"payload").unwrap();
        assert!(data_object.add_format(DataFormat::Other(CUSTOM_FORMAT), data));
        let iface: IDataObject = data_object.to_interface();
        let format_etc = FORMATETC {
            dwAspect: DVASPECT_DOCPRINT.0,
            ..get_format_etc_for_hglobal(DataFormat::Other(CUSTOM_FORMAT))
        };

        let result = unsafe { iface.QueryGetData(&raw const format_etc) };

        assert_eq!(result, DV_E_FORMATETC);
    }

    #[test]
    fn get_data_here_returns_e_notimpl() {
        let data_object: IDataObject = DataObject::new().into();
        let format_etc = get_format_etc_for_hglobal(DataFormat::Other(CUSTOM_FORMAT));
        let mut medium = hglobal_medium(TYMED_HGLOBAL.0, windows::Win32::Foundation::HGLOBAL::default());

        let err = unsafe { data_object.GetDataHere(&raw const format_etc, &raw mut medium) }.unwrap_err();

        assert_eq!(err.code(), E_NOTIMPL);
    }

    #[test]
    fn advise_methods_report_not_supported() {
        let data_object: IDataObject = DataObject::new().into();
        let format_etc = get_format_etc_for_hglobal(DataFormat::Other(CUSTOM_FORMAT));

        let dadvise = unsafe { data_object.DAdvise(&raw const format_etc, 0, None::<&IAdviseSink>) }.unwrap_err();
        let dunadvise = unsafe { data_object.DUnadvise(0) }.unwrap_err();
        // IEnumSTATDATA is not Debug, so inspect the error without unwrap_err.
        let enum_dadvise = unsafe { data_object.EnumDAdvise() }.err().map(|err| err.code());

        assert_eq!(dadvise.code(), OLE_E_ADVISENOTSUPPORTED);
        assert_eq!(dunadvise.code(), OLE_E_ADVISENOTSUPPORTED);
        assert_eq!(enum_dadvise, Some(OLE_E_ADVISENOTSUPPORTED));
    }

    #[test]
    fn is_data_object_format_available_reflects_presence() {
        let data_object = ComObject::new(DataObject::new());
        let data = hglobal_writer::new_bytes(b"payload").unwrap();
        assert!(data_object.add_format(DataFormat::Other(CUSTOM_FORMAT), data));
        let iface: IDataObject = data_object.to_interface();

        assert!(super::is_data_object_format_available(&iface, DataFormat::Other(CUSTOM_FORMAT)).unwrap());
        assert!(!super::is_data_object_format_available(&iface, DataFormat::Other(UNKNOWN_FORMAT)).unwrap());
    }

    #[test]
    fn set_data_with_null_medium_returns_e_pointer() {
        let data_object: IDataObject = DataObject::new().into();
        let format_etc = get_format_etc_for_hglobal(DataFormat::Other(CUSTOM_FORMAT));

        let err = unsafe { data_object.SetData(&raw const format_etc, core::ptr::null(), false) }.unwrap_err();

        assert_eq!(err.code(), E_POINTER);
    }
}
