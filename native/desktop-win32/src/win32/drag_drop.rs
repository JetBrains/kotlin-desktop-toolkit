#![allow(clippy::inline_always)]
#![allow(clippy::ref_as_ptr)]

use std::sync::OnceLock;

use anyhow::Context;
use windows::Win32::{
    Foundation::{COLORREF, E_POINTER, HWND, LPARAM, POINT, POINTL, SIZE, WPARAM},
    Graphics::Gdi::{DeleteObject, HGDIOBJ, ScreenToClient},
    System::{
        Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, IDataObject},
        Ole::{
            DROPEFFECT, DROPEFFECT_NONE, DoDragDrop, IDropSource, IDropSource_Impl, IDropTarget, IDropTarget_Impl, RegisterDragDrop,
            RevokeDragDrop,
        },
        SystemServices::{MK_LBUTTON, MK_MBUTTON, MK_RBUTTON, MK_XBUTTON1, MK_XBUTTON2, MODIFIERKEYS_FLAGS},
    },
    UI::{
        Controls::CLR_NONE,
        Input::KeyboardAndMouse::{GetKeyState, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON, VK_XBUTTON1, VK_XBUTTON2},
        Shell::{CLSID_DragDropHelper, DSH_ALLOWDROPDESCRIPTIONTEXT, IDragSourceHelper2, IDropTargetHelper, SHDRAGIMAGE},
        WindowsAndMessaging::{GetCursorPos, PostMessageW, WM_MOUSEMOVE},
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

/// Escape hatch for reproducing the stall [`seed_drag_input_queue`] prevents; nothing in production should
/// set `KDT_WIN32_DND_NO_SEED`.
fn seed_disabled_by_env() -> bool {
    static DISABLED: OnceLock<bool> = OnceLock::new();
    *DISABLED.get_or_init(|| {
        let disabled = std::env::var_os("KDT_WIN32_DND_NO_SEED").is_some_and(|value| value == "1");
        if disabled {
            log::warn!("KDT_WIN32_DND_NO_SEED=1: not seeding the input queue before DoDragDrop; drags may stall for seconds");
        }
        disabled
    })
}

/// Posts one `WM_MOUSEMOVE` to `window` so `DoDragDrop`'s modal loop has a mouse message to retrieve as soon
/// as it starts.
///
/// `DoDragDrop` establishes no initial state of its own: it fills the cursor position and key state from each
/// message it retrieves, so until one arrives it makes no progress. We call it synchronously from the handler
/// for the message that started the drag, which by then has been retrieved and dispatched — so without this
/// the queue is empty at exactly the wrong moment. The loop then spins at ~100% of a core without calling
/// `IDropSource::QueryContinueDrag` at all, for as long as 8 seconds, and on a thread whose only input
/// producer is itself (a UI-thread-driven test robot) it does not proceed until something else supplies
/// input. With the seed the first callback arrives in single-digit milliseconds.
///
/// The payload is not incidental. `ole32` reads the pressed buttons out of the message, so a `wParam`
/// claiming none makes `QueryContinueDrag` report a release and the drag ends immediately with a spurious
/// drop. The state is therefore read live here rather than hardcoded or inherited from the triggering event,
/// and no release is ever synthesised: reporting a button still down a moment after it was released costs one
/// extra loop iteration before the real `WM_LBUTTONUP` arrives, whereas under-reporting ends the drag.
///
/// `lParam` carries the cursor position for anything that dispatches the message; the drag loop itself reads
/// `MSG::pt`, which the system stamps with the real cursor position when the message is posted.
fn seed_drag_input_queue(window: &Window) {
    if seed_disabled_by_env() {
        return;
    }
    let hwnd = window.hwnd();
    let mut cursor = POINT::default();
    let position = if unsafe { GetCursorPos(&raw mut cursor) }.is_ok() && unsafe { ScreenToClient(hwnd, &raw mut cursor) }.as_bool() {
        LPARAM(((cursor.y & 0xffff) << 16) as isize | (cursor.x & 0xffff) as isize)
    } else {
        LPARAM(0)
    };
    let mut buttons = 0_u32;
    for (key, flag) in [
        (VK_LBUTTON, MK_LBUTTON),
        (VK_RBUTTON, MK_RBUTTON),
        (VK_MBUTTON, MK_MBUTTON),
        (VK_XBUTTON1, MK_XBUTTON1),
        (VK_XBUTTON2, MK_XBUTTON2),
    ] {
        // The high-order bit of GetKeyState is documented as "the key is down".
        if unsafe { GetKeyState(i32::from(key.0)) } < 0 {
            buttons |= flag.0;
        }
    }
    if let Err(err) = unsafe { PostMessageW(Some(hwnd), WM_MOUSEMOVE, WPARAM(buttons as usize), position) } {
        log::warn!("could not seed the drag input queue, the drag may not start until the mouse moves: {err}");
    }
}

pub fn start_drag_drop(
    window: &Window,
    data_object: &IDataObject,
    allowed_effects: u32,
    drag_image: Option<(&[u8], PhysicalPoint)>,
    callbacks: DragSourceCallbacks,
) -> anyhow::Result<u32> {
    if let Some((image_bytes, cursor_offset)) = drag_image {
        // Create the helper before decoding the image: until create_drag_image runs there is no
        // bitmap to clean up, so a helper-creation failure here leaks nothing.
        let helper: IDragSourceHelper2 =
            unsafe { CoCreateInstance(&CLSID_DragDropHelper, None, CLSCTX_INPROC_SERVER) }.context("create drag-drop helper")?;
        // Let a drop target's drop-description text render over our image. Cosmetic, so a failure must
        // not abort the drag; must be set before InitializeFromBitmap to take effect.
        let _ = unsafe { helper.SetFlags(DSH_ALLOWDROPDESCRIPTIONTEXT.0.cast_unsigned()) };
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
    // Required: `DoDragDrop`'s loop does not start until it retrieves an input message, and by this point
    // the message that started the drag has already been dispatched. See `seed_drag_input_queue`.
    seed_drag_input_queue(window);
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
        let helper: IDragSourceHelper2 =
            unsafe { CoCreateInstance(&CLSID_DragDropHelper, None, CLSCTX_INPROC_SERVER) }.expect("create drag-drop helper");
        unsafe { helper.SetFlags(DSH_ALLOWDROPDESCRIPTIONTEXT.0.cast_unsigned()) }.expect("SetFlags");
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
