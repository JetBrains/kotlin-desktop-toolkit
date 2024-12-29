use bitflags::bitflags;
use objc2_app_kit::{NSApplication, NSEventModifierFlags};
use objc2_foundation::MainThreadMarker;

use crate::common::{ArraySize, StrPtr};

use super::{application_api::MyNSApplication, application_menu::main_menu_update_impl};

// This file contains C API of the library
// The symbols listed here will be exported into .h file

// Application Menu:

bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy)]
    // same valus as in NSEventModifierFlags
    pub struct AppMenuKeyModifiers: u32 {
        const ModifierFlagCapsLock = 1<<16;
        const ModifierFlagShift = 1<<17;
        const ModifierFlagControl = 1<<18;
        const ModifierFlagOption = 1<<19;
        const ModifierFlagCommand = 1<<20;
        const ModifierFlagNumericPad = 1<<21;
        const ModifierFlagHelp = 1<<22;
        const ModifierFlagFunction = 1<<23;
        const ModifierFlagDeviceIndependentFlagsMask = 0xffff0000;
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct AppMenuKeystroke {
    // TODO Function keys, enter, arrows, etc
    pub key: StrPtr,
    pub modifiers: AppMenuKeyModifiers
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Debug)]
pub enum AppMenuItem {
    ActionItem {
        enabled: bool,
        title: StrPtr,
        macos_provided: bool,
        keystroke: *const AppMenuKeystroke, // todo replace nullable pointers with Option<&AppMenuKeystroke> here?
        perform: extern "C" fn()
    },
    SeparatorItem,
    SubMenuItem {
        title: StrPtr,
        special_tag: StrPtr,
        items: *const AppMenuItem,
        items_count: ArraySize,
    },
}

#[repr(C)]
#[derive(Debug)]
pub struct AppMenuStructure {
    pub items: *const AppMenuItem,
    pub items_count: ArraySize,
}

#[no_mangle]
pub extern "C" fn main_menu_update(menu: AppMenuStructure) {
    main_menu_update_impl(menu);
}

#[no_mangle]
pub extern "C" fn main_menu_set_none() {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let app = MyNSApplication::sharedApplication(mtm);
    app.setMainMenu(None);
}