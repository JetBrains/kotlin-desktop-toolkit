use std::ffi::c_void;
use std::ptr::NonNull;

use anyhow::Context;
use objc2::rc::Retained;
use objc2_app_kit::NSScreen;
use objc2_core_foundation::{CFRetained, CFUUID};
use objc2_foundation::{MainThreadMarker, NSNumber, ns_string};

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGDisplayCreateUUIDFromDisplayID(display: u32) -> *const c_void;
}

use desktop_common::{
    ffi_utils::{AutoDropArray, AutoDropStrPtr, RustAllocatedStrPtr},
    logger::ffi_boundary,
};

use crate::geometry::{LogicalPixels, LogicalPoint, LogicalRect, LogicalSize};

use super::string::copy_to_c_string;

pub type ScreenId = u32;

#[repr(C)]
pub struct ScreenInfo {
    pub screen_id: ScreenId,
    pub is_primary: bool,
    pub name: AutoDropStrPtr,
    /// Persistent UUID that survives reboots and reconnections
    pub uuid: AutoDropStrPtr,
    // relative to primary screen
    pub origin: LogicalPoint,
    pub size: LogicalSize,
    pub scale: f64,
    pub maximum_frames_per_second: u32,
    // todo color space?
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
                let name = screen.localizedName();
                let rect = screen.rect(mtm).unwrap();
                let uuid = screen.persistent_uuid().unwrap_or_default();
                ScreenInfo {
                    screen_id: screen.screen_id(),
                    // The screen containing the menu bar is always the first object (index 0) in the array returned by the screens method.
                    is_primary: num == 0,
                    name: copy_to_c_string(&name).unwrap().to_auto_drop(),
                    uuid: RustAllocatedStrPtr::allocate(uuid).unwrap().to_auto_drop(),
                    origin: rect.origin,
                    size: rect.size,
                    scale: screen.backingScaleFactor(),
                    maximum_frames_per_second: screen.maximumFramesPerSecond().try_into().unwrap(),
                }
            })
            .collect();
        Ok(ScreenInfoArray::new(screen_infos))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn screen_list_drop(arr: ScreenInfoArray) {
    ffi_boundary("screen_list_drop", || {
        drop(arr);
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
        let screen_id = self.me().deviceDescription().objectForKey(ns_string!("NSScreenNumber")).unwrap();
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

    /// Returns a persistent UUID for this screen that survives reboots and reconnections.
    fn persistent_uuid(&self) -> Option<String> {
        let display_id = self.screen_id();
        // SAFETY: CGDisplayCreateUUIDFromDisplayID is safe to call with any display ID
        // and returns a retained CFUUID that we must release (handled by CFRetained)
        let uuid_ptr = NonNull::new(unsafe { CGDisplayCreateUUIDFromDisplayID(display_id) }.cast_mut())?;
        let uuid: CFRetained<CFUUID> = unsafe { CFRetained::from_raw(uuid_ptr.cast()) };
        let bytes = uuid.uuid_bytes();
        // Format as standard UUID string
        Some(format!(
            "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
            bytes.byte0,
            bytes.byte1,
            bytes.byte2,
            bytes.byte3,
            bytes.byte4,
            bytes.byte5,
            bytes.byte6,
            bytes.byte7,
            bytes.byte8,
            bytes.byte9,
            bytes.byte10,
            bytes.byte11,
            bytes.byte12,
            bytes.byte13,
            bytes.byte14,
            bytes.byte15
        ))
    }
}

impl NSScreenExts for NSScreen {
    fn me(&self) -> &NSScreen {
        self
    }
}
