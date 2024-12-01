#![deny(unsafe_op_in_unsafe_fn)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
use std::cell::OnceCell;
use std::ptr::NonNull;
use std::time::Duration;

use bitflags::Flags;
use objc2::ffi::SEL;
use objc2::mutability::MainThreadOnly;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{declare_class, msg_send_id, mutability, sel, ClassType, DeclaredClass};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSBackingStoreType, NSEventModifierFlags, NSF1FunctionKey,
    NSF3FunctionKey, NSMenu, NSMenuItem, NSNormalWindowLevel, NSWindow, NSWindowLevel, NSWindowStyleMask,
};
use objc2_foundation::{
    ns_string, CGPoint, CGRect, CGSize, MainThreadMarker, NSCoding, NSCopying, NSDate, NSNotification, NSObject, NSObjectProtocol, NSSize,
    NSString,
};

use objc2_metal::{
    MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue, MTLCreateSystemDefaultDevice, MTLDevice, MTLLibrary, MTLPackedFloat3,
    MTLPrimitiveType, MTLRenderCommandEncoder, MTLRenderPipelineDescriptor, MTLRenderPipelineState,
};
use objc2_metal_kit::{MTKView, MTKViewDelegate};

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

    unsafe impl AppDelegate {
        #[method(handleAppMenu:)]
        fn handle_app_menu_bar(&self, sender: &NSMenuItem) {
            println!("handleAppMenu is called with: {sender:?}");
        }
    }

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[method(applicationDidFinishLaunching:)]
        fn did_finish_launching(&self, notification: &NSNotification) {
            let mtm = MainThreadMarker::new().unwrap();
            println!("Did finish launching!");
            // Do something with the notification
            dbg!(notification);
            let window1 = create_window(mtm, "First Window 1", 320.0, 240.0);
            create_mtk_view(&window1);
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

fn item_with_keystroke(mtm: MainThreadMarker, name: &str, key: &str, modifiers: NSEventModifierFlags) -> Retained<NSMenuItem> {
    let menu_item = NSMenuItem::new(mtm);
    let app = NSApplication::sharedApplication(mtm);
    unsafe {
        menu_item.setTitle(&NSString::from_str(name));
        menu_item.setEnabled(true);
        menu_item.setKeyEquivalent(&NSString::from_str(key));
        menu_item.setKeyEquivalentModifierMask(modifiers);
        if let Some(delegate) = app.delegate().map(|it| Retained::cast::<AnyObject>(it)) {
            menu_item.setTarget(Some(&*delegate));
        }
        menu_item.setAction(Some(sel!(handleAppMenu:)));
    }
    return menu_item;
}

fn add_item_with_submenu(mtm: MainThreadMarker, root: &NSMenu, title: &str) -> Retained<NSMenu> {
    let item = item_with_name(mtm, title);
    let submenu = unsafe { NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str(title)) };
    unsafe {
        submenu.setAutoenablesItems(false);
    };
    item.setSubmenu(Some(&submenu));
    root.addItem(&item);
    return submenu;
}

fn build_menu(menu_prefix: &str) -> Retained<NSMenu> {
    let mtm = MainThreadMarker::new().unwrap();
    let menu_root = NSMenu::new(mtm);
    unsafe {
        menu_root.setAutoenablesItems(false);
    };

    let first_submenu = add_item_with_submenu(mtm, &menu_root, "Fleet");
    first_submenu.addItem(&item_with_name(mtm, &"Important item1"));
    first_submenu.addItem(&item_with_name(mtm, &"Important item2"));
    first_submenu.addItem(&NSMenuItem::separatorItem(mtm));
    first_submenu.addItem(&item_with_name(mtm, &"Important item3"));

    let view_submenu = add_item_with_submenu(mtm, &menu_root, "View");
    view_submenu.addItem(&item_with_name(mtm, &"View1"));

    let strange_submenu = add_item_with_submenu(mtm, &menu_root, "Strange");
    strange_submenu.addItem(&item_with_name(mtm, &"Strange1"));
    strange_submenu.addItem(&item_with_keystroke(
        mtm,
        "Keyed Item1",
        "r",
        NSEventModifierFlags::NSEventModifierFlagControl | NSEventModifierFlags::NSEventModifierFlagOption,
    ));
    strange_submenu.addItem(&item_with_keystroke(mtm, "Keyed Item2", "r", NSEventModifierFlags::empty()));
    strange_submenu.addItem(&item_with_keystroke(mtm, "Keyed Item3", "rt", NSEventModifierFlags::empty()));
    strange_submenu.addItem(&item_with_keystroke(mtm, "Keyed Item4", "й", NSEventModifierFlags::empty()));
    strange_submenu.addItem(&item_with_keystroke(mtm, "Keyed Item5", "²", NSEventModifierFlags::empty()));
    strange_submenu.addItem(&item_with_keystroke(
        mtm,
        "Keyed Item5",
        &String::from_utf16(&[NSF3FunctionKey.try_into().unwrap()]).unwrap(),
        NSEventModifierFlags::empty(),
    ));

    strange_submenu.addItem(&NSMenuItem::separatorItem(mtm));
    strange_submenu.addItem(&NSMenuItem::separatorItem(mtm));
    strange_submenu.addItem(&NSMenuItem::separatorItem(mtm));
    strange_submenu.addItem(&item_with_name(mtm, &"Strange2"));

    let edit_submenu = add_item_with_submenu(mtm, &menu_root, "Edit");
    edit_submenu.addItem(&NSMenuItem::separatorItem(mtm));
    edit_submenu.addItem(&NSMenuItem::separatorItem(mtm));
    edit_submenu.addItem(&NSMenuItem::separatorItem(mtm));

    //    edit_submenu.addItem(&item_with_name(mtm, &"Edit1"));
    //    edit_submenu.addItem(&item_with_name(mtm, &"Edit2"));
    //    edit_submenu.addItem(&item_with_name(mtm, &"Edit3"));
    //    edit_submenu.addItem(&item_with_name(mtm, &"Edit4"));
    //    edit_submenu.addItem(&item_with_name(mtm, &"Edit5"));

    menu_root.addItem(&item_with_name(mtm, &"Lonely Item"));

    for item_num in 0..4 {
        let menu_item = item_with_name(mtm, &format!("{} Item #{}", menu_prefix, item_num));
        let submenu = unsafe { NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str(&format!("{} SubMenu #{}", menu_prefix, item_num))) };
        menu_item.setSubmenu(Some(&submenu));
        for subitem_num in 0..6 {
            let subitem = item_with_name(mtm, &format!("SubItem #{} from {}", subitem_num, item_num));
            submenu.addItem(&subitem);
            submenu.addItem(&NSMenuItem::separatorItem(mtm));
        }
        menu_root.addItem(&menu_item);
    }

    let app = NSApplication::sharedApplication(mtm);

    let window_menu = add_item_with_submenu(mtm, &menu_root, "Window");
    let services_menu = add_item_with_submenu(mtm, &menu_root, "Services");
    unsafe {
        app.setWindowsMenu(Some(&window_menu));
        app.setServicesMenu(Some(&services_menu));
        //        app.setHelpMenu();
    }

    add_item_with_submenu(mtm, &menu_root, "Help");

    //    let menu_item = NSMenuItem::new(mtm);

    menu_root
}

fn update_menu(menu: &NSMenu, new_title: &str) -> Option<()> {
    let item = unsafe { menu.itemAtIndex(3) }?;
    let submenu = unsafe { item.submenu() }?;
    unsafe {
        let new_submenu_title = &NSString::from_str(&format!("R[{}]", new_title));
        item.setTitle(new_submenu_title);
        submenu.setTitle(new_submenu_title);
    }
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

                //                let menu = build_menu(&format!("T: {}", x));
                //                app.setMainMenu(Some(&menu));
            });
            x += 1;
            std::thread::sleep(Duration::from_millis(2000));
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

macro_rules! idcell {
    ($name:ident => $this:expr) => {
        $this
            .ivars()
            .$name
            .set($name)
            .expect(&format!("ivar should not already be initialized: `{}`", stringify!($name)));
    };
    ($name:ident <= $this:expr) => {
        #[rustfmt::skip]
        let Some($name) = $this.ivars().$name.get() else {
            unreachable!(
                "ivar should be initialized: `{}`",
                stringify!($name)
            )
        };
    };
}

#[derive(Copy, Clone)]
#[repr(C)]
struct SceneProperties {
    time: f32,
}

#[derive(Copy, Clone)]
#[repr(C)]
struct VertexInput {
    position: MTLPackedFloat3,
    color: MTLPackedFloat3,
}

struct DelegateIvars {
    start_date: Retained<NSDate>,
    command_queue: OnceCell<Retained<ProtocolObject<dyn MTLCommandQueue>>>,
    pipeline_state: OnceCell<Retained<ProtocolObject<dyn MTLRenderPipelineState>>>,
}

declare_class!(
    struct Delegate;
    unsafe impl ClassType for Delegate {
        type Super = NSObject;
        type Mutability = MainThreadOnly;
        const NAME: &'static str = "Delegate";
    }

    impl DeclaredClass for Delegate {
        type Ivars = DelegateIvars;
    }

    unsafe impl NSObjectProtocol for Delegate {}

    unsafe impl MTKViewDelegate for Delegate {
        #[method(drawInMTKView:)]
        #[allow(non_snake_case)]
        unsafe fn drawInMTKView(&self, mtk_view: &MTKView) {
            println!("Draw!");
            idcell!(command_queue <= self);
            idcell!(pipeline_state <= self);

            // prepare for drawing
            let Some(current_drawable) = (unsafe { mtk_view.currentDrawable() }) else {
                return;
            };
            let Some(command_buffer) = command_queue.commandBuffer() else {
                return;
            };
            let Some(pass_descriptor) = (unsafe { mtk_view.currentRenderPassDescriptor() }) else {
                return;
            };
            let Some(encoder) = command_buffer.renderCommandEncoderWithDescriptor(&pass_descriptor)
            else {
                return;
            };

            // compute the scene properties
            let scene_properties_data = &SceneProperties {
                time: unsafe { self.ivars().start_date.timeIntervalSinceNow() } as f32,
            };
            // write the scene properties to the vertex shader argument buffer at index 0
            let scene_properties_bytes = NonNull::from(scene_properties_data);
            unsafe {
                encoder.setVertexBytes_length_atIndex(
                    scene_properties_bytes.cast::<core::ffi::c_void>(),
                    core::mem::size_of_val(scene_properties_data),
                    0,
                )
            };

            // compute the triangle geometry
            let vertex_input_data: &[VertexInput] = &[
                VertexInput {
                position: MTLPackedFloat3 {
                    x: -f32::sqrt(3.0) / 4.0,
                        y: -0.25,
                        z: 0.,
                    },
                    color: MTLPackedFloat3 {
                    x: 1.,
                        y: 0.,
                        z: 0.,
                    },
                },
                VertexInput {
                position: MTLPackedFloat3 {
                    x: f32::sqrt(3.0) / 4.0,
                        y: -0.25,
                        z: 0.,
                    },
                    color: MTLPackedFloat3 {
                    x: 0.,
                        y: 1.,
                        z: 0.,
                    },
                },
                VertexInput {
                position: MTLPackedFloat3 {
                    x: 0.,
                        y: 0.5,
                        z: 0.,
                    },
                    color: MTLPackedFloat3 {
                    x: 0.,
                        y: 0.,
                        z: 1.,
                    },
                },
            ];
            // write the triangle geometry to the vertex shader argument buffer at index 1
            let vertex_input_bytes = NonNull::from(vertex_input_data);
            unsafe {
                encoder.setVertexBytes_length_atIndex(
                    vertex_input_bytes.cast::<core::ffi::c_void>(),
                    core::mem::size_of_val(vertex_input_data),
                    1,
                )
            };

            // configure the encoder with the pipeline and draw the triangle
            encoder.setRenderPipelineState(pipeline_state);
            unsafe {
                encoder.drawPrimitives_vertexStart_vertexCount(MTLPrimitiveType::Triangle, 0, 3)
            };
            encoder.endEncoding();

            // schedule the command buffer for display and commit
            command_buffer.presentDrawable(ProtocolObject::from_ref(&*current_drawable));
            command_buffer.commit();
        }

        #[method(mtkView:drawableSizeWillChange:)]
        #[allow(non_snake_case)]
        unsafe fn mtkView_drawableSizeWillChange(&self, _view: &MTKView, _size: NSSize) {
            // println!("mtkView_drawableSizeWillChange");
        }
    }
);


impl Delegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(DelegateIvars {
            start_date: unsafe { NSDate::now() },
            command_queue: OnceCell::default(),
            pipeline_state: OnceCell::default(),
        });
        unsafe { msg_send_id![super(this), init] }
    }
}

fn create_mtk_view(window: &NSWindow) {
    println!("before mtk view initialized");
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let device = {
        let ptr = unsafe { MTLCreateSystemDefaultDevice() };
        unsafe { Retained::retain(ptr) }.expect("Failed to get default system device.")
    };
    // create the command queue
    let command_queue = device.newCommandQueue().expect("Failed to create a command queue.");

    let mtk_view = {
        let frame_rect = window.frame();
        unsafe { MTKView::initWithFrame_device(mtm.alloc(), frame_rect, Some(&device)) }
    };

    // create the pipeline descriptor
    let pipeline_descriptor = MTLRenderPipelineDescriptor::new();

    unsafe {
        pipeline_descriptor
            .colorAttachments()
            .objectAtIndexedSubscript(0)
            .setPixelFormat(mtk_view.colorPixelFormat());
    }

    // compile the shaders
    let library = device
        .newLibraryWithSource_options_error(ns_string!(include_str!("triangle.metal")), None)
        .expect("Failed to create a library.");

    // configure the vertex shader
    let vertex_function = library.newFunctionWithName(ns_string!("vertex_main"));
    pipeline_descriptor.setVertexFunction(vertex_function.as_deref());

    // configure the fragment shader
    let fragment_function = library.newFunctionWithName(ns_string!("fragment_main"));
    pipeline_descriptor.setFragmentFunction(fragment_function.as_deref());

    // create the pipeline state
    let pipeline_state = device
        .newRenderPipelineStateWithDescriptor_error(&pipeline_descriptor)
        .expect("Failed to create a pipeline state.");

    // configure the metal view delegate
    let delegate = Delegate::new(mtm);
    unsafe {
        let object = ProtocolObject::from_ref(&*delegate);
        mtk_view.setDelegate(Some(object));
    }
    unsafe {
        mtk_view.setPaused(false);
    }
    // configure the window
    window.setContentView(Some(&mtk_view));
    println!("mtk view initialized");
}

pub(crate) fn run() {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();

    println!("Sperator item title: {:?}", unsafe { NSMenuItem::separatorItem(mtm).title() });

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

    // configure the application delegate
    let delegate = AppDelegate::new(42, true, mtm);
    let object = ProtocolObject::from_ref(&*delegate);
    app.setDelegate(Some(object));

    //    start_background_thread();

    let _window2 = create_window(mtm, "First Window 2", 420.0, 240.0);
    start_background_thread();

    dispatch::Queue::main().exec_async(|| {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let app = NSApplication::sharedApplication(mtm);
        app.setMainMenu(Some(&build_menu("Initial")))
    });
    // run the app
    println!("Before starting an app!");
    unsafe { app.run() };
}
