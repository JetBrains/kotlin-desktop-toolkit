use std::ffi::CString;

use windows::{
    ApplicationModel::DataTransfer::HtmlFormatHelper,
    Win32::{
        Foundation::{HANDLE, HGLOBAL, POINT},
        System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock},
        UI::Shell::{DROPFILES, DragQueryFileW, HDROP},
    },
};
use windows_core::{Error as WinError, HSTRING, PWSTR, Result as WinResult};

use super::{
    clipboard::ClipboardFormat,
    strings::{copy_from_utf8_bytes, copy_from_wide_string},
};

pub struct ClipboardData {
    pub(crate) format_id: u32,
    pub(crate) content: HANDLE,
}

impl ClipboardData {
    pub fn new_text(text: &str) -> anyhow::Result<Self> {
        let content: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        Self::new(content.as_slice(), ClipboardFormat::Text.id())
    }

    pub fn new_bytes(content: &[u8], format: ClipboardFormat) -> anyhow::Result<Self> {
        Self::new(content, format.id())
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
        }
        global_unlock(mem)?;
        Ok(Self {
            format_id: ClipboardFormat::FileList.id(),
            content: HANDLE(mem.0),
        })
    }

    pub fn new_html(content: &HSTRING) -> anyhow::Result<Self> {
        let html_format = HtmlFormatHelper::CreateHtmlFormat(content)?;
        let cstr = copy_from_wide_string(&html_format)?;
        Self::new(cstr.to_bytes_with_nul(), ClipboardFormat::HtmlFragment.id())
    }

    fn new<T: Copy>(content: &[T], format_id: u32) -> anyhow::Result<Self> {
        let mem = unsafe { GlobalAlloc(GMEM_MOVEABLE, size_of_val(content))? };
        unsafe {
            let data: *mut T = GlobalLock(mem).cast();
            std::slice::from_raw_parts_mut(data, content.len()).copy_from_slice(content);
        }
        global_unlock(mem)?;
        Ok(Self {
            format_id,
            content: HANDLE(mem.0),
        })
    }

    pub fn get_text(&self) -> anyhow::Result<CString> {
        anyhow::ensure!(self.format_id == ClipboardFormat::Text.id(), "Unexpected data format.");
        let hglob = HGLOBAL(self.content.0);
        let content = unsafe { PWSTR(GlobalLock(hglob).cast()) };
        let cstr = copy_from_wide_string(unsafe { content.as_wide() })?;
        global_unlock(hglob)?;
        Ok(cstr)
    }

    pub fn get_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let hglob = HGLOBAL(self.content.0);
        let len = unsafe { GlobalSize(hglob) };
        let content = unsafe { GlobalLock(hglob) };
        let vec = unsafe { std::slice::from_raw_parts(content.cast(), len) }.to_vec();
        global_unlock(hglob)?;
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
            anyhow::ensure!(file_name_len != 0, WinError::from_thread());
            let buffer_len = usize::try_from(file_name_len)? + 1;
            let mut buffer = vec![0u16; buffer_len];
            let file_name_len = unsafe { DragQueryFileW(HDROP(content), i, Some(buffer.as_mut_slice())) };
            anyhow::ensure!(file_name_len != 0, WinError::from_thread());
            files.push(copy_from_wide_string(buffer.as_slice())?);
        }
        global_unlock(hglob)?;
        Ok(files)
    }

    pub fn get_html(&self) -> anyhow::Result<CString> {
        anyhow::ensure!(self.format_id == ClipboardFormat::HtmlFragment.id(), "Unexpected data format");
        let utf8_bytes = self.get_bytes()?;
        let html_format = copy_from_utf8_bytes(utf8_bytes.as_slice())?;
        let fragment = HtmlFormatHelper::GetStaticFragment(&html_format)?;
        copy_from_wide_string(&fragment)
    }
}

pub(crate) fn global_unlock(mem: HGLOBAL) -> WinResult<()> {
    unsafe { GlobalUnlock(mem) }.or_else(|err| if err.code().is_ok() { Ok(()) } else { Err(err) })
}
