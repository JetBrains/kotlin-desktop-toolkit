use desktop_common::{
    ffi_utils::{AutoDropArray, BorrowedArray, BorrowedStrPtr, RustAllocatedStrPtr},
    logger::ffi_boundary,
};
use std::sync::{
    LazyLock,
    atomic::{AtomicI64, Ordering},
};

use anyhow::Context;
use windows::Win32::System::Com::IDataObject;
use windows_core::ComObject;

use super::{
    com::ComInterfaceRawPtr,
    data_object::{DataObject, get_hglobal_from_data_object, is_data_object_format_available},
    data_transfer::DataFormat,
    global_data::{hglobal_reader, hglobal_writer},
    strings::copy_from_utf8_string,
};

/// cbindgen:ignore
static DATA_OBJECT_NEXT_ID: AtomicI64 = AtomicI64::new(0);
/// cbindgen:ignore
static DATA_OBJECT_REGISTRY: LazyLock<papaya::HashMap<i64, ComObject<DataObject>>> = LazyLock::new(papaya::HashMap::new);

#[inline]
fn with_data_object<R, F: FnOnce(&ComObject<DataObject>) -> R>(data_object_id: i64, func: F) -> anyhow::Result<R> {
    DATA_OBJECT_REGISTRY
        .pin()
        .get(&data_object_id)
        .context("unknown data object id")
        .map(func)
}

#[unsafe(no_mangle)]
pub extern "C" fn data_object_create() -> i64 {
    ffi_boundary("data_object_create", || {
        let data_object_id = DATA_OBJECT_NEXT_ID.fetch_add(1, Ordering::AcqRel);
        let data_object = ComObject::new(DataObject::new());
        DATA_OBJECT_REGISTRY
            .pin()
            .try_insert(data_object_id, data_object)
            .map_err(|_| anyhow::anyhow!("data object id {data_object_id} already in use"))?;
        Ok(data_object_id)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn data_object_add_from_bytes(data_object_id: i64, data_format: u32, content: BorrowedArray<u8>) -> bool {
    ffi_boundary("data_object_create_from_bytes", || {
        let data = hglobal_writer::new_bytes(content.as_slice()?)?;
        with_data_object(data_object_id, |data_object| {
            data_object.add_format(DataFormat::Other(data_format), data)
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn data_object_add_from_file_list(data_object_id: i64, content: BorrowedArray<BorrowedStrPtr>) -> bool {
    ffi_boundary("data_object_add_from_file_list", || {
        let files: anyhow::Result<Vec<&str>> = content.as_slice()?.iter().map(|str_ptr| str_ptr.as_str()).collect();
        let data = hglobal_writer::new_file_list(&files?)?;
        with_data_object(data_object_id, |data_object| data_object.add_format(DataFormat::FileList, data))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn data_object_add_from_html_fragment(data_object_id: i64, content: BorrowedStrPtr) -> bool {
    ffi_boundary("data_object_add_from_html_fragment", || {
        let fragment = copy_from_utf8_string(&content)?;
        let data = hglobal_writer::new_html(&fragment)?;
        with_data_object(data_object_id, |data_object| data_object.add_format(DataFormat::HtmlFragment, data))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn data_object_add_from_text(data_object_id: i64, content: BorrowedStrPtr) -> bool {
    ffi_boundary("data_object_add_from_text", || {
        let data = hglobal_writer::new_text(content.as_str()?)?;
        with_data_object(data_object_id, |data_object| data_object.add_format(DataFormat::Text, data))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn data_object_into_com_object(data_object_id: i64) -> ComInterfaceRawPtr {
    ffi_boundary("data_object_into_com_object", || {
        let registry = DATA_OBJECT_REGISTRY.pin();
        let data_object = registry.remove(&data_object_id).context("unknown data object id")?;
        Ok(ComInterfaceRawPtr::from_object(data_object)?)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn data_object_is_format_available(data_object_ptr: ComInterfaceRawPtr, data_format: u32) -> bool {
    ffi_boundary("data_object_is_format_available", || {
        let data_object = data_object_ptr.borrow::<IDataObject>()?;
        is_data_object_format_available(&data_object, DataFormat::Other(data_format))
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
