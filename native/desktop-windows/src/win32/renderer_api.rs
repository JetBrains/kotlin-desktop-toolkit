use desktop_common::{
    ffi_utils::{BorrowedStrPtr, RustAllocatedRawPtr},
    logger::ffi_boundary,
};

use super::{renderer_angle::AngleDevice, window::Window, window_api::WindowPtr};

pub type AngleDevicePtr<'a> = RustAllocatedRawPtr<'a>;

#[derive(Debug)]
#[repr(C)]
pub struct EglGetProcFuncData<'a> {
    pub f: extern "C" fn(ctx: AngleDevicePtr, name: BorrowedStrPtr) -> Option<extern "system" fn()>,
    pub ctx: AngleDevicePtr<'a>,
}

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
    let angle_device = ffi_boundary("renderer_angle_device_create", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        let window_id = window.id();
        let mut angle_device = AngleDevice::create_for_window(window_id)?;
        // initial surface
        angle_device.create_surface(0, 0)?;
        Ok(Some(angle_device))
    });
    AngleDevicePtr::from_value(angle_device)
}
