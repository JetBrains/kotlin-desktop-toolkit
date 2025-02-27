use std::{cell::Cell, ffi::c_void};

use anyhow::Context;
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_app_kit::{NSAutoresizingMaskOptions, NSView, NSViewLayerContentsPlacement, NSViewLayerContentsRedrawPolicy};
use objc2_foundation::{MainThreadMarker, NSString};
use objc2_metal::{MTLCommandBuffer, MTLCommandQueue, MTLCreateSystemDefaultDevice, MTLDevice, MTLDrawable, MTLPixelFormat, MTLTexture};
use objc2_quartz_core::{CAAutoresizingMask, CAMetalDrawable, CAMetalLayer, kCAGravityTopLeft};

use crate::common::RustAllocatedRawPtr;
use crate::logger::{PanicDefault, ffi_boundary};
use crate::{common::PhysicalSize, define_objc_ref};

#[repr(transparent)]
pub struct MetalDeviceRef {
    ptr: *mut c_void,
}
define_objc_ref!(MetalDeviceRef, ProtocolObject<dyn MTLDevice>);

impl PanicDefault for MetalDeviceRef {
    fn default() -> Self {
        Self { ptr: std::ptr::null_mut() }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn metal_create_device() -> MetalDeviceRef {
    ffi_boundary("metal_create_device", || {
        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let device = { MTLCreateSystemDefaultDevice().context("Failed to get default system device.")? };
        Ok(MetalDeviceRef::new(device))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn metal_deref_device(device: MetalDeviceRef) {
    ffi_boundary("metal_deref_device", || {
        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        unsafe { device.consume() };
        Ok(())
    });
}

#[repr(transparent)]
pub struct MetalCommandQueueRef {
    ptr: *mut c_void,
}
define_objc_ref!(MetalCommandQueueRef, ProtocolObject<dyn MTLCommandQueue>);

impl PanicDefault for MetalCommandQueueRef {
    fn default() -> Self {
        Self { ptr: std::ptr::null_mut() }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn metal_create_command_queue(device: MetalDeviceRef) -> MetalCommandQueueRef {
    ffi_boundary("metal_create_command_queue", || {
        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let device = unsafe { device.retain() };
        let queue = device.newCommandQueue().unwrap();
        Ok(MetalCommandQueueRef::new(queue))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn metal_deref_command_queue(queue: MetalCommandQueueRef) {
    ffi_boundary("metal_deref_command_queue", || {
        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        unsafe { queue.consume() };
        Ok(())
    });
}

pub(crate) struct MetalView {
    pub(crate) ns_view: Retained<NSView>,
    layer: Retained<CAMetalLayer>,
    drawable: Cell<Option<Retained<ProtocolObject<dyn CAMetalDrawable>>>>,
}

pub type MetalViewPtr<'a> = RustAllocatedRawPtr<'a, std::ffi::c_void>;

impl MetalView {
    pub(crate) fn ns_view(&self) -> &NSView {
        &self.ns_view
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn metal_create_view(device: MetalDeviceRef) -> MetalViewPtr<'static> {
    let metal_view = ffi_boundary("metal_create_view", || {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let device = unsafe { device.retain() };
        let layer = unsafe { CAMetalLayer::new() };
        unsafe {
            layer.setDevice(Some(ProtocolObject::from_ref(&*device)));
            layer.setPixelFormat(MTLPixelFormat::BGRA8Unorm);

            //        layer.setFramebufferOnly(false); // missing in zed

            layer.setAllowsNextDrawableTimeout(false);
            // layer.setDisplaySyncEnabled(false); JWM but why ignore vsync?

            // this are marked crucial for correct resize
            layer.setAutoresizingMask(CAAutoresizingMask::LayerHeightSizable | CAAutoresizingMask::LayerWidthSizable);
            // layer.setNeedsDisplayOnBoundsChange(true); // not sure that we need to call ::draw when it's resized
            layer.setPresentsWithTransaction(true);

            layer.setContentsGravity(kCAGravityTopLeft); // from JWM
            // fMetalLayer.magnificationFilter = kCAFilterNearest;  // from JWM
        }

        let layer_view = unsafe { NSView::new(mtm) };
        unsafe {
            layer_view.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable);

            layer_view.setLayerContentsRedrawPolicy(NSViewLayerContentsRedrawPolicy::DuringViewResize);
            layer_view.setLayerContentsPlacement(NSViewLayerContentsPlacement::ScaleAxesIndependently); // better to demonstrate glitches
            // layer_view.setLayerContentsPlacement(NSViewLayerContentsPlacement::TopLeft); // better if you have glitches
            layer_view.setLayer(Some(&layer));
        }

        layer_view.setWantsLayer(true);

        Ok(Some(MetalView {
            ns_view: layer_view,
            layer,
            drawable: Cell::new(None),
        }))
    });
    MetalViewPtr::from_value(metal_view)
}

#[unsafe(no_mangle)]
pub extern "C" fn metal_drop_view(view_ptr: MetalViewPtr) {
    ffi_boundary("metal_drop_view", || {
        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let _view = unsafe { view_ptr.to_owned::<MetalView>() };
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn metal_view_set_is_opaque(view_ptr: MetalViewPtr, value: bool) {
    ffi_boundary("metal_view_set_is_opaque", || {
        let view = unsafe { view_ptr.borrow::<MetalView>() };
        view.layer.setOpaque(value);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn metal_view_get_is_opaque(view_ptr: MetalViewPtr) -> bool {
    ffi_boundary("metal_view_get_is_opaque", || {
        let view = unsafe { view_ptr.borrow::<MetalView>() };
        Ok(view.layer.isOpaque())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn metal_view_present(view_ptr: MetalViewPtr, queue: MetalCommandQueueRef, wait_for_ca_transaction: bool) {
    ffi_boundary("metal_view_present", || {
        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let view = unsafe { view_ptr.borrow::<MetalView>() };
        if let Some(drawable) = view.drawable.replace(None) {
            let queue = unsafe { queue.retain() };
            let command_buffer = queue.commandBuffer().unwrap();
            command_buffer.setLabel(Some(&NSString::from_str("Present")));
            if wait_for_ca_transaction {
                unsafe {
                    view.layer.setPresentsWithTransaction(true);
                }
                command_buffer.commit();
                command_buffer.waitUntilScheduled();
                drawable.present();
            } else {
                unsafe {
                    view.layer.setPresentsWithTransaction(false);
                }
                command_buffer.presentDrawable(&ProtocolObject::from_retained(drawable));
                command_buffer.commit();
            }
        }
        Ok(())
    });
}

impl PanicDefault for PhysicalSize {
    fn default() -> Self {
        Self { width: 0.0, height: 0.0 }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn metal_view_get_texture_size(view_ptr: MetalViewPtr) -> PhysicalSize {
    ffi_boundary("metal_view_get_texture_size", || {
        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let view = unsafe { view_ptr.borrow::<MetalView>() };
        let ns_view = view.ns_view();
        let view_size = unsafe { ns_view.convertSizeToBacking(ns_view.bounds().size) };
        Ok(view_size.into())
    })
}

#[repr(transparent)]
pub struct MetalTextureRef {
    ptr: *mut c_void,
}
define_objc_ref!(MetalTextureRef, ProtocolObject<dyn MTLTexture>);

impl PanicDefault for MetalTextureRef {
    fn default() -> Self {
        Self { ptr: std::ptr::null_mut() }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn metal_view_next_texture(view_ptr: MetalViewPtr) -> MetalTextureRef {
    ffi_boundary("metal_view_next_texture", || {
        let view = unsafe { view_ptr.borrow::<MetalView>() };
        unsafe {
            const SCALE_DIFF_TOLERANCE: f64 = 0.001;
            let ns_view = view.ns_view();
            let view_size = ns_view.bounds().size;
            let drawable_size = view.layer.drawableSize();
            let new_drawable_size = ns_view.convertSizeToBacking(view_size);
            let scale = new_drawable_size.width / view_size.width;
            if new_drawable_size != drawable_size || (view.layer.contentsScale() - scale).abs() > SCALE_DIFF_TOLERANCE {
                view.layer.setDrawableSize(new_drawable_size);
                view.layer.setContentsScale(scale);
            }
        }
        let drawable = unsafe { view.layer.nextDrawable().expect("No drawable") };
        let texture = unsafe { drawable.texture() };
        view.drawable.set(Some(drawable));
        Ok(MetalTextureRef::new(texture))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn metal_deref_texture(texture: MetalTextureRef) {
    ffi_boundary("metal_deref_texture", || {
        unsafe { texture.consume() };
        Ok(())
    });
}
