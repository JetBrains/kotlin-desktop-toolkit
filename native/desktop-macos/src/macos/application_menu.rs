use core::slice;
use std::cell::OnceCell;

use anyhow::{Result, anyhow};

use objc2::{DeclaredClass, MainThreadOnly, define_class, msg_send, rc::Retained, sel};
use objc2_app_kit::{NSEventModifierFlags, NSMenu, NSMenuItem};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSString};

use crate::common::BorrowedStrPtr;

use super::{
    application_api::MyNSApplication,
    application_menu_api::{AppMenuItem, AppMenuStructure},
    keyboard::KeyModifiersSet,
    string::copy_to_ns_string,
};

struct SpecialTags {
    app_menu: Retained<NSString>,
    title_for_app_menu: Retained<NSString>,
    window: Retained<NSString>,
    services: Retained<NSString>,
}

thread_local! {
    pub static SPECIAL_TAGS: OnceCell<SpecialTags> = const { OnceCell::new() };
}

pub fn main_menu_update_impl(menu: &AppMenuStructure) {
    SPECIAL_TAGS.with(|special_tags_cell| {
        let special_tags = special_tags_cell.get_or_init(|| SpecialTags {
            app_menu: NSString::from_str("AppMenu"),
            title_for_app_menu: NSString::from_str(""),
            window: NSString::from_str("Window"),
            services: NSString::from_str("Services"),
        });
        let updated_menu = AppMenuStructureSafe::from_unsafe(menu, special_tags).unwrap(); // todo come up with some error handling facility
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
        reconcile_ns_menu_items(mtm, &menu_root, true, &updated_menu.items, special_tags);
    });
}

#[derive(Debug)]
struct AppMenuKeystrokeSafe {
    key: Retained<NSString>,
    modifiers: KeyModifiersSet,
}

type Callback = extern "C" fn();

#[derive(Debug)]
enum AppMenuItemSafe {
    Action {
        enabled: bool,
        macos_provided: bool,
        title: Retained<NSString>,
        keystroke: Option<AppMenuKeystrokeSafe>,
        perform: Callback,
    },
    Separator,
    SubMenu {
        title: Retained<NSString>,
        special_tag: Option<Retained<NSString>>,
        items: Vec<AppMenuItemSafe>,
    },
}

#[derive(Debug)]
struct AppMenuStructureSafe {
    items: Vec<AppMenuItemSafe>,
}

impl AppMenuStructureSafe {
    fn from_unsafe(menu: &AppMenuStructure, special_tags: &SpecialTags) -> Result<Self> {
        let items = {
            if menu.items.is_null() {
                return Err(anyhow!("Null found in {:?}", menu));
            }
            unsafe { slice::from_raw_parts(menu.items, menu.items_count) }
        };
        let safe_items: Result<Vec<_>> = items.iter().map(|e| AppMenuItemSafe::from_unsafe(e, special_tags)).collect();
        Ok(Self { items: safe_items? })
    }
}

impl AppMenuItemSafe {
    fn from_unsafe(item: &AppMenuItem, special_tags: &SpecialTags) -> Result<Self> {
        let safe_item = match item {
            &AppMenuItem::ActionItem {
                enabled,
                ref title,
                macos_provided,
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
                    macos_provided,
                    title: copy_to_ns_string(title)?,
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
                let safe_items: Result<Vec<_>> = items.iter().map(|e| Self::from_unsafe(e, special_tags)).collect();
                let special_tag = if let Some(special_tag) = special_tag {
                    Some(copy_to_ns_string(&BorrowedStrPtr::new(special_tag))?)
                } else {
                    None
                };
                let title = match &special_tag {
                    Some(v) if v == &special_tags.app_menu => special_tags.title_for_app_menu.clone(),
                    _ => copy_to_ns_string(title)?,
                };
                Self::SubMenu {
                    title,
                    special_tag,
                    items: safe_items?,
                }
            }
        };
        Ok(safe_item)
    }

    fn reconcile_action(
        item: &NSMenuItem,
        enabled: bool,
        macos_provided: bool,
        title: &NSString,
        keystroke: &Option<AppMenuKeystrokeSafe>,
        perform: Callback,
        mtm: MainThreadMarker,
    ) {
        unsafe {
            item.setTitle(title);
            item.setEnabled(enabled);
            if macos_provided {
                item.setHidden(false);
            }

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
        title: &NSString,
        special_tag: &Option<Retained<NSString>>,
        items: &[Self],
        special_tags: &SpecialTags,
    ) {
        let submenu = unsafe { item.submenu() }.unwrap();
        if special_tag.iter().all(|t| t != &special_tags.app_menu) {
            unsafe {
                item.setTitle(title);
                submenu.setTitle(title);
            }
        }
        // If we don't provide any items for macOS filled submenus we don't want to make it empty
        // todo this check can be removed as far we already reconcile only our items
        if !(special_tag.is_some() && items.is_empty()) {
            reconcile_ns_menu_items(mtm, &submenu, false, items, special_tags);
        }
    }

    fn reconcile_ns_menu_item(&self, mtm: MainThreadMarker, item: &NSMenuItem, special_tags: &SpecialTags) {
        match self {
            Self::Action {
                enabled,
                title,
                macos_provided,
                keystroke,
                perform,
            } => {
                Self::reconcile_action(item, *enabled, *macos_provided, title, keystroke, *perform, mtm);
            }
            Self::Separator => {
                assert!(unsafe { item.isSeparatorItem() });
            }
            Self::SubMenu { title, special_tag, items } => {
                Self::reconcile_ns_submenu(mtm, item, title, special_tag, items, special_tags);
            }
        }
    }

    fn create_ns_menu_item(&self, mtm: MainThreadMarker, special_tags: &SpecialTags) -> Option<Retained<NSMenuItem>> {
        match self {
            &Self::Action {
                enabled,
                ref title,
                macos_provided,
                ref keystroke,
                perform,
            } => {
                if macos_provided {
                    None
                } else {
                    let item = NSMenuItem::new(mtm);
                    Self::reconcile_action(&item, enabled, macos_provided, title, keystroke, perform, mtm);
                    Some(item)
                }
            }
            Self::Separator => {
                let item = NSMenuItem::separatorItem(mtm);
                let representer = MenuItemRepresenter::new(None, mtm);
                unsafe {
                    item.setTarget(Some(&representer));
                    item.setRepresentedObject(Some(&representer));
                };
                Some(item)
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
                match *special_tag {
                    Some(ref v) if v == &special_tags.window => {
                        let app = MyNSApplication::sharedApplication(mtm);
                        unsafe {
                            app.setWindowsMenu(Some(&submenu));
                        }
                    }
                    Some(ref v) if v == &special_tags.services => {
                        let app = MyNSApplication::sharedApplication(mtm);
                        unsafe {
                            app.setServicesMenu(Some(&submenu));
                        }
                    }
                    _ => {}
                };
                Self::reconcile_ns_submenu(mtm, &item, title, special_tag, items, special_tags);
                Some(item)
            }
        }
    }
}

#[derive(Debug)]
struct MenuItemRepresenterIvars {
    callback: Option<Callback>,
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
                callback();
            }
        }
    }
);

impl MenuItemRepresenter {
    fn new(callback: Option<Callback>, mtm: MainThreadMarker) -> Retained<Self> {
        let obj = Self::alloc(mtm).set_ivars(MenuItemRepresenterIvars { callback });
        unsafe { msg_send![super(obj), init] }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
enum ItemIdentity<'a> {
    Action { title: &'a NSString },
    Separator,
    SubMenu { title: &'a NSString },
    MacOSProvided,
}

impl<'a> ItemIdentity<'a> {
    fn new(item: &'a AppMenuItemSafe) -> Self {
        match item {
            AppMenuItemSafe::Action { title, .. } => Self::Action { title },
            AppMenuItemSafe::Separator => Self::Separator,
            AppMenuItemSafe::SubMenu { title, .. } => Self::SubMenu { title },
        }
    }
}

fn reconcile_ns_menu_items(
    mtm: MainThreadMarker,
    menu: &NSMenu,
    is_main_menu: bool,
    new_items: &[AppMenuItemSafe],
    special_tags: &SpecialTags,
) {
    let items_array = unsafe { menu.itemArray() };
    let menu_titles: Vec<_> = items_array.iter().map(|submenu| unsafe { submenu.title() }).collect();

    // sometimes macos can surround our items with new ones
    // but usually it just add items to the end
    let old_item_ids: Vec<ItemIdentity> = items_array
        .iter()
        .zip(menu_titles.iter())
        .enumerate()
        .map(|(i, (item, title))| {
            if is_main_menu && i == 0 {
                // Avoid duplicating the default (macOS provided) application menu
                ItemIdentity::SubMenu {
                    title: &special_tags.title_for_app_menu,
                }
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
                if let Some(new_ns_menu_item) = new_items[item_idx].create_ns_menu_item(mtm, special_tags) {
                    unsafe {
                        menu.insertItem_atIndex(&new_ns_menu_item, position as isize + position_shift);
                    }
                    position_shift += 1;
                }
            }
            Operation::Reconcile { position, item_idx } => {
                let ns_menu_item = unsafe { menu.itemAtIndex(position as isize + position_shift).unwrap() };
                new_items[item_idx].reconcile_ns_menu_item(mtm, &ns_menu_item, special_tags);
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

    fn fix_positions(operations: &mut [Operation]) {
        let mut shift: isize = 0;
        for operation in operations {
            match operation {
                Operation::Insert { position, item_idx } => {
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

    #[allow(clippy::needless_pass_by_value)]
    #[quickcheck]
    fn operations_turns_source_into_target(source: Vec<u32>, target: Vec<u32>) -> bool {
        let mut operations = edit_operations(&source, &target);
        fix_positions(&mut operations);
        let result = apply_operations(&source, &target, &operations).unwrap();
        target == result
    }
}
