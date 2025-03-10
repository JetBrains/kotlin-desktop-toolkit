mod utils;

use std::{ffi::CStr, ptr::NonNull};

use desktop_macos::{common::BorrowedStrPtr, macos::{
    events::{Event, KeyDownEvent}, keyboard::{KeyCode, KeyModifiersSet}, string::{borrow_ns_string, copy_to_ns_string}, text_operations::{TextChangedOperation, TextOperation}
}};
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

    let cstr = c"";
    for keys in &all_keys {
        test_data
            .events_to_send
            .push(make_ns_key_down_event(test_data.window_id, keys, NSEventModifierFlags(0), 0));
//        test_data
//            .expected_events
//            .push(Event::KeyDown(KeyDownEvent{
//                window_id: test_data.window_id,
//                modifiers: KeyModifiersSet(0),
//                code: KeyCode(0),
//                characters: BorrowedStrPtr::new(cstr),
//                key: BorrowedStrPtr::new(cstr),
//                is_repeat: false,
//                timestamp: 0.0,
//            }));
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

fn test_navigation_input() -> TestResult {
    let mtm = MainThreadMarker::new().unwrap();
    let mut test_data = TestData::default();

    let all_keys: Vec<(u16, &CStr)> = vec![
//        (36, c""), // -> Return
//        (48, c""), // -> Tab
//        (49, c""), // -> Space
//        (51, c""), // -> Delete
//        (53, c"\u{1B}"), // -> Escape
//        (57, c""), // -> CapsLock
//        (63, c""), // -> Function
//        (64, c""), // -> F17
//        (72, c""), // -> VolumeUp
//        (73, c""), // -> VolumeDown
//        (74, c""), // -> Mute
//        (79, c""), // -> F18
//        (80, c""), // -> F19
//        (90, c""), // -> F20
//        (96, c"\u{F708}"), // -> F5
//        (97, c"\u{F709}"), // -> F6
//        (98, c"\u{F70A}"), // -> F7
//        (99, c"\u{F706}"), // -> F3
//        (100, c"\u{F70B}"), // -> F8
//        (101, c"\u{F70C}"), // -> F9
//        (103, c"\u{F70E}"), // -> F11
//        (105, c""), // -> F13
//        (106, c""), // -> F16
//        (107, c""), // -> F14
//        (109, c"\u{F70D}"), // -> F10
//        (110, c""), // -> ContextualMenu
//        (111, c"\u{F70F}"), // -> F12
//        (113, c""), // -> F15
//        (114, c""), // -> Help
//        (115, c""), // -> Home
//        (116, c""), // -> PageUp
//        (117, c""), // -> ForwardDelete
//        (118, c"\u{F707}"), // -> F4
//        (119, c""), // -> End
//        (120, c"\u{F705}"), // -> F2
//        (121, c""), // -> PageDown
//        (122, c"\u{F704}"), // -> F1
        (123, c"\u{F702}"), // -> LeftArrow
        (124, c"\u{F703}"), // -> RightArrow
        (125, c"\u{F701}"), // -> DownArrow
        (126, c"\u{F700}"), // -> UpArrow
    ];
    for (key, cstr) in all_keys {
        test_data
            .events_to_send
            .push(make_ns_key_down_event(test_data.window_id, &copy_to_ns_string(&BorrowedStrPtr::new(cstr)).unwrap(), NSEventModifierFlags(0), key));
        test_data
            .expected_events
            .push(Event::KeyDown(KeyDownEvent{
                window_id: test_data.window_id,
                modifiers: KeyModifiersSet(0),
                code: KeyCode(key),
                characters: BorrowedStrPtr::new(cstr),
                key: BorrowedStrPtr::new(cstr),
                is_repeat: false,
                timestamp: 0.0,
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
//    tests.push(Trial::test("test_simple_text_input", test_simple_text_input));
    tests.push(Trial::test("test_navigation_input", test_navigation_input));

    libtest_mimic::run(&args, tests).exit();
}
