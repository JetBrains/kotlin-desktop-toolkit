use std::cell::RefCell;

use desktop_common::ffi_utils::{AutoDropStrPtr, RustAllocatedRawPtr};
use log::warn;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSDragOperation, NSDraggingInfo};

use crate::geometry::LogicalPoint;

use super::{application_api::AppState, string::copy_to_c_string, window::NSWindowExts, window_api::WindowId};

pub type DragOperation = usize;
pub type DragOperationsBitSet = usize;

#[repr(C)]
#[derive(Debug)]
pub struct DragInfo {
    destination_window_id: WindowId,
    location_in_window: LogicalPoint,
    allowed_operations: DragOperationsBitSet,
    // Identify current DnD session
    // For next session it will be different
    sequence_number: isize,
    pasteboard_name: AutoDropStrPtr,
}

impl DragInfo {
    pub fn new(info: &ProtocolObject<dyn NSDraggingInfo>) -> Self {
        let destination_window = unsafe { info.draggingDestinationWindow() }.expect("No window in drag event");
        let destination_window_id = destination_window.window_id();
        let window_height = destination_window.contentView().unwrap().frame().size.height;
        let location_in_window = LogicalPoint::from_macos_coords(unsafe { info.draggingLocation() }, window_height);
        let allowed_operations = unsafe { info.draggingSourceOperationMask() }.0;
        let sequence_number = unsafe { info.draggingSequenceNumber() };
        let pasteboard_name = copy_to_c_string(unsafe { info.draggingPasteboard().name() }.as_ref())
            .unwrap()
            .to_auto_drop();
        Self {
            destination_window_id,
            location_in_window,
            allowed_operations,
            sequence_number,
            pasteboard_name,
        }
    }
}

pub type DragEnteredCallback = extern "C" fn(info: DragInfo) -> DragOperation;
pub type DragUpdatedCallback = extern "C" fn(info: DragInfo) -> DragOperation;
pub type DragExitedCallback = extern "C" fn(info: RustAllocatedRawPtr);
pub type DragPerformCallback = extern "C" fn(info: DragInfo) -> bool;

#[allow(clippy::struct_field_names)]
#[derive(Debug)]
#[repr(C)]
pub struct DragAndDropCallbacks {
    drag_entered_callback: DragEnteredCallback,
    drag_updated_callback: DragUpdatedCallback,
    drag_exited_callback: DragExitedCallback,
    drag_perform_callback: DragPerformCallback,
}

#[derive(Default, Debug)]
pub(crate) struct DragAndDropHandlerState {
    callbacks: RefCell<Option<DragAndDropCallbacks>>,
}

#[unsafe(no_mangle)]
pub extern "C" fn set_drag_and_drop_callbacks(callbacks: DragAndDropCallbacks) {
    AppState::with(|state| {
        let mut old_callbacks = state.drag_and_drop_handler_state.callbacks.borrow_mut();
        if old_callbacks.is_some() {
            warn!("Overwrite old DragAndDropCallbacks {old_callbacks:?} -> {callbacks:?}");
        }
        *old_callbacks = Some(callbacks);
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn drop_drag_and_drop_callbacks() {
    AppState::with(|state| {
        let mut callbacks = state.drag_and_drop_handler_state.callbacks.borrow_mut();
        *callbacks = None;
    });
}

fn with_drag_callbacks<T>(f: impl FnOnce(&DragAndDropCallbacks) -> T) -> Option<T> {
    AppState::with(|state| {
        let callbacks = state.drag_and_drop_handler_state.callbacks.borrow();
        callbacks.as_ref().map(f)
    })
}

pub fn handle_drag_entered(info: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
    with_drag_callbacks(|callbacks| {
        let drag_info = DragInfo::new(info);
        let result = (callbacks.drag_entered_callback)(drag_info);
        NSDragOperation(result)
    })
    .unwrap_or(NSDragOperation::None)
}

pub fn handle_drag_updated(info: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
    with_drag_callbacks(|callbacks| {
        let drag_info = DragInfo::new(info);
        let result = (callbacks.drag_updated_callback)(drag_info);
        NSDragOperation(result)
    })
    .unwrap_or(NSDragOperation::None)
}

pub fn handle_drag_exited(info: Option<&ProtocolObject<dyn NSDraggingInfo>>) {
    with_drag_callbacks(|callbacks| {
        let drag_info = RustAllocatedRawPtr::from_value(info.map(DragInfo::new));
        (callbacks.drag_exited_callback)(drag_info.clone()); // cloning pointer here
        if !drag_info.is_null() {
            std::mem::drop(unsafe { drag_info.to_owned::<DragInfo>() });
        }
    });
}

pub fn handle_drag_perform(info: &ProtocolObject<dyn NSDraggingInfo>) -> bool {
    with_drag_callbacks(|callbacks| {
        let drag_info = DragInfo::new(info);
        (callbacks.drag_perform_callback)(drag_info)
    })
    .unwrap_or(false)
}
