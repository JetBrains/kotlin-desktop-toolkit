#![allow(clippy::inline_always)]
#![allow(clippy::ref_as_ptr)]

use std::mem::ManuallyDrop;

use anyhow::Context;
use windows::Win32::{
    Foundation::{DV_E_DVASPECT, DV_E_FORMATETC, DV_E_TYMED, E_NOTIMPL, E_POINTER, E_UNEXPECTED, HANDLE, OLE_E_ADVISENOTSUPPORTED, S_OK},
    System::{
        Com::{
            DATADIR, DATADIR_GET, DVASPECT_CONTENT, FORMATETC, IAdviseSink, IDataObject, IDataObject_Impl, IEnumFORMATETC, IEnumSTATDATA,
            IStream, STGMEDIUM, STGMEDIUM_0, TYMED, TYMED_HGLOBAL, TYMED_ISTREAM,
        },
        Ole::ReleaseStgMedium,
        WinRT::{AGILEREFERENCE_DEFAULT, IAgileReference, RoGetAgileReference},
    },
    UI::Shell::SHCreateStdEnumFmtEtc,
};
use windows_core::{BOOL, Error as WinError, HRESULT, Interface, Ref as WinRef, Result as WinResult, implement};

use super::{data_transfer::DataFormat, global_data::HGlobalData};

/// The data stored for a format, preserving its `TYMED`: an HGLOBAL's contents are copied,
/// a stream is kept by reference.
enum StoredMedium {
    HGlobal(HGlobalData),
    /// An agile reference to a stored stream: usable from any apartment and thread, unlike a raw
    /// `IStream`, which is bound to the apartment that produced it.
    Stream(IAgileReference),
}

// SAFETY: `HGlobalData` is `Send`/`Sync` (an HGLOBAL is process-global), and an `IAgileReference` is
// agile by construction — it may be held, resolved, and released from any apartment/thread, which
// `papaya`'s deferred reclamation may do on a thread other than the one that stored it. windows-core
// does not express agility in the type system, so the marker traits are asserted here.
#[expect(
    clippy::non_send_fields_in_send_ty,
    reason = "IAgileReference is agile: cross-thread hold/resolve/release is sound (see SAFETY above)"
)]
unsafe impl Send for StoredMedium {}
unsafe impl Sync for StoredMedium {}

impl StoredMedium {
    const fn tymed(&self) -> u32 {
        match self {
            Self::HGlobal(_) => TYMED_HGLOBAL.0.cast_unsigned(),
            Self::Stream(_) => TYMED_ISTREAM.0.cast_unsigned(),
        }
    }

    /// The `HRESULT` a `GetData`/`QueryGetData` for `format_etc` yields once its `cfFormat` matches
    /// this entry: `DV_E_DVASPECT` for an unsupported aspect, `DV_E_TYMED` when the requested medium
    /// excludes the one stored, otherwise `S_OK`.
    const fn query(&self, format_etc: &FORMATETC) -> HRESULT {
        if format_etc.dwAspect != DVASPECT_CONTENT.0 {
            DV_E_DVASPECT
        } else if (format_etc.tymed & self.tymed()) == 0 {
            DV_E_TYMED
        } else {
            S_OK
        }
    }

    /// Produces an owned `STGMEDIUM` for the caller to free via `ReleaseStgMedium`.
    fn duplicate(&self) -> WinResult<STGMEDIUM> {
        match self {
            Self::HGlobal(data) => Ok(STGMEDIUM {
                tymed: TYMED_HGLOBAL.0.cast_unsigned(),
                u: STGMEDIUM_0 { hGlobal: data.copied()? },
                pUnkForRelease: ManuallyDrop::new(None),
            }),
            Self::Stream(agile) => {
                let stream: IStream = unsafe { agile.Resolve() }?;
                Ok(STGMEDIUM {
                    tymed: TYMED_ISTREAM.0.cast_unsigned(),
                    u: STGMEDIUM_0 {
                        pstm: ManuallyDrop::new(Some(stream)),
                    },
                    pUnkForRelease: ManuallyDrop::new(None),
                })
            }
        }
    }
}

#[implement(IDataObject)]
pub struct DataObject {
    hash_map: papaya::HashMap<u32, StoredMedium>,
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
        self.hash_map
            .pin()
            .try_insert(data_format.id(), StoredMedium::HGlobal(data))
            .is_ok()
    }
}

#[allow(non_snake_case)]
impl IDataObject_Impl for DataObject_Impl {
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn GetData(&self, format_etc_in: *const FORMATETC) -> WinResult<STGMEDIUM> {
        let format_etc = unsafe { format_etc_in.as_ref() }.ok_or_else(|| WinError::from(E_POINTER))?;
        let map = self.hash_map.pin();
        let stored = map
            .get(&u32::from(format_etc.cfFormat))
            .ok_or_else(|| WinError::from(DV_E_FORMATETC))?;
        stored.query(format_etc).ok()?;
        stored.duplicate()
    }

    fn GetDataHere(&self, _format_etc: *const FORMATETC, _medium: *mut STGMEDIUM) -> WinResult<()> {
        // We don't support putting data into a caller-allocated medium.
        Err(E_NOTIMPL.into())
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn QueryGetData(&self, format_etc_in: *const FORMATETC) -> HRESULT {
        let Some(format_etc) = (unsafe { format_etc_in.as_ref() }) else {
            return E_POINTER;
        };
        match self.hash_map.pin().get(&u32::from(format_etc.cfFormat)) {
            Some(stored) => stored.query(format_etc),
            None => DV_E_FORMATETC,
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
        // Keep the medium's TYMED so GetData returns the format the way it was set: copy an HGLOBAL's
        // contents, hold a stream by agile reference. Other tymeds are unsupported.
        let medium_data: anyhow::Result<StoredMedium> = match TYMED(medium.tymed.cast_signed()) {
            TYMED_HGLOBAL => HGlobalData::copy_from(HANDLE(unsafe { medium.u.hGlobal }.0)).map(StoredMedium::HGlobal),
            TYMED_ISTREAM => match unsafe { (*medium.u.pstm).as_ref() } {
                Some(stream) => unsafe { RoGetAgileReference(AGILEREFERENCE_DEFAULT, &IStream::IID, stream) }
                    .map(StoredMedium::Stream)
                    .context("failed to create an agile reference to the IStream"),
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
        // fRelease == TRUE hands the medium's ownership to us; free it now that we hold our own
        // copy (HGLOBAL) or agile reference (stream).
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
                    .iter()
                    .map(|(cf_format, stored)| get_format_etc(*cf_format, stored.tymed()))
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

pub fn enum_data_object_format_ids(data_object: &IDataObject) -> anyhow::Result<std::collections::HashSet<u32>> {
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
    Ok(hash_set)
}

/// Returns a `FORMATETC` requesting `data_format` in any `TYMED` the data-transfer path handles —
/// HGLOBAL or ISTREAM, the media this object stores and the reader consumes.
#[inline]
pub(crate) fn get_format_etc_for_supported_tymeds(data_format: DataFormat) -> FORMATETC {
    get_format_etc(data_format.id(), (TYMED_HGLOBAL.0 | TYMED_ISTREAM.0).cast_unsigned())
}

#[inline]
const fn get_format_etc(cf_format: u32, tymed: u32) -> FORMATETC {
    FORMATETC {
        // Registered clipboard formats are identified by values in the range 0xC000 through 0xFFFF.
        // See: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerclipboardformatw
        #[allow(clippy::cast_possible_truncation)]
        cfFormat: cf_format as u16,
        ptd: core::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed,
    }
}

#[cfg(test)]
mod tests {
    use std::mem::ManuallyDrop;

    use windows::Win32::{
        Foundation::{
            DATA_S_SAMEFORMATETC, DV_E_DVASPECT, DV_E_FORMATETC, DV_E_TYMED, E_NOTIMPL, E_POINTER, E_UNEXPECTED, HANDLE,
            OLE_E_ADVISENOTSUPPORTED, RPC_E_CHANGED_MODE, S_FALSE, S_OK,
        },
        System::Com::{
            COINIT_APARTMENTTHREADED, CoInitializeEx, DATADIR_GET, DATADIR_SET, DVASPECT_DOCPRINT, FORMATETC, IAdviseSink, IDataObject,
            STGMEDIUM, STGMEDIUM_0, TYMED_GDI, TYMED_HGLOBAL, TYMED_ISTREAM,
        },
        UI::Shell::SHCreateMemStream,
    };
    use windows_core::ComObject;

    use super::{DataFormat, DataObject, get_format_etc};
    use crate::win32::data_reader::istream_reader;
    use crate::win32::global_data::{HGlobalData, hglobal_reader, hglobal_writer};

    const CUSTOM_FORMAT: u32 = 0xC123;
    const UNKNOWN_FORMAT: u32 = 0xC456;

    fn ensure_com_initialized() {
        // RoGetAgileReference needs COM initialized on the calling thread: initialize an STA and
        // tolerate the "already initialized" returns for repeated tests on one thread.
        let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        assert!(
            hr.is_ok() || hr == S_FALSE || hr == RPC_E_CHANGED_MODE,
            "CoInitializeEx failed: {hr:?}"
        );
    }

    fn hglobal_medium(tymed: i32, hglobal: windows::Win32::Foundation::HGLOBAL) -> STGMEDIUM {
        STGMEDIUM {
            tymed: tymed.cast_unsigned(),
            u: STGMEDIUM_0 { hGlobal: hglobal },
            pUnkForRelease: ManuallyDrop::new(None),
        }
    }

    #[inline]
    fn get_format_etc_for_hglobal(data_format: DataFormat) -> FORMATETC {
        get_format_etc(data_format.id(), TYMED_HGLOBAL.0.cast_unsigned())
    }

    fn read_get_data_bytes(data_object: &IDataObject, format_id: u32) -> Vec<u8> {
        let format_etc = get_format_etc_for_hglobal(DataFormat::Other(format_id));
        let medium = unsafe { data_object.GetData(&raw const format_etc) }.unwrap();
        let copied = HGlobalData::copy_from(HANDLE(unsafe { medium.u.hGlobal }.0)).unwrap();
        hglobal_reader::get_bytes(&copied).unwrap()
    }

    /// Stores `payload` as a `TYMED_ISTREAM` medium under `format_id`, with `fRelease == TRUE` so the
    /// data object takes the stream. Initializes COM because `SetData` calls `RoGetAgileReference`.
    fn set_istream_format(data_object: &IDataObject, format_id: u32, payload: &[u8]) {
        ensure_com_initialized();
        let stream = unsafe { SHCreateMemStream(Some(payload)) }.expect("SHCreateMemStream returned null");
        let medium = STGMEDIUM {
            tymed: TYMED_ISTREAM.0.cast_unsigned(),
            u: STGMEDIUM_0 {
                pstm: ManuallyDrop::new(Some(stream)),
            },
            pUnkForRelease: ManuallyDrop::new(None),
        };
        let format_etc = get_format_etc(format_id, TYMED_ISTREAM.0.cast_unsigned());
        unsafe { data_object.SetData(&raw const format_etc, &raw const medium, true) }.unwrap();
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
    fn set_data_with_istream_roundtrips_as_stream() {
        // A format set as TYMED_ISTREAM is retained by reference and served back as a stream, not
        // flattened to HGLOBAL.
        let payload: &[u8] = b"istream-drag-context";
        let data_object: IDataObject = DataObject::new().into();
        set_istream_format(&data_object, CUSTOM_FORMAT, payload);
        let format_etc = get_format_etc(CUSTOM_FORMAT, TYMED_ISTREAM.0.cast_unsigned());

        // GetData serves the format back as a stream; ReleaseStgMedium frees the returned copy.
        let mut out = unsafe { data_object.GetData(&raw const format_etc) }.unwrap();
        assert_eq!(
            out.tymed,
            TYMED_ISTREAM.0.cast_unsigned(),
            "an ISTREAM format must be served back as a stream"
        );
        let bytes = istream_reader::get_bytes(unsafe { (*out.u.pstm).as_ref() }.expect("stream medium")).unwrap();
        unsafe { windows::Win32::System::Ole::ReleaseStgMedium(&raw mut out) };

        assert_eq!(bytes, payload);
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
    fn query_get_data_with_non_content_aspect_returns_dv_e_dvaspect() {
        let data_object = ComObject::new(DataObject::new());
        let data = hglobal_writer::new_bytes(b"payload").unwrap();
        assert!(data_object.add_format(DataFormat::Other(CUSTOM_FORMAT), data));
        let iface: IDataObject = data_object.to_interface();
        let format_etc = FORMATETC {
            dwAspect: DVASPECT_DOCPRINT.0,
            ..get_format_etc_for_hglobal(DataFormat::Other(CUSTOM_FORMAT))
        };

        let result = unsafe { iface.QueryGetData(&raw const format_etc) };

        assert_eq!(result, DV_E_DVASPECT);
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
    fn set_data_with_null_medium_returns_e_pointer() {
        let data_object: IDataObject = DataObject::new().into();
        let format_etc = get_format_etc_for_hglobal(DataFormat::Other(CUSTOM_FORMAT));

        let err = unsafe { data_object.SetData(&raw const format_etc, core::ptr::null(), false) }.unwrap_err();

        assert_eq!(err.code(), E_POINTER);
    }

    #[test]
    fn set_data_replaces_existing_format() {
        // SetData overwrites an existing format, unlike add_format, which keeps the first value.
        let first = hglobal_writer::new_bytes(b"first").unwrap();
        let second = hglobal_writer::new_bytes(b"second").unwrap();
        let data_object: IDataObject = DataObject::new().into();
        let format_etc = get_format_etc_for_hglobal(DataFormat::Other(CUSTOM_FORMAT));

        let first_medium = hglobal_medium(TYMED_HGLOBAL.0, first.as_raw());
        unsafe { data_object.SetData(&raw const format_etc, &raw const first_medium, false) }.unwrap();
        let second_medium = hglobal_medium(TYMED_HGLOBAL.0, second.as_raw());
        unsafe { data_object.SetData(&raw const format_etc, &raw const second_medium, false) }.unwrap();

        assert_eq!(read_get_data_bytes(&data_object, CUSTOM_FORMAT), b"second");
    }

    #[test]
    fn query_get_data_returns_s_ok_for_stream_format() {
        let data_object: IDataObject = DataObject::new().into();
        set_istream_format(&data_object, CUSTOM_FORMAT, b"stream-payload");
        let format_etc = get_format_etc(CUSTOM_FORMAT, TYMED_ISTREAM.0.cast_unsigned());

        let result = unsafe { data_object.QueryGetData(&raw const format_etc) };

        assert_eq!(result, S_OK);
    }

    #[test]
    fn enum_format_etc_advertises_stream_tymed() {
        // EnumFormatEtc reports each format with the TYMED it was stored under, so a stream format is
        // advertised as TYMED_ISTREAM rather than the default HGLOBAL.
        let data_object: IDataObject = DataObject::new().into();
        set_istream_format(&data_object, CUSTOM_FORMAT, b"stream-payload");

        let iter = unsafe { data_object.EnumFormatEtc(DATADIR_GET.0.cast_unsigned()) }.unwrap();
        let mut formats = [FORMATETC::default(); 1];
        let mut enumerated = None;
        while S_OK == unsafe { iter.Next(&mut formats, None) } {
            if u32::from(formats[0].cfFormat) == CUSTOM_FORMAT {
                enumerated = Some(formats[0]);
            }
        }

        assert_eq!(
            enumerated.expect("format was enumerated").tymed,
            TYMED_ISTREAM.0.cast_unsigned(),
            "a stream format must be advertised as TYMED_ISTREAM"
        );
    }

    #[test]
    fn get_data_with_mismatched_tymed_returns_dv_e_tymed() {
        // A format stored only as a stream, requested HGLOBAL-only, is present but in the wrong
        // medium — DV_E_TYMED, not the DV_E_FORMATETC reserved for an unknown format.
        let data_object: IDataObject = DataObject::new().into();
        set_istream_format(&data_object, CUSTOM_FORMAT, b"stream-payload");
        let format_etc = get_format_etc_for_hglobal(DataFormat::Other(CUSTOM_FORMAT));

        let code = unsafe { data_object.GetData(&raw const format_etc) }.err().map(|err| err.code());

        assert_eq!(code, Some(DV_E_TYMED));
    }

    #[test]
    fn query_get_data_with_mismatched_tymed_returns_dv_e_tymed() {
        let data_object: IDataObject = DataObject::new().into();
        set_istream_format(&data_object, CUSTOM_FORMAT, b"stream-payload");
        let format_etc = get_format_etc_for_hglobal(DataFormat::Other(CUSTOM_FORMAT));

        let result = unsafe { data_object.QueryGetData(&raw const format_etc) };

        assert_eq!(result, DV_E_TYMED);
    }

    #[test]
    fn set_data_with_null_istream_returns_error() {
        // A TYMED_ISTREAM medium carrying a null stream is rejected, not dereferenced.
        let data_object: IDataObject = DataObject::new().into();
        let medium = STGMEDIUM {
            tymed: TYMED_ISTREAM.0.cast_unsigned(),
            u: STGMEDIUM_0 {
                pstm: ManuallyDrop::new(None),
            },
            pUnkForRelease: ManuallyDrop::new(None),
        };
        let format_etc = get_format_etc(CUSTOM_FORMAT, TYMED_ISTREAM.0.cast_unsigned());

        let err = unsafe { data_object.SetData(&raw const format_etc, &raw const medium, false) }.unwrap_err();

        assert_eq!(err.code(), E_UNEXPECTED);
    }
}
