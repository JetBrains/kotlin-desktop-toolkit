use std::ffi::CString;

use windows::Win32::{
    Foundation::{GlobalFree, HGLOBAL},
    System::Memory::{GMEM_FIXED, GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock},
    UI::Shell::{DragQueryFileW, HDROP},
};
use windows_core::{Error as WinError, Result as WinResult};

use super::strings::copy_from_wide_string;

pub struct HGlobalData {
    mem: HGLOBAL,
    is_owned: bool,
}

unsafe impl Send for HGlobalData {}
unsafe impl Sync for HGlobalData {}

impl HGlobalData {
    pub fn alloc_and_init<F: FnOnce(*mut core::ffi::c_void)>(content_len: usize, init: F) -> anyhow::Result<Self> {
        let mem = unsafe { GlobalAlloc(GMEM_MOVEABLE, content_len)? };
        let data = unsafe { GlobalLock(mem) };
        init(data);
        global_unlock(mem)?;
        Ok(Self { mem, is_owned: true })
    }

    pub fn alloc_from<T: Copy>(content: &[T]) -> anyhow::Result<Self> {
        Self::alloc_and_init(size_of_val(content), |data| unsafe {
            core::ptr::copy_nonoverlapping(content.as_ptr(), data.cast::<T>(), content.len());
        })
    }

    pub fn copy_from(mem: windows::Win32::Foundation::HANDLE) -> anyhow::Result<Self> {
        let mem = global_mem_copy(HGLOBAL(mem.0))?;
        Ok(Self { mem, is_owned: true })
    }

    #[inline]
    pub fn copied(&self) -> WinResult<HGLOBAL> {
        global_mem_copy(self.mem)
    }

    #[inline]
    pub const fn detach(&mut self) {
        self.is_owned = false;
    }

    #[inline]
    #[must_use]
    pub const fn as_raw(&self) -> HGLOBAL {
        self.mem
    }
}

impl Drop for HGlobalData {
    fn drop(&mut self) {
        if self.is_owned {
            let _ = unsafe { GlobalFree(Some(self.mem)) };
            self.mem = HGLOBAL::default();
            self.is_owned = false;
        }
    }
}

#[inline]
fn global_mem_copy(mem: HGLOBAL) -> WinResult<HGLOBAL> {
    let size = unsafe { GlobalSize(mem) };
    let source = unsafe { GlobalLock(mem) };
    let dest = unsafe { GlobalAlloc(GMEM_FIXED, size)? };
    unsafe { core::ptr::copy_nonoverlapping(source, dest.0, size) };
    global_unlock(mem)?;
    Ok(dest)
}

#[inline]
fn global_unlock(mem: HGLOBAL) -> WinResult<()> {
    unsafe { GlobalUnlock(mem) }.or_else(|err| if err.code().is_ok() { Ok(()) } else { Err(err) })
}

pub(crate) mod hglobal_writer {
    use windows::{
        ApplicationModel::DataTransfer::HtmlFormatHelper,
        Win32::{Foundation::POINT, UI::Shell::DROPFILES},
    };
    use windows_core::HSTRING;

    use crate::win32::{global_data::HGlobalData, strings::copy_from_wide_string};

    pub fn new_text(text: &str) -> anyhow::Result<HGlobalData> {
        let content: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        HGlobalData::alloc_from(&content)
    }

    pub fn new_bytes(content: &[u8]) -> anyhow::Result<HGlobalData> {
        HGlobalData::alloc_from(content)
    }

    pub fn new_file_list(file_names: &Vec<&str>) -> anyhow::Result<HGlobalData> {
        let header = DROPFILES {
            pFiles: size_of::<DROPFILES>().try_into()?,
            pt: POINT { x: 0, y: 0 },
            fNC: windows::Win32::Foundation::FALSE,
            fWide: windows::Win32::Foundation::TRUE,
        };
        let files_data: Vec<u16> = file_names.join("\0").encode_utf16().chain(std::iter::repeat_n(0, 2)).collect();
        let content_len = size_of::<DROPFILES>() + (files_data.len() * size_of::<u16>());
        HGlobalData::alloc_and_init(content_len, |data| unsafe {
            data.cast::<DROPFILES>().write(header);
            core::ptr::copy_nonoverlapping(
                files_data.as_ptr(),
                data.add(size_of::<DROPFILES>()).cast::<u16>(),
                files_data.len(),
            );
        })
    }

    pub fn new_html(content: &HSTRING) -> anyhow::Result<HGlobalData> {
        let html_format = HtmlFormatHelper::CreateHtmlFormat(content)?;
        let cstr = copy_from_wide_string(&html_format)?;
        HGlobalData::alloc_from(cstr.to_bytes_with_nul())
    }
}

pub(crate) mod hglobal_reader {
    use std::ffi::CString;

    use windows::{
        ApplicationModel::DataTransfer::HtmlFormatHelper,
        Win32::{
            System::Memory::{GlobalLock, GlobalSize},
            UI::Shell::HDROP,
        },
    };
    use windows_core::PWSTR;

    use crate::win32::{
        global_data::{HGlobalData, global_unlock, parse_file_list},
        strings::{copy_from_utf8_bytes, copy_from_wide_string},
    };

    pub fn get_text(data: &HGlobalData) -> anyhow::Result<CString> {
        let content = unsafe { PWSTR(GlobalLock(data.mem).cast()) };
        let cstr = copy_from_wide_string(unsafe { content.as_wide() });
        global_unlock(data.mem)?;
        cstr
    }

    pub fn get_bytes(data: &HGlobalData) -> anyhow::Result<Vec<u8>> {
        let len = unsafe { GlobalSize(data.mem) };
        let content = unsafe { GlobalLock(data.mem) };
        let vec = unsafe { core::slice::from_raw_parts(content.cast(), len) }.to_vec();
        global_unlock(data.mem)?;
        Ok(vec)
    }

    pub fn get_file_list(data: &HGlobalData) -> anyhow::Result<Vec<CString>> {
        let content = unsafe { GlobalLock(data.mem) };
        let files = unsafe { parse_file_list(HDROP(content)) };
        global_unlock(data.mem)?;
        files
    }

    pub fn get_html(data: &HGlobalData) -> anyhow::Result<CString> {
        let utf8_bytes = get_bytes(data)?;
        let html_format = copy_from_utf8_bytes(&utf8_bytes)?;
        let fragment = HtmlFormatHelper::GetStaticFragment(&html_format)?;
        copy_from_wide_string(&fragment)
    }
}

/// # Safety
/// `hdrop` must point to a valid `DROPFILES` structure followed by the double-NUL-terminated file list.
pub(crate) unsafe fn parse_file_list(hdrop: HDROP) -> anyhow::Result<Vec<CString>> {
    let num_files = unsafe { DragQueryFileW(hdrop, u32::MAX, None) };
    let mut files = Vec::with_capacity(num_files.try_into()?);
    for i in 0..num_files {
        let file_name_len = unsafe { DragQueryFileW(hdrop, i, None) };
        anyhow::ensure!(file_name_len != 0, WinError::from_thread());
        let mut buffer = vec![0u16; usize::try_from(file_name_len)? + 1];
        let file_name_len = unsafe { DragQueryFileW(hdrop, i, Some(&mut buffer)) };
        anyhow::ensure!(file_name_len != 0, WinError::from_thread());
        files.push(copy_from_wide_string(&buffer)?);
    }
    Ok(files)
}
