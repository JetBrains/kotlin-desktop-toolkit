use desktop_common::ffi_utils::AutoDropArray;
use desktop_common::logger::ffi_boundary;

#[unsafe(no_mangle)]
pub extern "C" fn rust_allocated_u8_array_drop(array: AutoDropArray<u8>) {
    ffi_boundary("rust_allocated_u8_array_drop", || {
        drop(array);
        Ok(())
    });
}
