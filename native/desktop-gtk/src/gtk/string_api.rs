use desktop_common::{ffi_utils::RustAllocatedStrPtr, logger::ffi_boundary};

#[unsafe(no_mangle)]
pub extern "C" fn string_drop(mut str_ptr: RustAllocatedStrPtr) {
    ffi_boundary("string_drop", || {
        str_ptr.deallocate();
        Ok(())
    });
}
