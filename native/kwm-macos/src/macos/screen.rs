use core::slice;
use std::ffi::{c_void, CStr, CString};

use objc2::{
    rc::{autoreleasepool, Retained},
    runtime::Bool,
};
use objc2_app_kit::{NSApplication, NSScreen};
use objc2_foundation::{MainThreadMarker, NSNotificationCenter, NSNumber, NSObjectNSKeyValueObserverRegistration, NSString};

use crate::common::{ArraySize, LogicalPoint, LogicalSize, StrPtr};

pub type ScreenId = u32;

#[repr(C)]
pub struct ScreenInfo {
    pub screen_id: ScreenId,
    pub is_main: bool,
    pub name: StrPtr,
    // relative to main screen
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
        let screen_infos = unsafe {
            let s = slice::from_raw_parts_mut(self.ptr, self.len.try_into().unwrap());
            Box::from_raw(s)
        };
        std::mem::drop(screen_infos);
    }
}

#[no_mangle]
pub extern "C" fn screen_list() -> ScreenInfoArray {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let screen_infos: Vec<_> = autoreleasepool(|pool| {
        NSScreen::screens(mtm)
            .iter()
            .enumerate()
            .map(|(num, screen)| {
                let name = unsafe { screen.localizedName() };
                let name = CString::new(name.as_str(pool)).unwrap();
                ScreenInfo {
                    screen_id: screen.screen_id(),
                    is_main: num == 0,
                    name: name.into_raw(),
                    origin: screen.frame().origin.into(),
                    size: screen.frame().size.into(),
                    scale: screen.backingScaleFactor(),
                }
            })
            .collect()
    });
    return ScreenInfoArray::new(screen_infos);
}

#[no_mangle]
pub extern "C" fn screen_list_drop(arr: ScreenInfoArray) {
    std::mem::drop(arr);
}

pub(crate) trait NSScreenExts {
    fn screen_id(&self) -> ScreenId;
}

impl NSScreenExts for NSScreen {
    fn screen_id(&self) -> ScreenId {
        return unsafe {
            let screen_id = self
                .deviceDescription()
                .objectForKey(&*NSString::from_str("NSScreenNumber"))
                .unwrap();
            let screen_id: Retained<NSNumber> = Retained::cast(screen_id);

            screen_id.unsignedIntValue()
        };
    }
}
