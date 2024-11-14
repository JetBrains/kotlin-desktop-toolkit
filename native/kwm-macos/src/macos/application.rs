#![deny(unsafe_op_in_unsafe_fn)]
use std::os::unix::thread;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{declare_class, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSBackingStoreType, NSMenu, NSMenuItem, NSNormalWindowLevel, NSWindow, NSWindowLevel, NSWindowStyleMask
};
use objc2_foundation::{
    ns_string, run_on_main, CGPoint, CGRect, CGSize, MainThreadMarker, NSCoding, NSCopying, NSNotification, NSObject, NSObjectProtocol,
    NSString,
};

#[derive(Debug)]
#[allow(unused)]
struct Ivars {
    ivar: u8,
    another_ivar: bool,
    box_ivar: Box<i32>,
    maybe_box_ivar: Option<Box<i32>>,
    id_ivar: Retained<NSString>,
    maybe_id_ivar: Option<Retained<NSString>>,
}

declare_class!(
    struct AppDelegate;

    // SAFETY:
    // - The superclass NSObject does not have any subclassing requirements.
    // - Main thread only mutability is correct, since this is an application delegate.
    // - `AppDelegate` does not implement `Drop`.
    unsafe impl ClassType for AppDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "MyAppDelegate";
    }

    impl DeclaredClass for AppDelegate {
        type Ivars = Ivars;
    }

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[method(applicationDidFinishLaunching:)]
        fn did_finish_launching(&self, notification: &NSNotification) {
            println!("Did finish launching!");
            // Do something with the notification
            dbg!(notification);
        }

        #[method(applicationWillTerminate:)]
        fn will_terminate(&self, _notification: &NSNotification) {
            println!("Will terminate!");
        }
    }
);

impl AppDelegate {
    fn new(ivar: u8, another_ivar: bool, mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(Ivars {
            ivar,
            another_ivar,
            box_ivar: Box::new(2),
            maybe_box_ivar: None,
            id_ivar: NSString::from_str("abc"),
            maybe_id_ivar: Some(ns_string!("def").copy()),
        });
        unsafe { msg_send_id![super(this), init] }
    }
}

fn item_with_name(mtm: MainThreadMarker, name: &str) -> Retained<NSMenuItem> {
    let menu_item = NSMenuItem::new(mtm);
    unsafe {
        menu_item.setTitle(&NSString::from_str(name));
    }
    return menu_item;
}

fn add_item_with_submenu(mtm: MainThreadMarker, root: &NSMenu, title: &str) -> Retained<NSMenu> {
    let item = item_with_name(mtm, title);
    let submenu = unsafe { NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str(title)) };
    item.setSubmenu(Some(&submenu));
    root.addItem(&item);
    return submenu;
}

fn build_menu(menu_prefix: &str) -> Retained<NSMenu> {
    let mtm = MainThreadMarker::new().unwrap();
    let menu_root = NSMenu::new(mtm);

    add_item_with_submenu(mtm, &menu_root, "Fleet");
    add_item_with_submenu(mtm, &menu_root, "Strange");
    add_item_with_submenu(mtm, &menu_root, "Edit");

    for item_num in 0..4 {
        let menu_item = item_with_name(mtm, &format!("{} Item #{}", menu_prefix, item_num));
        let submenu = unsafe { NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str(&format!("{} SubMenu #{}", menu_prefix, item_num))) };
        menu_item.setSubmenu(Some(&submenu));
        for subitem_num in 0..6 {
            let subitem = item_with_name(mtm, &format!("SubItem #{} from {}", subitem_num, item_num));
            submenu.addItem(&subitem);
        }
        menu_root.addItem(&menu_item);
    }

    let app = NSApplication::sharedApplication(mtm);

    let window_menu = add_item_with_submenu(mtm, &menu_root, "Window");
    let services_menu = add_item_with_submenu(mtm, &menu_root, "Services");
    unsafe {
        app.setWindowsMenu(Some(&window_menu));
        app.setServicesMenu(Some(&services_menu));
    }

    add_item_with_submenu(mtm, &menu_root, "Help");

    //    let menu_item = NSMenuItem::new(mtm);

    menu_root
}

fn update_menu(menu: &NSMenu, new_title: &str) -> Option<()> {
    let item = unsafe { menu.itemAtIndex(3) }?;
    let submenu = unsafe { item.submenu() }?;
    let item = unsafe { submenu.itemAtIndex(2) }?;
    unsafe {
        item.setTitle(&NSString::from_str(new_title));
    }
    return Some(());
}

fn start_background_thread() {
    std::thread::spawn(|| {
        let mut x = 0;
        loop {
            dispatch::Queue::main().exec_sync(move || {
                let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();

                let app = NSApplication::sharedApplication(mtm);

                if let Some(menu) = unsafe { app.mainMenu() } {
                    update_menu(&menu, &format!("T: {}", x));
                }

                //                let menu = build_menu(&format!("T: {}", x));
                //                app.setMainMenu(Some(&menu));
            });
            x += 1;
            std::thread::sleep(Duration::from_millis(500));
        }
    });
}

fn create_window(mtm: MainThreadMarker, title: &str, x: f32, y: f32) -> Retained<NSWindow> {
    let window = unsafe {
        let rect = CGRect::new(CGPoint::new(x.into(), y.into()), CGSize::new(320.0, 240.0));
        let style =
            NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Miniaturizable | NSWindowStyleMask::Resizable;
        let window = NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc(),
            rect,
            style,
            NSBackingStoreType::NSBackingStoreBuffered,
            false,
        );
        window.setTitle(&NSString::from_str(title));
        window.setReleasedWhenClosed(false);
        window.makeKeyAndOrderFront(None);
        window.setLevel(NSNormalWindowLevel);
        window
    };
    return window;
}

pub(crate) fn run() {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

    // configure the application delegate
    let delegate = AppDelegate::new(42, true, mtm);
    let object = ProtocolObject::from_ref(&*delegate);
    app.setDelegate(Some(object));

    app.setMainMenu(Some(&build_menu("Initial")));
    start_background_thread();

    let window1 = create_window(mtm, "First Window 1", 320.0, 240.0);
    let window2 = create_window(mtm, "First Window 2", 420.0, 240.0);

    dispatch::Queue::main().exec_async(|| {
        println!("Hello from main thread!");
    });
    // run the app
    println!("Before starting an app!");
    unsafe { app.run() };
}