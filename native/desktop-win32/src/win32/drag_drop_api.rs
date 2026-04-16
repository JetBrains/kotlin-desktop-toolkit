use desktop_common::logger::ffi_boundary;

use windows::Win32::System::Com::IDataObject;

use super::{
    com::ComInterfaceRawPtr,
    drag_drop::{DragSourceCallbacks, DropTargetCallbacks, register_drop_target, revoke_drop_target, start_drag_drop},
    window_api::{WindowPtr, with_window},
};

#[unsafe(no_mangle)]
pub extern "C" fn drag_drop_register_target(window_ptr: WindowPtr, callbacks: DropTargetCallbacks) {
    with_window(&window_ptr, "drag_drop_register_target", |window| {
        register_drop_target(window, callbacks)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn drag_drop_start(data_object: ComInterfaceRawPtr, callbacks: DragSourceCallbacks) {
    ffi_boundary("drag_drop_start", || {
        let data_object = data_object.borrow::<IDataObject>()?;
        start_drag_drop(&data_object, callbacks)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn drag_drop_revoke_target(window_ptr: WindowPtr) {
    with_window(&window_ptr, "drag_drop_revoke_target", revoke_drop_target);
}
