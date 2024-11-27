use core::slice;
use std::{ffi::CStr, fmt::format, slice::from_raw_parts};

use anyhow::{anyhow, Result};

use objc2::{
    declare_class, msg_send_id, mutability, rc::{autoreleasepool, Retained}, runtime::{AnyObject, Ivar}, sel, ClassType, DeclaredClass
};
use objc2_app_kit::{NSApplication, NSControlStateValueOn, NSEventModifierFlags, NSMenu, NSMenuItem};
use objc2_foundation::{MainThreadMarker, NSObject, NSString, NSObjectProtocol};

use crate::common::{ArraySize, StrPtr};

// todo add keystrokes
// todo callbacks

#[allow(dead_code)]
#[repr(C)]
#[derive(Debug)]
pub enum AppMenuItem {
    ActionItem {
        enabled: bool,
        title: StrPtr,
        macos_provided: bool,
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
    items: *const AppMenuItem,
    items_count: ArraySize,
}

#[no_mangle]
pub extern "C" fn main_menu_update(menu: AppMenuStructure) {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);

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

    let updated_menu = AppMenuStructureSafe::from_unsafe(&menu).unwrap(); // todo come up with some error handling facility

    reconcile_ns_menu_items(mtm, &menu_root, &updated_menu.items);
}

#[no_mangle]
pub extern "C" fn main_menu_set_none() {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    app.setMainMenu(None);
}

#[derive(Debug)]
enum AppMenuItemSafe<'a> {
    Action {
        enabled: bool,
        macos_provided: bool,
        title: &'a str,
    },
    Separator,
    SubMenu {
        title: &'a str,
        special_tag: Option<&'a str>,
        items: Vec<AppMenuItemSafe<'a>>,
    },
}

#[derive(Debug)]
struct AppMenuStructureSafe<'a> {
    items: Vec<AppMenuItemSafe<'a>>,
}

impl<'a> AppMenuStructureSafe<'a> {
    fn from_unsafe(menu: &'a AppMenuStructure) -> Result<AppMenuStructureSafe<'a>> {
        let items = unsafe {
            let AppMenuStructure { items, items_count } = *menu;
            if items.is_null() {
                return Err(anyhow!("Null found in {:?}", menu));
            } else {
                from_raw_parts(items, items_count as usize)
            }
        };
        let safe_items: Result<Vec<_>> = items.iter().map(|unsafe_item| AppMenuItemSafe::from_unsafe(unsafe_item)).collect();
        Ok(AppMenuStructureSafe { items: safe_items? })
    }
}

impl<'a> AppMenuItemSafe<'a> {
    fn from_unsafe(item: &'a AppMenuItem) -> Result<AppMenuItemSafe<'a>> {
        let safe_item = match item {
            AppMenuItem::ActionItem {
                enabled,
                title,
                macos_provided,
            } => AppMenuItemSafe::Action {
                enabled: *enabled,
                macos_provided: *macos_provided,
                title: unsafe { CStr::from_ptr(*title) }.to_str()?,
            },
            AppMenuItem::SeparatorItem => AppMenuItemSafe::Separator,
            sub_menu @ AppMenuItem::SubMenuItem {
                title,
                special_tag,
                items,
                items_count,
            } => {
                let items = unsafe {
                    if !(*items).is_null() {
                        slice::from_raw_parts(*items, *items_count as usize)
                    } else {
                        return Err(anyhow!("Null found in {:?}", sub_menu));
                    }
                };
                let safe_items: Result<Vec<_>> = items.iter().map(|item| AppMenuItemSafe::from_unsafe(item)).collect();
                let special_tag = if !special_tag.is_null() {
                    Some(unsafe { CStr::from_ptr(*special_tag) }.to_str()?)
                } else {
                    None
                };
                AppMenuItemSafe::SubMenu {
                    title: unsafe { CStr::from_ptr(*title) }.to_str()?,
                    special_tag: special_tag,
                    items: safe_items?,
                }
            }
        };
        Ok(safe_item)
    }

    fn reconcile_action(item: &NSMenuItem, enabled: bool, macos_provided: bool, title: &str) {
        unsafe {
            item.setTitle(&NSString::from_str(title));
            item.setKeyEquivalent(&NSString::from_str("x"));
            item.setKeyEquivalentModifierMask(NSEventModifierFlags::NSEventModifierFlagCommand);
            item.setEnabled(enabled);
            if macos_provided {
                item.setHidden(false);
            }
        }
    }

    fn reconcile_ns_submenu(mtm: MainThreadMarker, item: &NSMenuItem, title: &str, special_tag: Option<&str>, items: &[Self]) {
        let ns_title = NSString::from_str(title);
        unsafe {
            item.setTitle(&ns_title);
        };
        let submenu = unsafe { item.submenu() }.unwrap();
        unsafe {
            submenu.setTitle(&ns_title);
        }
        // If we don't provide any items for macOS filled submenus we don't want to make it empty
        // todo this check can be removed as far we already reconcile only our items
        if !(special_tag.is_some() && items.is_empty()) {
            reconcile_ns_menu_items(mtm, &submenu, items);
        }
    }

    fn reconcile_ns_menu_item(&self, mtm: MainThreadMarker, item: &NSMenuItem) {
        match self {
            AppMenuItemSafe::Action {
                enabled,
                title,
                macos_provided,
            } => {
                AppMenuItemSafe::reconcile_action(item, *enabled, *macos_provided, title);
            }
            AppMenuItemSafe::Separator => {
                assert!(unsafe { item.isSeparatorItem() });
            }
            AppMenuItemSafe::SubMenu { title, special_tag, items } => {
                AppMenuItemSafe::reconcile_ns_submenu(mtm, item, title, *special_tag, items);
            }
        }
    }

    fn create_ns_menu_item(&self, mtm: MainThreadMarker) -> Option<Retained<NSMenuItem>> {
        match self {
            AppMenuItemSafe::Action {
                enabled,
                title,
                macos_provided,
            } => {
                if !macos_provided {
                    let item = NSMenuItem::new(mtm);
                    unsafe {
                        item.setRepresentedObject(Some(&MenuItemRepresenter::new()))
                    };
                    AppMenuItemSafe::reconcile_action(&item, *enabled, *macos_provided, title);
                    Some(item)
                } else {
                    None
                }
            }
            AppMenuItemSafe::Separator => {
                let item = NSMenuItem::separatorItem(mtm);
                unsafe {
                    item.setRepresentedObject(Some(&MenuItemRepresenter::new()))
                };
                Some(item)
            },
            AppMenuItemSafe::SubMenu { title, special_tag, items } => {
                let item = NSMenuItem::new(mtm);
                // todo fixme use some meaningful object!
                unsafe {
                    item.setRepresentedObject(Some(&MenuItemRepresenter::new()))
                };
                let submenu = NSMenu::new(mtm);
                unsafe {
                    submenu.setAutoenablesItems(false);
                }
                item.setSubmenu(Some(&submenu));
                match *special_tag {
                    Some("Window") => {
                        let app = NSApplication::sharedApplication(mtm);
                        unsafe {
                            app.setWindowsMenu(Some(&submenu));
                        }
                    }
                    Some("Services") => {
                        let app = NSApplication::sharedApplication(mtm);
                        unsafe {
                            app.setServicesMenu(Some(&submenu));
                        }
                    }
                    _ => {}
                };
                AppMenuItemSafe::reconcile_ns_submenu(mtm, &item, title, *special_tag, items);
                Some(item)
            }
        }
    }
}


#[derive(Clone)]
struct MenuItemRepresenterIvars {
}

declare_class!(
    struct MenuItemRepresenter;

    unsafe impl ClassType for MenuItemRepresenter {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
        const NAME: &'static str = "MenuItemRepresenter";
    }

    impl DeclaredClass for MenuItemRepresenter {
        type Ivars = MenuItemRepresenterIvars;
    }
    unsafe impl NSObjectProtocol for MenuItemRepresenter {}
);

impl MenuItemRepresenter {
    fn new() -> Retained<Self> {
        let obj = Self::alloc().set_ivars(MenuItemRepresenterIvars {});
        unsafe { msg_send_id![super(obj), init] }
    }

    fn from_any_object(obj: Retained<AnyObject>) -> Option<Retained<Self>> {
        if obj.class().responds_to(sel!(isKindOfClass:)) {
            unsafe {
                let obj = Retained::cast::<NSObject>(obj);
                if obj.is_kind_of::<Self>() {
                    Some(Retained::cast::<Self>(obj))
                } else {
                    None
                }
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
enum ItemIdentity<'a> {
    Action { title: &'a str },
    Separator,
    SubMenu { title: &'a str },
    MacOSProvided
}

impl<'a> ItemIdentity<'a> {
    fn new(item: &AppMenuItemSafe<'a>) -> ItemIdentity<'a> {
        return match item {
            AppMenuItemSafe::Action { title, .. } => Self::Action { title },
            AppMenuItemSafe::Separator => Self::Separator,
            AppMenuItemSafe::SubMenu { title, .. } => Self::SubMenu { title },
        };
    }
}

fn reconcile_ns_menu_items<'a>(mtm: MainThreadMarker, menu: &NSMenu, new_items: &[AppMenuItemSafe<'a>]) {
    autoreleasepool(|pool| {
        let items_array = unsafe { menu.itemArray() };
        let menu_titles: Vec<_> = items_array.iter().map(|submenu| unsafe { submenu.title() }).collect();

        // sometimes macos can surround our items with new ones
        // but usually it just add items to the end
        let old_item_ids: Vec<ItemIdentity> = items_array
            .iter()
            .zip(menu_titles.iter())
            .map(|(item, title)| {
                unsafe { item.representedObject() }.map(|it| MenuItemRepresenter::from_any_object(it)).map(|rep_obj| {
                    let item_id = if unsafe { item.isSeparatorItem() } {
                        ItemIdentity::Separator
                    } else if unsafe { item.hasSubmenu() } {
                        ItemIdentity::SubMenu { title: title.as_str(pool) }
                    } else {
                        ItemIdentity::Action { title: title.as_str(pool) }
                    };
                    item_id
                }).unwrap_or(ItemIdentity::MacOSProvided)
            })
            .collect();

        let new_item_ids: Vec<_> = new_items.iter().map(|item| ItemIdentity::new(item)).collect();

        let first_item = old_item_ids.iter().position(|it| *it != ItemIdentity::MacOSProvided);
        let last_item = old_item_ids.iter().rposition(|it| *it != ItemIdentity::MacOSProvided);
        let (old_item_ids, base_position) = match (first_item, last_item) {
            (Some(first_item), Some(last_item)) => {
                (&old_item_ids[first_item..=last_item], first_item)
            },
            // All items in menu are macOS provided
            // Our items will be placed before them
            _ => {
                ([].as_slice(), 0)
            }
        };

        let operations = edit_operations(&old_item_ids, &new_item_ids);
        let mut position_shift: isize = base_position as isize;
        for op in operations {
            match op {
                Operation::Insert { position, item_idx } => {
                    if let Some(new_ns_menu_item) = new_items[item_idx].create_ns_menu_item(mtm) {
                        unsafe {
                            menu.insertItem_atIndex(&new_ns_menu_item, (position as isize + position_shift).try_into().unwrap());
                        }
                        position_shift += 1;
                    }
                }
                Operation::Reconcile { position, item_idx } => {
                    let ns_menu_item = unsafe { menu.itemAtIndex((position as isize + position_shift).try_into().unwrap()).unwrap() };
                    new_items[item_idx].reconcile_ns_menu_item(mtm, &ns_menu_item);
                }
                Operation::Remove { position } => {
                    let ns_menu_item = unsafe { menu.itemAtIndex((position as isize + position_shift).try_into().unwrap()).unwrap() };
                    let rep_obj = unsafe { ns_menu_item.representedObject() }.map(|it| MenuItemRepresenter::from_any_object(it));
                    // Just skip remove commands for macOS provided items
                    if rep_obj.is_some() {
                        unsafe {
                            menu.removeItemAtIndex((position as isize + position_shift).try_into().unwrap());
                        };
                        position_shift -= 1;
                    }
                }
            }
        }
    });
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
        return Ok(v);
    }

    fn fix_positions(operations: &mut [Operation]) {
        let mut shift: isize = 0;
        for operation in operations {
            match operation {
                Operation::Insert {
                    ref mut position,
                    item_idx,
                } => {
                    *position = (*position as isize + shift).try_into().unwrap();
                    shift += 1;
                }
                Operation::Reconcile { position, item_idx } => {
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
        eprintln!("src: {source:?}, dst: {target:?}");
        eprintln!("{operations:?}");
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

    #[quickcheck]
    fn operations_turns_source_into_target(source: Vec<u32>, target: Vec<u32>) -> bool {
        let mut operations = edit_operations(&source, &target);
        fix_positions(&mut operations);
        let result = apply_operations(&source, &target, &operations).unwrap();
        target == result
    }
}
