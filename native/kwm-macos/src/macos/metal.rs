use std::ffi::c_void;

use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_foundation::{CGPoint, CGRect, CGSize, MainThreadMarker};
use objc2_metal::{MTLCommandQueue, MTLCreateSystemDefaultDevice, MTLDevice};
use objc2_metal_kit::MTKView;

use crate::define_ref;

use super::window::WindowRef;

#[repr(transparent)]
pub struct MetalDeviceRef { ptr: *mut c_void }
define_ref!(MetalDeviceRef, ProtocolObject<dyn MTLDevice>);

#[no_mangle]
pub extern "C" fn metal_create_device() -> MetalDeviceRef {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let device = {
        let ptr = unsafe { MTLCreateSystemDefaultDevice() };
        unsafe { Retained::retain(ptr) }.expect("Failed to get default system device.")
    };
    return MetalDeviceRef::new(device);
}

#[no_mangle]
pub extern "C" fn metal_deref_device(device: MetalDeviceRef) {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    unsafe { device.consume() };
}

#[repr(transparent)]
pub struct MetalCommandQueueRef { ptr: *mut c_void }
define_ref!(MetalCommandQueueRef, ProtocolObject<dyn MTLCommandQueue>);

#[no_mangle]
pub extern "C" fn metal_create_command_queue(device: MetalDeviceRef) -> MetalCommandQueueRef {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let device = unsafe { device.retain() };
    let queue = device.newCommandQueue().unwrap();
    return MetalCommandQueueRef::new(queue);
}

#[no_mangle]
pub extern "C" fn metal_deref_command_queue(queue: MetalCommandQueueRef) {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    unsafe { queue.consume() };
}

#[repr(transparent)]
pub struct MetalViewRef { ptr: *mut c_void }
define_ref!(MetalViewRef, MTKView);

#[no_mangle]
pub extern "C" fn metal_create_view(device: MetalDeviceRef) -> MetalViewRef {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let device = unsafe { device.retain() };
    // it will be resized when attached to the window
    let frame_rect = CGRect::new(CGPoint::ZERO, CGSize::ZERO);
    let view = unsafe { MTKView::initWithFrame_device(mtm.alloc(), frame_rect, Some(&device)) };
    return MetalViewRef::new(view);
}

#[no_mangle]
pub extern "C" fn metal_deref_view(view: MetalViewRef) {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    unsafe { view.consume() };
}

#[no_mangle]
pub extern "C" fn metal_view_attach_to_window(view: MetalViewRef, window: WindowRef) {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let view = unsafe { view.retain() };
    let window = unsafe { window.retain() };
    window.setContentView(Some(&view));
}


// Provide a callback which will be called when drawing performed

//    val context = DirectContext.makeMetal(devicePtr, queuePtr)
//    val mtkViewPtr = ...
//    Surface.makeFromMTKView(context, mtkViewPtr, ...)