use windows::Win32::{
    Foundation::HGLOBAL,
    System::Memory::{GMEM_FIXED, GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock},
};
use windows_core::Result as WinResult;

pub struct HGlobalData {
    mem: HGLOBAL,
}

impl HGlobalData {
    fn new<T: Copy>(content: &[T]) -> anyhow::Result<Self> {
        let mem = unsafe { GlobalAlloc(GMEM_MOVEABLE, size_of_val(content))? };
        unsafe {
            let data: *mut T = GlobalLock(mem).cast();
            core::ptr::copy_nonoverlapping(content.as_ptr(), data, content.len());
        }
        global_unlock(mem)?;
        Ok(Self { mem })
    }

    pub fn copy_from(mem: windows::Win32::Foundation::HANDLE) -> anyhow::Result<Self> {
        let mem = global_mem_copy(HGLOBAL(mem.0))?;
        Ok(Self { mem })
    }

    pub fn copied(&self) -> WinResult<HGLOBAL> {
        global_mem_copy(self.mem)
    }

    #[must_use]
    pub const fn as_raw(&self) -> HGLOBAL {
        self.mem
    }
}

pub(crate) mod hglobal_writer {
    use windows::{
        ApplicationModel::DataTransfer::HtmlFormatHelper,
        Win32::{
            Foundation::POINT,
            System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock},
            UI::Shell::DROPFILES,
        },
    };
    use windows_core::HSTRING;

    use super::{HGlobalData, global_unlock};
    use crate::win32::strings::copy_from_wide_string;

    pub fn new_text(text: &str) -> anyhow::Result<HGlobalData> {
        let content: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        HGlobalData::new(&content)
    }

    pub fn new_bytes(content: &[u8]) -> anyhow::Result<HGlobalData> {
        HGlobalData::new(content)
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
        let mem = unsafe { GlobalAlloc(GMEM_MOVEABLE, content_len)? };
        unsafe {
            let data = GlobalLock(mem);
            data.cast::<DROPFILES>().write(header);
            core::ptr::copy_nonoverlapping(
                files_data.as_ptr(),
                data.add(size_of::<DROPFILES>()).cast::<u16>(),
                files_data.len(),
            );
        }
        global_unlock(mem)?;
        Ok(HGlobalData { mem })
    }

    pub fn new_html(content: &HSTRING) -> anyhow::Result<HGlobalData> {
        let html_format = HtmlFormatHelper::CreateHtmlFormat(content)?;
        let cstr = copy_from_wide_string(&html_format)?;
        HGlobalData::new(cstr.to_bytes_with_nul())
    }
}

pub(crate) mod hglobal_reader {
    use std::ffi::CString;

    use windows::{
        ApplicationModel::DataTransfer::HtmlFormatHelper,
        Win32::{
            System::Memory::{GlobalLock, GlobalSize},
            UI::Shell::{DragQueryFileW, HDROP},
        },
    };
    use windows_core::{Error as WinError, PWSTR};

    use super::{HGlobalData, global_unlock};
    use crate::win32::strings::{copy_from_utf8_bytes, copy_from_wide_string};

    pub fn get_text(data: &HGlobalData) -> anyhow::Result<CString> {
        let content = unsafe { PWSTR(GlobalLock(data.mem).cast()) };
        let cstr = copy_from_wide_string(unsafe { content.as_wide() })?;
        global_unlock(data.mem)?;
        Ok(cstr)
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
        let num_files = unsafe { DragQueryFileW(HDROP(content), u32::MAX, None) };
        let mut files = Vec::with_capacity(num_files.try_into()?);
        for i in 0..num_files {
            let file_name_len = unsafe { DragQueryFileW(HDROP(content), i, None) };
            anyhow::ensure!(file_name_len != 0, WinError::from_thread());
            let mut buffer = vec![0u16; usize::try_from(file_name_len)? + 1];
            let file_name_len = unsafe { DragQueryFileW(HDROP(content), i, Some(&mut buffer)) };
            anyhow::ensure!(file_name_len != 0, WinError::from_thread());
            files.push(copy_from_wide_string(&buffer)?);
        }
        global_unlock(data.mem)?;
        Ok(files)
    }

    pub fn get_html(data: &HGlobalData) -> anyhow::Result<CString> {
        let utf8_bytes = get_bytes(data)?;
        let html_format = copy_from_utf8_bytes(&utf8_bytes)?;
        let fragment = HtmlFormatHelper::GetStaticFragment(&html_format)?;
        copy_from_wide_string(&fragment)
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
