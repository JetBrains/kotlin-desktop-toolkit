use desktop_common::{
    ffi_utils::{AutoDropArray, FfiOption, RustAllocatedStrPtr},
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
pub extern "C" fn native_optional_string_drop(mut optional: FfiOption<RustAllocatedStrPtr>) {
    ffi_boundary("native_optional_string_drop", || {
        if let Some(mut str) = optional.try_take() {
            str.deallocate();
        }
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
