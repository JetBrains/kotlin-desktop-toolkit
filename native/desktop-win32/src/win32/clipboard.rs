use windows::Win32::{
    Foundation::{ERROR_SUCCESS, GetLastError, HANDLE, HWND, SetLastError, WIN32_ERROR},
    System::DataExchange::{
        CloseClipboard, CountClipboardFormats, EmptyClipboard, EnumClipboardFormats, GetClipboardData, GetClipboardSequenceNumber,
        IsClipboardFormatAvailable, OpenClipboard, SetClipboardData,
    },
};
use windows_core::Error as WinError;

use super::{clipboard_result::ClipboardFailure, data_transfer::DataFormat, global_data::HGlobalData, window::Window};

pub struct Clipboard {
    is_open: bool,
}

impl Clipboard {
    pub fn open_for_window(window: &Window) -> anyhow::Result<Self> {
        Self::open_for_hwnd_once(window.hwnd())
    }

    pub fn open_for_hwnd_once(hwnd: HWND) -> anyhow::Result<Self> {
        match unsafe { OpenClipboard(Some(hwnd)) } {
            Ok(()) => Ok(Self { is_open: true }),
            Err(err) if ClipboardFailure::is_open_clipboard_contention(&err) => Err(ClipboardFailure::busy_from_win_error(&err).into()),
            Err(err) => Err(err.into()),
        }
    }

    #[must_use]
    pub fn get_sequence_number() -> u32 {
        unsafe { GetClipboardSequenceNumber() }
    }

    pub fn ensure_sequence_unchanged(&self, expected_sequence: u32) -> anyhow::Result<()> {
        anyhow::ensure!(self.is_open, "Clipboard has been closed.");
        Self::ensure_sequence_number_unchanged(expected_sequence)
    }

    pub fn ensure_sequence_number_unchanged(expected_sequence: u32) -> anyhow::Result<()> {
        let actual_sequence = Self::get_sequence_number();
        if actual_sequence != expected_sequence {
            return Err(ClipboardFailure::changed(actual_sequence).into());
        }
        Ok(())
    }

    pub fn count_available_formats(&self) -> anyhow::Result<i32> {
        anyhow::ensure!(self.is_open, "Clipboard has been closed.");
        unsafe { SetLastError(WIN32_ERROR(0)) };
        let count = unsafe { CountClipboardFormats() };
        if count == 0 {
            let err = unsafe { GetLastError() };
            anyhow::ensure!(err == ERROR_SUCCESS, WinError::from(err));
        }
        Ok(count)
    }

    pub fn enum_available_formats(&self) -> anyhow::Result<Vec<u32>> {
        anyhow::ensure!(self.is_open, "Clipboard has been closed.");
        let mut formats = vec![];
        unsafe { SetLastError(WIN32_ERROR(0)) };
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

    pub fn get_data(&self, format: DataFormat) -> anyhow::Result<HGlobalData> {
        anyhow::ensure!(self.is_open, "Clipboard has been closed.");
        let format_id = format.id();
        if !self.is_format_available(format_id)? {
            return Err(ClipboardFailure::format_unavailable(format_id).into());
        }
        let mem = unsafe { GetClipboardData(format_id)? };
        HGlobalData::copy_from(mem)
    }

    pub fn set_data_items(&self, items: &mut [(DataFormat, HGlobalData)]) -> anyhow::Result<()> {
        anyhow::ensure!(self.is_open, "Clipboard has been closed.");
        unsafe { EmptyClipboard()? };
        for (format, data) in items {
            if let Err(err) = Self::set_data_after_empty(*format, data) {
                let _ = unsafe { EmptyClipboard() };
                return Err(err);
            }
        }
        Ok(())
    }

    fn set_data_after_empty(format: DataFormat, data: &mut HGlobalData) -> anyhow::Result<()> {
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
