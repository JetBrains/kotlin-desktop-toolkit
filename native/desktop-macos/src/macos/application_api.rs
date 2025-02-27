use anyhow::{anyhow, Context};
use log::{error, info};
use objc2::{declare_class, define_class, msg_send, rc::Retained, runtime::ProtocolObject, sel, ClassType, DeclaredClass, MainThreadOnly};
use objc2_app_kit::{
    NSApp, NSAppearance, NSAppearanceCustomization, NSAppearanceNameDarkAqua, NSApplication, NSApplicationActivationPolicy,
    NSApplicationDelegate, NSApplicationDidChangeScreenParametersNotification, NSApplicationPresentationOptions,
    NSApplicationTerminateReply, NSBackingStoreType, NSEvent, NSEventModifierFlags, NSEventType, NSNormalWindowLevel, NSWindow,
    NSWindowStyleMask,
};
use objc2_foundation::{
    MainThreadMarker, NSNotification, NSNotificationCenter, NSObject, NSObjectProtocol, NSPoint, NSString, NSUserDefaults,
};
use std::{
    cell::{Cell, OnceCell},
    os::unix::thread,
};

use crate::{
    common::StrPtr,
    logger::ffi_boundary,
    macos::events::{handle_application_did_finish_launching, handle_display_configuration_change},
};

use super::{events::EventHandler, text_operations::TextOperationHandler};

thread_local! {
    pub static APP_STATE: OnceCell<AppState> = const { OnceCell::new() };
}

#[derive(Debug)]
pub(crate) struct AppState {
    #[allow(dead_code)]
    pub(crate) app: Retained<MyNSApplication>,
    #[allow(dead_code)]
    app_delegate: Retained<AppDelegate>,
    pub(crate) event_handler: EventHandler,
    pub(crate) mtm: MainThreadMarker,
    pub(crate) text_operation_handler: TextOperationHandler,
}

impl AppState {
    pub(crate) fn with<T, F>(f: F) -> T
    where
        F: FnOnce(&AppState) -> T,
    {
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
    // otherwise termination will be canceled
    on_should_terminate: extern "C" fn() -> bool,
    on_will_terminate: extern "C" fn(),
    event_handler: EventHandler,
    text_operation_handler: TextOperationHandler,
}

#[repr(C)]
#[derive(Debug)]
pub struct ApplicationConfig {
    pub disable_dictation_menu_item: bool,
    pub disable_character_palette_menu_item: bool,
}

#[no_mangle]
pub extern "C" fn application_init(config: &ApplicationConfig, callbacks: ApplicationCallbacks) {
    ffi_boundary("application_init", || {
        info!("Application Init");
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        //    unsafe { NSUserDefaults::resetStandardUserDefaults() };
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
        let app = MyNSApplication::sharedApplication(mtm);
        //    unsafe {
        //        if let Some(apperance) = NSAppearance::appearanceNamed(NSAppearanceNameDarkAqua) {
        //            app.setAppearance(Some(&apperance));
        //        }
        //    }

        //    let default_presentation_options = app.presentationOptions();
        //    app.setPresentationOptions(default_presentation_options | NSApplicationPresentationOptions::NSApplicationPresentationFullScreen);
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        let event_handler = callbacks.event_handler;
        let text_operation_handler = callbacks.text_operation_handler;
        let app_delegate = AppDelegate::new(mtm, callbacks);
        app.setDelegate(Some(ProtocolObject::from_ref(&*app_delegate)));
        APP_STATE.with(|app_state| {
            // app_state.
            app_state
                .set(AppState {
                    app,
                    app_delegate,
                    event_handler,
                    text_operation_handler,
                    mtm,
                })
                .map_err(|_| anyhow!("Can't initialize second time!"))?;
            Ok(())
        })
    });
}

#[no_mangle]
pub extern "C" fn application_shutdown() {
    ffi_boundary("application_shutdown", || {
        // todo
        Ok(())
    });
}

#[no_mangle]
pub extern "C" fn application_run_event_loop() {
    ffi_boundary("application_run_event_loop", || {
        info!("Start event loop");
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        app.run();
        Ok(())
    });
}

#[no_mangle]
pub extern "C" fn application_stop_event_loop() {
    ffi_boundary("application_stop_event_loop", || {
        info!("Stop event loop");
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        app.stop(None);
        // In case application_stop_event_loop is not called in response to a UI event, we need to trigger
        // a dummy event so the UI processing loop picks up the stop request.
        let dummy_event = unsafe {
            NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
                NSEventType::ApplicationDefined,
                NSPoint::ZERO,
                NSEventModifierFlags::empty(),
                0f64,
                0,
                None,
                0,
                0,
                0,
            )
        }
        .unwrap();
        app.postEvent_atStart(&dummy_event, true);
        Ok(())
    });
}

#[no_mangle]
pub extern "C" fn application_request_termination() {
    ffi_boundary("application_request_termination", || {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        unsafe {
            app.terminate(None);
        }
        Ok(())
    });
}

#[derive(Debug)]
pub(crate) struct MyNSApplicationIvars {}

define_class!(
    #[unsafe(super(NSApplication))]
    #[name = "MyNSApplication"]
    #[ivars = MyNSApplicationIvars]
    #[derive(Debug)]
    pub(crate) struct MyNSApplication;

    unsafe impl NSObjectProtocol for MyNSApplication {}

    impl MyNSApplication {
        // https://bugzilla.mozilla.org/show_bug.cgi?id=1299553
        // The default sendEvent turns key downs into performKeyEquivalent when
        // modifiers are down, but swallows the key up if the modifiers include
        // command.  This one makes all modifiers consistent by always sending key ups.
        #[unsafe(method(sendEvent:))]
        fn send_event(&self, event: &NSEvent) {
            if unsafe { event.r#type() } == NSEventType::KeyUp {
                if let Some(window) = self.keyWindow() {
                    window.sendEvent(event);
                    return;
                }
            }
            unsafe {
                let _:() = msg_send![super(self), sendEvent: event];
            }
        }
    }
);

impl MyNSApplication {
    #[allow(non_snake_case)]
    pub(crate) fn sharedApplication(_mtm: MainThreadMarker) -> Retained<MyNSApplication> {
        return unsafe { msg_send!(MyNSApplication::class(), sharedApplication) };
    }
}

#[derive(Debug)]
struct AppDelegateIvars {
    callbacks: ApplicationCallbacks,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "AppDelegate"]
    #[ivars = AppDelegateIvars]
    #[derive(Debug)]
    struct AppDelegate;

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationDidChangeScreenParameters:))]
        fn did_change_screen_parameters(&self, _notification: &NSNotification) {
            handle_display_configuration_change();
        }

        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, _notification: &NSNotification) {
            handle_application_did_finish_launching();
        }

        #[unsafe(method(applicationShouldTerminate:))]
        fn should_terminate(&self, _sender: &NSApplication) -> NSApplicationTerminateReply {
            let result = (self.ivars().callbacks.on_should_terminate)();
            return if result {
                NSApplicationTerminateReply::TerminateNow
            } else {
                NSApplicationTerminateReply::TerminateCancel
            };
        }

        #[unsafe(method(applicationWillTerminate:))]
        fn will_terminate(&self, _notification: &NSNotification) {
            (self.ivars().callbacks.on_will_terminate)();
        }
    }
);

impl AppDelegate {
    fn new(mtm: MainThreadMarker, callbacks: ApplicationCallbacks) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(AppDelegateIvars { callbacks });
        unsafe { msg_send![super(this), init] }
    }
}
