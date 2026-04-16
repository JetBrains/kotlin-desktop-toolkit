use std::sync::LazyLock;

use windows::Win32::System::{
    DataExchange::RegisterClipboardFormatW,
    Ole::{CF_HDROP, CF_UNICODETEXT},
};
use windows_core::{HSTRING, h};

/// cbindgen:ignore
static HTML_FORMAT: LazyLock<u32> = LazyLock::new(|| register_format(h!("HTML Format")));

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataFormat {
    Text,
    FileList,
    HtmlFragment,
    Other(u32),
}

impl DataFormat {
    #[inline]
    #[must_use]
    pub fn id(self) -> u32 {
        match self {
            Self::Text => u32::from(CF_UNICODETEXT.0),
            Self::FileList => u32::from(CF_HDROP.0),
            Self::HtmlFragment => *HTML_FORMAT,
            Self::Other(id) => id,
        }
    }
}

#[must_use]
pub fn register_format(format_name: &HSTRING) -> u32 {
    unsafe { RegisterClipboardFormatW(format_name) }
}
