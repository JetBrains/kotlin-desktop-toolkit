use desktop_common::ffi_utils::{ArraySize, BorrowedStrPtr};
use desktop_common::logger::ffi_boundary;
use objc2_foundation::MainThreadMarker;

use super::keyboard::KeyModifiersSet;
use super::{application_api::MyNSApplication, application_menu::main_menu_update_impl};

// This file contains C API of the library
// The symbols listed here will be exported into .h file

// Application Menu:

#[repr(C)]
#[derive(Debug)]
pub struct AppMenuKeystroke<'a> {
    pub key: BorrowedStrPtr<'a>,
    pub modifiers: KeyModifiersSet,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubMenuItemSpecialTag {
    None,
    AppNameMenu,
    Window,
    Services,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionMenuItemSpecialTag {
    None,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionItemState {
    On,
    Off,
    Mixed,
}

#[repr(C)]
pub enum AppMenuTrigger {
    Keystroke,
    Other,
}

pub type AppMenuItemCallback = extern "C" fn(trigger: AppMenuTrigger);

#[repr(C)]
#[derive(Debug)]
pub enum AppMenuItem<'a> {
    ActionItem {
        enabled: bool,
        state: ActionItemState,
        title: BorrowedStrPtr<'a>,
        special_tag: ActionMenuItemSpecialTag,
        keystroke: Option<&'a AppMenuKeystroke<'a>>,
        perform: AppMenuItemCallback,
    },
    SeparatorItem,
    SubMenuItem {
        title: BorrowedStrPtr<'a>,
        special_tag: SubMenuItemSpecialTag,
        items: *const AppMenuItem<'a>,
        items_count: ArraySize,
    },
}

#[repr(C)]
#[derive(Debug)]
pub struct AppMenuStructure<'a> {
    pub items: *const AppMenuItem<'a>,
    pub items_count: ArraySize,
}

#[unsafe(no_mangle)]
pub extern "C" fn main_menu_update(menu: AppMenuStructure) {
    ffi_boundary("main_menu_update", || {
        main_menu_update_impl(&menu);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn main_menu_set_none() {
    ffi_boundary("main_menu_set_none", || {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        app.setMainMenu(None);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn main_menu_offer_current_event() -> bool {
    ffi_boundary("main_menu_offer_current_event", || {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        let result = match (app.currentEvent(), unsafe { app.mainMenu() }) {
            (Some(event), Some(menu)) => unsafe { menu.performKeyEquivalent(&event) },
            _ => false,
        };
        Ok(result)
    })
}
