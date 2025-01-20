use anyhow::{bail, Context, Error, Ok};
use log::{error, info};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use objc2::rc::Retained;
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType};
use objc2_foundation::NSString;

#[derive(Debug)]
pub(crate) struct KeyEventInfo {
    pub(crate) is_press: bool,
    pub(crate) is_repeat: bool,
    pub(crate) code: KeyCode,
//    chars: Retained<NSString>,
//    with_modifiers: Retained<NSString>,
//    no_modifiers: Retained<NSString>
}

// todo unpack modifiers
pub(crate) fn unpack_key_event(ns_event: &NSEvent) -> anyhow::Result<KeyEventInfo> {
    let is_press = match unsafe { ns_event.r#type() } {
        NSEventType::KeyDown => true,
        NSEventType::KeyUp => false,
        _ => bail!("Unexpected type of event {:?}", ns_event)
    };

    let is_repeat = unsafe { ns_event.isARepeat() };
    let code = unsafe { ns_event.keyCode() };

    let chars = unsafe { ns_event.characters() }.with_context(|| {
        format!("No characters field in {ns_event:?}")
    })?;

    let no_modifiers = unsafe {
        ns_event.charactersByApplyingModifiers(NSEventModifierFlags::empty())
    }.with_context(|| { format!("Event contains invalid data: {ns_event:?}") })?;

    // though we apply the same modifiers, it's not the same as characters
    // there are number of differences:
    // * for dead keys `characters` will be empty, but this string will contain symbol representing the key
    // * for for keys like F1..F12 characters will contain codepoints from private use area defined in `KeyCodePoints`,
    // but this function will try to return some meaniingful code points
    // * for all F1..F16 keys this function will return the same codepoint: \u{10} for F17 it will be empty line
    let with_modifiers = unsafe {
        ns_event.charactersByApplyingModifiers(ns_event.modifierFlags())
    }.with_context(|| { format!("Event contains invalid data: {ns_event:?}") })?;

    let code = KeyCode::from_u16(code).with_context(|| { format!("Event with unexpected key code: {ns_event:?}") })?;
    let key = KeyEventInfo {
        is_press,
        is_repeat,
        code,
//        chars,
//        no_modifiers,
//        with_modifiers
    };
    Ok(key)
}

//pub enum KeyCodePoints {
//    NSUpArrowFunctionKey = 0xF700,
//    NSDownArrowFunctionKey = 0xF701,
//    NSLeftArrowFunctionKey = 0xF702,
//    NSRightArrowFunctionKey = 0xF703,
//    NSF1FunctionKey = 0xF704,
//    NSF2FunctionKey = 0xF705,
//    NSF3FunctionKey = 0xF706,
//    NSF4FunctionKey = 0xF707,
//    NSF5FunctionKey = 0xF708,
//    NSF6FunctionKey = 0xF709,
//    NSF7FunctionKey = 0xF70A,
//    NSF8FunctionKey = 0xF70B,
//    NSF9FunctionKey = 0xF70C,
//    NSF10FunctionKey = 0xF70D,
//    NSF11FunctionKey = 0xF70E,
//    NSF12FunctionKey = 0xF70F,
//    NSF13FunctionKey = 0xF710,
//    NSF14FunctionKey = 0xF711,
//    NSF15FunctionKey = 0xF712,
//    NSF16FunctionKey = 0xF713,
//    NSF17FunctionKey = 0xF714,
//    NSF18FunctionKey = 0xF715,
//    NSF19FunctionKey = 0xF716,
//    NSF20FunctionKey = 0xF717,
//    NSF21FunctionKey = 0xF718,
//    NSF22FunctionKey = 0xF719,
//    NSF23FunctionKey = 0xF71A,
//    NSF24FunctionKey = 0xF71B,
//    NSF25FunctionKey = 0xF71C,
//    NSF26FunctionKey = 0xF71D,
//    NSF27FunctionKey = 0xF71E,
//    NSF28FunctionKey = 0xF71F,
//    NSF29FunctionKey = 0xF720,
//    NSF30FunctionKey = 0xF721,
//    NSF31FunctionKey = 0xF722,
//    NSF32FunctionKey = 0xF723,
//    NSF33FunctionKey = 0xF724,
//    NSF34FunctionKey = 0xF725,
//    NSF35FunctionKey = 0xF726,
//    NSInsertFunctionKey = 0xF727,
//    NSDeleteFunctionKey = 0xF728,
//    NSHomeFunctionKey = 0xF729,
//    NSBeginFunctionKey = 0xF72A,
//    NSEndFunctionKey = 0xF72B,
//    NSPageUpFunctionKey = 0xF72C,
//    NSPageDownFunctionKey = 0xF72D,
//    NSPrintScreenFunctionKey = 0xF72E,
//    NSScrollLockFunctionKey = 0xF72F,
//    NSPauseFunctionKey = 0xF730,
//    NSSysReqFunctionKey = 0xF731,
//    NSBreakFunctionKey = 0xF732,
//    NSResetFunctionKey = 0xF733,
//    NSStopFunctionKey = 0xF734,
//    NSMenuFunctionKey = 0xF735,
//    NSUserFunctionKey = 0xF736,
//    NSSystemFunctionKey = 0xF737,
//    NSPrintFunctionKey = 0xF738,
//    NSClearLineFunctionKey = 0xF739,
//    NSClearDisplayFunctionKey = 0xF73A,
//    NSInsertLineFunctionKey = 0xF73B,
//    NSDeleteLineFunctionKey = 0xF73C,
//    NSInsertCharFunctionKey = 0xF73D,
//    NSDeleteCharFunctionKey = 0xF73E,
//    NSPrevFunctionKey = 0xF73F,
//    NSNextFunctionKey = 0xF740,
//    NSSelectFunctionKey = 0xF741,
//    NSExecuteFunctionKey = 0xF742,
//    NSUndoFunctionKey = 0xF743,
//    NSRedoFunctionKey = 0xF744,
//    NSFindFunctionKey = 0xF745,
//    NSHelpFunctionKey = 0xF746,
//    NSModeSwitchFunctionKey = 0xF747,
//}

/*  MacOSX15.2
 *  Summary:
 *    Virtual keycodes
 *
 *  Discussion:
 *    These constants are the virtual keycodes defined originally in
 *    Inside Mac Volume V, pg. V-191. They identify physical keys on a
 *    keyboard. Those constants with "ANSI" in the name are labeled
 *    according to the key position on an ANSI-standard US keyboard.
 *    For example, kVK_ANSI_A indicates the virtual keycode for the key
 *    with the letter 'A' in the US keyboard layout. Other keyboard
 *    layouts may have the 'A' key label on a different physical key;
 *    in this case, pressing 'A' will generate a different virtual
 *    keycode.
 */
#[derive(Debug, Clone, Copy, FromPrimitive, ToPrimitive)]
#[repr(C)]
#[allow(non_camel_case_types)]
pub enum KeyCode {
    VK_ANSI_A                    = 0x00,
    VK_ANSI_S                    = 0x01,
    VK_ANSI_D                    = 0x02,
    VK_ANSI_F                    = 0x03,
    VK_ANSI_H                    = 0x04,
    VK_ANSI_G                    = 0x05,
    VK_ANSI_Z                    = 0x06,
    VK_ANSI_X                    = 0x07,
    VK_ANSI_C                    = 0x08,
    VK_ANSI_V                    = 0x09,
    VK_ANSI_B                    = 0x0B,
    VK_ANSI_Q                    = 0x0C,
    VK_ANSI_W                    = 0x0D,
    VK_ANSI_E                    = 0x0E,
    VK_ANSI_R                    = 0x0F,
    VK_ANSI_Y                    = 0x10,
    VK_ANSI_T                    = 0x11,
    VK_ANSI_1                    = 0x12,
    VK_ANSI_2                    = 0x13,
    VK_ANSI_3                    = 0x14,
    VK_ANSI_4                    = 0x15,
    VK_ANSI_6                    = 0x16,
    VK_ANSI_5                    = 0x17,
    VK_ANSI_Equal                = 0x18,
    VK_ANSI_9                    = 0x19,
    VK_ANSI_7                    = 0x1A,
    VK_ANSI_Minus                = 0x1B,
    VK_ANSI_8                    = 0x1C,
    VK_ANSI_0                    = 0x1D,
    VK_ANSI_RightBracket         = 0x1E,
    VK_ANSI_O                    = 0x1F,
    VK_ANSI_U                    = 0x20,
    VK_ANSI_LeftBracket          = 0x21,
    VK_ANSI_I                    = 0x22,
    VK_ANSI_P                    = 0x23,
    VK_ANSI_L                    = 0x25,
    VK_ANSI_J                    = 0x26,
    VK_ANSI_Quote                = 0x27,
    VK_ANSI_K                    = 0x28,
    VK_ANSI_Semicolon            = 0x29,
    VK_ANSI_Backslash            = 0x2A,
    VK_ANSI_Comma                = 0x2B,
    VK_ANSI_Slash                = 0x2C,
    VK_ANSI_N                    = 0x2D,
    VK_ANSI_M                    = 0x2E,
    VK_ANSI_Period               = 0x2F,
    VK_ANSI_Grave                = 0x32,
    VK_ANSI_KeypadDecimal        = 0x41,
    VK_ANSI_KeypadMultiply       = 0x43,
    VK_ANSI_KeypadPlus           = 0x45,
    VK_ANSI_KeypadClear          = 0x47,
    VK_ANSI_KeypadDivide         = 0x4B,
    VK_ANSI_KeypadEnter          = 0x4C,
    VK_ANSI_KeypadMinus          = 0x4E,
    VK_ANSI_KeypadEquals         = 0x51,
    VK_ANSI_Keypad0              = 0x52,
    VK_ANSI_Keypad1              = 0x53,
    VK_ANSI_Keypad2              = 0x54,
    VK_ANSI_Keypad3              = 0x55,
    VK_ANSI_Keypad4              = 0x56,
    VK_ANSI_Keypad5              = 0x57,
    VK_ANSI_Keypad6              = 0x58,
    VK_ANSI_Keypad7              = 0x59,
    VK_ANSI_Keypad8              = 0x5B,
    VK_ANSI_Keypad9              = 0x5C,

    /* keycodes for keys that are independent of keyboard layout*/
    VK_Return                    = 0x24,
    VK_Tab                       = 0x30,
    VK_Space                     = 0x31,
    VK_Delete                    = 0x33,
    VK_Escape                    = 0x35,
    VK_Command                   = 0x37,
    VK_Shift                     = 0x38,
    VK_CapsLock                  = 0x39,
    VK_Option                    = 0x3A,
    VK_Control                   = 0x3B,
    VK_RightCommand              = 0x36,
    VK_RightShift                = 0x3C,
    VK_RightOption               = 0x3D,
    VK_RightControl              = 0x3E,
    VK_Function                  = 0x3F,
    VK_F17                       = 0x40,
    VK_VolumeUp                  = 0x48,
    VK_VolumeDown                = 0x49,
    VK_Mute                      = 0x4A,
    VK_F18                       = 0x4F,
    VK_F19                       = 0x50,
    VK_F20                       = 0x5A,
    VK_F5                        = 0x60,
    VK_F6                        = 0x61,
    VK_F7                        = 0x62,
    VK_F3                        = 0x63,
    VK_F8                        = 0x64,
    VK_F9                        = 0x65,
    VK_F11                       = 0x67,
    VK_F13                       = 0x69,
    VK_F16                       = 0x6A,
    VK_F14                       = 0x6B,
    VK_F10                       = 0x6D,
    VK_ContextualMenu            = 0x6E,
    VK_F12                       = 0x6F,
    VK_F15                       = 0x71,
    VK_Help                      = 0x72,
    VK_Home                      = 0x73,
    VK_PageUp                    = 0x74,
    VK_ForwardDelete             = 0x75,
    VK_F4                        = 0x76,
    VK_End                       = 0x77,
    VK_F2                        = 0x78,
    VK_PageDown                  = 0x79,
    VK_F1                        = 0x7A,
    VK_LeftArrow                 = 0x7B,
    VK_RightArrow                = 0x7C,
    VK_DownArrow                 = 0x7D,
    VK_UpArrow                   = 0x7E,

    /* ISO keyboards only*/
    VK_ISO_Section               = 0x0A,

    VK_JIS_Yen                   = 0x5D,
    VK_JIS_Underscore            = 0x5E,
    VK_JIS_KeypadComma           = 0x5F,
    VK_JIS_Eisu                  = 0x66,
    VK_JIS_Kana                  = 0x68
}