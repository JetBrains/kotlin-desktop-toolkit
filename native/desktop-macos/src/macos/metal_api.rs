#![allow(clippy::missing_safety_doc)]

use std::{cell::Cell, ffi::c_void};

use anyhow::Context;
use objc2::{define_class, msg_send, AllocAnyThread, DefinedClass, MainThreadOnly};
use objc2::rc::autoreleasepool;
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_app_kit::{NSAutoresizingMaskOptions, NSView, NSViewLayerContentsPlacement, NSViewLayerContentsRedrawPolicy};
use objc2_foundation::{ns_string, MainThreadMarker, NSObject, NSObjectProtocol, NSSize};
use objc2_metal::{MTLCommandBuffer, MTLCommandQueue, MTLCreateSystemDefaultDevice, MTLDevice, MTLDrawable, MTLPixelFormat, MTLTexture};
use objc2_quartz_core::{kCAGravityResize, CAAutoresizingMask, CALayer, CALayerDelegate, CAMetalDrawable, CAMetalLayer};

use crate::geometry::PhysicalSize;
use desktop_common::ffi_utils::RustAllocatedRawPtr;
use desktop_common::logger::{catch_panic, ffi_boundary, PanicDefault};

macro_rules! define_objc_ref {
    ($name:ident, $otype:ty) => {
        #[allow(dead_code)]
        impl $name {
            #[must_use]
            pub fn new(obj: Retained<$otype>) -> Self {
                return Self {
                    ptr: Retained::into_raw(obj).cast::<c_void>(),
                };
            }

            #[must_use]
            pub unsafe fn retain(&self) -> Retained<$otype> {
                return unsafe { Retained::retain(self.ptr.cast::<$otype>()) }.unwrap();
            }

            pub unsafe fn consume(self) -> Retained<$otype> {
                return unsafe { Retained::from_raw(self.ptr.cast::<$otype>()) }.unwrap();
            }
        }
    };
}

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
        let device = { MTLCreateSystemDefaultDevice().context("Failed to get default system device.")? };
        Ok(MetalDeviceRef::new(device))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn metal_deref_device(device: MetalDeviceRef) {
    ffi_boundary("metal_deref_device", || {
        std::mem::drop(unsafe { device.consume() });
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
        let device = unsafe { device.retain() };
        let queue = device.newCommandQueue().unwrap();
        Ok(MetalCommandQueueRef::new(queue))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn metal_deref_command_queue(queue: MetalCommandQueueRef) {
    ffi_boundary("metal_deref_command_queue", || {
        std::mem::drop(unsafe { queue.consume() });
        Ok(())
    });
}

pub(crate) struct MetalView {
    pub(crate) layer_view: Retained<MetalLayerView>,
    layer: Retained<CAMetalLayer>,
    #[allow(dead_code)]
    layer_delegate: Retained<LayerDelegate>,
    drawable: Cell<Option<Retained<ProtocolObject<dyn CAMetalDrawable>>>>,
}

pub type MetalViewPtr<'a> = RustAllocatedRawPtr<'a, std::ffi::c_void>;

impl MetalView {
    pub(crate) fn ns_view(&self) -> &NSView {
        &self.layer_view
    }
}

pub(crate)  struct MetalLayerViewIvars {}

define_class!(
    #[unsafe(super(NSView))]
    #[name = "MetalLayerView"]
    #[ivars = MetalLayerViewIvars]
    #[thread_kind = MainThreadOnly]
    pub(crate) struct MetalLayerView;

    impl MetalLayerView {
        #[unsafe(method(setFrameSize:))]
        fn set_frame_size(&self, new_size: NSSize) {
            catch_panic(|| {
                let _: () = unsafe { msg_send![super(self), setFrameSize: new_size] };
                // the order is important, otherwise resize might be glitchy
                self.update_layer_size_and_scale();
                Ok(())
            });
        }

        #[unsafe(method(viewDidChangeBackingProperties))]
        fn view_did_change_backing_properties(&self) {
            catch_panic(|| {
                let _: () = unsafe { msg_send![super(self), viewDidChangeBackingProperties] };
                // the order is important, otherwise resize might be glitchy
                self.update_layer_size_and_scale();
                Ok(())
            });
        }
    }
);

impl MetalLayerView {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = MainThreadOnly::alloc(mtm).set_ivars(MetalLayerViewIvars {});
        unsafe { msg_send![super(this), init] }
    }

    fn update_layer_size_and_scale(&self) {
        let layer = unsafe { self.layer().unwrap() };
        let metal_layer: &CAMetalLayer = layer.downcast_ref().unwrap();

        let view_size = self.bounds().size;
        let new_drawable_size = unsafe { self.convertSizeToBacking(view_size) };
        let scale = new_drawable_size.width / view_size.width;
        unsafe {
            metal_layer.setDrawableSize(new_drawable_size);
            metal_layer.setContentsScale(scale);
        };
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "LayerDelegate"]
    pub(crate) struct LayerDelegate;

    unsafe impl NSObjectProtocol for LayerDelegate {}

    unsafe impl CALayerDelegate for LayerDelegate {
        #[unsafe(method(displayLayer:))]
        fn display_layer(&self, layer: &CALayer) {
            catch_panic(|| {
                println!("Layer {layer:?} redraw requested");
                Ok(())
            });
        }
    }
);

impl LayerDelegate {
    fn new() -> Retained<Self> {
        let this = Self::alloc().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn metal_create_view(device: MetalDeviceRef) -> MetalViewPtr<'static> {
    let metal_view = ffi_boundary("metal_create_view", || {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let device = unsafe { device.retain() };
        let layer = unsafe { CAMetalLayer::new() };
        let layer_delegate = LayerDelegate::new();
        unsafe {
            layer.setDevice(Some(ProtocolObject::from_ref(&*device)));
            layer.setPixelFormat(MTLPixelFormat::BGRA8Unorm);
            // For Fleet use case we need to sample the texture
            // e.g. to implment frost glass effect for tabs
            layer.setFramebufferOnly(false);

            // layer.setFramebufferOnly(false); // missing in zed

            layer.setMaximumDrawableCount(3);
            layer.setAllowsNextDrawableTimeout(false);
            // layer.setDisplaySyncEnabled(false); //JWM but why ignore vsync?

            // this are marked crucial for correct resize
            layer.setAutoresizingMask(CAAutoresizingMask::LayerHeightSizable | CAAutoresizingMask::LayerWidthSizable);
            //layer.setNeedsDisplayOnBoundsChange(true); // not sure that we need to call ::draw when it's resized
            layer.setPresentsWithTransaction(true);

            layer.setContentsGravity(kCAGravityResize);
            // fMetalLayer.magnificationFilter = kCAFilterNearest;  // from JWM

            layer.setDelegate(Some(ProtocolObject::from_ref(&*layer_delegate)));
        }

        let layer_view = MetalLayerView::new(mtm);
        unsafe {
            layer_view.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable);

            layer_view.setLayerContentsRedrawPolicy(NSViewLayerContentsRedrawPolicy::DuringViewResize);
            layer_view.setLayerContentsPlacement(NSViewLayerContentsPlacement::ScaleAxesIndependently); // better to demonstrate glitches
            // layer_view.setLayerContentsPlacement(NSViewLayerContentsPlacement::TopLeft); // better if you have glitches
            layer_view.setLayer(Some(&layer));
        }

        layer_view.setWantsLayer(true);

        Ok(Some(MetalView {
            layer_view,
            layer,
            layer_delegate,
            drawable: Cell::new(None),
        }))
    });
    MetalViewPtr::from_value(metal_view)
}

#[unsafe(no_mangle)]
pub extern "C" fn metal_drop_view(view_ptr: MetalViewPtr) {
    ffi_boundary("metal_drop_view", || {
        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        std::mem::drop(unsafe { view_ptr.to_owned::<MetalView>() });
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn metal_view_set_is_opaque(view_ptr: MetalViewPtr, value: bool) {
    ffi_boundary("metal_view_set_is_opaque", || {
        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
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
        autoreleasepool(|_| {
            let view = unsafe { view_ptr.borrow::<MetalView>() };
            if let Some(drawable) = view.drawable.replace(None) {
                let queue = unsafe { queue.retain() };
                let command_buffer = queue.commandBuffer().unwrap();
                command_buffer.setLabel(Some(ns_string!("Present")));
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
                    let drawable = ProtocolObject::from_retained(drawable);
                    command_buffer.presentDrawable(&drawable);
                    command_buffer.commit();
                };
            }
        });
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
        autoreleasepool(|_| {
            let view = unsafe { view_ptr.borrow::<MetalView>() };
            let drawable = unsafe { view.layer.nextDrawable().expect("No drawable") };
            let texture = unsafe { drawable.texture() };
            view.drawable.set(Some(drawable));
            Ok(MetalTextureRef::new(texture))
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn metal_deref_texture(texture: MetalTextureRef) {
    ffi_boundary("metal_deref_texture", || {
        std::mem::drop(unsafe { texture.consume() });
        Ok(())
    });
}
