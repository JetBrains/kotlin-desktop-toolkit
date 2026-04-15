use crate::gtk::application_api::{
    FfiObjDealloc, FfiQueryDragAndDropTarget, FfiRetrieveSurroundingText, FfiSupportedActionsForMime, FfiTransferDataGetter,
};
use crate::gtk::data_transfer_api::{DataSource, DragAndDropQueryData};
use crate::gtk::events::WindowId;
use crate::gtk::text_input::SurroundingTextWithSelection;
use desktop_common::ffi_utils::BorrowedUtf8;

#[derive(Clone, Copy)]
pub struct TransferDataGetter {
    pub ffi_get: FfiTransferDataGetter,
    pub ffi_dealloc: FfiObjDealloc,
}

impl TransferDataGetter {
    pub fn get(&self, clipboard_type: DataSource, mime_type: &str) -> Option<Vec<u8>> {
        let ffi_response = (self.ffi_get)(clipboard_type, BorrowedUtf8::new(mime_type));
        let ret = ffi_response.data.as_optional_slice().map(Into::into);
        (self.ffi_dealloc)(ffi_response.obj_id);
        ret
    }
}

#[derive(Clone, Copy)]
pub struct QueryDragAndDropTarget {
    pub ffi_get: FfiQueryDragAndDropTarget,
    pub ffi_dealloc: FfiObjDealloc,
}

impl QueryDragAndDropTarget {
    pub fn with<T>(&self, query: &DragAndDropQueryData, f: impl FnOnce(&[FfiSupportedActionsForMime<'static>]) -> T) -> T {
        let ffi_response = (self.ffi_get)(query);
        let callback_arg = ffi_response.supported_actions_per_mime.as_slice().unwrap();
        let ret = f(callback_arg);

        (self.ffi_dealloc)(ffi_response.obj_id);

        ret
    }
}

#[derive(Clone, Copy)]
pub struct RetrieveSurroundingText {
    pub ffi_get: FfiRetrieveSurroundingText,
    pub ffi_dealloc: FfiObjDealloc,
}

impl RetrieveSurroundingText {
    pub fn with<T>(&self, window_id: WindowId, f: impl FnOnce(anyhow::Result<SurroundingTextWithSelection>) -> T) -> T {
        let ffi_response = (self.ffi_get)(window_id);
        let converted = SurroundingTextWithSelection::new(&ffi_response);
        let ret = f(converted);

        (self.ffi_dealloc)(ffi_response.obj_id);

        ret
    }
}
