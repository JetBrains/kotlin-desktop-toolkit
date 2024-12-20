use std::ffi::{c_void, CStr};

use objc2::{
    declare_class, msg_send_id,
    mutability::{self, MainThreadOnly},
    rc::Retained,
    runtime::{AnyObject, Bool, ProtocolObject},
    sel, ClassType, DeclaredClass,
};
use objc2_app_kit::{NSAutoresizingMaskOptions, NSBackingStoreType, NSEvent, NSNormalWindowLevel, NSView, NSWindow, NSWindowCollectionBehavior, NSWindowDelegate, NSWindowStyleMask};
use objc2_foundation::{CGPoint, CGRect, CGSize, MainThreadMarker, NSNotification, NSNumber, NSObject, NSObjectProtocol, NSString};

use crate::{
    common::{LogicalPoint, LogicalSize, StrPtr},
    define_objc_ref,
    macos::{application_api::AppState, events::{handle_mouse_moved, handle_window_close_request, handle_window_focus_change, handle_window_move, handle_window_resize, handle_window_screen_change}},
};

use super::{events::{Event, MouseMovedEvent}, metal_api::MetalView, screen::{NSScreenExts, ScreenId}};

#[repr(transparent)]
pub struct WindowRef {
    ptr: *mut c_void,
}
define_objc_ref!(WindowRef, NSWindow);

pub struct Window {
    ns_window: Retained<NSWindow>,
    delegate: Retained<WindowDelegate>,
    root_view: Retained<RootView>
}

pub type WindowId = i64;

#[repr(C)]
pub struct WindowParams {
    pub origin: LogicalPoint,
    pub size: LogicalSize,
    pub title: StrPtr,
    // resizeable not resizable
    // min max size
    // allow full screen or not?
}

impl WindowParams {
    fn title(&self) -> &str {
        unsafe { CStr::from_ptr(self.title) }.to_str().unwrap()
    }
}

#[no_mangle]
pub extern "C" fn window_create(params: WindowParams) -> Box<Window> {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let window = create_window(mtm, &params);
    return Box::new(window)
}

#[no_mangle]
pub extern "C" fn window_drop(window: Box<Window>) {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    window.ns_window.close();
    std::mem::drop(window);
}

#[no_mangle]
pub extern "C" fn window_get_window_id(window: &Window) -> WindowId {
    return window.ns_window.window_id();
}

#[no_mangle]
pub extern "C" fn window_get_screen_id(window: &Window) -> ScreenId {
    return window.ns_window.screen().unwrap().screen_id();
}

#[no_mangle]
pub extern "C" fn window_scale_factor(window: &Window) -> f64 {
    return window.ns_window.backingScaleFactor();
}

#[no_mangle]
pub extern "C" fn window_attach_layer(window: &Window, layer: &MetalView) {
    let content_view = window.ns_window.contentView().unwrap();
    let layer_view = &layer.ns_view;
    unsafe {
        layer_view.setFrameSize(content_view.frame().size);
        content_view.addSubview(&*layer.ns_view);
    }
}

#[no_mangle]
pub extern "C" fn window_get_origin(window: &Window) -> LogicalPoint {
    return window.ns_window.get_origin()
}

#[no_mangle]
pub extern "C" fn window_get_size(window: &Window) -> LogicalSize {
    return window.ns_window.get_size()
}

#[no_mangle]
pub extern "C" fn window_set_rect(window: &Window, origin: LogicalPoint, size: LogicalSize, animate: bool) {
    window.ns_window.set_rect(origin.into(), size.into(), animate);
}

#[no_mangle]
pub extern "C" fn window_is_key(window: &Window) -> bool {
    return window.ns_window.isKeyWindow();
}

#[no_mangle]
pub extern "C" fn window_is_main(window: &Window) -> bool {
    return unsafe {
        window.ns_window.isMainWindow()
    }
}

#[no_mangle]
pub extern "C" fn window_get_max_size(window: &Window) -> LogicalSize {
    return window.ns_window.get_max_size();
}

#[no_mangle]
pub extern "C" fn window_set_max_size(window: &Window, size: LogicalSize) {
    window.ns_window.set_max_size(size);
}

#[no_mangle]
pub extern "C" fn window_get_min_size(window: &Window) -> LogicalSize {
    return window.ns_window.get_min_size();
}

#[no_mangle]
pub extern "C" fn window_set_min_size(window: &Window, size: LogicalSize) {
    window.ns_window.set_min_size(size);
}

pub(crate) trait NSWindowExts {
    fn window_id(&self) -> WindowId;
    fn get_size(&self) -> LogicalSize;
    fn get_origin(&self) -> LogicalPoint;
    fn set_rect(&self, origin: LogicalPoint, size: LogicalSize, animate: bool);

    fn set_max_size(&self, size: LogicalSize);
    fn set_min_size(&self, size: LogicalSize);
    fn get_max_size(&self) -> LogicalSize;
    fn get_min_size(&self) -> LogicalSize;
}

impl NSWindowExts for NSWindow {
    fn window_id(&self) -> WindowId {
        unsafe {
            self.windowNumber() as WindowId
        }
    }

    fn get_size(&self) -> LogicalSize {
        self.frame().size.into()
    }

    fn get_origin(&self) -> LogicalPoint {
        self.frame().origin.into()
    }

    fn set_rect(&self, origin: LogicalPoint, size: LogicalSize, animate: bool) {
        unsafe {
            self.setFrame_display_animate(CGRect::new(origin.into(), size.into()), true, animate);
        }
    }

    fn set_max_size(&self, size: LogicalSize) {
        self.setMaxSize(size.into());
    }

    fn set_min_size(&self, size: LogicalSize) {
        self.setMinSize(size.into());
    }

    fn get_max_size(&self) -> LogicalSize {
        return unsafe {
            self.maxSize().into()
        };
    }

    fn get_min_size(&self) -> LogicalSize {
        return unsafe {
            self.minSize().into()
        };
    }
}

fn create_window(mtm: MainThreadMarker, params: &WindowParams) -> Window {
    let rect = CGRect::new(params.origin.into(), params.size.into());
    let style = NSWindowStyleMask::Titled
                | NSWindowStyleMask::Closable
                | NSWindowStyleMask::Miniaturizable
                | NSWindowStyleMask::Resizable;
    let ns_window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer_screen(
            mtm.alloc(),
            rect,
            style,
            // the only non depricated NSBackingStoreType
            NSBackingStoreType::NSBackingStoreBuffered,
            // When true, the window server defers creating the window device until the window is moved onscreen.
            false,
            // Screen
            // When sceen is specified the rect considered to be in its coordinate system
            // By default it's relative to primary screen
            None
        )
    };
    unsafe {
        // todo
        // https://developer.apple.com/library/archive/documentation/General/Conceptual/MOSXAppProgrammingGuide/FullScreenApp/FullScreenApp.html#:~:text=Full%2Dscreen%20support%20in%20NSApplication,is%20also%20key%2Dvalue%20observable.
        ns_window.setCollectionBehavior(NSWindowCollectionBehavior::FullScreenPrimary);
    }
    ns_window.setTitle(&NSString::from_str(params.title()));
    unsafe {
        ns_window.setReleasedWhenClosed(false);
    }
    ns_window.makeKeyAndOrderFront(None);
    ns_window.setLevel(NSNormalWindowLevel);
    unsafe {
        ns_window.setRestorable(false);
    }

    let delegate = WindowDelegate::new(mtm, ns_window.clone());
    ns_window.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

    let root_view = RootView::new(mtm);
    ns_window.setAcceptsMouseMovedEvents(true);
    ns_window.setContentView(Some(&*root_view));
    assert!(ns_window.makeFirstResponder(Some(&*root_view)) == true); // todo remove assert

    return Window {
        ns_window,
        delegate,
        root_view
    };
}

pub(crate) struct WindowDelegateIvars {
    ns_window: Retained<NSWindow>
}

declare_class!(
    pub(crate) struct WindowDelegate;

    unsafe impl ClassType for WindowDelegate {
        type Super = NSObject;
        type Mutability = MainThreadOnly;
        const NAME: &'static str = "WindowDelegate";
    }

    impl DeclaredClass for WindowDelegate {
        type Ivars = WindowDelegateIvars;
    }

    unsafe impl NSObjectProtocol for WindowDelegate {}

    unsafe impl NSWindowDelegate for WindowDelegate {
        #[method(windowDidResize:)]
        #[allow(non_snake_case)]
        unsafe fn windowDidResize(&self, _notification: &NSNotification) {
            handle_window_resize(&*self.ivars().ns_window);
        }

        #[method(windowDidChangeScreen:)]
        #[allow(non_snake_case)]
        unsafe fn windowDidChangeScreen(&self, _notification: &NSNotification) {
            handle_window_screen_change(&*self.ivars().ns_window);
        }

        #[method(windowDidMove:)]
        #[allow(non_snake_case)]
        unsafe fn windowDidMove(&self, _notification: &NSNotification) {
            handle_window_move(&*self.ivars().ns_window);
        }

        #[method(windowDidBecomeKey:)]
        #[allow(non_snake_case)]
        unsafe fn windowDidBecomeKey(&self, _notification: &NSNotification) {
            handle_window_focus_change(&*self.ivars().ns_window);
        }

        #[method(windowDidResignKey:)]
        #[allow(non_snake_case)]
        unsafe fn windowDidResignKey(&self, _notification: &NSNotification) {
            handle_window_focus_change(&*self.ivars().ns_window);
        }

        #[method(windowDidBecomeMain:)]
        #[allow(non_snake_case)]
        unsafe fn windowDidBecomeMain(&self, _notification: &NSNotification) {
            handle_window_focus_change(&*self.ivars().ns_window);
        }

        #[method(windowDidResignMain:)]
        #[allow(non_snake_case)]
        unsafe fn windowDidResignMain(&self, _notification: &NSNotification) {
            handle_window_focus_change(&*self.ivars().ns_window);
        }

        #[method(windowShouldClose:)]
        #[allow(non_snake_case)]
        unsafe fn windowShouldClose(&self, _notification: &NSNotification) -> bool {
            handle_window_close_request(&*self.ivars().ns_window);
            false
        }
    }
);

impl WindowDelegate {
    pub(crate) fn new(mtm: MainThreadMarker, ns_window: Retained<NSWindow>) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(WindowDelegateIvars { ns_window });
        unsafe { msg_send_id![super(this), init] }
    }
}

pub(crate) struct RootViewIvars {}

declare_class!(
    pub(crate) struct RootView;

    unsafe impl ClassType for RootView {
        type Super = NSView;
        type Mutability = MainThreadOnly;
        const NAME: &'static str = "RootView";
    }

    impl DeclaredClass for RootView {
        type Ivars = RootViewIvars;
    }

    unsafe impl NSObjectProtocol for RootView {}

    unsafe impl RootView {
        #[method(mouseMoved:)]
        fn mouse_moved(&self, event: &NSEvent) {
            handle_mouse_moved(event); // todo pass to next responder if it's not handled
        }
        #[method(mouseDown:)]
        fn mouse_down(&self, event: &NSEvent) {
            println!("Down Event: {event:?}");
        }
    }
);

impl RootView {
    pub(crate) fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(RootViewIvars {});
        let root_view: Retained<Self> = unsafe { msg_send_id![super(this), init] };
        unsafe {
            root_view.setAutoresizesSubviews(true);
            root_view.setAutoresizingMask(NSAutoresizingMaskOptions::NSViewWidthSizable | NSAutoresizingMaskOptions::NSViewHeightSizable);
        }
        root_view
    }
}
