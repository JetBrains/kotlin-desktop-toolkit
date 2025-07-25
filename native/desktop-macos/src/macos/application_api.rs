use std::{cell::OnceCell, ffi::c_void};

use anyhow::{Context, anyhow};
use desktop_common::{
    ffi_utils::{BorrowedStrPtr, RustAllocatedStrPtr},
    logger::{catch_panic, ffi_boundary},
};
use log::info;
use objc2::{
    ClassType, DeclaredClass, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSApplicationTerminateReply, NSEvent, NSEventModifierFlags,
    NSEventType, NSImage, NSRunningApplication, NSWorkspace,
};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSData, NSDictionary, NSKeyValueChangeKey, NSKeyValueObservingOptions, NSNotification, NSObject,
    NSObjectNSKeyValueObserverRegistration, NSObjectProtocol, NSPoint, NSString, NSURL, NSUserDefaults, ns_string,
};

use crate::macos::events::{
    handle_application_appearance_change, handle_application_did_finish_launching, handle_application_open_urls,
    handle_display_configuration_change,
};

use super::{
    appearance::Appearance,
    drag_and_drop::DragAndDropHandlerState,
    events::EventHandler,
    string::{copy_to_c_string, copy_to_ns_string},
};

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
    pub(crate) drag_and_drop_handler_state: DragAndDropHandlerState,
    pub(crate) mtm: MainThreadMarker,
}

impl AppState {
    pub(crate) fn with<T, F>(f: F) -> T
    where
        F: FnOnce(&Self) -> T,
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
    pub on_should_terminate: extern "C" fn() -> bool,
    pub on_will_terminate: extern "C" fn(),
    pub event_handler: EventHandler,
}

#[repr(C)]
#[derive(Debug)]
pub struct ApplicationConfig {
    pub disable_dictation_menu_item: bool,
    pub disable_character_palette_menu_item: bool,
}

#[unsafe(no_mangle)]
pub extern "C" fn application_init(config: &ApplicationConfig, callbacks: ApplicationCallbacks) {
    ffi_boundary("application_init", || {
        info!("Application Init");
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        //    unsafe { NSUserDefaults::resetStandardUserDefaults() };
        let user_defaults = unsafe { NSUserDefaults::standardUserDefaults() };
        unsafe {
            user_defaults.setBool_forKey(config.disable_dictation_menu_item, ns_string!("NSDisabledDictationMenuItem"));
            user_defaults.setBool_forKey(
                config.disable_character_palette_menu_item,
                ns_string!("NSDisabledCharacterPaletteMenuItem"),
            );
        };
        let app = MyNSApplication::sharedApplication(mtm);
        //    let default_presentation_options = app.presentationOptions();
        //    app.setPresentationOptions(default_presentation_options | NSApplicationPresentationOptions::NSApplicationPresentationFullScreen);
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        let event_handler = callbacks.event_handler;
        let app_delegate = AppDelegate::new(mtm, callbacks);
        app.setDelegate(Some(ProtocolObject::from_ref(&*app_delegate)));
        app.set_appearance_observer(&app_delegate);
        APP_STATE.with(|app_state| {
            app_state
                .set(AppState {
                    app,
                    app_delegate,
                    event_handler,
                    drag_and_drop_handler_state: DragAndDropHandlerState::default(),
                    mtm,
                })
                .map_err(|_| anyhow!("Can't initialize second time!"))?;
            Ok(())
        })
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_get_appearance() -> Appearance {
    ffi_boundary("application_get_appearance", || -> Result<Appearance, anyhow::Error> {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        let appearance = app.effectiveAppearance();
        Ok(Appearance::from_ns_appearance(&appearance))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_shutdown() {
    ffi_boundary("application_shutdown", || {
        // todo
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_run_event_loop() {
    ffi_boundary("application_run_event_loop", || {
        info!("Start event loop");
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        app.run();
        Ok(())
    });
}

#[unsafe(no_mangle)]
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

#[unsafe(no_mangle)]
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

#[unsafe(no_mangle)]
pub extern "C" fn application_get_name() -> RustAllocatedStrPtr {
    ffi_boundary("application_name", || {
        match unsafe { NSRunningApplication::currentApplication().localizedName() } {
            Some(name) => copy_to_c_string(&name),
            None => Ok(RustAllocatedStrPtr::null()),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_hide() {
    ffi_boundary("application_hide", || {
        let mtm = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        app.hide(None);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_hide_other_applications() {
    ffi_boundary("application_hide_other_applications", || {
        let mtm = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        app.hideOtherApplications(None);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_unhide_all_applications() {
    ffi_boundary("application_unhide_all_applications", || {
        let mtm = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        unsafe {
            app.unhideAllApplications(None);
        }
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_is_active() -> bool {
    ffi_boundary("application_is_active", || {
        let mtm = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        let is_active = unsafe { app.isActive() };
        Ok(is_active)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_activate_ignoring_other_apps() {
    ffi_boundary("application_activate_ignoring_other_apps", || {
        let mtm = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        #[allow(deprecated)]
        app.activateIgnoringOtherApps(true);
        Ok(())
    });
}

/// # Safety
///
/// `data` must be a valid, non-null, pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn application_set_dock_icon(data: *mut u8, data_length: u64) {
    ffi_boundary("application_set_dock_icon", || {
        let mtm = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        assert!(!data.is_null());
        let bytes = unsafe { std::slice::from_raw_parts_mut(data, data_length.try_into().unwrap()) };
        let data = NSData::with_bytes(bytes);
        let image = NSImage::initWithData(mtm.alloc(), &data).context("Can't create image from data")?;
        unsafe {
            app.setApplicationIconImage(Some(&image));
        }
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_order_front_character_palete() {
    ffi_boundary("application_order_front_character_palete", || {
        let mtm = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        app.orderFrontCharacterPalette(None);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_open_url(url: BorrowedStrPtr) -> bool {
    ffi_boundary("application_open_url", || {
        let url_string = copy_to_ns_string(&url)?;
        let url = unsafe { NSURL::URLWithString(&url_string).context("Can't create NSURL from string")? };
        let was_openned = unsafe { NSWorkspace::sharedWorkspace().openURL(&url) };
        Ok(was_openned)
    })
}

define_class!(
    #[unsafe(super(NSApplication))]
    #[name = "MyNSApplication"]
    #[ivars = ()]
    #[derive(Debug)]
    pub(crate) struct MyNSApplication;

    unsafe impl NSObjectProtocol for MyNSApplication {}

    impl MyNSApplication {
        #[unsafe(method(sendEvent:))]
        fn send_event(&self, event: &NSEvent) {
            catch_panic(|| {
                self.send_event_impl(event);
                Ok(())
            });
        }
    }
);

impl MyNSApplication {
    #[allow(non_snake_case)]
    pub(crate) fn sharedApplication(_mtm: MainThreadMarker) -> Retained<Self> {
        unsafe { msg_send!(Self::class(), sharedApplication) }
    }

    // https://bugzilla.mozilla.org/show_bug.cgi?id=1299553
    // The default sendEvent turns key downs into performKeyEquivalent when
    // modifiers are down, but swallows the key up if the modifiers include
    // command.  This one makes all modifiers consistent by always sending key ups.
    fn send_event_impl(&self, event: &NSEvent) {
        if unsafe { event.r#type() } == NSEventType::KeyUp {
            let mtm: MainThreadMarker = self.mtm();
            if let Some(window) = unsafe { event.window(mtm) } {
                window.sendEvent(event);
                return;
            }
        }
        let _: () = unsafe { msg_send![super(self), sendEvent: event] };
    }

    fn set_appearance_observer(&self, delegate: &NSObject) {
        unsafe {
            self.addObserver_forKeyPath_options_context(
                delegate,
                ns_string!("effectiveAppearance"),
                NSKeyValueObservingOptions::New,
                std::ptr::null_mut(),
            );
        }
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
            catch_panic(|| {
                handle_display_configuration_change();
                Ok(())
            });
        }

        #[unsafe(method(application:openURLs:))]
        unsafe fn application_open_urls(&self, _application: &NSApplication, urls: &NSArray<NSURL>) {
            catch_panic(|| {
                handle_application_open_urls(urls);
                Ok(())
            });
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

        #[unsafe(method(observeValueForKeyPath:ofObject:change:context:))]
        fn observe_value(
            &self,
            key_path: Option<&NSString>,
            object: Option<&AnyObject>,
            change: Option<&NSDictionary<NSKeyValueChangeKey, AnyObject>>,
            context: *mut c_void,
        ) {
            catch_panic(|| {
                match (object, key_path) {
                    (Some(object), Some(key_path))
                        if object.class().superclass() == Some(MyNSApplication::class())
                            && key_path == ns_string!("effectiveAppearance") =>
                    {
                        handle_application_appearance_change();
                    }
                    _ => unsafe {
                        let _: () = msg_send![super(self), observeValueForKeyPath: key_path,
                                                                     ofObject: object,
                                                                       change: change,
                                                                      context: context];
                    },
                }
                Ok(())
            });
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
