#![allow(clippy::inline_always)]
#![allow(clippy::ref_as_ptr)]

use anyhow::Context;
use windows::Win32::{
    Foundation::{COLORREF, E_POINTER, HWND, POINT, POINTL, SIZE},
    Graphics::Gdi::{DeleteObject, HGDIOBJ},
    System::{
        Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, IDataObject},
        Ole::{
            DROPEFFECT, DROPEFFECT_NONE, DoDragDrop, IDropSource, IDropSource_Impl, IDropTarget, IDropTarget_Impl, RegisterDragDrop,
            RevokeDragDrop,
        },
        SystemServices::MODIFIERKEYS_FLAGS,
    },
    UI::{
        Controls::CLR_NONE,
        Shell::{CLSID_DragDropHelper, IDragSourceHelper, IDropTargetHelper, SHDRAGIMAGE},
    },
};
use windows_core::{BOOL, HRESULT, Ref as WinRef, Result as WinResult, implement};

use super::{com::ComInterfaceRawPtr, geometry::PhysicalPoint, wic_image::WicBitmap, window::Window};

#[allow(clippy::struct_field_names)]
#[repr(C)]
pub struct DropTargetCallbacks {
    drag_enter_handler: extern "C" fn(ComInterfaceRawPtr, u32, PhysicalPoint, u32) -> u32,
    drag_over_handler: extern "C" fn(u32, PhysicalPoint, u32) -> u32,
    drag_leave_handler: extern "C" fn(),
    drop_handler: extern "C" fn(ComInterfaceRawPtr, u32, PhysicalPoint, u32) -> u32,
}

#[allow(clippy::struct_field_names)]
#[repr(C)]
pub struct DragSourceCallbacks {
    query_continue_drag_handler: extern "C" fn(bool, u32) -> DragDropContinueResult,
}

pub fn register_drop_target(window: &Window, callbacks: DropTargetCallbacks) -> anyhow::Result<()> {
    // The drag-image helper lets the Shell render the OS drag image over our window. It is purely
    // cosmetic, so a creation failure leaves `None` and the drop still works.
    let helper: Option<IDropTargetHelper> = unsafe { CoCreateInstance(&CLSID_DragDropHelper, None, CLSCTX_INPROC_SERVER) }
        .inspect_err(|err| log::warn!("drop-target drag-image helper unavailable: {err}"))
        .ok();
    let target: IDropTarget = DropTarget {
        callbacks,
        helper,
        hwnd: window.hwnd(),
    }
    .into();
    unsafe { RegisterDragDrop(window.hwnd(), &target)? };
    Ok(())
}

pub fn start_drag_drop(
    data_object: &IDataObject,
    allowed_effects: u32,
    drag_image: Option<(&[u8], PhysicalPoint)>,
    callbacks: DragSourceCallbacks,
) -> anyhow::Result<u32> {
    if let Some((image_bytes, cursor_offset)) = drag_image {
        // Create the helper before decoding the image: until create_drag_image runs there is no
        // bitmap to clean up, so a helper-creation failure here leaks nothing.
        let helper: IDragSourceHelper =
            unsafe { CoCreateInstance(&CLSID_DragDropHelper, None, CLSCTX_INPROC_SERVER) }.context("create drag-drop helper")?;
        let shdi = create_drag_image(image_bytes, cursor_offset)?;
        // InitializeFromBitmap takes ownership of shdi.hbmpDragImage on success; on failure the helper
        // does not take it, so free it here. This is the only manual cleanup point for the bitmap.
        if let Err(err) = unsafe { helper.InitializeFromBitmap(&raw const shdi, data_object) } {
            let _ = unsafe { DeleteObject(HGDIOBJ(shdi.hbmpDragImage.0)) };
            anyhow::bail!("failed to initialize drag image: {err}");
        }
    }
    let source: IDropSource = DragSource { callbacks }.into();
    let mut effect = DROPEFFECT_NONE;
    unsafe { DoDragDrop(data_object, &source, DROPEFFECT(allowed_effects), &raw mut effect).ok()? };
    Ok(effect.0)
}

pub fn revoke_drop_target(window: &Window) -> anyhow::Result<()> {
    unsafe { RevokeDragDrop(window.hwnd())? };
    Ok(())
}

// Builds the `SHDRAGIMAGE` for a source-initiated drag from an encoded image (PNG, JPEG, …). The
// caller owns the returned `hbmpDragImage` and must free it once the drag-image helper is done.
pub(crate) fn create_drag_image(image_bytes: &[u8], cursor_offset: PhysicalPoint) -> anyhow::Result<SHDRAGIMAGE> {
    let image = WicBitmap::decode_from_bytes(image_bytes)?;
    let size = image.size();
    Ok(SHDRAGIMAGE {
        sizeDragImage: SIZE {
            cx: size.width.0,
            cy: size.height.0,
        },
        ptOffset: POINT {
            x: cursor_offset.x.0,
            y: cursor_offset.y.0,
        },
        hbmpDragImage: image.into_handle(),
        // No color key; the DIB uses per-pixel alpha.
        crColorKey: COLORREF(CLR_NONE.cast_unsigned()),
    })
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
    helper: Option<IDropTargetHelper>,
    hwnd: HWND,
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[allow(non_snake_case)]
impl IDropTarget_Impl for DropTarget_Impl {
    fn DragEnter(
        &self,
        data_obj: WinRef<IDataObject>,
        key_state: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> WinResult<()> {
        let effect = unsafe { effect.as_mut() }.ok_or(E_POINTER)?;
        let data_object = data_obj.as_ref().ok_or(E_POINTER)?;
        let data_obj_ptr = ComInterfaceRawPtr::from_interface(data_object)?;
        let result = (self.callbacks.drag_enter_handler)(data_obj_ptr, key_state.0, PhysicalPoint::new(pt.x, pt.y), effect.0);
        if let Some(helper) = &self.helper {
            let point = POINT { x: pt.x, y: pt.y };
            // Cosmetic only: ignore helper errors so the app's resolved effect is what we return.
            let _ = unsafe { helper.DragEnter(self.hwnd, data_object, &raw const point, DROPEFFECT(result)) };
        }
        *effect = DROPEFFECT(result);
        Ok(())
    }

    fn DragOver(&self, key_state: MODIFIERKEYS_FLAGS, pt: &POINTL, effect: *mut DROPEFFECT) -> WinResult<()> {
        let effect = unsafe { effect.as_mut() }.ok_or(E_POINTER)?;
        let result = (self.callbacks.drag_over_handler)(key_state.0, PhysicalPoint::new(pt.x, pt.y), effect.0);
        if let Some(helper) = &self.helper {
            let point = POINT { x: pt.x, y: pt.y };
            // Cosmetic only: ignore helper errors so the app's resolved effect is what we return.
            let _ = unsafe { helper.DragOver(&raw const point, DROPEFFECT(result)) };
        }
        *effect = DROPEFFECT(result);
        Ok(())
    }

    fn DragLeave(&self) -> WinResult<()> {
        (self.callbacks.drag_leave_handler)();
        if let Some(helper) = &self.helper {
            // Cosmetic only: ignore helper errors.
            let _ = unsafe { helper.DragLeave() };
        }
        Ok(())
    }

    fn Drop(&self, data_obj: WinRef<IDataObject>, key_state: MODIFIERKEYS_FLAGS, pt: &POINTL, effect: *mut DROPEFFECT) -> WinResult<()> {
        let effect = unsafe { effect.as_mut() }.ok_or(E_POINTER)?;
        let data_object = data_obj.as_ref().ok_or(E_POINTER)?;
        let data_obj_ptr = ComInterfaceRawPtr::from_interface(data_object)?;
        let result = (self.callbacks.drop_handler)(data_obj_ptr, key_state.0, PhysicalPoint::new(pt.x, pt.y), effect.0);
        if let Some(helper) = &self.helper {
            let point = POINT { x: pt.x, y: pt.y };
            // Cosmetic only: ignore helper errors so the app's resolved effect is what we return.
            let _ = unsafe { helper.Drop(data_object, &raw const point, DROPEFFECT(result)) };
        }
        *effect = DROPEFFECT(result);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::win32::data_object::DataObject;
    use windows::Win32::{
        Foundation::{RPC_E_CHANGED_MODE, S_FALSE},
        System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx},
    };

    // 3x2 (non-square) RGBA PNG the Shell helper can decode. Non-square so a width/height
    // transposition in the SHDRAGIMAGE size is observable.
    const TEST_PNG_3X2: [u8; 74] = [
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00,
        0x00, 0x02, 0x08, 0x06, 0x00, 0x00, 0x00, 0x9d, 0x74, 0x66, 0x1a, 0x00, 0x00, 0x00, 0x11, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63,
        0xf8, 0xcf, 0xc0, 0xf0, 0x1f, 0x86, 0x19, 0x90, 0x39, 0x00, 0x9b, 0x7e, 0x0b, 0xf5, 0x72, 0xb0, 0xb9, 0x3c, 0x00, 0x00, 0x00, 0x00,
        0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    fn ensure_com_initialized() {
        // The Shell drag-drop helper is an STA in-proc object; creating it from an MTA thread fails
        // to find a cross-apartment proxy (E_NOINTERFACE). Match production (application.rs calls
        // OleInitialize, which is STA) by initializing this thread as an STA. Cargo unit tests have
        // no OleInitialize of their own; tolerate the "already initialized" returns.
        let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        assert!(
            hr.is_ok() || hr == S_FALSE || hr == RPC_E_CHANGED_MODE,
            "CoInitializeEx failed: {hr:?}"
        );
    }

    // The drag-image helper requires the data object to accept its private formats (stored via
    // IDataObject::SetData) and a valid SHDRAGIMAGE bitmap; this checks that InitializeFromBitmap
    // succeeds against our data object and a decoded image. On success the helper owns
    // shdi.hbmpDragImage, so it is NOT freed here; the single handle lives until the test process
    // exits, which is acceptable.
    #[test]
    fn initialize_from_bitmap_accepts_our_data_object() {
        ensure_com_initialized();
        let data_object: IDataObject = DataObject::new().into();
        let helper: IDragSourceHelper =
            unsafe { CoCreateInstance(&CLSID_DragDropHelper, None, CLSCTX_INPROC_SERVER) }.expect("create drag-drop helper");
        let shdi = create_drag_image(&TEST_PNG_3X2, PhysicalPoint::new(0, 0)).expect("create drag image");
        let result = unsafe { helper.InitializeFromBitmap(&raw const shdi, &data_object) };
        assert!(result.is_ok(), "InitializeFromBitmap failed: {result:?}");
    }

    // create_drag_image maps the decoded size into SHDRAGIMAGE.sizeDragImage and the cursor offset
    // into ptOffset. The asymmetric offset (x != y) catches an x/y swap; the non-square 3x2 image
    // catches a width/height transposition.
    #[test]
    fn create_drag_image_maps_offset_and_size() {
        ensure_com_initialized();
        let image = create_drag_image(&TEST_PNG_3X2, PhysicalPoint::new(7, 11)).expect("create drag image");
        let offset = (image.ptOffset.x, image.ptOffset.y);
        let size = (image.sizeDragImage.cx, image.sizeDragImage.cy);
        // This layer owns the raw HBITMAP; free it once the fields are read.
        let deleted = unsafe { DeleteObject(HGDIOBJ(image.hbmpDragImage.0)) };
        assert!(deleted.as_bool(), "failed to delete drag-image bitmap");
        assert_eq!(offset, (7, 11), "ptOffset must mirror the cursor offset");
        assert_eq!(size, (3, 2), "sizeDragImage must be (width, height), not transposed");
    }
}
