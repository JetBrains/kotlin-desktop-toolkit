use desktop_macos::{
    common::{BorrowedStrPtr, LogicalPoint, LogicalSize},
    macos::{
        application_api::{
            ApplicationCallbacks, ApplicationConfig, application_init, application_run_event_loop, application_stop_event_loop,
        },
        events::{CallbackUserData, Event},
        text_operations::TextOperation,
        window_api::{WindowId, WindowParams, window_create, window_get_window_id},
    },
};

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSApplication, NSEvent};

pub type TestResult = Result<(), libtest_mimic::Failed>;

#[derive(Default)]
pub struct TestData {
    pub window_id: WindowId,
    pub events_to_send: Vec<Retained<NSEvent>>,
    pub expected_events: Vec<Event<'static>>,
    pub expected_text_operations: Vec<TextOperation<'static>>,
}

pub fn run_test<F: FnOnce(&mut TestData) -> ()>(fill_test_data: F) -> TestResult {
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

    let mut test_data = TestData::default();

    extern "C" fn event_handler(e: &Event, user_data: CallbackUserData) -> bool {
        let test_data: &mut TestData = unsafe { &mut *(user_data.cast()) };

        if let Event::ApplicationDidFinishLaunching = e {
            let mtm = MainThreadMarker::new().unwrap();
            let app = NSApplication::sharedApplication(mtm);
            for e in &test_data.events_to_send {
                unsafe { app.sendEvent(e) };
            }
        } else {
            if let Some(expected_event) = test_data.expected_events.first() {
                if compare_events(e, expected_event) {
                    test_data.expected_events.remove(0);
                    if !test_data.expected_events.is_empty() {
                        return true;
                    }
                }
            }
            application_stop_event_loop();
        }
        true
    }

    extern "C" fn text_operation_handler(e: &TextOperation, user_data: CallbackUserData) -> bool {
        let test_data: &mut TestData = unsafe { &mut *(user_data.cast()) };
        if let Some(expected_op) = test_data.expected_text_operations.first() {
            if compare_text_operations(e, expected_op) {
                test_data.expected_text_operations.remove(0);
                if !test_data.expected_text_operations.is_empty() {
                    return true;
                }
            }
        }

        application_stop_event_loop();
        true
    }
    extern "C" fn on_should_terminate() -> bool {
        true
    }
    extern "C" fn on_will_terminate() {}

    let config = ApplicationConfig {
        disable_dictation_menu_item: false,
        disable_character_palette_menu_item: false,
    };
    let callbacks = ApplicationCallbacks {
        on_should_terminate,
        on_will_terminate,
        event_handler,
        event_handler_user_data: (&raw mut test_data).cast(),
        text_operation_handler,
        text_operation_handler_user_data: (&raw mut test_data).cast(),
    };
    application_init(&config, callbacks);
    let window_ptr = window_create(&params);
    test_data.window_id = window_get_window_id(window_ptr);
    fill_test_data(&mut test_data);

    application_run_event_loop();
    assert_eq!(
        test_data.expected_text_operations.len(),
        0,
        "{:?}",
        test_data.expected_text_operations
    );
    assert_eq!(test_data.expected_events.len(), 0, "{:?}", test_data.expected_events);
    Ok(())
}

fn compare_borrowed_strings(lhs: &BorrowedStrPtr, rhs: &BorrowedStrPtr) -> bool {
    lhs.as_str().unwrap() == rhs.as_str().unwrap()
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
        (Event::ModifiersChanged(lhs), Event::ModifiersChanged(rhs)) => {
            lhs.window_id == rhs.window_id && lhs.modifiers.0 == rhs.modifiers.0 && lhs.code.0 == rhs.code.0
        }
        (Event::MouseMoved(lhs), Event::MouseMoved(rhs)) => lhs.window_id == rhs.window_id,
        (Event::MouseDragged(lhs), Event::MouseDragged(rhs)) => lhs.window_id == rhs.window_id && lhs.button.0 == rhs.button.0,
        (Event::MouseEntered(lhs), Event::MouseEntered(rhs)) => lhs.window_id == rhs.window_id,
        (Event::MouseExited(lhs), Event::MouseExited(rhs)) => lhs.window_id == rhs.window_id,
        (Event::MouseDown(lhs), Event::MouseDown(rhs)) => lhs.window_id == rhs.window_id && lhs.button.0 == rhs.button.0,
        (Event::MouseUp(lhs), Event::MouseUp(rhs)) => lhs.window_id == rhs.window_id && lhs.button.0 == rhs.button.0,
        (Event::ScrollWheel(lhs), Event::ScrollWheel(rhs)) => lhs.window_id == rhs.window_id,
        (Event::WindowScreenChange(lhs), Event::WindowScreenChange(rhs)) => {
            lhs.window_id == rhs.window_id && lhs.new_screen_id == rhs.new_screen_id
        }
        (Event::WindowResize(lhs), Event::WindowResize(rhs)) => lhs.window_id == rhs.window_id,
        (Event::WindowMove(lhs), Event::WindowMove(rhs)) => {
            lhs.window_id == rhs.window_id // && lhs.origin == rhs.origin
        }
        (Event::WindowFocusChange(lhs), Event::WindowFocusChange(rhs)) => {
            lhs.window_id == rhs.window_id && lhs.is_key == rhs.is_key && lhs.is_main == rhs.is_main
        }
        (Event::WindowCloseRequest(lhs), Event::WindowCloseRequest(rhs)) => lhs.window_id == rhs.window_id,
        (Event::WindowFullScreenToggle(lhs), Event::WindowFullScreenToggle(rhs)) => {
            lhs.window_id == rhs.window_id && lhs.is_full_screen == rhs.is_full_screen
        }
        (Event::DisplayConfigurationChange, Event::DisplayConfigurationChange) => true,
        (Event::ApplicationDidFinishLaunching, Event::ApplicationDidFinishLaunching) => true,
        _ => false,
    }
}

fn compare_text_operations(lhs: &TextOperation, rhs: &TextOperation) -> bool {
    match (lhs, rhs) {
        (TextOperation::TextCommand(lhs), TextOperation::TextCommand(rhs)) => {
            lhs.window_id == rhs.window_id && compare_borrowed_strings(&lhs.command, &rhs.command)
        }
        (TextOperation::TextChanged(lhs), TextOperation::TextChanged(rhs)) => {
            lhs.window_id == rhs.window_id && compare_borrowed_strings(&lhs.text, &rhs.text)
        }
        _ => false,
    }
}
