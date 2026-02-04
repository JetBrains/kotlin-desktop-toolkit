use super::{
    appearance::Appearance,
    drag_and_drop::DragAndDropHandlerState,
    events::EventHandler,
    string::{copy_to_c_string, copy_to_ns_string},
    text_direction::TextDirection,
};
use crate::macos::application_menu::{handle_app_menu_callback, set_initial_app_menu};
use crate::macos::application_menu_api::ItemId;
use crate::macos::events::{
    handle_application_appearance_change, handle_application_did_finish_launching, handle_application_open_urls,
    handle_display_configuration_change,
};
use crate::macos::image::Image;
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
    NSEventType, NSMenuItem, NSRequestUserAttentionType, NSRunningApplication, NSWorkspace,
};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSDictionary, NSKeyValueChangeKey, NSKeyValueObservingOptions, NSNotification, NSObject,
    NSObjectNSKeyValueObserverRegistration, NSObjectProtocol, NSPoint, NSString, NSURL, NSUserDefaults, ns_string,
};
use std::{cell::OnceCell, ffi::c_void};

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
            let app_state = app_state.get().expect("Can't access the app state before initialization!");
            f(app_state)
        })
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ApplicationCallbacks {
    // returns true if the application should terminate,
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
        // NSUserDefaults::resetStandardUserDefaults();
        let user_defaults = NSUserDefaults::standardUserDefaults();
        user_defaults.setBool_forKey(config.disable_dictation_menu_item, ns_string!("NSDisabledDictationMenuItem"));
        user_defaults.setBool_forKey(
            config.disable_character_palette_menu_item,
            ns_string!("NSDisabledCharacterPaletteMenuItem"),
        );
        // Disable autofill heuristic controller because it makes the app not responsive on macOS 26
        // Similar case: https://github.com/ghostty-org/ghostty/pull/8625
        user_defaults.setBool_forKey(false, ns_string!("NSAutoFillHeuristicControllerEnabled"));
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
pub extern "C" fn application_get_text_direction() -> TextDirection {
    ffi_boundary("application_get_text_direction", || -> Result<TextDirection, anyhow::Error> {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        let layout_direction = app.userInterfaceLayoutDirection();
        Ok(TextDirection::from_ns_layout_direction(layout_direction))
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
        let dummy_event = NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
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
        app.terminate(None);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_get_name() -> RustAllocatedStrPtr {
    ffi_boundary("application_name", || {
        match NSRunningApplication::currentApplication().localizedName() {
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
        app.unhideAllApplications(None);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_is_active() -> bool {
    ffi_boundary("application_is_active", || {
        let mtm = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        let is_active = app.isActive();
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

#[unsafe(no_mangle)]
pub extern "C" fn application_set_dock_icon(image: Image) {
    ffi_boundary("application_set_dock_icon", || {
        let mtm = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        let image = image.to_ns_image(mtm)?;
        unsafe {
            app.setApplicationIconImage(Some(&image));
        }
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_set_dock_icon_badge(label: BorrowedStrPtr) {
    ffi_boundary("application_set_dock_icon_badge", || {
        let mtm = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        let label = copy_to_ns_string(&label)?;
        app.dockTile().setBadgeLabel(Some(&label));
        Ok(())
    });
}

type AttentionRequestId = isize;

#[unsafe(no_mangle)]
pub extern "C" fn application_request_user_attention(is_critical: bool) -> AttentionRequestId {
    ffi_boundary("application_set_dock_icon_badge", || {
        let mtm = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        let attention_type = if is_critical {
            NSRequestUserAttentionType::CriticalRequest
        } else {
            NSRequestUserAttentionType::InformationalRequest
        };
        Ok(app.requestUserAttention(attention_type))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_cancel_request_user_attention(request_id: AttentionRequestId) {
    ffi_boundary("application_cancel_request_user_attention", || {
        let mtm = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        app.cancelUserAttentionRequest(request_id);
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
        let url = NSURL::URLWithString(&url_string).context("Can't create NSURL from string")?;
        let was_opened = NSWorkspace::sharedWorkspace().openURL(&url);
        Ok(was_opened)
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
        let isKeyUp = event.r#type() == NSEventType::KeyUp;
        let isEisuDown = event.r#type() == NSEventType::KeyDown && event.keyCode() == 102;
        let isKanaDown = event.r#type() == NSEventType::KeyDown && event.keyCode() == 104;

        if isKeyUp || isEisuDown || isKanaDown {
            let mtm: MainThreadMarker = self.mtm();
            if let Some(window) = event.window(mtm) {
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

    impl AppDelegate {
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

        #[unsafe(method(itemCallback:))]
        fn item_callback(&self, sender: &NSMenuItem) {
            handle_app_menu_callback(sender.tag() as ItemId);
        }

        // We need this strange layer of delegation to give MacOS a chance to handle
        // application menu events before we handle them ourselves.
        // For example, copy, paste and others wouldn't work in open dialogs without this.

        #[unsafe(method(undo:))]
        fn undo(&self, sender: &NSMenuItem) {
            handle_app_menu_callback(sender.tag() as ItemId);
        }

        #[unsafe(method(redo:))]
        fn redo(&self, sender: &NSMenuItem) {
            handle_app_menu_callback(sender.tag() as ItemId);
        }

        #[unsafe(method(cut:))]
        fn cut(&self, sender: &NSMenuItem) {
            handle_app_menu_callback(sender.tag() as ItemId);
        }

        #[unsafe(method(copy:))]
        fn copy(&self, sender: &NSMenuItem) {
            handle_app_menu_callback(sender.tag() as ItemId);
        }

        #[unsafe(method(paste:))]
        fn paste(&self, sender: &NSMenuItem) {
            handle_app_menu_callback(sender.tag() as ItemId);
        }

        #[unsafe(method(selectAll:))]
        fn select_all(&self, sender: &NSMenuItem) {
            handle_app_menu_callback(sender.tag() as ItemId);
        }
    }

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

        #[unsafe(method(applicationWillFinishLaunching:))]
        fn application_will_finish_launching(&self, _notification: &NSNotification) {
            catch_panic(|| {
                set_initial_app_menu();
                Ok(())
            });
        }

        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, _notification: &NSNotification) {
            catch_panic(|| {
                handle_application_did_finish_launching();
                Ok(())
            });
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
