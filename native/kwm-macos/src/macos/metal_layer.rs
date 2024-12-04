use std::ffi::c_void;

use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_foundation::MainThreadMarker;
use objc2_metal::{MTLCommandQueue, MTLCreateSystemDefaultDevice, MTLDevice};

use super::application::WindowId;

macro_rules! define_ref {
    ($name:ident, $otype:ty) => {
        #[repr(transparent)]
        pub struct $name { ptr: *mut c_void }

        impl $name {
            fn new(obj: Retained<$otype>) -> Self {
                return Self {
                    ptr: Retained::into_raw(obj) as *mut c_void
                }
            }

            unsafe fn retain(&self) -> Retained<$otype> {
                return Retained::retain(self.ptr as *mut $otype).unwrap()
            }

            unsafe fn consume(self) -> Retained<$otype> {
                return Retained::from_raw(self.ptr as *mut $otype).unwrap()
            }
        }
    };
}

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
pub extern "C" fn metal_deref_device(device_ref: MetalDeviceRef) {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    unsafe { device_ref.consume() };
}

define_ref!(MetalQueueRef, ProtocolObject<dyn MTLCommandQueue>);

#[no_mangle]
pub extern "C" fn metal_create_command_queue(device_ref: MetalDeviceRef) -> MetalQueueRef {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let device = unsafe { device_ref.retain() };
    let queue = device.newCommandQueue().unwrap();
    return MetalQueueRef::new(queue);
}

#[no_mangle]
pub extern "C" fn metal_deref_command_queue(queue_ref: MetalQueueRef) {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    unsafe { queue_ref.consume() };
}

#[no_mangle]
pub extern "C" fn metal_create_layer() {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
}

#[no_mangle]
pub extern "C" fn metal_layer_attach_to_window() {

}


// Provide a callback which will be called

//    val context = DirectContext.makeMetal(devicePtr, queuePtr)
//    val mtkViewPtr = ...
//    Surface.makeFromMTKView(context, mtkViewPtr, ...)