use desktop_common::{
    ffi_utils::{AutoDropArray, BorrowedArray, BorrowedStrPtr, RustAllocatedStrPtr},
    logger::ffi_boundary,
};

use windows::Win32::System::Com::IDataObject;

use super::{
    com::ComInterfaceRawPtr,
    data_object::{DataObject, get_hglobal_from_data_object, is_data_object_format_available},
    data_transfer::DataFormat,
    global_data::{hglobal_reader, hglobal_writer},
    strings::copy_from_utf8_string,
};

#[unsafe(no_mangle)]
pub extern "C" fn data_object_is_format_available(data_object_ptr: ComInterfaceRawPtr, data_format: u32) -> bool {
    ffi_boundary("data_object_is_format_available", || {
        let data_object = data_object_ptr.borrow::<IDataObject>()?;
        is_data_object_format_available(&data_object, DataFormat::Other(data_format))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn data_object_create_from_bytes(data_format: u32, content: BorrowedArray<u8>) -> ComInterfaceRawPtr {
    ffi_boundary("data_object_create_from_bytes", || {
        let data = hglobal_writer::new_bytes(content.as_slice()?)?;
        let data_object: IDataObject = DataObject::create(DataFormat::Other(data_format), data).into();
        Ok(ComInterfaceRawPtr::new(&data_object)?)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn data_object_create_from_file_list(content: BorrowedArray<BorrowedStrPtr>) -> ComInterfaceRawPtr {
    ffi_boundary("data_object_create_from_file_list", || {
        let files: anyhow::Result<Vec<&str>> = content.as_slice()?.iter().map(|str_ptr| str_ptr.as_str()).collect();
        let data = hglobal_writer::new_file_list(&files?)?;
        let data_object: IDataObject = DataObject::create(DataFormat::FileList, data).into();
        Ok(ComInterfaceRawPtr::new(&data_object)?)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn data_object_create_from_html_fragment(content: BorrowedStrPtr) -> ComInterfaceRawPtr {
    ffi_boundary("data_object_create_from_html_fragment", || {
        let fragment = copy_from_utf8_string(&content)?;
        let data = hglobal_writer::new_html(&fragment)?;
        let data_object: IDataObject = DataObject::create(DataFormat::HtmlFragment, data).into();
        Ok(ComInterfaceRawPtr::new(&data_object)?)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn data_object_create_from_text(content: BorrowedStrPtr) -> ComInterfaceRawPtr {
    ffi_boundary("data_object_create_from_text", || {
        let data = hglobal_writer::new_text(content.as_str()?)?;
        let data_object: IDataObject = DataObject::create(DataFormat::Text, data).into();
        Ok(ComInterfaceRawPtr::new(&data_object)?)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn data_object_read_bytes(data_object_ptr: ComInterfaceRawPtr, data_format: u32) -> AutoDropArray<u8> {
    ffi_boundary("data_object_read_bytes", || {
        let data_object = data_object_ptr.borrow::<IDataObject>()?;
        get_hglobal_from_data_object(&data_object, DataFormat::Other(data_format))
            .and_then(|data| hglobal_reader::get_bytes(&data))
            .map(|bytes| AutoDropArray::new(bytes.into_boxed_slice()))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn data_object_read_file_list(data_object_ptr: ComInterfaceRawPtr) -> AutoDropArray<RustAllocatedStrPtr> {
    ffi_boundary("data_object_read_file_list", || {
        let data_object = data_object_ptr.borrow::<IDataObject>()?;
        get_hglobal_from_data_object(&data_object, DataFormat::FileList)
            .and_then(|data| hglobal_reader::get_file_list(&data))
            .map(|file_list| file_list.into_iter().map(RustAllocatedStrPtr::from_c_string).collect())
            .map(AutoDropArray::new)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn data_object_read_html_fragment(data_object_ptr: ComInterfaceRawPtr) -> RustAllocatedStrPtr {
    ffi_boundary("data_object_read_html_fragment", || {
        let data_object = data_object_ptr.borrow::<IDataObject>()?;
        get_hglobal_from_data_object(&data_object, DataFormat::HtmlFragment)
            .and_then(|data| hglobal_reader::get_html(&data))
            .map(RustAllocatedStrPtr::from_c_string)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn data_object_read_text(data_object_ptr: ComInterfaceRawPtr) -> RustAllocatedStrPtr {
    ffi_boundary("data_object_read_text", || {
        let data_object = data_object_ptr.borrow::<IDataObject>()?;
        get_hglobal_from_data_object(&data_object, DataFormat::Text)
            .and_then(|data| hglobal_reader::get_text(&data))
            .map(RustAllocatedStrPtr::from_c_string)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn data_object_release(data_object_ptr: ComInterfaceRawPtr) {
    ffi_boundary("data_object_release", || {
        drop(data_object_ptr);
        Ok(())
    });
}
