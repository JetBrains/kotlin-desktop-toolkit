#![allow(clippy::inline_always)]
#![allow(clippy::ref_as_ptr)]

use windows::Win32::{
    Foundation::{E_POINTER, POINTL},
    System::{
        Com::IDataObject,
        Ole::{DROPEFFECT, DROPEFFECT_NONE, DoDragDrop, IDropSource, IDropSource_Impl, IDropTarget, IDropTarget_Impl, RegisterDragDrop},
        SystemServices::MODIFIERKEYS_FLAGS,
    },
};
use windows_core::{BOOL, HRESULT, Ref as WinRef, Result as WinResult, implement};

use super::{geometry::PhysicalPoint, window::Window};

#[allow(clippy::struct_field_names)]
#[repr(C)]
pub struct DropTargetCallbacks {
    drag_enter_handler: extern "C" fn(u32, PhysicalPoint, u32) -> u32,
    drag_over_handler: extern "C" fn(u32, PhysicalPoint, u32) -> u32,
    drag_leave_handler: extern "C" fn(),
    drop_handler: extern "C" fn(u32, PhysicalPoint, u32) -> u32,
}

#[allow(clippy::struct_field_names)]
#[repr(C)]
pub struct DragSourceCallbacks {
    query_continue_drag_handler: extern "C" fn(bool, u32) -> DragDropContinueResult,
}

pub fn register_drop_target(window: &Window, callbacks: DropTargetCallbacks) -> anyhow::Result<()> {
    let target: IDropTarget = DropTarget { callbacks }.into();
    unsafe { RegisterDragDrop(window.hwnd(), &target)? };
    Ok(())
}

pub fn start_drag_drop(data_object: &IDataObject, callbacks: DragSourceCallbacks) -> anyhow::Result<()> {
    let source: IDropSource = DragSource { callbacks }.into();
    let mut effect = DROPEFFECT::default();
    unsafe { DoDragDrop(data_object, &source, DROPEFFECT_NONE, &raw mut effect).ok()? };
    Ok(())
}

#[repr(u32)]
pub enum DragDropContinueResult {
    Continue,
    Cancel,
    Drop,
}

#[implement(IDropSource)]
pub struct DragSource {
    callbacks: DragSourceCallbacks,
}

#[allow(non_snake_case)]
impl IDropSource_Impl for DragSource_Impl {
    fn QueryContinueDrag(&self, escape_pressed: BOOL, key_state: MODIFIERKEYS_FLAGS) -> HRESULT {
        match (self.callbacks.query_continue_drag_handler)(escape_pressed.as_bool(), key_state.0) {
            DragDropContinueResult::Continue => windows::Win32::Foundation::S_OK,
            DragDropContinueResult::Cancel => windows::Win32::Foundation::DRAGDROP_S_CANCEL,
            DragDropContinueResult::Drop => windows::Win32::Foundation::DRAGDROP_S_DROP,
        }
    }

    fn GiveFeedback(&self, _effect: DROPEFFECT) -> HRESULT {
        windows::Win32::Foundation::DRAGDROP_S_USEDEFAULTCURSORS
    }
}

#[allow(clippy::struct_field_names)]
#[implement(IDropTarget)]
pub struct DropTarget {
    callbacks: DropTargetCallbacks,
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[allow(non_snake_case)]
impl IDropTarget_Impl for DropTarget_Impl {
    fn DragEnter(
        &self,
        _data_obj: WinRef<IDataObject>,
        key_state: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> WinResult<()> {
        let effect = unsafe { effect.as_mut() }.ok_or(E_POINTER)?;
        let result = (self.callbacks.drag_enter_handler)(key_state.0, PhysicalPoint::new(pt.x, pt.y), (*effect).0);
        *effect = DROPEFFECT(result);
        Ok(())
    }

    fn DragOver(&self, key_state: MODIFIERKEYS_FLAGS, pt: &POINTL, effect: *mut DROPEFFECT) -> WinResult<()> {
        let effect = unsafe { effect.as_mut() }.ok_or(E_POINTER)?;
        let result = (self.callbacks.drag_over_handler)(key_state.0, PhysicalPoint::new(pt.x, pt.y), (*effect).0);
        *effect = DROPEFFECT(result);
        Ok(())
    }

    fn DragLeave(&self) -> WinResult<()> {
        (self.callbacks.drag_leave_handler)();
        Ok(())
    }

    fn Drop(&self, _data_obj: WinRef<IDataObject>, key_state: MODIFIERKEYS_FLAGS, pt: &POINTL, effect: *mut DROPEFFECT) -> WinResult<()> {
        let effect = unsafe { effect.as_mut() }.ok_or(E_POINTER)?;
        let result = (self.callbacks.drop_handler)(key_state.0, PhysicalPoint::new(pt.x, pt.y), (*effect).0);
        *effect = DROPEFFECT(result);
        Ok(())
    }
}
