use std::ffi::{c_void, CStr};

use objc2::rc::Retained;
use objc2_app_kit::{NSBackingStoreType, NSNormalWindowLevel, NSWindow, NSWindowStyleMask};
use objc2_foundation::{CGPoint, CGRect, CGSize, MainThreadMarker, NSString};

use crate::{common::StrPtr, define_ref};

#[repr(transparent)]
pub struct WindowRef { ptr: *mut c_void }
define_ref!(WindowRef, NSWindow);

#[no_mangle]
pub extern "C" fn window_create(title: StrPtr, x: f32, y: f32) -> WindowRef {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let title = unsafe { CStr::from_ptr(title) }.to_str().unwrap();
    let window = create_window(mtm, title, x, y);
    return WindowRef::new(window);
}

#[no_mangle]
pub extern "C" fn window_deref(window: WindowRef) {
    unsafe {
        window.consume();
    }
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