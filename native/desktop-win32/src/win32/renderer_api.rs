use desktop_common::{
    ffi_utils::{BorrowedStrPtr, RustAllocatedRawPtr},
    logger::{PanicDefault, ffi_boundary},
};

use super::{
    renderer_angle::AngleDevice,
    window_api::{WindowPtr, with_window},
};

pub type AngleDevicePtr<'a> = RustAllocatedRawPtr<'a>;

#[derive(Debug)]
#[repr(C)]
pub struct EglGetProcFuncData<'a> {
    pub f: extern "C" fn(ctx: AngleDevicePtr, name: BorrowedStrPtr) -> Option<extern "system" fn()>,
    pub ctx: AngleDevicePtr<'a>,
}

#[derive(Debug)]
#[repr(C)]
pub struct EglSurfaceData {
    pub framebuffer_binding: i32,
}

impl PanicDefault for EglSurfaceData {
    fn default() -> Self {
        Self { framebuffer_binding: 0 }
    }
}

/// cbindgen:ignore
extern "C" fn egl_get_proc_address(ctx_ptr: AngleDevicePtr, name_ptr: BorrowedStrPtr) -> Option<extern "system" fn()> {
    let name = name_ptr.as_str().unwrap();
    let angle_device = unsafe { ctx_ptr.borrow::<AngleDevice>() };
    angle_device.get_proc_address(name)
}

#[unsafe(no_mangle)]
pub extern "C" fn renderer_angle_get_egl_get_proc_func(angle_device_ptr: AngleDevicePtr) -> EglGetProcFuncData {
    EglGetProcFuncData {
        f: egl_get_proc_address,
        ctx: angle_device_ptr,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn renderer_angle_device_create(window_ptr: WindowPtr) -> AngleDevicePtr {
    let angle_device = with_window(&window_ptr, "renderer_angle_device_create", |window| {
        let angle_device = AngleDevice::create_for_window(window)?;
        Ok(Some(angle_device))
    });
    AngleDevicePtr::from_value(angle_device)
}

#[unsafe(no_mangle)]
pub extern "C" fn renderer_angle_resize_surface(mut angle_device_ptr: AngleDevicePtr, width: i32, height: i32) -> EglSurfaceData {
    ffi_boundary("renderer_angle_resize_surface", || {
        let angle_device = unsafe { angle_device_ptr.borrow_mut::<AngleDevice>() };
        angle_device.resize_surface(width, height)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn renderer_angle_make_current(angle_device_ptr: AngleDevicePtr) {
    ffi_boundary("renderer_angle_make_current", || {
        let angle_device = unsafe { angle_device_ptr.borrow::<AngleDevice>() };
        angle_device.make_current()
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn renderer_angle_swap_buffers(angle_device_ptr: AngleDevicePtr, wait_for_vsync: bool) {
    ffi_boundary("renderer_angle_swap_buffers", || {
        let angle_device = unsafe { angle_device_ptr.borrow::<AngleDevice>() };
        angle_device.swap_buffers(wait_for_vsync)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn renderer_angle_drop(angle_device_ptr: AngleDevicePtr) {
    ffi_boundary("renderer_angle_drop", || {
        let _angle_device = unsafe { angle_device_ptr.to_owned::<AngleDevice>() };
        Ok(())
    });
}
