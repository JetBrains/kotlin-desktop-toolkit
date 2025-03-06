mod utils;

use desktop_macos::{
    common::BorrowedStrPtr,
    macos::text_operations::{TextChangedOperation, TextOperation},
};
use libtest_mimic::Trial;
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType};
use objc2_foundation::{NSPoint, NSString, NSTimeInterval};
use utils::test_utils::{TestResult, run_test};

fn test_simple_text_input() -> TestResult {
    run_test(|user_data| {
        user_data.events_to_send.push(unsafe {
            NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
                NSEventType::KeyDown,
                NSPoint::default(),
                NSEventModifierFlags(0),
                NSTimeInterval::default(),
                user_data.window_id,
                None,
                &NSString::from_str("a"),
                &NSString::from_str("a"),
                false,
                0,
            ).unwrap()
        });

        user_data
            .expected_text_operations
            .push(TextOperation::TextChanged(TextChangedOperation {
                window_id: user_data.window_id,
                original_event: None, // not yet checked in tests
                text: BorrowedStrPtr::new(c"a"),
            }));
    })
}

// TODO: add the following tests
// * Send [Option+U, u] -> TextChangedOperation(text: "ü", original_event: Some)
// * Send [Option+U, m] -> TextChangedOperation(text: "¨m")
// * Send [Option+U, ESC]   -> TextChangedOperation(text: "¨")
// * Send [Option+U, SPACE] -> TextChangedOperation(text: "¨")
// * Send [Option+U, Option+U, o] -> TextChangedOperation(text: "¨ö")
// * Send [LONG_PRESS a, SPACE] -> TextChangedOperation(text: "a ")
// * Send [LONG_PRESS a, RIGHT_ARROW, ENTER] -> TextChangedOperation(text: "à")
// * Send [LONG_PRESS a, RIGHT_ARROW, SPACE] -> TextChangedOperation(text: "à ")
// * Send [LONG_PRESS a, RIGHT_ARROW, LEFT_ARROW, SPACE] -> TextChangedOperation(text: " ")
// * Send [LONG_PRESS a, MOUSE_CLICK(first item)] -> TextChangedOperation(text: "à")
// * Send [LONG_PRESS a, 1] -> TextChangedOperation(text: "à")
// * Send [NSEvent::KeyDown(Ctrl+Tab)] -> Event::KeyDown(Ctrl+Tab)
// * Send [NSEvent::KeyDown(Ctrl+/)] -> Event::KeyDown(Ctrl+/)
// * Send [NSEvent::KeyDown(Cmd+a), NSEvent::KeyUp(Cmd+a)] -> [Event::KeyDown(Cmd+a), Event::KeyUp(Cmd+a)]
// * Add menu with shortcut Cmd+A, send NSEvent::KeyDown(Cmd+a), trigger menu item and don't receive Event::KeyDown

fn main() {
    let mut args = libtest_mimic::Arguments::from_args();
    args.test_threads = Some(1);

    let tests = vec![Trial::test("test_simple_text_input", test_simple_text_input)];

    libtest_mimic::run(&args, tests).exit();
}
