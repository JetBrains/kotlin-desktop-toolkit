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

//declare_class!(
//    struct AppDelegate;
//
//    // SAFETY:
//    // - The superclass NSObject does not have any subclassing requirements.
//    // - Main thread only mutability is correct, since this is an application delegate.
//    // - `AppDelegate` does not implement `Drop`.
//    unsafe impl ClassType for AppDelegate {
//        type Super = NSObject;
//        type Mutability = mutability::MainThreadOnly;
//        const NAME: &'static str = "MyAppDelegate";
//    }
//
//    impl DeclaredClass for AppDelegate {
//        type Ivars = Ivars;
//    }
//
//    unsafe impl NSObjectProtocol for AppDelegate {}
//
//    unsafe impl NSApplicationDelegate for AppDelegate {
//        #[method(applicationDidFinishLaunching:)]
//        fn did_finish_launching(&self, notification: &NSNotification) {
//            println!("Did finish launching!");
//            // Do something with the notification
//            dbg!(notification);
//        }
//
//        #[method(applicationWillTerminate:)]
//        fn will_terminate(&self, _notification: &NSNotification) {
//            println!("Will terminate!");
//        }
//    }
//);
//
//impl AppDelegate {
//    fn new(ivar: u8, another_ivar: bool, mtm: MainThreadMarker) -> Retained<Self> {
//        let this = mtm.alloc();
//        let this = this.set_ivars(Ivars {
//            ivar,
//            another_ivar,
//            box_ivar: Box::new(2),
//            maybe_box_ivar: None,
//            id_ivar: NSString::from_str("abc"),
//            maybe_id_ivar: Some(ns_string!("def").copy()),
//        });
//        unsafe { msg_send_id![super(this), init] }
//    }
//}