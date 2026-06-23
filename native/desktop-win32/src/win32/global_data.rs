use std::ffi::CString;

use windows::Win32::{
    Foundation::{E_POINTER, ERROR_SUCCESS, GetLastError, GlobalFree, HGLOBAL, SetLastError, WIN32_ERROR},
    System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock},
    UI::Shell::{DragQueryFileW, HDROP},
};
use windows_core::{Error as WinError, Result as WinResult};

use super::{clipboard_result::ClipboardFailure, strings::copy_from_wide_string};

/// Matches Chromium's defensive clipboard payload cap.
/// cbindgen:ignore
pub(crate) const MAX_CLIPBOARD_DATA_BYTES: usize = 256 * 1024 * 1024;

pub(crate) fn ensure_clipboard_data_size(size: usize) -> anyhow::Result<()> {
    if size > MAX_CLIPBOARD_DATA_BYTES {
        return Err(ClipboardFailure::data_too_large(size, MAX_CLIPBOARD_DATA_BYTES).into());
    }
    Ok(())
}

pub(crate) fn invalid_clipboard_data(message: impl Into<String>) -> anyhow::Error {
    ClipboardFailure::invalid_data(message).into()
}

pub struct HGlobalData {
    mem: HGLOBAL,
    is_owned: bool,
}

unsafe impl Send for HGlobalData {}
unsafe impl Sync for HGlobalData {}

impl HGlobalData {
    pub fn alloc_and_init<F: FnOnce(*mut core::ffi::c_void)>(content_len: usize, init: F) -> anyhow::Result<Self> {
        ensure_clipboard_data_size(content_len)?;
        let mem = unsafe { GlobalAlloc(GMEM_MOVEABLE, content_len)? };
        if content_len != 0 {
            let data = match global_lock(mem) {
                Ok(data) => data,
                Err(err) => {
                    let _ = unsafe { GlobalFree(Some(mem)) };
                    return Err(err.into());
                }
            };
            init(data);
            if let Err(err) = global_unlock(mem) {
                let _ = unsafe { GlobalFree(Some(mem)) };
                return Err(err.into());
            }
        }
        Ok(Self { mem, is_owned: true })
    }

    pub fn alloc_from<T: Copy>(content: &[T]) -> anyhow::Result<Self> {
        Self::alloc_and_init(size_of_val(content), |data| unsafe {
            core::ptr::copy_nonoverlapping(content.as_ptr(), data.cast::<T>(), content.len());
        })
    }

    pub fn copy_from(mem: windows::Win32::Foundation::HANDLE) -> anyhow::Result<Self> {
        let mem = HGLOBAL(mem.0);
        let size = global_size(mem)?;
        ensure_clipboard_data_size(size)?;
        let mem = global_mem_copy(mem, size)?;
        Ok(Self { mem, is_owned: true })
    }

    #[inline]
    pub fn copied(&self) -> WinResult<HGLOBAL> {
        let size = global_size(self.mem)?;
        global_mem_copy(self.mem, size)
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
fn global_mem_copy(mem: HGLOBAL, size: usize) -> WinResult<HGLOBAL> {
    if mem.0.is_null() {
        return Err(WinError::from(E_POINTER));
    }

    if size == 0 {
        return unsafe { GlobalAlloc(GMEM_MOVEABLE, size) };
    }

    let source = global_lock(mem)?;
    let dest = match unsafe { GlobalAlloc(GMEM_MOVEABLE, size) } {
        Ok(dest) => dest,
        Err(err) => {
            let _ = global_unlock(mem);
            return Err(err);
        }
    };
    let dest_ptr = match global_lock(dest) {
        Ok(dest_ptr) => dest_ptr,
        Err(err) => {
            let _ = global_unlock(mem);
            let _ = unsafe { GlobalFree(Some(dest)) };
            return Err(err);
        }
    };
    unsafe { core::ptr::copy_nonoverlapping(source, dest_ptr, size) };
    let source_unlock_result = global_unlock(mem);
    let dest_unlock_result = global_unlock(dest);
    if let Err(err) = source_unlock_result {
        let _ = unsafe { GlobalFree(Some(dest)) };
        return Err(err);
    }
    if let Err(err) = dest_unlock_result {
        let _ = unsafe { GlobalFree(Some(dest)) };
        return Err(err);
    }
    Ok(dest)
}

#[inline]
fn global_size(mem: HGLOBAL) -> WinResult<usize> {
    unsafe { SetLastError(WIN32_ERROR(0)) };
    let size = unsafe { GlobalSize(mem) };
    if size == 0 {
        let err = unsafe { GetLastError() };
        if err != ERROR_SUCCESS {
            return Err(WinError::from(err));
        }
    }
    Ok(size)
}

#[inline]
fn global_lock(mem: HGLOBAL) -> WinResult<*mut core::ffi::c_void> {
    let data = unsafe { GlobalLock(mem) };
    if data.is_null() { Err(WinError::from_thread()) } else { Ok(data) }
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

    pub fn new_file_list(file_names: &[&str]) -> anyhow::Result<HGlobalData> {
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

    use windows::{ApplicationModel::DataTransfer::HtmlFormatHelper, Win32::UI::Shell::HDROP};

    use crate::win32::{
        global_data::{HGlobalData, ensure_clipboard_data_size, global_size, global_unlock, parse_file_list},
        strings::{copy_from_utf8_bytes, copy_from_wide_string},
    };

    pub fn get_text(data: &HGlobalData) -> anyhow::Result<CString> {
        let bytes = get_bytes(data)?;
        let (chunks, []) = bytes.as_chunks::<2>() else {
            return Err(super::invalid_clipboard_data("UTF-16 clipboard data has odd byte length"));
        };
        let wide: Vec<u16> = chunks.iter().map(|&pair| u16::from_le_bytes(pair)).collect();
        let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
        copy_from_wide_string(&wide[..len]).map_err(|err| super::invalid_clipboard_data(format!("invalid UTF-16 clipboard data: {err}")))
    }

    pub fn get_bytes(data: &HGlobalData) -> anyhow::Result<Vec<u8>> {
        let len = global_size(data.mem)?;
        ensure_clipboard_data_size(len)?;
        if len == 0 {
            return Ok(Vec::new());
        }
        let content = super::global_lock(data.mem)?;
        let vec = unsafe { core::slice::from_raw_parts(content.cast(), len) }.to_vec();
        // Always unlock, but don't let a GlobalUnlock error (only possible on lock-count
        // underflow) mask successfully read bytes; surface the data either way.
        if let Err(unlock_err) = global_unlock(data.mem) {
            log::warn!("failed to unlock HGLOBAL after reading bytes: {unlock_err:?}");
        }
        Ok(vec)
    }

    pub fn get_file_list(data: &HGlobalData) -> anyhow::Result<Vec<CString>> {
        let content = super::global_lock(data.mem)?;
        let files = unsafe { parse_file_list(HDROP(content)) };
        // Always unlock, but don't let a GlobalUnlock error (only possible on lock-count
        // underflow) mask a successfully parsed list; surface the parse result either way.
        if let Err(unlock_err) = global_unlock(data.mem) {
            log::warn!("failed to unlock DROPFILES HGLOBAL: {unlock_err:?}");
        }
        files
    }

    pub fn get_html(data: &HGlobalData) -> anyhow::Result<CString> {
        let utf8_bytes = get_bytes(data)?;
        let html_format = copy_from_utf8_bytes(&utf8_bytes)
            .map_err(|err| super::invalid_clipboard_data(format!("invalid HTML clipboard data: {err}")))?;
        let fragment = HtmlFormatHelper::GetStaticFragment(&html_format)
            .map_err(|err| super::invalid_clipboard_data(format!("invalid HTML clipboard data: {err:?}")))?;
        copy_from_wide_string(&fragment).map_err(|err| super::invalid_clipboard_data(format!("invalid HTML clipboard data: {err}")))
    }
}

/// # Safety
/// `hdrop` must point to a valid `DROPFILES` structure followed by the double-NUL-terminated file list.
pub(crate) unsafe fn parse_file_list(hdrop: HDROP) -> anyhow::Result<Vec<CString>> {
    let num_files = unsafe { DragQueryFileW(hdrop, u32::MAX, None) };
    let mut files = Vec::with_capacity(num_files.try_into()?);
    for i in 0..num_files {
        let file_name_len = unsafe { DragQueryFileW(hdrop, i, None) };
        if file_name_len == 0 {
            return Err(invalid_clipboard_data(format!("DROPFILES entry {i} has an empty file name")));
        }
        let mut buffer = vec![0u16; usize::try_from(file_name_len)? + 1];
        let file_name_len = unsafe { DragQueryFileW(hdrop, i, Some(&mut buffer)) };
        if file_name_len == 0 {
            return Err(invalid_clipboard_data(format!("failed to read DROPFILES entry {i}")));
        }
        files.push(copy_from_wide_string(&buffer).map_err(|err| invalid_clipboard_data(format!("invalid DROPFILES entry {i}: {err}")))?);
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use crate::win32::clipboard_result::{ClipboardFailure, ClipboardStatus};

    use super::{HGlobalData, MAX_CLIPBOARD_DATA_BYTES, hglobal_reader, hglobal_writer};

    #[test]
    fn get_text_accepts_missing_trailing_nul() {
        let data = HGlobalData::alloc_from(&[u16::from(b'H'), u16::from(b'i')]).unwrap();

        let text = hglobal_reader::get_text(&data).unwrap();

        assert_eq!(text.to_str().unwrap(), "Hi");
    }

    #[test]
    fn get_text_rejects_odd_utf16_byte_count() {
        let data = hglobal_writer::new_bytes(b"H").unwrap();

        let err = hglobal_reader::get_text(&data).unwrap_err();

        assert!(err.to_string().contains("odd byte length"));
        let failure = err.downcast_ref::<ClipboardFailure>().expect("expected clipboard failure");
        assert_eq!(failure.status(), ClipboardStatus::InvalidData);
    }

    #[test]
    fn get_bytes_accepts_empty_global() {
        let data = hglobal_writer::new_bytes(&[]).unwrap();

        let bytes = hglobal_reader::get_bytes(&data).unwrap();

        assert!(bytes.is_empty());
    }

    #[test]
    fn oversized_global_allocation_is_rejected_before_allocating() {
        let result = HGlobalData::alloc_and_init(MAX_CLIPBOARD_DATA_BYTES + 1, |_| {});
        assert!(result.is_err());
        let err = result.err().unwrap();

        assert!(err.to_string().contains("too large"));
    }

    #[test]
    fn copied_data_remains_readable_after_original_drops() {
        let data = hglobal_writer::new_bytes(b"clipboard").unwrap();
        let copied = HGlobalData {
            mem: data.copied().unwrap(),
            is_owned: true,
        };
        drop(data);

        let bytes = hglobal_reader::get_bytes(&copied).unwrap();

        assert_eq!(bytes, b"clipboard");
    }
}
