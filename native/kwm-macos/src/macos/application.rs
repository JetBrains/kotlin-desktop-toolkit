use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use objc2_foundation::{MainThreadMarker, NSString, NSUserDefaults};

#[no_mangle]
pub extern "C" fn application_init() {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
//    unsafe { NSUserDefaults::resetStandardUserDefaults() };
//    let user_defaults = unsafe { NSUserDefaults::standardUserDefaults() };
//    unsafe {
//        user_defaults.setBool_forKey(false, &NSString::from_str("NSDisabledDictationMenuItem"));
//        user_defaults.setBool_forKey(false, &NSString::from_str("NSDisabledCharacterPaletteMenuItem"));
//    };
//    eprintln!("User defaults: {:?}", user_defaults);
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
}

#[no_mangle]
pub extern "C" fn application_run_event_loop() {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    unsafe { app.run() };
}