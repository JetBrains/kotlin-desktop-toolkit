use objc2_app_kit::{NSApplication, NSEventModifierFlags};
use objc2_foundation::MainThreadMarker;

use crate::common::{ArraySize, StrPtr};
use crate::logger::ffi_boundary;
use super::keyboard::KeyModifiers;
use super::{application_api::MyNSApplication, application_menu::main_menu_update_impl};

// This file contains C API of the library
// The symbols listed here will be exported into .h file

// Application Menu:

#[repr(C)]
#[derive(Debug)]
pub struct AppMenuKeystroke {
    pub key: StrPtr,
    pub modifiers: KeyModifiers
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
    ffi_boundary("main_menu_update", || {
        main_menu_update_impl(menu);
        Ok(())
    });
}

#[no_mangle]
pub extern "C" fn main_menu_set_none() {
    ffi_boundary("main_menu_set_none", || {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        app.setMainMenu(None);
        Ok(())
    });
}