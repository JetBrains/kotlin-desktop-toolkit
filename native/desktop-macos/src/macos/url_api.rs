use crate::macos::string::{copy_to_c_string, copy_to_ns_string};
use desktop_common::ffi_utils::{BorrowedStrPtr, RustAllocatedStrPtr};
use desktop_common::logger::ffi_boundary;
use log::debug;
use objc2_foundation::NSURL;

/// Converts a file path to a file URL string.
/// Returns NULL if the path is invalid.
#[unsafe(no_mangle)]
pub extern "C" fn url_file_path_to_file_url(file_path: BorrowedStrPtr) -> RustAllocatedStrPtr {
    ffi_boundary("url_file_path_to_file_url", || {
        let path_str = file_path.as_str()?;
        let ns_path = copy_to_ns_string(&file_path)?;
        let url = NSURL::fileURLWithPath(&ns_path);
        let Some(url_string) = url.absoluteString() else {
            debug!("url_file_path_to_file_url: failed to get absoluteString for path '{path_str}'");
            return Ok(RustAllocatedStrPtr::null());
        };
        copy_to_c_string(&url_string)
    })
}

/// Converts a URL string to a file path.
///
/// Works with both regular file URLs (`file://`) and file reference URLs (`file:///.file/id=`).
/// Returns NULL if the URL is invalid or cannot be interpreted as a file path.
#[unsafe(no_mangle)]
pub extern "C" fn url_to_file_path(url: BorrowedStrPtr) -> RustAllocatedStrPtr {
    ffi_boundary("url_to_file_path", || {
        let url_str = url.as_str()?;
        let ns_url_string = copy_to_ns_string(&url)?;
        let ns_url =
            NSURL::URLWithString(&ns_url_string).ok_or_else(|| anyhow::format_err!("url_to_file_path: invalid URL string '{url_str}'"))?;
        let Some(file_path_url) = ns_url.filePathURL() else {
            debug!("url_to_file_path: URL cannot be interpreted as file path '{url_str}'");
            return Ok(RustAllocatedStrPtr::null());
        };
        let Some(path) = file_path_url.path() else {
            debug!("url_to_file_path: resolved URL has no path '{url_str}'");
            return Ok(RustAllocatedStrPtr::null());
        };
        copy_to_c_string(&path)
    })
}

/// Converts a file path to a file reference URL string.
///
/// File reference URLs use a unique identifier for the file that persists across renames/moves.
/// Returns NULL if the path is invalid or the file doesn't exist.
#[unsafe(no_mangle)]
pub extern "C" fn url_file_path_to_file_reference_url(file_path: BorrowedStrPtr) -> RustAllocatedStrPtr {
    ffi_boundary("url_file_path_to_file_reference_url", || {
        let path_str = file_path.as_str()?;
        let ns_path = copy_to_ns_string(&file_path)?;
        let file_url = NSURL::fileURLWithPath(&ns_path);
        let Some(reference_url) = file_url.fileReferenceURL() else {
            debug!("url_file_path_to_file_reference_url: failed to create file reference URL for '{path_str}' (file may not exist)");
            return Ok(RustAllocatedStrPtr::null());
        };
        let Some(url_string) = reference_url.absoluteString() else {
            debug!("url_file_path_to_file_reference_url: failed to get absoluteString for '{path_str}'");
            return Ok(RustAllocatedStrPtr::null());
        };
        copy_to_c_string(&url_string)
    })
}
