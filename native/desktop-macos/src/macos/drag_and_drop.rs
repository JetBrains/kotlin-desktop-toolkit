use std::cell::RefCell;

use desktop_common::ffi_utils::{AutoDropStrPtr, RustAllocatedRawPtr};
use log::warn;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSDragOperation, NSDraggingContext, NSDraggingInfo, NSDraggingSession, NSScreen};
use objc2_foundation::{MainThreadMarker, NSPoint};

use crate::geometry::LogicalPoint;

use super::{application_api::AppState, screen::NSScreenExts, string::copy_to_c_string, window::NSWindowExts, window_api::WindowId};

pub type DragOperation = usize;
pub type DragOperationsBitSet = usize;
pub type SequenceNumber = isize;

#[repr(C)]
#[derive(Debug)]
pub struct DragTargetInfo {
    destination_window_id: WindowId,
    location_in_window: LogicalPoint,
    allowed_operations: DragOperationsBitSet,
    // Identify current DnD session
    // For next session it will be different
    sequence_number: SequenceNumber,
    pasteboard_name: AutoDropStrPtr,
}

impl DragTargetInfo {
    pub fn new(info: &ProtocolObject<dyn NSDraggingInfo>) -> Self {
        let destination_window = info.draggingDestinationWindow().expect("No window in drag event");
        let destination_window_id = destination_window.window_id();
        let window_height = destination_window.contentView().unwrap().frame().size.height;
        let location_in_window = LogicalPoint::from_macos_coords(info.draggingLocation(), window_height);
        let allowed_operations = info.draggingSourceOperationMask().0;
        let sequence_number = info.draggingSequenceNumber();
        let pasteboard_name = copy_to_c_string(info.draggingPasteboard().name().as_ref()).unwrap().to_auto_drop();
        Self {
            destination_window_id,
            location_in_window,
            allowed_operations,
            sequence_number,
            pasteboard_name,
        }
    }
}

pub type DragTargetEnteredCallback = extern "C" fn(info: DragTargetInfo) -> DragOperation;
pub type DragTargetUpdatedCallback = extern "C" fn(info: DragTargetInfo) -> DragOperation;
pub type DragTargetExitedCallback = extern "C" fn(info: RustAllocatedRawPtr); // we use a pointer because info is optional for this callback
pub type DragTargetPerformCallback = extern "C" fn(info: DragTargetInfo) -> bool;

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct DraggingContext(pub isize);

pub type DragSourceOperationMaskCallback =
    extern "C" fn(source_window_id: WindowId, sequence_number: SequenceNumber, context: DraggingContext) -> DragOperationsBitSet;
pub type DragSourceSessionWillBeginAt =
    extern "C" fn(source_window_id: WindowId, sequence_number: SequenceNumber, location_on_screen: LogicalPoint);
pub type DragSourceSessionMovedTo =
    extern "C" fn(source_window_id: WindowId, sequence_number: SequenceNumber, location_on_screen: LogicalPoint);
pub type DragSourceSessionEndedAt = extern "C" fn(
    source_window_id: WindowId,
    sequence_number: SequenceNumber,
    location_on_screen: LogicalPoint,
    drag_operation: DragOperation,
);

#[derive(Debug)]
#[repr(C)]
#[allow(clippy::struct_field_names)]
pub struct DragAndDropCallbacks {
    drag_target_entered_callback: DragTargetEnteredCallback,
    drag_target_updated_callback: DragTargetUpdatedCallback,
    drag_target_exited_callback: DragTargetExitedCallback,
    drag_target_perform_callback: DragTargetPerformCallback,

    drag_source_operation_mask_callback: DragSourceOperationMaskCallback,
    drag_source_session_will_begin_at: DragSourceSessionWillBeginAt,
    drag_source_session_moved_to: DragSourceSessionMovedTo,
    drag_source_session_ended_at: DragSourceSessionEndedAt,
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

pub fn handle_drag_target_entered(info: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
    with_drag_callbacks(|callbacks| {
        let drag_info = DragTargetInfo::new(info);
        let result = (callbacks.drag_target_entered_callback)(drag_info);
        NSDragOperation(result)
    })
    .unwrap_or(NSDragOperation::None)
}

pub fn handle_drag_target_updated(info: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
    with_drag_callbacks(|callbacks| {
        let drag_info = DragTargetInfo::new(info);
        let result = (callbacks.drag_target_updated_callback)(drag_info);
        NSDragOperation(result)
    })
    .unwrap_or(NSDragOperation::None)
}

pub fn handle_drag_target_exited(info: Option<&ProtocolObject<dyn NSDraggingInfo>>) {
    with_drag_callbacks(|callbacks| {
        let drag_info = RustAllocatedRawPtr::from_value(info.map(DragTargetInfo::new));
        (callbacks.drag_target_exited_callback)(drag_info.clone()); // cloning pointer here
        if !drag_info.is_null() {
            drop(unsafe { drag_info.to_owned::<DragTargetInfo>() });
        }
    });
}

pub fn handle_drag_target_perform(info: &ProtocolObject<dyn NSDraggingInfo>) -> bool {
    with_drag_callbacks(|callbacks| {
        let drag_info = DragTargetInfo::new(info);
        (callbacks.drag_target_perform_callback)(drag_info)
    })
    .unwrap_or(false)
}

pub fn handle_drag_source_operation_mask(
    source_window_id: WindowId,
    session: &NSDraggingSession,
    context: NSDraggingContext,
) -> DragOperationsBitSet {
    let sequence_number = session.draggingSequenceNumber();
    with_drag_callbacks(|callbacks| {
        (callbacks.drag_source_operation_mask_callback)(source_window_id, sequence_number, DraggingContext(context.0))
    })
    .unwrap_or(0)
}

pub fn handle_drag_source_session_will_begin_at(
    source_window_id: WindowId,
    session: &NSDraggingSession,
    screen_point: NSPoint,
    mtm: MainThreadMarker,
) {
    let sequence_number = session.draggingSequenceNumber();
    if let Ok(screen) = NSScreen::primary(mtm) {
        let screen_height = screen.height();
        let location = LogicalPoint::from_macos_coords(screen_point, screen_height);
        with_drag_callbacks(|callbacks| {
            (callbacks.drag_source_session_will_begin_at)(source_window_id, sequence_number, location);
        });
    }
}

pub fn handle_drag_source_session_moved_to(
    source_window_id: WindowId,
    session: &NSDraggingSession,
    screen_point: NSPoint,
    mtm: MainThreadMarker,
) {
    let sequence_number = session.draggingSequenceNumber();
    if let Ok(screen) = NSScreen::primary(mtm) {
        let screen_height = screen.height();
        let location = LogicalPoint::from_macos_coords(screen_point, screen_height);
        with_drag_callbacks(|callbacks| {
            (callbacks.drag_source_session_moved_to)(source_window_id, sequence_number, location);
        });
    }
}

pub fn handle_drag_source_session_ended_at(
    source_window_id: WindowId,
    session: &NSDraggingSession,
    screen_point: NSPoint,
    operation: NSDragOperation,
    mtm: MainThreadMarker,
) {
    let sequence_number = session.draggingSequenceNumber();
    let drag_operation = operation.0;
    if let Ok(screen) = NSScreen::primary(mtm) {
        let screen_height = screen.height();
        let location = LogicalPoint::from_macos_coords(screen_point, screen_height);
        with_drag_callbacks(|callbacks| {
            (callbacks.drag_source_session_ended_at)(source_window_id, sequence_number, location, drag_operation);
        });
    }
}
