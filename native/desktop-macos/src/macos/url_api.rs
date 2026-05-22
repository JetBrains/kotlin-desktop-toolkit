use crate::macos::string::{copy_to_c_string, copy_to_ns_string};
use desktop_common::ffi_utils::{BorrowedStrPtr, RustAllocatedStrPtr};
use desktop_common::logger::ffi_boundary;
use log::debug;
use objc2::rc::Retained;
use objc2_foundation::{NSCharacterSet, NSMutableCopying, NSString, NSURL, ns_string};

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

        // Try to resolve the URL directly. If it fails (typically because of
        // unencoded characters such as literal spaces) percent-encode the
        // string and retry.
        //
        // We deliberately avoid `URLWithString:encodingInvalidCharacters:` —
        // it only exists on macOS 14+ (we now target older versions), and
        // its `encodingInvalidCharacters` argument is silently ignored when
        // the loading binary is linked against an SDK older than macOS 14
        // (e.g. LC_BUILD_VERSION 13.3), defeating the whole point.
        let path = resolve_file_path(&ns_url_string).or_else(|| {
            let encoded = percent_encode_url_string(&ns_url_string)?;
            resolve_file_path(&encoded)
        });

        let Some(path) = path else {
            debug!("url_to_file_path: cannot interpret as file path '{url_str}'");
            return Ok(RustAllocatedStrPtr::null());
        };
        copy_to_c_string(&path)
    })
}

fn resolve_file_path(url_string: &NSString) -> Option<Retained<NSString>> {
    NSURL::URLWithString(url_string)?.filePathURL()?.path()
}

fn percent_encode_url_string(url_string: &NSString) -> Option<Retained<NSString>> {
    // URLFragmentAllowedCharacterSet is the most permissive of the URL
    // component sets (it covers scheme delimiters, paths, and query syntax).
    // We additionally allow '%' so that any existing percent-encoded
    // sequences in the input are preserved instead of being re-encoded to
    // '%25XX'.
    let allowed = NSCharacterSet::URLFragmentAllowedCharacterSet().mutableCopy();
    allowed.addCharactersInString(ns_string!("%"));
    url_string.stringByAddingPercentEncodingWithAllowedCharacters(&allowed)
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
