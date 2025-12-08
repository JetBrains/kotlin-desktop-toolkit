use anyhow::Context;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::NSImage;
use objc2_foundation::NSData;

#[derive(Debug)]
#[repr(C)]
pub struct Image {
    data: *mut u8,
    data_length: u64,
}

impl Image {
    pub(crate) fn to_ns_image(&self, mtm: MainThreadMarker) -> anyhow::Result<Retained<NSImage>> {
        let bytes = unsafe { std::slice::from_raw_parts_mut(self.data, self.data_length.try_into()?) };
        let data = NSData::with_bytes(bytes);
        NSImage::initWithData(mtm.alloc(), &data).context("Can't create image from data")
    }
}
