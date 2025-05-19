use objc2::rc::Retained;
use objc2_foundation::{NSString, NSURL};

pub(crate) fn url_to_file_path_string(url: &NSURL) -> Option<Retained<NSString>> {
    unsafe { url.filePathURL() }.map(|url| unsafe { url.path().expect("Expected path here") })
}

pub(crate) fn url_to_absolute_string(url: &NSURL) -> Option<Retained<NSString>> {
    unsafe { url.absoluteString() }
}
