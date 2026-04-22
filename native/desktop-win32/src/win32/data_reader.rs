use std::ffi::CString;

use anyhow::Context;
use windows::Win32::{
    Foundation::{DV_E_TYMED, E_POINTER, HANDLE},
    System::{
        Com::{DVASPECT_CONTENT, FORMATETC, IDataObject, IStream, STGMEDIUM, TYMED, TYMED_HGLOBAL, TYMED_ISTREAM},
        Ole::ReleaseStgMedium,
    },
};
use windows_core::Error as WinError;

use super::{
    data_transfer::DataFormat,
    global_data::{HGlobalData, hglobal_reader},
};

struct StgMediumGuard {
    medium: STGMEDIUM,
}

impl Drop for StgMediumGuard {
    fn drop(&mut self) {
        unsafe { ReleaseStgMedium(&raw mut self.medium) };
    }
}

enum DataSource {
    HGlobal(HGlobalData),
    IStream(IStream),
}

pub struct DataReader {
    source: DataSource,
    #[expect(dead_code, reason = "retained solely for its Drop side-effect")]
    guard: StgMediumGuard,
}

impl DataReader {
    pub fn create(data_object: &IDataObject, data_format: DataFormat) -> anyhow::Result<Self> {
        let format_desc = FORMATETC {
            cfFormat: data_format.id().try_into()?,
            ptd: core::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: (TYMED_HGLOBAL.0 | TYMED_ISTREAM.0).cast_unsigned(),
        };
        let medium = unsafe { data_object.GetData(&raw const format_desc)? };
        let source = match TYMED(medium.tymed.cast_signed()) {
            TYMED_HGLOBAL => {
                let mem = unsafe { medium.u.hGlobal };
                DataSource::HGlobal(HGlobalData::copy_from(HANDLE(mem.0))?)
            }
            TYMED_ISTREAM => {
                // cloning a COM object merely increases its reference counter
                let stream = unsafe { medium.u.pstm.as_ref().context(WinError::from(E_POINTER))?.clone() };
                DataSource::IStream(stream)
            }
            _ => anyhow::bail!(WinError::from(DV_E_TYMED)),
        };
        Ok(Self {
            source,
            guard: StgMediumGuard { medium },
        })
    }

    pub fn get_bytes(&self) -> anyhow::Result<Vec<u8>> {
        match &self.source {
            DataSource::HGlobal(data) => hglobal_reader::get_bytes(data),
            DataSource::IStream(stream) => istream_reader::get_bytes(stream),
        }
    }

    pub fn get_file_list(&self) -> anyhow::Result<Vec<CString>> {
        match &self.source {
            DataSource::HGlobal(data) => hglobal_reader::get_file_list(data),
            DataSource::IStream(stream) => istream_reader::get_file_list(stream),
        }
    }

    pub fn get_html(&self) -> anyhow::Result<CString> {
        match &self.source {
            DataSource::HGlobal(data) => hglobal_reader::get_html(data),
            DataSource::IStream(stream) => istream_reader::get_html(stream),
        }
    }

    pub fn get_text(&self) -> anyhow::Result<CString> {
        match &self.source {
            DataSource::HGlobal(data) => hglobal_reader::get_text(data),
            DataSource::IStream(stream) => istream_reader::get_text(stream),
        }
    }
}

pub mod istream_reader {
    use std::ffi::CString;

    use windows::{
        ApplicationModel::DataTransfer::HtmlFormatHelper,
        Win32::{
            System::Com::{IStream, STATFLAG_NONAME, STATSTG, STREAM_SEEK_SET},
            UI::Shell::HDROP,
        },
    };

    use crate::win32::{
        global_data::parse_file_list,
        strings::{copy_from_utf8_bytes, copy_from_wide_string},
    };

    pub fn get_bytes(stream: &IStream) -> anyhow::Result<Vec<u8>> {
        let mut stat = STATSTG::default();
        unsafe {
            stream.Stat(&raw mut stat, STATFLAG_NONAME)?;
            stream.Seek(0, STREAM_SEEK_SET, None)?;
        }
        let len = stat.cbSize.try_into()?;
        let mut vec = vec![0u8; len];
        let mut offset = 0_usize;
        while offset < len {
            let mut read = 0;
            unsafe {
                stream.Read(
                    vec[offset..].as_mut_ptr().cast(),
                    (len - offset).try_into().unwrap_or(u32::MAX),
                    Some(&raw mut read),
                )
            }
            .ok()?;
            if read == 0 {
                break;
            }
            offset += usize::try_from(read)?;
        }
        anyhow::ensure!(offset == len, "Stream couldn't be read.");
        Ok(vec)
    }

    pub fn get_file_list(stream: &IStream) -> anyhow::Result<Vec<CString>> {
        let bytes = get_bytes(stream)?;
        let content = bytes.as_ptr().cast::<core::ffi::c_void>().cast_mut();
        unsafe { parse_file_list(HDROP(content)) }
    }

    pub fn get_html(stream: &IStream) -> anyhow::Result<CString> {
        let utf8_bytes = get_bytes(stream)?;
        let html_format = copy_from_utf8_bytes(&utf8_bytes)?;
        let fragment = HtmlFormatHelper::GetStaticFragment(&html_format)?;
        copy_from_wide_string(&fragment)
    }

    pub fn get_text(stream: &IStream) -> anyhow::Result<CString> {
        // Assumes the OLE convention that a TYMED_ISTREAM payload for a standard clipboard format
        // carries the same byte layout as the equivalent TYMED_HGLOBAL payload. For CF_UNICODETEXT
        // that means raw UTF-16 LE with an optional trailing NUL. This is deliberately not compatible
        // with shlwapi's IStream_ReadStr / IStream_WriteStr length-prefixed wire format, which is
        // used for application-private persistence streams, not for clipboard data.
        // Malformed input surfaces as an Err via String::from_utf16; it cannot panic.
        let bytes = get_bytes(stream)?;
        let (chunks, []) = bytes.as_chunks::<2>() else {
            anyhow::bail!("UTF-16 byte stream has odd length")
        };
        let wide: Vec<u16> = chunks.iter().map(|&pair| u16::from_le_bytes(pair)).collect();
        copy_from_wide_string(&wide)
    }
}
