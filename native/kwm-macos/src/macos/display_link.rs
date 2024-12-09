use objc2::{declare_class, msg_send_id, mutability::{self, MainThreadOnly}, rc::Retained, runtime::ProtocolObject, sel, ClassType, DeclaredClass};
use objc2_app_kit::NSWindow;
use objc2_foundation::{MainThreadMarker, NSDefaultRunLoopMode, NSObject, NSObjectProtocol, NSRunLoop, NSRunLoopCommonModes, NSRunLoopMode};
use objc2_quartz_core::CADisplayLink;

use crate::macos::display_link;

use super::window::WindowRef;

struct DisplayLink {
    link: Retained<CADisplayLink>,
    delegate: Retained<DisplayLinkDelegate>
}

#[no_mangle]
extern "C" fn display_link_create(window: WindowRef, on_next_frame: DisplayLinkCallback) -> Box<DisplayLink> {
    let mtm = MainThreadMarker::new().unwrap();
    let window = unsafe { window.retain() };
    let delegate = DisplayLinkDelegate::new(mtm, on_next_frame);
    // todo the api is available since macOS 14.0+
    let display_link: Retained<CADisplayLink> = unsafe {
         msg_send_id![&window, displayLinkWithTarget: &*delegate,
                                            selector: sel!(onNextFrame:)]
    };
    unsafe {
        // when using the NSDefaultRunLoopMode we don't get the notification when window is resizing
        display_link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes);
    }
    return Box::new(DisplayLink {
        link: display_link,
        delegate: delegate
    });
}

#[no_mangle]
extern "C" fn display_link_set_paused(display_link: &DisplayLink, value: bool) {
    unsafe {
        display_link.link.setPaused(value);
    }
}

#[no_mangle]
extern "C" fn display_link_drop(display_link: Box<DisplayLink>) {
    unsafe {
        display_link.link.invalidate();
    }
    drop(display_link);
}

type DisplayLinkCallback = extern "C" fn();

pub(crate) struct DisplayLinkDelegateIvars {
    on_next_frame: DisplayLinkCallback
}

declare_class!(
    pub(crate) struct DisplayLinkDelegate;

    unsafe impl ClassType for DisplayLinkDelegate {
        type Super = NSObject;
        type Mutability = MainThreadOnly;
        const NAME: &'static str = "DisplayLinkDelegate";
    }

    impl DeclaredClass for DisplayLinkDelegate {
        type Ivars = DisplayLinkDelegateIvars;
    }

    unsafe impl DisplayLinkDelegate {
        #[method(onNextFrame:)]
        fn on_next_frame(&self, display_link: &CADisplayLink) {
            (self.ivars().on_next_frame)();
        }
    }

    unsafe impl NSObjectProtocol for DisplayLinkDelegate {}
);

impl DisplayLinkDelegate {
    pub(crate) fn new(mtm: MainThreadMarker, on_next_frame: DisplayLinkCallback) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(DisplayLinkDelegateIvars {
            on_next_frame
        });
        unsafe { msg_send_id![super(this), init] }
    }
}