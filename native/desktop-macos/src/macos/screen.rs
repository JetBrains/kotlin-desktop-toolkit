use anyhow::Context;
use objc2::rc::Retained;
use objc2_app_kit::NSScreen;
use objc2_foundation::{MainThreadMarker, NSNumber, NSString};

use crate::{
    common::{AutoDropArray, AutoDropStrPtr, LogicalPixels, LogicalPoint, LogicalRect, LogicalSize},
    logger::{PanicDefault, ffi_boundary},
};

use super::string::copy_to_c_string;

pub type ScreenId = u32;

#[repr(C)]
pub struct ScreenInfo {
    pub screen_id: ScreenId,
    pub is_primary: bool,
    pub name: AutoDropStrPtr,
    // relative to primary screen
    pub origin: LogicalPoint,
    pub size: LogicalSize,
    pub scale: f64,
    pub maximum_frames_per_second: u32,
    // todo color space?
    // todo stable uuid?
}

impl<T> PanicDefault for AutoDropArray<T> {
    fn default() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            len: 0,
        }
    }
}

type ScreenInfoArray = AutoDropArray<ScreenInfo>;

#[unsafe(no_mangle)]
pub extern "C" fn screen_list() -> ScreenInfoArray {
    ffi_boundary("screen_list", || {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let screen_infos: Box<_> = NSScreen::screens(mtm)
            .iter()
            .enumerate()
            .map(|(num, screen)| {
                let name = unsafe { screen.localizedName() };
                let rect = screen.rect(mtm).unwrap();
                ScreenInfo {
                    screen_id: screen.screen_id(),
                    // The screen containing the menu bar is always the first object (index 0) in the array returned by the screens method.
                    is_primary: num == 0,
                    name: AutoDropStrPtr(copy_to_c_string(&name).unwrap()),
                    origin: rect.origin,
                    size: rect.size,
                    scale: screen.backingScaleFactor(),
                    maximum_frames_per_second: unsafe { screen.maximumFramesPerSecond() }.try_into().unwrap(),
                }
            })
            .collect();
        Ok(ScreenInfoArray::new(screen_infos))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn screen_list_drop(arr: ScreenInfoArray) {
    ffi_boundary("screen_list_drop", || {
        std::mem::drop(arr);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn screen_get_main_screen_id() -> ScreenId {
    ffi_boundary("screen_get_main_screen_id", || {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        Ok(NSScreen::mainScreen(mtm).context("No main screen")?.screen_id())
    })
}

pub(crate) trait NSScreenExts {
    fn me(&self) -> &NSScreen;

    fn primary(mtm: MainThreadMarker) -> anyhow::Result<Retained<NSScreen>> {
        NSScreen::screens(mtm).firstObject().context("Screen list is empty")
    }

    fn screen_id(&self) -> ScreenId {
        let screen_id = self
            .me()
            .deviceDescription()
            .objectForKey(&*NSString::from_str("NSScreenNumber"))
            .unwrap();
        let screen_id: Retained<NSNumber> = Retained::downcast(screen_id).unwrap();

        screen_id.unsignedIntValue()
    }

    #[allow(dead_code)]
    fn width(&self) -> LogicalPixels {
        self.me().frame().size.width
    }

    fn height(&self) -> LogicalPixels {
        self.me().frame().size.height
    }

    fn rect(&self, mtm: MainThreadMarker) -> anyhow::Result<LogicalRect> {
        let height = Self::primary(mtm)?.frame().size.height;
        let rect = LogicalRect::from_macos_coords(self.me().frame(), height);
        Ok(rect)
    }
}

impl NSScreenExts for NSScreen {
    fn me(&self) -> &NSScreen {
        self
    }
}
