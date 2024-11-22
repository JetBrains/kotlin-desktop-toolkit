use std::ffi::CStr;

use objc2::rc::Retained;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSWindow, NSWindowStyleMask, NSNormalWindowLevel};
use objc2_foundation::{MainThreadMarker, NSString, NSUserDefaults, CGRect, CGPoint, CGSize};

use crate::common::StrPtr;

#[no_mangle]
pub extern "C" fn application_init() {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    unsafe { NSUserDefaults::resetStandardUserDefaults() };
    let user_defaults = unsafe { NSUserDefaults::standardUserDefaults() };
    unsafe {
        user_defaults.setBool_forKey(false, &NSString::from_str("NSDisabledDictationMenuItem"));
        user_defaults.setBool_forKey(false, &NSString::from_str("NSDisabledCharacterPaletteMenuItem"));
    };
    eprintln!("User defaults: {:?}", user_defaults);
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
}

#[no_mangle]
pub extern "C" fn application_run_event_loop() {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    unsafe { app.run() };
}

#[no_mangle]
pub extern "C" fn application_create_window(title: StrPtr, x: f32, y: f32) {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let title = unsafe { CStr::from_ptr(title) }.to_str().unwrap();
    create_window(mtm, title, x, y);
}

fn create_window(mtm: MainThreadMarker, title: &str, x: f32, y: f32) -> Retained<NSWindow> {
    let window = unsafe {
        let rect = CGRect::new(CGPoint::new(x.into(), y.into()), CGSize::new(320.0, 240.0));
        let style =
            NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Miniaturizable | NSWindowStyleMask::Resizable;
        let window = NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc(),
            rect,
            style,
            NSBackingStoreType::NSBackingStoreBuffered,
            false,
        );
        window.setTitle(&NSString::from_str(title));
        window.setReleasedWhenClosed(false);
        window.makeKeyAndOrderFront(None);
        window.setLevel(NSNormalWindowLevel);
        window
    };
    return window;
}