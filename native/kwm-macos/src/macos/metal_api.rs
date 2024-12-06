use std::{cell::Cell, ffi::c_void};

use objc2::{rc::Retained, runtime::ProtocolObject, ClassType};
use objc2_app_kit::{NSView, NSViewLayerContentsPlacement, NSViewLayerContentsRedrawPolicy};
use objc2_foundation::{CGPoint, CGRect, CGSize, MainThreadMarker, NSString};
use objc2_metal::{MTLClearColor, MTLCommandBuffer, MTLCommandQueue, MTLCreateSystemDefaultDevice, MTLDevice, MTLDrawable, MTLPixelFormat, MTLTexture};
use objc2_metal_kit::MTKView;
use objc2_quartz_core::{CAAutoresizingMask, CAMetalDrawable, CAMetalLayer};

use crate::{common::Size, define_objc_ref};

use super::{window::WindowRef};

#[repr(transparent)]
pub struct MetalDeviceRef { ptr: *mut c_void }
define_objc_ref!(MetalDeviceRef, ProtocolObject<dyn MTLDevice>);

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
define_objc_ref!(MetalCommandQueueRef, ProtocolObject<dyn MTLCommandQueue>);

#[no_mangle]
pub extern "C" fn metal_create_command_queue(device: MetalDeviceRef) -> MetalCommandQueueRef {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let device = unsafe { device.retain() };
    let queue = device.newCommandQueue().unwrap();
    return MetalCommandQueueRef::new(queue);
}

#[no_mangle]
pub extern "C" fn metal_command_queue_commit(queue: MetalCommandQueueRef) {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let queue = unsafe { queue.retain() };

    let command_buffer = queue.commandBuffer().unwrap();
    command_buffer.setLabel(Some(&NSString::from_str("Present")));
    command_buffer.commit();
    command_buffer.waitUntilScheduled();
}

#[no_mangle]
pub extern "C" fn metal_deref_command_queue(queue: MetalCommandQueueRef) {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    unsafe { queue.consume() };
}

//pub(crate) type MetalViewDrawCallback = extern "C" fn();

pub struct MetalView {
    ns_view: Retained<NSView>,
    layer: Retained<CAMetalLayer>,
    drawable: Cell<Option<Retained<ProtocolObject<dyn CAMetalDrawable>>>>
}

#[no_mangle]
pub extern "C" fn metal_create_view(device: MetalDeviceRef) -> Box<MetalView> {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let device = unsafe { device.retain() };
    let ns_view = unsafe { NSView::new(mtm) };
    let layer = unsafe { CAMetalLayer::new() };
    unsafe {
        layer.setDevice(Some(ProtocolObject::from_ref(&*device)));
        layer.setPixelFormat(MTLPixelFormat::BGRA8Unorm);
//        layer.setFramebufferOnly(false); // missing in zed

        layer.setAllowsNextDrawableTimeout(false);
        // layer.setDisplaySyncEnabled(false); JWM but why ignore vsync?

        // this are marked crucial for correct resize
        layer.setAutoresizingMask(CAAutoresizingMask::kCALayerHeightSizable | CAAutoresizingMask::kCALayerWidthSizable);
        layer.setNeedsDisplayOnBoundsChange(true); // not sure that we need to call ::draw when it's resized
        layer.setPresentsWithTransaction(true);

//        fMetalLayer.contentsGravity = kCAGravityTopLeft; // from JWM
//        fMetalLayer.magnificationFilter = kCAFilterNearest;  // from JWM
    }

    unsafe {
        ns_view.setLayerContentsRedrawPolicy(NSViewLayerContentsRedrawPolicy::NSViewLayerContentsRedrawDuringViewResize);
//        ns_view.setLayerContentsPlacement(NSViewLayerContentsPlacement::ScaleAxesIndependently); // better to demonstrate glitches
        ns_view.setLayerContentsPlacement(NSViewLayerContentsPlacement::TopLeft); // better if you have glitches
        ns_view.setLayer(Some(&layer));
        ns_view.setWantsLayer(true);
    }
    Box::new(MetalView {
        ns_view,
        layer,
        drawable: Cell::new(None)
    })
}

#[no_mangle]
pub extern "C" fn metal_drop_view(view: Box<MetalView>) {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    drop(view);
}

#[no_mangle]
pub extern "C" fn metal_view_present(view: &MetalView) {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    if let Some(drawable) = view.drawable.replace(None) {
        drawable.present();
    }
}

#[no_mangle]
pub extern "C" fn metal_view_get_texture_size(view: &MetalView) -> Size {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let drawable_size = unsafe { view.layer.drawableSize() };
    let view_size = view.ns_view.bounds().size;
    view_size.into()
}

#[repr(transparent)]
pub struct MetalTextureRef { ptr: *mut c_void }
define_objc_ref!(MetalTextureRef, ProtocolObject<dyn MTLTexture>);

#[no_mangle]
pub extern "C" fn metal_view_next_texture(view: &MetalView) -> MetalTextureRef {
    unsafe {
        let view_size = view.ns_view.bounds().size;
        let drawable_size = view.layer.drawableSize();
        if view_size != drawable_size {
            view.layer.setDrawableSize(view_size);
            view.layer.setContentsScale(2f64); // todo fixme!!!
        }
        view.layer.setDrawableSize(view.ns_view.bounds().size);
    }
    let drawable = unsafe {
        view.layer.nextDrawable().unwrap()
    };
    let texture = unsafe { drawable.texture() };
    view.drawable.set(Some(drawable));
    return MetalTextureRef::new(texture);
}

#[no_mangle]
pub extern "C" fn metal_deref_texture(texture: MetalTextureRef) {
    unsafe { texture.consume() };
}

#[no_mangle]
pub extern "C" fn metal_view_attach_to_window(view: &MetalView, window: WindowRef) {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let window = unsafe { window.retain() };
    window.setContentView(Some(&view.ns_view));
}