use std::ffi::CString;

use windows::{
    Win32::{
        Foundation::{ERROR_INVALID_DATA, ERROR_SUCCESS, GetLastError, HANDLE, HGLOBAL},
        System::{
            DataExchange::{
                CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
                RegisterClipboardFormatW, SetClipboardData,
            },
            Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock},
            Ole::CF_UNICODETEXT,
        },
    },
    core::Result as WinResult,
};

use super::{strings::copy_from_wide_string, window::Window};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardFormat {
    Text,
    Other(u32),
}

pub const CLIPBOARD_TEXT_FORMAT: u32 = CF_UNICODETEXT.0 as u32;

pub struct Clipboard {
    is_open: bool,
}

impl Clipboard {
    pub fn open_for_window(window: &Window) -> anyhow::Result<Self> {
        unsafe { OpenClipboard(Some(window.hwnd()))? };
        Ok(Self { is_open: true })
    }

    pub fn register_format(format_name: &windows::core::HSTRING) -> anyhow::Result<u32> {
        Ok(unsafe { RegisterClipboardFormatW(format_name) })
    }

    pub fn list_available_formats(&self) -> anyhow::Result<Vec<u32>> {
        anyhow::ensure!(self.is_open, "Clipboard has been closed.");
        let mut formats = vec![];
        let mut next_format = unsafe { EnumClipboardFormats(0) };
        while next_format != 0 {
            formats.push(next_format);
            next_format = unsafe { EnumClipboardFormats(next_format) };
        }
        let err = unsafe { GetLastError() };
        if err != ERROR_SUCCESS {
            anyhow::bail!(windows::core::Error::from(err));
        }
        Ok(formats)
    }

    pub fn empty(&self) -> anyhow::Result<()> {
        anyhow::ensure!(self.is_open, "Clipboard has been closed.");
        unsafe { EmptyClipboard()? };
        Ok(())
    }

    pub fn get_data(&self, format: ClipboardFormat) -> anyhow::Result<ClipboardData> {
        anyhow::ensure!(self.is_open, "Clipboard has been closed.");
        let format_id = match format {
            ClipboardFormat::Text => CLIPBOARD_TEXT_FORMAT,
            ClipboardFormat::Other(format_id) => format_id,
        };
        unsafe { IsClipboardFormatAvailable(format_id)? };
        let mem = unsafe { GetClipboardData(format_id)? };
        Ok(ClipboardData { format_id, content: mem })
    }

    pub fn set_data(&self, data: &ClipboardData) -> anyhow::Result<()> {
        anyhow::ensure!(self.is_open, "Clipboard has been closed.");
        unsafe { SetClipboardData(data.format_id, Some(data.content))? };
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

pub struct ClipboardData {
    format_id: u32,
    content: HANDLE,
}

impl ClipboardData {
    pub fn new_text(text: &str) -> WinResult<Self> {
        let content: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        Self::new(content.as_slice(), CLIPBOARD_TEXT_FORMAT)
    }

    pub fn new_bytes(content: &[u8], format_id: u32) -> WinResult<Self> {
        Self::new(content, format_id)
    }

    fn new<T: Copy>(content: &[T], format_id: u32) -> WinResult<Self> {
        let mem = unsafe { GlobalAlloc(GMEM_MOVEABLE, content.len())? };
        unsafe {
            let data: *mut T = GlobalLock(mem).cast();
            std::slice::from_raw_parts_mut(data, content.len()).copy_from_slice(content);
            GlobalUnlock(mem)?;
        }
        Ok(Self {
            format_id,
            content: HANDLE(mem.0),
        })
    }

    pub fn get_text(&self) -> WinResult<CString> {
        if self.format_id != u32::from(CF_UNICODETEXT.0) {
            return Err(ERROR_INVALID_DATA.into());
        }
        let hglob = HGLOBAL(self.content.0);
        let content = unsafe { windows::core::PWSTR(GlobalLock(hglob).cast()) };
        let cstr = copy_from_wide_string(unsafe { content.as_wide() })?;
        unsafe { GlobalUnlock(hglob)? };
        Ok(cstr)
    }

    pub fn get_bytes(&self) -> WinResult<Vec<u8>> {
        let hglob = HGLOBAL(self.content.0);
        let content = unsafe { GlobalLock(hglob) };
        let len = unsafe { GlobalSize(hglob) };
        let vec = unsafe { std::slice::from_raw_parts(content.cast(), len) }.to_vec();
        unsafe { GlobalUnlock(hglob)? };
        Ok(vec)
    }
}
