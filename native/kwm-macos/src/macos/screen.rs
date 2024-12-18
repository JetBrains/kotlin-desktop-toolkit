use objc2::rc::Retained;
use objc2_app_kit::NSScreen;
use objc2_foundation::{NSNumber, NSString};


pub type ScreenId = u32;

pub(crate) trait NSScreenExts {
    fn screen_id(&self) -> ScreenId;
}

impl NSScreenExts for NSScreen {
    fn screen_id(&self) -> ScreenId {
        return unsafe {
            let screen_id = self.deviceDescription().objectForKey(&*NSString::from_str("NSScreenNumber")).unwrap();
            let screen_id: Retained<NSNumber> = Retained::cast(screen_id);
            screen_id.unsignedIntValue()
        }
    }
}