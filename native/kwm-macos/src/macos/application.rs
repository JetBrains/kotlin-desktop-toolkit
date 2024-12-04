use std::ffi::CStr;

use objc2::rc::Retained;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSWindow, NSWindowStyleMask, NSNormalWindowLevel};
use objc2_foundation::{MainThreadMarker, NSString, NSUserDefaults, CGRect, CGPoint, CGSize};

use crate::common::StrPtr;

#[repr(C)]
#[derive(Debug)]
pub struct ApplicationConfig {
    pub disable_dictation_menu_item: bool,
    pub disable_character_palette_menu_item: bool,
}

#[no_mangle]
pub extern "C" fn application_init(config: &ApplicationConfig) {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    unsafe { NSUserDefaults::resetStandardUserDefaults() };
    let user_defaults = unsafe { NSUserDefaults::standardUserDefaults() };
    unsafe {
        user_defaults.setBool_forKey(config.disable_dictation_menu_item, &NSString::from_str("NSDisabledDictationMenuItem"));
        user_defaults.setBool_forKey(config.disable_character_palette_menu_item, &NSString::from_str("NSDisabledCharacterPaletteMenuItem"));
    };
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
}

#[no_mangle]
pub extern "C" fn application_run_event_loop() {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    unsafe { app.run() };
}