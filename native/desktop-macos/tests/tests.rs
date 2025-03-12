mod utils;

use std::{ffi::CStr, ptr::NonNull};

use desktop_macos::{
    common::BorrowedStrPtr,
    macos::{
        events::{Event, KeyDownEvent, KeyUpEvent},
        keyboard::{KeyCode, KeyModifiersSet},
        string::{borrow_ns_string, copy_to_ns_string},
        text_operations::{SetMarkedTextOperation, TextChangedOperation, TextOperation, TextRange},
    },
};
use libtest_mimic::Trial;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::NSEventModifierFlags;
use objc2_foundation::{NSString, NSUTF32LittleEndianStringEncoding};
use utils::test_utils::{TestData, TestResult, custom_ime_handler, init_tests, make_ns_key_down_event, make_ns_key_up_event};

fn ns_string_from_codepoint(mtm: MainThreadMarker, codepoint: u32) -> Retained<NSString> {
    let key = char::from_u32(codepoint).unwrap();
    let d = NonNull::<std::ffi::c_void>::new((&raw const key).cast_mut().cast()).unwrap();
    unsafe { NSString::initWithBytes_length_encoding(mtm.alloc(), d, 4, NSUTF32LittleEndianStringEncoding) }.unwrap()
}

fn get_printable_unicode_characters() -> Vec<Retained<NSString>> {
    let mtm = MainThreadMarker::new().unwrap();
    (0x0020..0x007E)
        .chain(0x00A1..0x00BF)
        .chain(0x00C0..0x00FF)
        .chain(0x0400..0x04FF)
        .chain(0x0600..0x06FF)
        .chain(0x1D00..0x1EFF)
        .chain(0x30A0..0x30FF)
        .chain(0x1F600..0x1F64F)
        .chain(0x31350..0x313AF)
        .map(|codepoint| ns_string_from_codepoint(mtm, codepoint))
        .collect()
}

fn test_simple_text_input() -> TestResult {
    let mut test_data = TestData::default();
    let all_keys = get_printable_unicode_characters();

    for keys in &all_keys {
        test_data
            .events_to_send
            .push(make_ns_key_down_event(test_data.window_id, keys, keys, NSEventModifierFlags(0), 0));
        test_data.expected_events.push(Event::KeyDown(KeyDownEvent {
            window_id: test_data.window_id,
            modifiers: KeyModifiersSet(0),
            code: KeyCode(0),
            characters: borrow_ns_string(&keys),
            key: borrow_ns_string(&keys),
            is_repeat: false,
            timestamp: 0.0,
        }));
        test_data
            .expected_text_operations
            .push(TextOperation::TextChanged(TextChangedOperation {
                window_id: test_data.window_id,
                text: borrow_ns_string(keys),
            }));
    }
    test_data.ignore_non_text_events = true;
    test_data.run_test()
}

struct RawKeyInfo {
    key_code: u16,
    us_unmodchars: Retained<NSString>,
    ctrl_chars: Retained<NSString>,
}

fn test_input_with_modifiers2() -> TestResult {
    let mut test_data = TestData::default();
    let mtm = MainThreadMarker::new().unwrap();
    let all_raw_key_info = [
        RawKeyInfo {
            key_code: 53,
            us_unmodchars: ns_string_from_codepoint(mtm, 27),
            ctrl_chars: ns_string_from_codepoint(mtm, 27),
        },
        RawKeyInfo {
            key_code: 122,
            us_unmodchars: ns_string_from_codepoint(mtm, 63236),
            ctrl_chars: ns_string_from_codepoint(mtm, 63236),
        },
        RawKeyInfo {
            key_code: 120,
            us_unmodchars: ns_string_from_codepoint(mtm, 63237),
            ctrl_chars: ns_string_from_codepoint(mtm, 63237),
        },
        RawKeyInfo {
            key_code: 99,
            us_unmodchars: ns_string_from_codepoint(mtm, 63238),
            ctrl_chars: ns_string_from_codepoint(mtm, 63238),
        },
        RawKeyInfo {
            key_code: 118,
            us_unmodchars: ns_string_from_codepoint(mtm, 63239),
            ctrl_chars: ns_string_from_codepoint(mtm, 63239),
        },
        RawKeyInfo {
            key_code: 96,
            us_unmodchars: ns_string_from_codepoint(mtm, 63240),
            ctrl_chars: ns_string_from_codepoint(mtm, 63240),
        },
        RawKeyInfo {
            key_code: 97,
            us_unmodchars: ns_string_from_codepoint(mtm, 63241),
            ctrl_chars: ns_string_from_codepoint(mtm, 63241),
        },
        RawKeyInfo {
            key_code: 98,
            us_unmodchars: ns_string_from_codepoint(mtm, 63242),
            ctrl_chars: ns_string_from_codepoint(mtm, 63242),
        },
        RawKeyInfo {
            key_code: 100,
            us_unmodchars: ns_string_from_codepoint(mtm, 63243),
            ctrl_chars: ns_string_from_codepoint(mtm, 63243),
        },
        RawKeyInfo {
            key_code: 101,
            us_unmodchars: ns_string_from_codepoint(mtm, 63244),
            ctrl_chars: ns_string_from_codepoint(mtm, 63244),
        },
        RawKeyInfo {
            key_code: 109,
            us_unmodchars: ns_string_from_codepoint(mtm, 63245),
            ctrl_chars: ns_string_from_codepoint(mtm, 63245),
        },
        RawKeyInfo {
            key_code: 110,
            us_unmodchars: ns_string_from_codepoint(mtm, 63246),
            ctrl_chars: ns_string_from_codepoint(mtm, 63246),
        },
        RawKeyInfo {
            key_code: 111,
            us_unmodchars: ns_string_from_codepoint(mtm, 63247),
            ctrl_chars: ns_string_from_codepoint(mtm, 63247),
        },
        RawKeyInfo {
            key_code: 50,
            us_unmodchars: ns_string_from_codepoint(mtm, 96),
            ctrl_chars: ns_string_from_codepoint(mtm, 96),
        },
        RawKeyInfo {
            key_code: 18,
            us_unmodchars: ns_string_from_codepoint(mtm, 49),
            ctrl_chars: ns_string_from_codepoint(mtm, 49),
        },
        RawKeyInfo {
            key_code: 19,
            us_unmodchars: ns_string_from_codepoint(mtm, 50),
            ctrl_chars: ns_string_from_codepoint(mtm, 50),
        },
        RawKeyInfo {
            key_code: 20,
            us_unmodchars: ns_string_from_codepoint(mtm, 51),
            ctrl_chars: ns_string_from_codepoint(mtm, 51),
        },
        RawKeyInfo {
            key_code: 21,
            us_unmodchars: ns_string_from_codepoint(mtm, 52),
            ctrl_chars: ns_string_from_codepoint(mtm, 52),
        },
        RawKeyInfo {
            key_code: 23,
            us_unmodchars: ns_string_from_codepoint(mtm, 53),
            ctrl_chars: ns_string_from_codepoint(mtm, 53),
        },
        RawKeyInfo {
            key_code: 22,
            us_unmodchars: ns_string_from_codepoint(mtm, 54),
            ctrl_chars: ns_string_from_codepoint(mtm, 54),
        },
        RawKeyInfo {
            key_code: 26,
            us_unmodchars: ns_string_from_codepoint(mtm, 55),
            ctrl_chars: ns_string_from_codepoint(mtm, 55),
        },
        RawKeyInfo {
            key_code: 28,
            us_unmodchars: ns_string_from_codepoint(mtm, 56),
            ctrl_chars: ns_string_from_codepoint(mtm, 56),
        },
        RawKeyInfo {
            key_code: 25,
            us_unmodchars: ns_string_from_codepoint(mtm, 57),
            ctrl_chars: ns_string_from_codepoint(mtm, 57),
        },
        RawKeyInfo {
            key_code: 29,
            us_unmodchars: ns_string_from_codepoint(mtm, 48),
            ctrl_chars: ns_string_from_codepoint(mtm, 48),
        },
        RawKeyInfo {
            key_code: 27,
            us_unmodchars: ns_string_from_codepoint(mtm, 45),
            ctrl_chars: ns_string_from_codepoint(mtm, 31),
        },
        RawKeyInfo {
            key_code: 24,
            us_unmodchars: ns_string_from_codepoint(mtm, 61),
            ctrl_chars: ns_string_from_codepoint(mtm, 61),
        },
        RawKeyInfo {
            key_code: 51,
            us_unmodchars: ns_string_from_codepoint(mtm, 127),
            ctrl_chars: ns_string_from_codepoint(mtm, 127),
        },
        RawKeyInfo {
            key_code: 48,
            us_unmodchars: ns_string_from_codepoint(mtm, 9),
            ctrl_chars: ns_string_from_codepoint(mtm, 9),
        },
        // TODO: [Ctrl+Q, Ctrl+[any key]] produce the unexpected TextChangedOperation
        //RawKeyInfo {
        //    key_code: 12,
        //    us_unmodchars: ns_string_from_codepoint(mtm, 113),
        //    ctrl_chars: ns_string_from_codepoint(mtm, 17),
        //},
        RawKeyInfo {
            key_code: 13,
            us_unmodchars: ns_string_from_codepoint(mtm, 119),
            ctrl_chars: ns_string_from_codepoint(mtm, 23),
        },
        RawKeyInfo {
            key_code: 14,
            us_unmodchars: ns_string_from_codepoint(mtm, 101),
            ctrl_chars: ns_string_from_codepoint(mtm, 5),
        },
        RawKeyInfo {
            key_code: 15,
            us_unmodchars: ns_string_from_codepoint(mtm, 114),
            ctrl_chars: ns_string_from_codepoint(mtm, 18),
        },
        RawKeyInfo {
            key_code: 17,
            us_unmodchars: ns_string_from_codepoint(mtm, 116),
            ctrl_chars: ns_string_from_codepoint(mtm, 20),
        },
        RawKeyInfo {
            key_code: 16,
            us_unmodchars: ns_string_from_codepoint(mtm, 121),
            ctrl_chars: ns_string_from_codepoint(mtm, 25),
        },
        RawKeyInfo {
            key_code: 32,
            us_unmodchars: ns_string_from_codepoint(mtm, 117),
            ctrl_chars: ns_string_from_codepoint(mtm, 21),
        },
        RawKeyInfo {
            key_code: 34,
            us_unmodchars: ns_string_from_codepoint(mtm, 105),
            ctrl_chars: ns_string_from_codepoint(mtm, 9),
        },
        RawKeyInfo {
            key_code: 31,
            us_unmodchars: ns_string_from_codepoint(mtm, 111),
            ctrl_chars: ns_string_from_codepoint(mtm, 15),
        },
        RawKeyInfo {
            key_code: 35,
            us_unmodchars: ns_string_from_codepoint(mtm, 112),
            ctrl_chars: ns_string_from_codepoint(mtm, 16),
        },
        RawKeyInfo {
            key_code: 33,
            us_unmodchars: ns_string_from_codepoint(mtm, 91),
            ctrl_chars: ns_string_from_codepoint(mtm, 27),
        },
        RawKeyInfo {
            key_code: 30,
            us_unmodchars: ns_string_from_codepoint(mtm, 93),
            ctrl_chars: ns_string_from_codepoint(mtm, 29),
        },
        RawKeyInfo {
            key_code: 42,
            us_unmodchars: ns_string_from_codepoint(mtm, 92),
            ctrl_chars: ns_string_from_codepoint(mtm, 28),
        },
        RawKeyInfo {
            key_code: 0,
            us_unmodchars: ns_string_from_codepoint(mtm, 97),
            ctrl_chars: ns_string_from_codepoint(mtm, 1),
        },
        RawKeyInfo {
            key_code: 1,
            us_unmodchars: ns_string_from_codepoint(mtm, 115),
            ctrl_chars: ns_string_from_codepoint(mtm, 19),
        },
        RawKeyInfo {
            key_code: 2,
            us_unmodchars: ns_string_from_codepoint(mtm, 100),
            ctrl_chars: ns_string_from_codepoint(mtm, 4),
        },
        RawKeyInfo {
            key_code: 3,
            us_unmodchars: ns_string_from_codepoint(mtm, 102),
            ctrl_chars: ns_string_from_codepoint(mtm, 6),
        },
        RawKeyInfo {
            key_code: 5,
            us_unmodchars: ns_string_from_codepoint(mtm, 103),
            ctrl_chars: ns_string_from_codepoint(mtm, 7),
        },
        RawKeyInfo {
            key_code: 4,
            us_unmodchars: ns_string_from_codepoint(mtm, 104),
            ctrl_chars: ns_string_from_codepoint(mtm, 8),
        },
        RawKeyInfo {
            key_code: 38,
            us_unmodchars: ns_string_from_codepoint(mtm, 106),
            ctrl_chars: ns_string_from_codepoint(mtm, 10),
        },
        RawKeyInfo {
            key_code: 40,
            us_unmodchars: ns_string_from_codepoint(mtm, 107),
            ctrl_chars: ns_string_from_codepoint(mtm, 11),
        },
        RawKeyInfo {
            key_code: 37,
            us_unmodchars: ns_string_from_codepoint(mtm, 108),
            ctrl_chars: ns_string_from_codepoint(mtm, 12),
        },
        RawKeyInfo {
            key_code: 41,
            us_unmodchars: ns_string_from_codepoint(mtm, 59),
            ctrl_chars: ns_string_from_codepoint(mtm, 59),
        },
        RawKeyInfo {
            key_code: 39,
            us_unmodchars: ns_string_from_codepoint(mtm, 39),
            ctrl_chars: ns_string_from_codepoint(mtm, 39),
        },
        RawKeyInfo {
            key_code: 36,
            us_unmodchars: ns_string_from_codepoint(mtm, 13),
            ctrl_chars: ns_string_from_codepoint(mtm, 13),
        },
        RawKeyInfo {
            key_code: 6,
            us_unmodchars: ns_string_from_codepoint(mtm, 122),
            ctrl_chars: ns_string_from_codepoint(mtm, 26),
        },
        RawKeyInfo {
            key_code: 7,
            us_unmodchars: ns_string_from_codepoint(mtm, 120),
            ctrl_chars: ns_string_from_codepoint(mtm, 24),
        },
        RawKeyInfo {
            key_code: 8,
            us_unmodchars: ns_string_from_codepoint(mtm, 99),
            ctrl_chars: ns_string_from_codepoint(mtm, 3),
        },
        RawKeyInfo {
            key_code: 9,
            us_unmodchars: ns_string_from_codepoint(mtm, 118),
            ctrl_chars: ns_string_from_codepoint(mtm, 22),
        },
        RawKeyInfo {
            key_code: 11,
            us_unmodchars: ns_string_from_codepoint(mtm, 98),
            ctrl_chars: ns_string_from_codepoint(mtm, 2),
        },
        RawKeyInfo {
            key_code: 45,
            us_unmodchars: ns_string_from_codepoint(mtm, 110),
            ctrl_chars: ns_string_from_codepoint(mtm, 14),
        },
        RawKeyInfo {
            key_code: 46,
            us_unmodchars: ns_string_from_codepoint(mtm, 109),
            ctrl_chars: ns_string_from_codepoint(mtm, 13),
        },
        RawKeyInfo {
            key_code: 43,
            us_unmodchars: ns_string_from_codepoint(mtm, 44),
            ctrl_chars: ns_string_from_codepoint(mtm, 44),
        },
        RawKeyInfo {
            key_code: 47,
            us_unmodchars: ns_string_from_codepoint(mtm, 46),
            ctrl_chars: ns_string_from_codepoint(mtm, 46),
        },
        RawKeyInfo {
            key_code: 44,
            us_unmodchars: ns_string_from_codepoint(mtm, 47),
            ctrl_chars: ns_string_from_codepoint(mtm, 47),
        },
    ];

    let flags = NSEventModifierFlags::Control;

    for raw_key_info in &all_raw_key_info {
        test_data.events_to_send.push(make_ns_key_down_event(
            test_data.window_id,
            &raw_key_info.ctrl_chars,
            &raw_key_info.us_unmodchars,
            flags,
            raw_key_info.key_code,
        ));
        test_data.events_to_send.push(make_ns_key_up_event(
            test_data.window_id,
            &raw_key_info.ctrl_chars,
            &raw_key_info.us_unmodchars,
            flags,
            raw_key_info.key_code,
        ));

        test_data.expected_events.push(Event::KeyDown(KeyDownEvent {
            window_id: test_data.window_id,
            modifiers: KeyModifiersSet(NSEventModifierFlags::Control.0),
            code: KeyCode(raw_key_info.key_code),
            characters: borrow_ns_string(&raw_key_info.ctrl_chars),
            key: borrow_ns_string(&raw_key_info.ctrl_chars),
            is_repeat: false,
            timestamp: 0.0,
        }));
        test_data.expected_events.push(Event::KeyUp(KeyUpEvent {
            window_id: test_data.window_id,
            modifiers: KeyModifiersSet(NSEventModifierFlags::Control.0),
            code: KeyCode(raw_key_info.key_code),
            characters: borrow_ns_string(&raw_key_info.ctrl_chars),
            key: borrow_ns_string(&raw_key_info.ctrl_chars),
            timestamp: 0.0,
        }));
    }
    test_data.run_test()
}

fn test_input_with_modifiers() -> TestResult {
    let mut test_data = TestData::default();
    let all_keys = get_printable_unicode_characters();
    let q_key = NSString::from_str("q");

    for keys in &all_keys {
        if keys == &q_key {
            // TODO: [Ctrl+Q, Ctrl+[any key]] produce the unexpected TextChangedOperation
            continue;
        }
        test_data.events_to_send.push(make_ns_key_down_event(
            test_data.window_id,
            keys,
            keys,
            NSEventModifierFlags::Control,
            0,
        ));
        test_data.events_to_send.push(make_ns_key_up_event(
            test_data.window_id,
            keys,
            keys,
            NSEventModifierFlags::Control,
            0,
        ));

        test_data.expected_events.push(Event::KeyDown(KeyDownEvent {
            window_id: test_data.window_id,
            modifiers: KeyModifiersSet(NSEventModifierFlags::Control.0),
            code: KeyCode(0),
            characters: borrow_ns_string(keys),
            key: borrow_ns_string(keys),
            is_repeat: false,
            timestamp: 0.0,
        }));

        test_data.expected_events.push(Event::KeyUp(KeyUpEvent {
            window_id: test_data.window_id,
            modifiers: KeyModifiersSet(NSEventModifierFlags::Control.0),
            code: KeyCode(0),
            characters: borrow_ns_string(keys),
            key: borrow_ns_string(keys),
            timestamp: 0.0,
        }));
    }
    test_data.run_test()
}

fn test_navigation_input() -> TestResult {
    let mut test_data = TestData::default();

    let all_keys: Vec<(u16, &CStr)> = vec![
        //(36, c""), // -> Return
        //(48, c""), // -> Tab
        //(49, c""), // -> Space
        (51, c"\u{7F}"),    // -> Delete
        (53, c"\u{1B}"),    // -> Escape
        (126, c"\u{F700}"), // -> UpArrow
        (125, c"\u{F701}"), // -> DownArrow
        (123, c"\u{F702}"), // -> LeftArrow
        (124, c"\u{F703}"), // -> RightArrow
        (122, c"\u{F704}"), // -> F1
        (120, c"\u{F705}"), // -> F2
        (99, c"\u{F706}"),  // -> F3
        (118, c"\u{F707}"), // -> F4
        (96, c"\u{F708}"),  // -> F5
        (97, c"\u{F709}"),  // -> F6
        (98, c"\u{F70A}"),  // -> F7
        (100, c"\u{F70B}"), // -> F8
        (101, c"\u{F70C}"), // -> F9
        (109, c"\u{F70D}"), // -> F10
        (103, c"\u{F70E}"), // -> F11
        (111, c"\u{F70F}"), // -> F12
        (105, c"\u{F710}"), // -> F13
        (107, c"\u{F711}"), // -> F14
        (113, c"\u{F712}"), // -> F15
        (106, c"\u{F713}"), // -> F16
        (64, c"\u{F714}"),  // -> F17
        (79, c"\u{F715}"),  // -> F18
        (80, c"\u{F716}"),  // -> F19
        (90, c"\u{F717}"),  // -> F20
        //(110, c""), // -> ContextualMenu
        //(114, c""), // -> Help
        (117, c"\u{F728}"), // -> ForwardDelete
        (115, c"\u{F729}"), // -> Home
        (119, c"\u{F72B}"), // -> End
        (116, c"\u{F72C}"), // -> PageUp
        (121, c"\u{F72D}"), // -> PageDown
    ];
    for (key, cstr) in all_keys {
        let chars = copy_to_ns_string(&BorrowedStrPtr::new(cstr)).unwrap();
        test_data.events_to_send.push(make_ns_key_down_event(
            test_data.window_id,
            &chars,
            &chars,
            NSEventModifierFlags(0),
            key,
        ));
        test_data.expected_events.push(Event::KeyDown(KeyDownEvent {
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

fn test_dead_keys() -> TestResult {
    let empty_string = NSString::from_str("");
    let unmodchars1 = NSString::from_str("`");

    let chars2 = NSString::from_str("¨");
    let unmodchars2 = NSString::from_str(" ");

    let flags = NSEventModifierFlags::Option;

    let mut test_data = TestData::default();
    test_data.custom_ime_handler = Some(custom_ime_handler);
    test_data.events_to_send = vec![
        make_ns_key_down_event(test_data.window_id, &empty_string, &unmodchars1, flags, 50),
        make_ns_key_down_event(test_data.window_id, &chars2, &unmodchars2, NSEventModifierFlags(0), 49),
    ];
    test_data.expected_events = vec![Event::KeyDown(KeyDownEvent {
        window_id: test_data.window_id,
        modifiers: KeyModifiersSet(flags.0),
        code: KeyCode(50),
        characters: borrow_ns_string(&empty_string),
        key: borrow_ns_string(&empty_string),
        is_repeat: false,
        timestamp: 0.0,
    })];
    test_data.expected_text_operations = vec![
        TextOperation::SetMarkedText(SetMarkedTextOperation {
            window_id: test_data.window_id,
            text: borrow_ns_string(&unmodchars1),
            selected_range: TextRange { location: 1, length: 0 },
            replacement_range: TextRange {
                location: 9223372036854775807,
                length: 0,
            },
        }),
        TextOperation::TextChanged(TextChangedOperation {
            window_id: test_data.window_id,
            text: borrow_ns_string(&unmodchars1),
        }),
    ];

    test_data.ignore_non_text_events = true;
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
    tests.push(Trial::test("test_navigation_input", test_navigation_input));
    tests.push(Trial::test("test_input_with_modifiers", test_input_with_modifiers));
    tests.push(Trial::test("test_input_with_modifiers2", test_input_with_modifiers2));
    tests.push(Trial::test("test_dead_keys", test_dead_keys));

    libtest_mimic::run(&args, tests).exit();
}
