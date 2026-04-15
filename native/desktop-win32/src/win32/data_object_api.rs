use desktop_common::{ffi_utils::AutoDropArray, logger::ffi_boundary};

use windows::Win32::System::Com::IDataObject;

use super::{com::ComInterfaceRawPtr, data_object::data_object_reader};

#[unsafe(no_mangle)]
pub extern "C" fn data_object_read_bytes(data_object_ptr: ComInterfaceRawPtr, data_format: u32) -> AutoDropArray<u8> {
    ffi_boundary("data_object_read_bytes", || {
        let data_object = data_object_ptr.borrow::<IDataObject>()?;
        let bytes = data_object_reader::read_bytes(&data_object, data_format)?;
        Ok(AutoDropArray::new(bytes.into_boxed_slice()))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn data_object_release(data_object_ptr: ComInterfaceRawPtr) {
    ffi_boundary("data_object_release", || {
        drop(data_object_ptr);
        Ok(())
    });
}
