use objc2::{declare_class, msg_send_id, mutability::{self, MainThreadOnly}, rc::Retained, sel, ClassType, DeclaredClass};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSSize};
use objc2_metal_kit::{MTKView, MTKViewDelegate};

//use super::metal_api::MetalViewDrawCallback;
//
//pub(crate) struct MetalViewDelegateIvars {
//    pub(crate) on_draw: MetalViewDrawCallback
//}
//
//declare_class!(
//    pub(crate) struct MetalViewDelegate;
//    unsafe impl ClassType for MetalViewDelegate {
//        type Super = NSObject;
//        type Mutability = MainThreadOnly;
//        const NAME: &'static str = "MetalViewDelegate";
//    }
//
//    impl DeclaredClass for MetalViewDelegate {
//        type Ivars = MetalViewDelegateIvars;
//    }
//
//    unsafe impl NSObjectProtocol for MetalViewDelegate {}
//
//    unsafe impl MTKViewDelegate for MetalViewDelegate {
//        #[method(drawInMTKView:)]
//        #[allow(non_snake_case)]
//        unsafe fn drawInMTKView(&self, mtk_view: &MTKView) {
//            (self.ivars().on_draw)();
//            // todo
//            // command_buffer.presentDrawable(ProtocolObject::from_ref(&*current_drawable));
//            // command_buffer.commit();
//        }
//
//        #[method(mtkView:drawableSizeWillChange:)]
//        #[allow(non_snake_case)]
//        unsafe fn mtkView_drawableSizeWillChange(&self, mtk_view: &MTKView, size: NSSize) {
//            println!("Resize: {mtk_view:?} size: {size:?}");
//        }
//    }
//);
//
//impl MetalViewDelegate {
//    pub(crate) fn new(mtm: MainThreadMarker, on_draw: MetalViewDrawCallback) -> Retained<Self> {
//        let this = mtm.alloc();
//        let this = this.set_ivars(MetalViewDelegateIvars {
//            on_draw,
//        });
//        unsafe { msg_send_id![super(this), init] }
//    }
//}



// todo add metal view here