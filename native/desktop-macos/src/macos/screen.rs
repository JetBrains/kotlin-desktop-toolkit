use core::{panic, slice};
use std::ffi::{c_void, CStr, CString};

use anyhow::Context;
use log::warn;
use objc2::{
    rc::{autoreleasepool, Retained},
    runtime::Bool,
    ClassType,
};
use objc2_app_kit::{NSApplication, NSScreen};
use objc2_foundation::{MainThreadMarker, NSNotificationCenter, NSNumber, NSObjectNSKeyValueObserverRegistration, NSString};

use crate::{
    common::{ArraySize, LogicalPixels, LogicalPoint, LogicalRect, LogicalSize, StrPtr},
    logger::{ffi_boundary, PanicDefault},
};

use super::string::copy_to_c_string;

pub type ScreenId = u32;

#[repr(C)]
pub struct ScreenInfo {
    pub screen_id: ScreenId,
    pub is_primary: bool,
    pub name: StrPtr,
    // relative to primary screen
    pub origin: LogicalPoint,
    pub size: LogicalSize,
    pub scale: f64,
    // todo color space?
    // todo stable uuid?
}

impl Drop for ScreenInfo {
    fn drop(&mut self) {
        let name = unsafe { CString::from_raw(self.name) };
        std::mem::drop(name);
    }
}

#[repr(C)]
pub struct ScreenInfoArray {
    pub ptr: *mut ScreenInfo,
    pub len: ArraySize,
}

impl ScreenInfoArray {
    fn new(screen_infos: Vec<ScreenInfo>) -> Self {
        let screen_infos = Vec::leak(screen_infos);
        return ScreenInfoArray {
            ptr: screen_infos.as_mut_ptr(),
            len: screen_infos.len().try_into().unwrap(),
        };
    }
}

impl Drop for ScreenInfoArray {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            let screen_infos = unsafe {
                let s = slice::from_raw_parts_mut(self.ptr, self.len.try_into().unwrap());
                Box::from_raw(s)
            };
            std::mem::drop(screen_infos);
        } else {
            warn!("Got null pointer in ScreenInfoArray")
        }
    }
}

impl PanicDefault for ScreenInfoArray {
    fn default() -> Self {
        ScreenInfoArray {
            ptr: std::ptr::null_mut(),
            len: 0,
        }
    }
}

#[no_mangle]
pub extern "C" fn screen_list() -> ScreenInfoArray {
    ffi_boundary("screen_list", || {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let screen_infos: Vec<_> = autoreleasepool(|pool| {
            NSScreen::screens(mtm)
                .iter()
                .enumerate()
                .map(|(num, screen)| {
                    let name = unsafe { screen.localizedName() };
                    let rect = screen.rect(mtm).unwrap();
                    ScreenInfo {
                        screen_id: screen.screen_id(),
                        // The screen containing the menu bar is always the first object (index 0) in the array returned by the screens method.
                        is_primary: num == 0,
                        name: copy_to_c_string(&name, pool).unwrap(),
                        origin: rect.origin,
                        size: rect.size,
                        scale: screen.backingScaleFactor(),
                    }
                })
                .collect()
        });
        Ok(ScreenInfoArray::new(screen_infos))
    })
}

#[no_mangle]
pub extern "C" fn screen_list_drop(arr: ScreenInfoArray) {
    ffi_boundary("screen_list_drop", || {
        std::mem::drop(arr);
        Ok(())
    })
}

#[no_mangle]
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

        return screen_id.unsignedIntValue();
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
        return self;
    }
}
