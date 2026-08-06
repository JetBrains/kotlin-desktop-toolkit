use desktop_common::ffi_utils::BorrowedArray;

use windows::Win32::System::Com::IDataObject;

use super::{
    com::ComInterfaceRawPtr,
    drag_drop::{DragSourceCallbacks, DropTargetCallbacks, register_drop_target, revoke_drop_target, start_drag_drop},
    geometry::PhysicalPoint,
    window_api::{WindowPtr, with_window},
};

#[unsafe(no_mangle)]
pub extern "C" fn drag_drop_register_target(window_ptr: WindowPtr, callbacks: DropTargetCallbacks) {
    with_window(&window_ptr, "drag_drop_register_target", |window| {
        register_drop_target(window, callbacks)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn drag_drop_start(
    window_ptr: WindowPtr,
    data_object: ComInterfaceRawPtr,
    allowed_effects: u32,
    drag_image_bytes: BorrowedArray<u8>,
    drag_image_offset: PhysicalPoint,
    callbacks: DragSourceCallbacks,
) -> u32 {
    // Takes the window because `DoDragDrop` needs a mouse message waiting in the queue before its loop
    // will start; `start_drag_drop` posts one to this window. See `seed_drag_input_queue`.
    with_window(&window_ptr, "drag_drop_start", |window| {
        let data_object = data_object.cast::<IDataObject>()?;
        // A null drag_image_bytes array means "no custom drag image"; otherwise pair the bytes with
        // the cursor offset.
        let drag_image = drag_image_bytes.as_optional_slice().map(|bytes| (bytes, drag_image_offset));
        start_drag_drop(window, &data_object, allowed_effects, drag_image, callbacks)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn drag_drop_revoke_target(window_ptr: WindowPtr) {
    with_window(&window_ptr, "drag_drop_revoke_target", revoke_drop_target);
}
