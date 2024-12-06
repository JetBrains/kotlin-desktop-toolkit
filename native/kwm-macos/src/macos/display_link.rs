use objc2::extern_class;
use objc2_foundation::NSObject;
use objc2_quartz_core::CADisplayLink;

use super::window::WindowRef;



#[no_mangle]
extern "C" fn create_display_link(window: WindowRef) {
    let window = unsafe { window.retain() };
//    window.displayLinkWithTarget()
}

//extern_class!(
//    #[derive(Debug, PartialEq, Eq, Hash)]
//    pub struct CADisplayLink;
//
//    unsafe impl ClassType for CADisplayLink {
//        type Super = NSObject;
//    }
//);