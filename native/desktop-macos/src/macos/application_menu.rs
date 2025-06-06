use core::slice;

use anyhow::{Result, anyhow};

use objc2::{DeclaredClass, MainThreadOnly, define_class, msg_send, rc::Retained, sel};
use objc2_app_kit::{
    NSControlStateValueMixed, NSControlStateValueOff, NSControlStateValueOn, NSEventModifierFlags, NSEventType, NSMenu, NSMenuItem,
};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSString};

use super::{
    application_api::MyNSApplication,
    application_menu_api::{
        ActionItemState, ActionMenuItemSpecialTag, AppMenuItem, AppMenuItemCallback, AppMenuStructure, AppMenuTrigger,
        SubMenuItemSpecialTag,
    },
    keyboard::KeyModifiersSet,
    string::copy_to_ns_string,
};

pub fn main_menu_update_impl(menu: &AppMenuStructure) {
    let updated_menu = AppMenuStructureSafe::from_unsafe(menu).unwrap(); // todo come up with some error handling facility
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let app = MyNSApplication::sharedApplication(mtm);

    let menu_root = if let Some(menu) = unsafe { app.mainMenu() } {
        menu
    } else {
        let new_menu_root = NSMenu::new(mtm);
        app.setMainMenu(Some(&new_menu_root));
        new_menu_root
    };
    unsafe {
        menu_root.setAutoenablesItems(false);
    }
    reconcile_ns_menu_items(mtm, &menu_root, true, &updated_menu.items);
}

#[derive(Debug)]
struct AppMenuKeystrokeSafe {
    key: Retained<NSString>,
    modifiers: KeyModifiersSet,
}

#[derive(Debug)]
enum AppMenuItemSafe {
    Action {
        enabled: bool,
        state: ActionItemState,
        title: Retained<NSString>,
        special_tag: ActionMenuItemSpecialTag,
        keystroke: Option<AppMenuKeystrokeSafe>,
        perform: AppMenuItemCallback,
    },
    Separator,
    SubMenu {
        title: Retained<NSString>,
        special_tag: SubMenuItemSpecialTag,
        items: Vec<AppMenuItemSafe>,
    },
}

#[derive(Debug)]
struct AppMenuStructureSafe {
    items: Vec<AppMenuItemSafe>,
}

impl AppMenuStructureSafe {
    fn from_unsafe(menu: &AppMenuStructure) -> Result<Self> {
        let items = {
            if menu.items.is_null() {
                return Err(anyhow!("Null found in {:?}", menu));
            }
            unsafe { slice::from_raw_parts(menu.items, menu.items_count) }
        };
        let safe_items: Result<Vec<_>> = items.iter().map(|e| AppMenuItemSafe::from_unsafe(e)).collect();
        Ok(Self { items: safe_items? })
    }
}

impl AppMenuItemSafe {
    fn from_unsafe(item: &AppMenuItem) -> Result<Self> {
        let safe_item = match item {
            &AppMenuItem::ActionItem {
                enabled,
                state,
                ref title,
                special_tag,
                keystroke,
                perform,
            } => {
                let keystroke = if let Some(keystroke) = keystroke {
                    Some(AppMenuKeystrokeSafe {
                        key: copy_to_ns_string(&keystroke.key)?,
                        modifiers: keystroke.modifiers,
                    })
                } else {
                    None
                };
                Self::Action {
                    enabled,
                    state,
                    title: copy_to_ns_string(title)?,
                    special_tag,
                    keystroke,
                    perform,
                }
            }
            AppMenuItem::SeparatorItem => Self::Separator,
            sub_menu @ &AppMenuItem::SubMenuItem {
                ref title,
                special_tag,
                items,
                items_count,
            } => {
                let items = {
                    if items.is_null() {
                        return Err(anyhow!("Null found in {:?}", sub_menu));
                    }
                    unsafe { slice::from_raw_parts(items, items_count) }
                };
                let safe_items: Result<Vec<_>> = items.iter().map(|e| Self::from_unsafe(e)).collect();
                Self::SubMenu {
                    title: copy_to_ns_string(title)?,
                    special_tag,
                    items: safe_items?,
                }
            }
        };
        Ok(safe_item)
    }

    #[allow(clippy::too_many_arguments)]
    fn reconcile_action(
        item: &NSMenuItem,
        enabled: bool,
        state: ActionItemState,
        title: &Retained<NSString>,
        _special_tag: ActionMenuItemSpecialTag,
        keystroke: Option<&AppMenuKeystrokeSafe>,
        perform: AppMenuItemCallback,
        mtm: MainThreadMarker,
    ) {
        unsafe {
            item.setTitle(title);
            item.setEnabled(enabled);
            let state = match state {
                ActionItemState::On => NSControlStateValueOn,
                ActionItemState::Off => NSControlStateValueOff,
                ActionItemState::Mixed => NSControlStateValueMixed,
            };
            item.setState(state);

            let representer = MenuItemRepresenter::new(Some(perform), mtm);
            item.setTarget(Some(&representer));
            item.setRepresentedObject(Some(&representer));
            item.setAction(Some(sel!(itemCallback:)));

            if let Some(keystroke) = keystroke {
                item.setKeyEquivalent(&keystroke.key);
                item.setKeyEquivalentModifierMask(keystroke.modifiers.into());
            } else {
                item.setKeyEquivalent(&NSString::new());
                item.setKeyEquivalentModifierMask(NSEventModifierFlags::empty());
            }
        }
    }

    fn reconcile_ns_submenu(
        mtm: MainThreadMarker,
        item: &NSMenuItem,
        title: &Retained<NSString>,
        special_tag: SubMenuItemSpecialTag,
        items: &[Self],
    ) {
        let submenu = unsafe { item.submenu() }.unwrap();
        if special_tag != SubMenuItemSpecialTag::AppNameMenu {
            unsafe {
                item.setTitle(title);
                submenu.setTitle(title);
            }
        }
        reconcile_ns_menu_items(mtm, &submenu, false, items);
    }

    fn reconcile_ns_menu_item(&self, mtm: MainThreadMarker, item: &NSMenuItem) {
        match self {
            Self::Action {
                enabled,
                state,
                title,
                special_tag,
                keystroke,
                perform,
            } => {
                Self::reconcile_action(item, *enabled, *state, title, *special_tag, keystroke.as_ref(), *perform, mtm);
            }
            Self::Separator => {
                assert!(unsafe { item.isSeparatorItem() });
            }
            Self::SubMenu { title, special_tag, items } => {
                Self::reconcile_ns_submenu(mtm, item, title, *special_tag, items);
            }
        }
    }

    fn create_ns_menu_item(&self, mtm: MainThreadMarker) -> Retained<NSMenuItem> {
        match self {
            &Self::Action {
                enabled,
                state,
                ref title,
                special_tag,
                ref keystroke,
                perform,
            } => {
                let item = NSMenuItem::new(mtm);
                Self::reconcile_action(&item, enabled, state, title, special_tag, keystroke.as_ref(), perform, mtm);
                item
            }
            Self::Separator => {
                let item = NSMenuItem::separatorItem(mtm);
                let representer = MenuItemRepresenter::new(None, mtm);
                unsafe {
                    item.setTarget(Some(&representer));
                    item.setRepresentedObject(Some(&representer));
                };
                item
            }
            Self::SubMenu { title, special_tag, items } => {
                let item = NSMenuItem::new(mtm);
                let representer = MenuItemRepresenter::new(None, mtm);
                unsafe {
                    item.setTarget(Some(&representer));
                    item.setRepresentedObject(Some(&representer));
                };
                let submenu = NSMenu::new(mtm);
                unsafe {
                    submenu.setAutoenablesItems(false);
                }
                item.setSubmenu(Some(&submenu));
                match special_tag {
                    SubMenuItemSpecialTag::Window => {
                        let app = MyNSApplication::sharedApplication(mtm);
                        unsafe { app.setWindowsMenu(Some(&submenu)) };
                    }
                    SubMenuItemSpecialTag::Services => {
                        let app = MyNSApplication::sharedApplication(mtm);
                        unsafe { app.setServicesMenu(Some(&submenu)) };
                    }
                    _ => {}
                }
                Self::reconcile_ns_submenu(mtm, &item, title, *special_tag, items);
                item
            }
        }
    }
}

#[derive(Debug)]
struct MenuItemRepresenterIvars {
    callback: Option<AppMenuItemCallback>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "MenuItemRepresenter"]
    #[ivars = MenuItemRepresenterIvars]
    #[derive(Debug)]
    struct MenuItemRepresenter;

    unsafe impl NSObjectProtocol for MenuItemRepresenter {}

    impl MenuItemRepresenter {
        #[unsafe(method(itemCallback:))]
        fn item_callback(&self, _sender: &NSMenuItem) {
            if let Some(callback) = self.ivars().callback {
                callback(self.guess_trigger());
            }
        }
    }
);

impl MenuItemRepresenter {
    fn new(callback: Option<AppMenuItemCallback>, mtm: MainThreadMarker) -> Retained<Self> {
        let obj = Self::alloc(mtm).set_ivars(MenuItemRepresenterIvars { callback });
        unsafe { msg_send![super(obj), init] }
    }

    fn guess_trigger(&self) -> AppMenuTrigger {
        let mtm = self.mtm();
        let app = MyNSApplication::sharedApplication(mtm);
        match app.currentEvent() {
            Some(ns_event) if unsafe { ns_event.r#type() } == NSEventType::KeyDown => AppMenuTrigger::Keystroke,
            _ => AppMenuTrigger::Other,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
enum ItemIdentity<'a> {
    Action { title: &'a Retained<NSString> },
    Separator,
    AppNameSubMenu,
    SubMenu { title: &'a Retained<NSString> },
    MacOSProvided,
}

impl<'a> ItemIdentity<'a> {
    const fn new(item: &'a AppMenuItemSafe) -> Self {
        match item {
            AppMenuItemSafe::Action { title, .. } => Self::Action { title },
            AppMenuItemSafe::Separator => Self::Separator,
            AppMenuItemSafe::SubMenu {
                special_tag: SubMenuItemSpecialTag::AppNameMenu,
                ..
            } => Self::AppNameSubMenu,
            AppMenuItemSafe::SubMenu { title, .. } => Self::SubMenu { title },
        }
    }
}

#[allow(clippy::cast_possible_wrap)]
fn reconcile_ns_menu_items(mtm: MainThreadMarker, menu: &NSMenu, is_top_level: bool, new_items: &[AppMenuItemSafe]) {
    let items_array = unsafe { menu.itemArray() };
    let menu_titles: Vec<_> = items_array.iter().map(|submenu| unsafe { submenu.title() }).collect();

    // sometimes macos can surround our items with new ones
    // but usually it just add items to the end
    let old_item_ids: Vec<ItemIdentity> = items_array
        .iter()
        .zip(menu_titles.iter())
        .enumerate()
        .map(|(i, (item, title))| {
            if is_top_level && i == 0 {
                // the first item in top level menu is always item wiht the app name
                ItemIdentity::AppNameSubMenu
            } else {
                unsafe { item.representedObject() }
                    .map(Retained::downcast::<MenuItemRepresenter>)
                    .map_or(ItemIdentity::MacOSProvided, |_rep_obj| {
                        if unsafe { item.isSeparatorItem() } {
                            ItemIdentity::Separator
                        } else if unsafe { item.hasSubmenu() } {
                            ItemIdentity::SubMenu { title }
                        } else {
                            ItemIdentity::Action { title }
                        }
                    })
            }
        })
        .collect();

    let new_item_ids: Vec<_> = new_items.iter().map(ItemIdentity::new).collect();

    let first_item = old_item_ids.iter().position(|it| *it != ItemIdentity::MacOSProvided);
    let last_item = old_item_ids.iter().rposition(|it| *it != ItemIdentity::MacOSProvided);
    let (old_item_ids, base_position) = match (first_item, last_item) {
        (Some(first_item), Some(last_item)) => (&old_item_ids[first_item..=last_item], first_item),
        // All items in menu are macOS provided
        // Our items will be placed before them
        _ => ([].as_slice(), 0),
    };

    let operations = edit_operations(old_item_ids, &new_item_ids);
    let mut position_shift: isize = base_position as isize;
    for op in operations {
        match op {
            Operation::Insert { position, item_idx } => {
                let new_ns_menu_item = new_items[item_idx].create_ns_menu_item(mtm);
                unsafe {
                    menu.insertItem_atIndex(&new_ns_menu_item, position as isize + position_shift);
                }
                position_shift += 1;
            }
            Operation::Reconcile { position, item_idx } => {
                let ns_menu_item = unsafe { menu.itemAtIndex(position as isize + position_shift).unwrap() };
                new_items[item_idx].reconcile_ns_menu_item(mtm, &ns_menu_item);
            }
            Operation::Remove { position } => {
                let ns_menu_item = unsafe { menu.itemAtIndex(position as isize + position_shift).unwrap() };
                let rep_obj = unsafe { ns_menu_item.representedObject() }.map(Retained::downcast::<MenuItemRepresenter>);
                // Just skip remove commands for macOS provided items
                if rep_obj.is_some() {
                    unsafe {
                        menu.removeItemAtIndex(position as isize + position_shift);
                    };
                    position_shift -= 1;
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Operation {
    Insert { position: usize, item_idx: usize },
    Reconcile { position: usize, item_idx: usize },
    Remove { position: usize },
}

fn edit_operations<T: Eq>(source: &[T], target: &[T]) -> Vec<Operation> {
    let m = source.len();
    let n = target.len();

    let mut dp = vec![vec![0; n + 1]; m + 1];

    for i in 1..=m {
        for j in 1..=n {
            if source[i - 1] == target[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    let mut operations = Vec::new();

    let mut i = m;
    let mut j = n;
    while i > 0 && j > 0 {
        if source[i - 1] == target[j - 1] {
            operations.push(Operation::Reconcile {
                position: i - 1,
                item_idx: j - 1,
            });
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            operations.push(Operation::Remove { position: i - 1 });
            i -= 1;
        } else {
            operations.push(Operation::Insert {
                position: i,
                item_idx: j - 1,
            });
            j -= 1;
        }
    }

    while i > 0 {
        operations.push(Operation::Remove { position: i - 1 });
        i -= 1;
    }
    while j > 0 {
        operations.push(Operation::Insert {
            position: i,
            item_idx: j - 1,
        });
        j -= 1;
    }

    operations.reverse();

    operations
}

#[cfg(test)]
mod tests {
    use std::{char, fmt::Debug};

    use log::debug;
    use quickcheck_macros::quickcheck;

    use super::*;

    fn chs(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    fn apply_operations<T: Debug + Clone + Copy + Eq>(source: &[T], target: &[T], operations: &[Operation]) -> Result<Vec<T>> {
        let mut v = source.to_vec();
        for op in operations {
            match *op {
                Operation::Insert { position, item_idx } => {
                    v.insert(position, target[item_idx]);
                }
                Operation::Reconcile { position, item_idx } => {
                    if v[position] != target[item_idx] {
                        return Err(anyhow!("{:?} != {:?}", v[position], target[item_idx]));
                    }
                }
                Operation::Remove { position } => {
                    v.remove(position);
                }
            }
        }
        Ok(v)
    }

    #[allow(clippy::cast_possible_wrap)]
    fn fix_positions(operations: &mut [Operation]) {
        let mut shift: isize = 0;
        for operation in operations {
            match operation {
                Operation::Insert { position, item_idx: _ } => {
                    *position = (*position as isize + shift).try_into().unwrap();
                    shift += 1;
                }
                Operation::Reconcile { position, item_idx: _ } => {
                    *position = (*position as isize + shift).try_into().unwrap();
                }
                Operation::Remove { position } => {
                    *position = (*position as isize + shift).try_into().unwrap();
                    shift -= 1;
                }
            }
        }
    }

    fn test_with(source: &str, target: &str) {
        let source = chs(source);
        let target = chs(target);
        let mut operations = edit_operations(&source, &target);
        fix_positions(&mut operations);
        debug!("src: {source:?}, dst: {target:?}");
        debug!("{operations:?}");
        let result = apply_operations(&source, &target, &operations).unwrap();
        assert_eq!(result, target);
    }

    #[test]
    fn test_edit_operations_smoke() {
        test_with("", "");
        test_with("x", "x");
        test_with("", "abcde");
        test_with("abcde", "");
        test_with("abc", "cba");
        test_with("xxxxxxxx", "yy");
        test_with("ab", "ba");
        test_with("xxabcde", "abcde");
        test_with("abcde", "xxabcde");
        test_with("abcdexx", "abcde");
        test_with("abcde", "abcdexx");
    }

    #[allow(clippy::needless_pass_by_value)]
    #[quickcheck]
    fn operations_turns_source_into_target(source: Vec<u32>, target: Vec<u32>) -> bool {
        let mut operations = edit_operations(&source, &target);
        fix_positions(&mut operations);
        let result = apply_operations(&source, &target, &operations).unwrap();
        target == result
    }
}
