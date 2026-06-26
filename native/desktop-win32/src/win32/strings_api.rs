use desktop_common::{
    ffi_utils::{AutoDropArray, RustAllocatedStrPtr},
    logger::ffi_boundary,
};

#[unsafe(no_mangle)]
pub extern "C" fn native_string_drop(mut str_ptr: RustAllocatedStrPtr) {
    ffi_boundary("native_string_drop", || {
        str_ptr.deallocate();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn native_string_array_drop(str_array: AutoDropArray<RustAllocatedStrPtr>) {
    ffi_boundary("native_string_array_drop", || {
        drop(str_array);
        Ok(())
    });
}
