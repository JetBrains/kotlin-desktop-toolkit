use desktop_common::{ffi_utils::BorrowedArray, logger::ffi_boundary};

use super::{
    data_object::DataObject,
    drag_drop::{DragSourceCallbacks, DropTargetCallbacks, register_drop_target, start_drag_drop},
    global_data::HGlobalData,
    window_api::{WindowPtr, with_window},
};

#[unsafe(no_mangle)]
pub extern "C" fn drag_drop_register_target(window_ptr: WindowPtr, callbacks: DropTargetCallbacks) {
    with_window(&window_ptr, "drag_drop_register_target", |window| {
        register_drop_target(window, callbacks)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn drag_drop_start(data_format: u32, dragged_data: BorrowedArray<u8>, callbacks: DragSourceCallbacks) {
    ffi_boundary("drag_drop_start", || {
        let data = dragged_data.as_slice().and_then(HGlobalData::alloc_from)?;
        let data_object = DataObject::create(data_format, data).into();
        start_drag_drop(&data_object, callbacks)
    });
}
