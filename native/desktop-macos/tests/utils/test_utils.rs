use std::ffi::c_ushort;

use desktop_macos::{
    common::{BorrowedStrPtr, LogicalPoint, LogicalSize},
    logger_api::{LogLevel, LoggerConfiguration, logger_init},
    macos::{
        application_api::{
            ApplicationCallbacks, ApplicationConfig, application_init, application_run_event_loop, application_stop_event_loop,
        },
        events::{CallbackUserData, Event},
        text_operations::TextOperation,
        window::{CustomImeHandler, RootView, Window},
        window_api::{WindowCallbacks, WindowId, WindowParams, window_create, window_drop},
    },
};

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSApplication, NSEvent, NSEventModifierFlags, NSEventType, NSTextInputClient};
use objc2_foundation::{NSNotFound, NSPoint, NSRange, NSString, NSTimeInterval, NSUInteger};

pub type TestResult = Result<(), libtest_mimic::Failed>;

#[derive(Debug, Default)]
pub struct TestData<'a> {
    pub window_id: WindowId,
    pub events_to_send: Vec<Retained<NSEvent>>,
    pub expected_events: Vec<Event<'a>>,
    pub expected_text_operations: Vec<TextOperation<'a>>,
    pub ignore_non_text_events: bool,
    pub custom_ime_handler: Option<CustomImeHandler>,
    encountered_error: Option<String>,
}

extern "C" fn event_handler(e: &Event, user_data: CallbackUserData) -> bool {
    let test_data: &mut TestData = unsafe { &mut *(user_data.cast()) };
    eprintln!(
        "test_utils event_handler: expected_events.len: {}, {e:?}",
        test_data.expected_events.len()
    );

    if !test_data.events_to_send.is_empty() {
        let mtm = MainThreadMarker::new().unwrap();
        let app = NSApplication::sharedApplication(mtm);
        let events_to_send = std::mem::take(&mut test_data.events_to_send);
        for e in &events_to_send {
            if test_data.encountered_error.is_some() {
                application_stop_event_loop();
                return false;
            }
            eprintln!("Sending event: {e:?}");
            unsafe { app.sendEvent(e) };
        }
        test_data.events_to_send.clear();
        eprintln!(
            "test_utils event_handler: sent all events, events_to_send.len: {}",
            test_data.events_to_send.len()
        );
    } else {
        match e {
            Event::WindowResize(_) | Event::WindowMove(_) | Event::WindowFocusChange(_) | Event::MouseMoved(_) => return true,
            _ => {}
        }
        if test_data.expected_events.is_empty() {
            let msg = format!("Unexpected event (expected list is empty): {e:?}");
            eprintln!("{}", msg);
            test_data.encountered_error = Some(msg);
        } else {
            let expected_event = test_data.expected_events.remove(0);
            if !compare_events(e, &expected_event) {
                let msg = format!("Unexpected event: {e:?}\n    Expected: {expected_event:?}");
                eprintln!("{}", msg);
                test_data.encountered_error = Some(msg);
            }
        }
    }
    if test_data.expected_events.is_empty() && test_data.expected_text_operations.is_empty() {
        application_stop_event_loop();
    }
    !test_data.ignore_non_text_events
}

extern "C" fn text_operation_handler(e: &TextOperation, user_data: CallbackUserData) -> bool {
    let test_data: &mut TestData = unsafe { &mut *(user_data.cast()) };
    eprintln!(
        "test_utils text_operation_handler: expected_text_operations.len: {}, {e:?}",
        test_data.expected_events.len()
    );
    let mut handled = true;
    if let TextOperation::TextCommand(_) = e {
        eprintln!("Returning false for {e:?}");
        handled = false;
    } else {
        if test_data.expected_text_operations.is_empty() {
            let msg = format!("Unexpected event (expected list is empty): {e:?}");
            eprintln!("{}", msg);
            test_data.encountered_error = Some(msg);
            application_stop_event_loop();
        } else {
            let expected_op = test_data.expected_text_operations.remove(0);
            if !compare_text_operations(e, &expected_op) {
                let msg = format!("Unexpected text operation: {e:?}\n    Expected: {expected_op:?}");
                eprintln!("{}", msg);
                test_data.encountered_error = Some(msg);
                application_stop_event_loop();
            }
        }
    }
    if test_data.expected_events.is_empty() && test_data.expected_text_operations.is_empty() {
        application_stop_event_loop();
    }

    handled
}

pub fn init_tests() {
    extern "C" fn on_should_terminate() -> bool {
        true
    }
    extern "C" fn dummy() {}

    let config = ApplicationConfig {
        disable_dictation_menu_item: false,
        disable_character_palette_menu_item: false,
    };

    let callbacks = ApplicationCallbacks {
        on_should_terminate,
        on_will_terminate: dummy,
        on_did_change_screen_parameters: dummy,
        on_did_finish_launching: dummy,
    };
    eprintln!("application_init");

    logger_init(&LoggerConfiguration {
        file_path: BorrowedStrPtr::new(c"/tmp/a"),
        console_level: LogLevel::Debug,
        file_level: LogLevel::Error,
    });
    application_init(&config, callbacks);
}

pub fn custom_ime_handler(e: &NSEvent, text_input_client: &RootView) -> bool {
    const DEFAULT_REPLACEMENT_RANGE: NSRange = NSRange {
        location: NSNotFound as NSUInteger,
        length: 0,
    };
    const DEFAULT_SELECTED_RANGE: NSRange = NSRange { location: 1, length: 0 };

    match unsafe { e.r#type() } {
        NSEventType::KeyDown => {
            if unsafe { e.modifierFlags().contains(NSEventModifierFlags::Option) } {
                match unsafe { e.keyCode() } {
                    50 => {
                        let string = NSString::from_str("`");
                        unsafe {
                            text_input_client.setMarkedText_selectedRange_replacementRange(
                                &string,
                                DEFAULT_SELECTED_RANGE,
                                DEFAULT_REPLACEMENT_RANGE,
                            )
                        };
                        true
                    }
                    _ => false,
                }
            } else {
                if unsafe { text_input_client.hasMarkedText() } {
                    let string = NSString::from_str("`"); // TODO: cache last diacritic
                    unsafe { text_input_client.insertText_replacementRange(&string, DEFAULT_REPLACEMENT_RANGE) };
                }
                true
            }
        }
        NSEventType::KeyUp => false,
        _ => false,
    }
}

impl<'a> TestData<'a> {
    pub fn run_test(&mut self) -> TestResult {
        let test_data_ptr: *mut TestData = self;
        let window_callbacks = WindowCallbacks {
            event_handler,
            event_handler_user_data: test_data_ptr.cast(),
            text_operation_handler,
            text_operation_handler_user_data: test_data_ptr.cast(),
        };
        let params = WindowParams {
            origin: LogicalPoint { x: 0.0, y: 0.0 },
            size: LogicalSize { width: 0.0, height: 0.0 },
            title: BorrowedStrPtr::new(c"Test Window 1"),

            is_resizable: false,
            is_closable: true,
            is_miniaturizable: true,

            is_full_screen_allowed: false,
            use_custom_titlebar: false,
            titlebar_height: 0.0,
        };

        let window_ptr = window_create(&params, window_callbacks);
        let window = unsafe { window_ptr.borrow::<Window>() };
        window.root_view.set_custom_ime_handler(self.custom_ime_handler);

        application_run_event_loop();

        window_drop(window_ptr);

        if let Some(e) = &self.encountered_error {
            Err(e.into())
        } else {
            Ok(())
        }
    }
}

fn compare_borrowed_strings(lhs: &BorrowedStrPtr, rhs: &BorrowedStrPtr) -> bool {
    let s1 = lhs.as_str().unwrap();
    let s2 = rhs.as_str().unwrap();
    s1 == s2
}

fn compare_events(lhs: &Event, rhs: &Event) -> bool {
    match (lhs, rhs) {
        (Event::KeyDown(lhs), Event::KeyDown(rhs)) => {
            lhs.window_id == rhs.window_id
                && lhs.modifiers.0 == rhs.modifiers.0
                && lhs.code.0 == rhs.code.0
                && compare_borrowed_strings(&lhs.characters, &rhs.characters)
                && compare_borrowed_strings(&lhs.key, &rhs.key)
                && lhs.is_repeat == rhs.is_repeat
        }
        (Event::KeyUp(lhs), Event::KeyUp(rhs)) => {
            lhs.window_id == rhs.window_id
                && lhs.modifiers.0 == rhs.modifiers.0
                && lhs.code.0 == rhs.code.0
                && compare_borrowed_strings(&lhs.characters, &rhs.characters)
                && compare_borrowed_strings(&lhs.key, &rhs.key)
        }
        (Event::ModifiersChanged(lhs), Event::ModifiersChanged(rhs)) => lhs.modifiers.0 == rhs.modifiers.0 && lhs.code.0 == rhs.code.0,
        (Event::MouseMoved(lhs), Event::MouseMoved(rhs)) => lhs.window_id == rhs.window_id,
        (Event::MouseDragged(lhs), Event::MouseDragged(rhs)) => lhs.button.0 == rhs.button.0,
        (Event::MouseEntered(lhs), Event::MouseEntered(rhs)) => lhs.window_id == rhs.window_id,
        (Event::MouseExited(lhs), Event::MouseExited(rhs)) => lhs.window_id == rhs.window_id,
        (Event::MouseDown(lhs), Event::MouseDown(rhs)) => lhs.button.0 == rhs.button.0,
        (Event::MouseUp(lhs), Event::MouseUp(rhs)) => lhs.button.0 == rhs.button.0,
        (Event::ScrollWheel(lhs), Event::ScrollWheel(rhs)) => lhs.window_id == rhs.window_id,
        (Event::WindowScreenChange(_), Event::WindowScreenChange(_)) => true,
        (Event::WindowResize(lhs), Event::WindowResize(rhs)) => lhs.window_id == rhs.window_id,
        (Event::WindowMove(lhs), Event::WindowMove(rhs)) => {
            lhs.window_id == rhs.window_id // && lhs.origin == rhs.origin
        }
        (Event::WindowFocusChange(lhs), Event::WindowFocusChange(rhs)) => lhs.is_key == rhs.is_key && lhs.is_main == rhs.is_main,
        (Event::WindowCloseRequest(lhs), Event::WindowCloseRequest(rhs)) => lhs.window_id == rhs.window_id,
        (Event::WindowFullScreenToggle(lhs), Event::WindowFullScreenToggle(rhs)) => lhs.is_full_screen == rhs.is_full_screen,
        _ => false,
    }
}

fn compare_text_operations(lhs: &TextOperation, rhs: &TextOperation) -> bool {
    match (lhs, rhs) {
        (TextOperation::TextCommand(lhs), TextOperation::TextCommand(rhs)) => compare_borrowed_strings(&lhs.command, &rhs.command),
        (TextOperation::TextChanged(lhs), TextOperation::TextChanged(rhs)) => compare_borrowed_strings(&lhs.text, &rhs.text),
        (TextOperation::SetMarkedText(lhs), TextOperation::SetMarkedText(rhs)) => {
            lhs.selected_range == rhs.selected_range
                && lhs.replacement_range == rhs.replacement_range
                && compare_borrowed_strings(&lhs.text, &rhs.text)
        }
        (TextOperation::UnmarkText(_), TextOperation::UnmarkText(_)) => true,
        _ => false,
    }
}

pub fn make_ns_key_down_event(
    w_num: WindowId,
    chars: &NSString,
    unmodchars: &NSString,
    flags: NSEventModifierFlags,
    code: c_ushort,
) -> Retained<NSEvent> {
    let location = NSPoint::default();
    let time = NSTimeInterval::default();
    let is_repeat = false;
    unsafe {
        NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
            NSEventType::KeyDown,
            location,
            flags,
            time,
            w_num,
            None,
            chars,
            unmodchars,
            is_repeat,
            code,
        ).unwrap()
    }
}

pub fn make_ns_key_up_event(
    w_num: WindowId,
    chars: &NSString,
    unmodchars: &NSString,
    flags: NSEventModifierFlags,
    code: c_ushort,
) -> Retained<NSEvent> {
    let location = NSPoint::default();
    let time = NSTimeInterval::default();
    let is_repeat = false;
    unsafe {
        NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
            NSEventType::KeyUp,
            location,
            flags,
            time,
            w_num,
            None,
            chars,
            unmodchars,
            is_repeat,
            code,
        ).unwrap()
    }
}
