use std::{ffi::c_void, marker::PhantomData, rc::Rc, sync::Arc};

pub(crate) type StrPtr = *const std::ffi::c_char;
pub (crate) type ArraySize = i64;

#[repr(C)]
pub struct Array {
    arr: *const c_void,
    len: ArraySize,
}

#[repr(C)]
pub struct Size {
    pub width: f64,
    pub height: f64
}

#[macro_export]
macro_rules! define_objc_ref {
    ($name:ident, $otype:ty) => {
        impl $name {
            pub(crate) fn new(obj: Retained<$otype>) -> Self {
                return Self {
                    ptr: Retained::into_raw(obj) as *mut c_void
                }
            }

            pub(crate) unsafe fn retain(&self) -> Retained<$otype> {
                return Retained::retain(self.ptr as *mut $otype).unwrap()
            }

            pub(crate) unsafe fn consume(self) -> Retained<$otype> {
                return Retained::from_raw(self.ptr as *mut $otype).unwrap()
            }
        }
    };
}

//#[repr(transparent)]
//pub struct SomeRef<T> {
//    ptr: *mut c_void,
//    p: PhantomData<T>
//}
//
//impl <T> SomeRef<T> {
//    fn new(obj: Arc<T>) -> Self {
//        SomeRef {
//            ptr: Arc::into_raw(obj) as *mut c_void,
//            p: PhantomData,
//        }
//    }
//
//    unsafe fn retain(&self) -> Arc<T> { // never be unique => no &mut
//        let arc = Arc::from_raw(self.ptr as *mut T);
//        let local_copy = arc.clone();
//        let _ = Arc::into_raw(arc);
//        local_copy
//    }
//
//    unsafe fn consume(self) -> Arc<T> {
//        Arc::from_raw(self.ptr as *mut T)
//    }
//}
