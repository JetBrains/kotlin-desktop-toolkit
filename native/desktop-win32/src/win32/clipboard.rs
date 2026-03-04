use std::ffi::CString;

use windows::{
    Win32::{
        Foundation::{ERROR_INVALID_PARAMETER, ERROR_SUCCESS, GetLastError, HANDLE, HGLOBAL, POINT},
        System::{
            DataExchange::{
                CloseClipboard, CountClipboardFormats, EmptyClipboard, EnumClipboardFormats, GetClipboardData, GetClipboardSequenceNumber,
                IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
            },
            Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock},
            Ole::{CF_HDROP, CF_UNICODETEXT},
        },
        UI::Shell::{DROPFILES, DragQueryFileW, HDROP},
    },
    core::Error as WinError,
};

use super::{strings::copy_from_wide_string, window::Window};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardFormat {
    Text,
    FileList,
    Other(u32),
}

impl ClipboardFormat {
    fn id(self) -> u32 {
        match self {
            Self::Text => u32::from(CF_UNICODETEXT.0),
            Self::FileList => u32::from(CF_HDROP.0),
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

    pub fn register_format(format_name: &windows::core::HSTRING) -> anyhow::Result<u32> {
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

    pub fn get_data(&self, format: ClipboardFormat) -> anyhow::Result<ClipboardData> {
        anyhow::ensure!(self.is_open, "Clipboard has been closed.");
        let format_id = format.id();
        anyhow::ensure!(self.is_format_available(format_id)?, WinError::from(ERROR_INVALID_PARAMETER));
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
    pub fn new_text(text: &str) -> anyhow::Result<Self> {
        let content: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        Self::new(content.as_slice(), ClipboardFormat::Text.id())
    }

    pub fn new_bytes(content: &[u8], format_id: u32) -> anyhow::Result<Self> {
        Self::new(content, format_id)
    }

    pub fn new_file_list(file_names: &Vec<&str>) -> anyhow::Result<Self> {
        let header = DROPFILES {
            pFiles: size_of::<DROPFILES>().try_into()?,
            pt: POINT { x: 0, y: 0 },
            fNC: windows::Win32::Foundation::FALSE,
            fWide: windows::Win32::Foundation::TRUE,
        };
        let files_data: Vec<u16> = file_names.join("\0").encode_utf16().chain(std::iter::repeat_n(0, 2)).collect();
        let content_len = size_of::<DROPFILES>() + (files_data.len() * size_of::<u16>());
        let mem = unsafe { GlobalAlloc(GMEM_MOVEABLE, content_len)? };
        unsafe {
            let data = GlobalLock(mem);
            data.cast::<DROPFILES>().write(header);
            std::slice::from_raw_parts_mut(data.add(size_of::<DROPFILES>()).cast::<u16>(), files_data.len())
                .copy_from_slice(files_data.as_slice());
            GlobalUnlock(mem)?;
        }
        Ok(Self {
            format_id: ClipboardFormat::FileList.id(),
            content: HANDLE(mem.0),
        })
    }

    fn new<T: Copy>(content: &[T], format_id: u32) -> anyhow::Result<Self> {
        let mem = unsafe { GlobalAlloc(GMEM_MOVEABLE, size_of_val(content))? };
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

    pub fn get_text(&self) -> anyhow::Result<CString> {
        anyhow::ensure!(self.format_id == ClipboardFormat::Text.id(), "Unexpected data format.");
        let hglob = HGLOBAL(self.content.0);
        let content = unsafe { windows::core::PWSTR(GlobalLock(hglob).cast()) };
        let cstr = copy_from_wide_string(unsafe { content.as_wide() })?;
        unsafe { GlobalUnlock(hglob)? };
        Ok(cstr)
    }

    pub fn get_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let hglob = HGLOBAL(self.content.0);
        let content = unsafe { GlobalLock(hglob) };
        let len = unsafe { GlobalSize(hglob) };
        let vec = unsafe { std::slice::from_raw_parts(content.cast(), len) }.to_vec();
        unsafe { GlobalUnlock(hglob)? };
        Ok(vec)
    }

    pub fn get_file_list(&self) -> anyhow::Result<Vec<CString>> {
        anyhow::ensure!(self.format_id == ClipboardFormat::FileList.id(), "Unexpected data format");
        let hglob = HGLOBAL(self.content.0);
        let content = unsafe { GlobalLock(hglob) };
        let num_files = unsafe { DragQueryFileW(HDROP(content), u32::MAX, None) };
        let mut files = Vec::with_capacity(num_files.try_into()?);
        for i in 0..num_files {
            let file_name_len = unsafe { DragQueryFileW(HDROP(content), i, None) };
            anyhow::ensure!(file_name_len != 0, windows::core::Error::from_thread());
            let buffer_len = usize::try_from(file_name_len)? + 1;
            let mut buffer = vec![0u16; buffer_len];
            let file_name_len = unsafe { DragQueryFileW(HDROP(content), i, Some(buffer.as_mut_slice())) };
            anyhow::ensure!(file_name_len != 0, windows::core::Error::from_thread());
            files.push(copy_from_wide_string(buffer.as_slice())?);
        }
        unsafe { GlobalUnlock(hglob)? };
        Ok(files)
    }
}
