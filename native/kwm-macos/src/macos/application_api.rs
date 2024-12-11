use std::cell::{Cell, OnceCell};

use objc2::{declare_class, msg_send_id, mutability, rc::Retained, runtime::ProtocolObject, ClassType, DeclaredClass};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSApplicationTerminateReply, NSBackingStoreType,
    NSNormalWindowLevel, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{CGPoint, CGRect, CGSize, MainThreadMarker, NSNotification, NSObject, NSObjectProtocol, NSString, NSUserDefaults};

use crate::common::StrPtr;

use super::events::{Event, EventHandler};

thread_local! {
    pub static APP_STATE: OnceCell<AppState> = const { OnceCell::new() };
}

#[derive(Debug)]
pub(crate) struct AppState {
    pub(crate) app: Retained<NSApplication>,
    app_delegate: Retained<AppDelegate>,
    pub(crate) event_handler: EventHandler,
    pub(crate) mtm: MainThreadMarker,
}

impl AppState {
    pub(crate) fn with<T, F>(f: F) -> T where F: FnOnce(&AppState) -> T {
        APP_STATE.with(|app_state| {
            let app_state = app_state.get().expect("Can't access app state before initialization!"); // todo handle error
            f(app_state)
        })
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ApplicationCallbacks {
    // returns true if application should terminate,
    // oterwise termination will be caneled
    on_should_terminate: extern "C" fn() -> bool,
    on_will_terminate: extern "C" fn(),
    event_handler: EventHandler,
}

#[repr(C)]
#[derive(Debug)]
pub struct ApplicationConfig {
    pub disable_dictation_menu_item: bool,
    pub disable_character_palette_menu_item: bool,
}

#[no_mangle]
pub extern "C" fn application_init(config: &ApplicationConfig, callbacks: ApplicationCallbacks) {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    unsafe { NSUserDefaults::resetStandardUserDefaults() };
    let user_defaults = unsafe { NSUserDefaults::standardUserDefaults() };
    unsafe {
        user_defaults.setBool_forKey(
            config.disable_dictation_menu_item,
            &NSString::from_str("NSDisabledDictationMenuItem"),
        );
        user_defaults.setBool_forKey(
            config.disable_character_palette_menu_item,
            &NSString::from_str("NSDisabledCharacterPaletteMenuItem"),
        );
    };
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    let event_handler = callbacks.event_handler;
    let app_delegate = AppDelegate::new(mtm, callbacks);
    app.setDelegate(Some(ProtocolObject::from_ref(&*app_delegate)));
    APP_STATE.with(|app_state| {
        app_state.set(AppState {
            app,
            app_delegate,
            event_handler,
            mtm
        }).expect("Can't initialize second time!");
    });
}

#[no_mangle]
pub extern "C" fn application_shutdown() {
}

#[no_mangle]
pub extern "C" fn application_run_event_loop() {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    unsafe { app.run() };
}

#[no_mangle]
pub extern "C" fn application_stop_event_loop() {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    app.stop(None);
}

#[no_mangle]
pub extern "C" fn application_request_termination() {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    unsafe {
        app.terminate(None);
    }
}

struct AppDelegateIvars {
    callbacks: ApplicationCallbacks,
}

declare_class!(
    #[derive(Debug)]
    struct AppDelegate;

    // SAFETY:
    // - The superclass NSObject does not have any subclassing requirements.
    // - Main thread only mutability is correct, since this is an application delegate.
    // - `AppDelegate` does not implement `Drop`.
    unsafe impl ClassType for AppDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "MyAppDelegate";
    }

    impl DeclaredClass for AppDelegate {
        type Ivars = AppDelegateIvars;
    }

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[method(applicationDidFinishLaunching:)]
        fn did_finish_launching(&self, _notification: &NSNotification) {

        }

        #[method(applicationShouldTerminate:)]
        fn should_terminate(&self, _sender: &NSApplication) -> NSApplicationTerminateReply {
            let result = (self.ivars().callbacks.on_should_terminate)();
            return if result {
                NSApplicationTerminateReply::NSTerminateNow
            } else {
                NSApplicationTerminateReply::NSTerminateCancel
            }
        }

        #[method(applicationWillTerminate:)]
        fn will_terminate(&self, _notification: &NSNotification) {
            (self.ivars().callbacks.on_will_terminate)();
        }
    }
);

impl AppDelegate {
    fn new(mtm: MainThreadMarker, callbacks: ApplicationCallbacks) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(AppDelegateIvars { callbacks });
        unsafe { msg_send_id![super(this), init] }
    }
}
