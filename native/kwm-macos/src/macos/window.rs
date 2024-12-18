use std::ffi::{c_void, CStr};

use objc2::{
    declare_class, msg_send_id,
    mutability::{self, MainThreadOnly},
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
    sel, ClassType, DeclaredClass,
};
use objc2_app_kit::{NSAutoresizingMaskOptions, NSBackingStoreType, NSEvent, NSNormalWindowLevel, NSView, NSWindow, NSWindowDelegate, NSWindowStyleMask};
use objc2_foundation::{CGPoint, CGRect, CGSize, MainThreadMarker, NSNotification, NSNumber, NSObject, NSObjectProtocol, NSString};

use crate::{
    common::{LogicalSize, StrPtr},
    define_objc_ref,
    macos::{application_api::AppState, events::{handle_mouse_moved, handle_window_screen_change, handle_window_resize}},
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

#[no_mangle]
pub extern "C" fn window_create(title: StrPtr, x: f32, y: f32) -> Box<Window> {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let title = unsafe { CStr::from_ptr(title) }.to_str().unwrap();
    let window = create_window(mtm, title, x, y);
    return Box::new(window)
}

#[no_mangle]
pub extern "C" fn window_deref(window: Box<Window>) {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
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

pub(crate) trait NSWindowExts {
    fn window_id(&self) -> WindowId;
    fn logical_size(&self) -> LogicalSize;
}

impl NSWindowExts for NSWindow {
    fn window_id(&self) -> WindowId {
        unsafe {
            self.windowNumber() as WindowId
        }
    }

    fn logical_size(&self) -> LogicalSize {
        self.frame().size.into()
    }
}

fn create_window(mtm: MainThreadMarker, title: &str, x: f32, y: f32) -> Window {
    let rect = CGRect::new(CGPoint::new(x.into(), y.into()), CGSize::new(320.0, 240.0));
    let style = NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Miniaturizable | NSWindowStyleMask::Resizable;
    let ns_window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc(),
            rect,
            style,
            NSBackingStoreType::NSBackingStoreBuffered,
            false,
        )
    };
    ns_window.setTitle(&NSString::from_str(title));
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
