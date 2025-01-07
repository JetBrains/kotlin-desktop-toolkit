use std::{
    cell::{Cell, OnceCell},
    ffi::c_void,
};

use anyhow::Context;
use objc2::{
    class, declare_class, msg_send, msg_send_id, mutability::MainThreadOnly, rc::Retained, runtime::ProtocolObject, ClassType,
    DeclaredClass,
};
use objc2_app_kit::{NSAutoresizingMaskOptions, NSColor, NSView, NSViewLayerContentsPlacement, NSViewLayerContentsRedrawPolicy};
use objc2_foundation::{CGPoint, CGRect, CGSize, MainThreadMarker, NSObjectProtocol, NSRect, NSSize, NSString};
use objc2_metal::{
    MTLClearColor, MTLCommandBuffer, MTLCommandQueue, MTLCreateSystemDefaultDevice, MTLDevice, MTLDrawable, MTLPixelFormat, MTLTexture,
};
use objc2_metal_kit::MTKView;
use objc2_quartz_core::{kCAGravityTopLeft, CAAutoresizingMask, CAMetalDrawable, CAMetalLayer};

use crate::{
    common::{LogicalSize, PhysicalSize},
    define_objc_ref,
};

#[repr(transparent)]
pub struct MetalDeviceRef {
    ptr: *mut c_void,
}
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
pub struct MetalCommandQueueRef {
    ptr: *mut c_void,
}
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

pub struct MetalView {
    pub(crate) ns_view: OnceCell<Retained<NSView>>,
    pub(crate) layer: Retained<CAMetalLayer>,
    pub(crate) drawable: Cell<Option<Retained<ProtocolObject<dyn CAMetalDrawable>>>>,
}

impl MetalView {
    pub(crate) fn ns_view(&self) -> anyhow::Result<&NSView> {
        return self.ns_view.get().map(|ns_view| &**ns_view).context("Layer isn't attached yet");
    }

    pub(crate) fn attach_to_view(&self, ns_view: Retained<NSView>) -> anyhow::Result<()> {
        unsafe {
            // ns_view.setTranslatesAutoresizingMaskIntoConstraints(false); // it actually changes nothing
            //            ns_view.setAutoresizingMask(NSAutoresizingMaskOptions::NSViewWidthSizable | NSAutoresizingMaskOptions::NSViewHeightSizable);

            ns_view.setLayerContentsRedrawPolicy(NSViewLayerContentsRedrawPolicy::NSViewLayerContentsRedrawDuringViewResize);
            ns_view.setLayerContentsPlacement(NSViewLayerContentsPlacement::ScaleAxesIndependently); // better to demonstrate glitches
                                                                                                     //        ns_view.setLayerContentsPlacement(NSViewLayerContentsPlacement::TopLeft); // better if you have glitches
            ns_view.setLayer(Some(&self.layer));
            ns_view.setWantsLayer(true);
        }
        self.ns_view.set(ns_view).ok().context("Can't attach the layer second time")?;
        return Ok(());
    }
}

#[no_mangle]
pub extern "C" fn metal_create_view(device: MetalDeviceRef) -> Box<MetalView> {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let device = unsafe { device.retain() };
    let layer = unsafe { CAMetalLayer::new() };
    unsafe {
        layer.setDevice(Some(ProtocolObject::from_ref(&*device)));
        layer.setPixelFormat(MTLPixelFormat::BGRA8Unorm);

        layer.setOpaque(false);

        //        layer.setFramebufferOnly(false); // missing in zed

        layer.setAllowsNextDrawableTimeout(false);
        // layer.setDisplaySyncEnabled(false); JWM but why ignore vsync?

        // this are marked crucial for correct resize
        layer.setAutoresizingMask(CAAutoresizingMask::kCALayerHeightSizable | CAAutoresizingMask::kCALayerWidthSizable);
        // layer.setNeedsDisplayOnBoundsChange(true); // not sure that we need to call ::draw when it's resized
        layer.setPresentsWithTransaction(true);

        layer.setContentsGravity(kCAGravityTopLeft); // from JWM
        // fMetalLayer.magnificationFilter = kCAFilterNearest;  // from JWM
    }

    Box::new(MetalView {
        ns_view: OnceCell::new(),
        layer,
        drawable: Cell::new(None),
    })
}

#[no_mangle]
pub extern "C" fn metal_drop_view(view: Box<MetalView>) {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    std::mem::drop(view);
}

#[no_mangle]
pub extern "C" fn metal_view_present(view: &MetalView) {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    if let Some(drawable) = view.drawable.replace(None) {
        drawable.present();
    }
}

#[no_mangle]
pub extern "C" fn metal_view_get_texture_size(view: &MetalView) -> PhysicalSize {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let ns_view = view.ns_view().unwrap();
    let view_size = unsafe { ns_view.convertSizeToBacking(ns_view.bounds().size) };
    view_size.into()
}

#[repr(transparent)]
pub struct MetalTextureRef {
    ptr: *mut c_void,
}
define_objc_ref!(MetalTextureRef, ProtocolObject<dyn MTLTexture>);

#[no_mangle]
pub extern "C" fn metal_view_next_texture(view: &MetalView) -> MetalTextureRef {
    unsafe {
        let ns_view = view.ns_view().unwrap();
        let view_size = ns_view.bounds().size;
        let drawable_size = view.layer.drawableSize();
        let new_drawable_size = ns_view.convertSizeToBacking(view_size);
        let scale = new_drawable_size.width / view_size.width;
        if new_drawable_size != drawable_size || view.layer.contentsScale() != scale {
            view.layer.setDrawableSize(new_drawable_size);
            view.layer.setContentsScale(scale);
        }
    }
    let drawable = unsafe { view.layer.nextDrawable().expect("No drawable") };
    let texture = unsafe { drawable.texture() };
    view.drawable.set(Some(drawable));
    return MetalTextureRef::new(texture);
}

#[no_mangle]
pub extern "C" fn metal_deref_texture(texture: MetalTextureRef) {
    unsafe { texture.consume() };
}
