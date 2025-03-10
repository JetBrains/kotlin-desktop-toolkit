mod utils;

use std::ptr::NonNull;

use desktop_macos::macos::{
    string::borrow_ns_string,
    text_operations::{TextChangedOperation, TextOperation},
};
use libtest_mimic::Trial;
use objc2::MainThreadMarker;
use objc2_app_kit::NSEventModifierFlags;
use objc2_foundation::{NSString, NSUTF32LittleEndianStringEncoding};
use utils::test_utils::{TestData, TestResult, init_tests, make_ns_key_down_event};

fn test_simple_text_input() -> TestResult {
    let mtm = MainThreadMarker::new().unwrap();
    let mut test_data = TestData::default();

    let all_keys = (0x0020..0x007E)
        .chain(0x0020..0x007E)
        .chain(0x00A1..0x00BF)
        .chain(0x00C0..0x00FF)
        .chain(0x0400..0x04FF)
        .chain(0x0600..0x06FF)
        .chain(0x1D00..0x1EFF)
        .chain(0x30A0..0x30FF)
        .chain(0x1F600..0x1F64F)
        .chain(0x31350..0x313AF)
        .map(|v| {
            let key = char::from_u32(v).unwrap();
            let d = NonNull::<std::ffi::c_void>::new((&raw const key).cast_mut().cast()).unwrap();
            unsafe { NSString::initWithBytes_length_encoding(mtm.alloc(), d, 4, NSUTF32LittleEndianStringEncoding) }.unwrap()
        })
        .collect::<Vec<_>>();

    for keys in &all_keys {
        test_data
            .events_to_send
            .push(make_ns_key_down_event(test_data.window_id, keys, NSEventModifierFlags(0)));
        test_data
            .expected_text_operations
            .push(TextOperation::TextChanged(TextChangedOperation {
                window_id: test_data.window_id,
                original_event: None, // not yet checked in tests
                text: borrow_ns_string(keys),
            }));
    }
    test_data.run_test()
}

// TODO: add the following tests
// * Send [ARROW_RIGHT] -> Event::KeyDown(ARROW_RIGHT)
// * Send [Ctrl+ARROW_RIGHT] -> Event::KeyDown(Ctrl+ARROW_RIGHT)
// * Send [Opt+ARROW_RIGHT] -> Event::KeyDown(Option+ARROW_RIGHT)
// * Send [Cmd+ARROW_RIGHT] -> Event::KeyDown(Command+ARROW_RIGHT)
// * Send [Option+U, u] -> TextChangedOperation(text: "ü", original_event: Some)
// * Send [Option+U, m] -> TextChangedOperation(text: "¨m")
// * Send [Option+U, ESC]   -> TextChangedOperation(text: "¨")
// * Send [Option+U, SPACE] -> TextChangedOperation(text: "¨")
// * Send [Option+U, Option+U, o] -> TextChangedOperation(text: "¨ö")
// * Send [LONG_PRESS a, SPACE] -> TextChangedOperation(text: "a ")
// * Send [LONG_PRESS a, RIGHT_ARROW, ENTER] -> TextChangedOperation(text: "à")
// * Send [LONG_PRESS a, Ctrl+RIGHT_ARROW, ENTER] -> TextChangedOperation(text: "à")
// * Send [LONG_PRESS a, Cmd+RIGHT_ARROW, ENTER] -> TextChangedOperation(text: "à")
// * Send [LONG_PRESS a, Opt+RIGHT_ARROW, ENTER] -> TextChangedOperation(text: "à")
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
    init_tests();

    let mut tests = Vec::<Trial>::new();
    tests.push(Trial::test("test_simple_text_input", test_simple_text_input));

    libtest_mimic::run(&args, tests).exit();
}
