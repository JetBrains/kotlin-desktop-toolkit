use std::sync::LazyLock;

use windows::Win32::{
    Foundation::{ERROR_SUCCESS, GetLastError, HANDLE},
    System::{
        DataExchange::{
            CloseClipboard, CountClipboardFormats, EmptyClipboard, EnumClipboardFormats, GetClipboardData, GetClipboardSequenceNumber,
            IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
        },
        Ole::{CF_HDROP, CF_UNICODETEXT},
    },
};
use windows_core::{Error as WinError, HSTRING, w};

use super::{global_data::HGlobalData, window::Window};

/// cbindgen:ignore
static HTML_FORMAT: LazyLock<u32> = LazyLock::new(|| unsafe { RegisterClipboardFormatW(w!("HTML Format")) });

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardFormat {
    Text,
    FileList,
    HtmlFragment,
    Other(u32),
}

impl ClipboardFormat {
    #[inline]
    #[must_use]
    pub fn id(self) -> u32 {
        match self {
            Self::Text => u32::from(CF_UNICODETEXT.0),
            Self::FileList => u32::from(CF_HDROP.0),
            Self::HtmlFragment => *HTML_FORMAT,
            Self::Other(fmt) => fmt,
        }
    }
}

pub struct Clipboard {
    is_open: bool,
}

impl Clipboard {
    pub fn open_for_window(window: &Window) -> anyhow::Result<Self> {
        unsafe { OpenClipboard(Some(window.hwnd()))? };
        Ok(Self { is_open: true })
    }

    pub fn register_format(format_name: &HSTRING) -> anyhow::Result<u32> {
        Ok(unsafe { RegisterClipboardFormatW(format_name) })
    }

    #[must_use]
    pub fn get_sequence_number() -> u32 {
        unsafe { GetClipboardSequenceNumber() }
    }

    pub fn count_available_formats(&self) -> anyhow::Result<i32> {
        let count = unsafe { CountClipboardFormats() };
        anyhow::ensure!(count != 0, WinError::from_thread());
        Ok(count)
    }

    pub fn enum_available_formats(&self) -> anyhow::Result<Vec<u32>> {
        anyhow::ensure!(self.is_open, "Clipboard has been closed.");
        let mut formats = vec![];
        let mut next_format = unsafe { EnumClipboardFormats(0) };
        while next_format != 0 {
            formats.push(next_format);
            next_format = unsafe { EnumClipboardFormats(next_format) };
        }
        let err = unsafe { GetLastError() };
        anyhow::ensure!(err == ERROR_SUCCESS, WinError::from(err));
        Ok(formats)
    }

    pub fn is_format_available(&self, format_id: u32) -> anyhow::Result<bool> {
        match unsafe { IsClipboardFormatAvailable(format_id) } {
            Ok(()) => Ok(true),
            Err(err) if err.code().is_ok() => Ok(false),
            Err(err) => Err(err.into()),
        }
    }

    pub fn empty(&self) -> anyhow::Result<()> {
        anyhow::ensure!(self.is_open, "Clipboard has been closed.");
        unsafe { EmptyClipboard()? };
        Ok(())
    }

    pub fn get_data(&self, format: ClipboardFormat) -> anyhow::Result<HGlobalData> {
        anyhow::ensure!(self.is_open, "Clipboard has been closed.");
        let format_id = format.id();
        anyhow::ensure!(self.is_format_available(format_id)?, "specified Clipboard format is unavailable");
        let mem = unsafe { GetClipboardData(format_id)? };
        HGlobalData::copy_from(mem)
    }

    pub fn set_data(&self, format: ClipboardFormat, data: &mut HGlobalData) -> anyhow::Result<()> {
        anyhow::ensure!(self.is_open, "Clipboard has been closed.");
        unsafe { SetClipboardData(format.id(), Some(HANDLE(data.as_raw().0)))? };
        data.detach();
        Ok(())
    }
}

impl Drop for Clipboard {
    fn drop(&mut self) {
        if self.is_open {
            let _ = unsafe { CloseClipboard() }.inspect_err(|err| log::error!("failed to close the clipboard: {err:?}"));
            self.is_open = false;
        }
    }
}
