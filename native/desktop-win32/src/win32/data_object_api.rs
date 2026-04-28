use desktop_common::{
    ffi_utils::{AutoDropArray, BorrowedArray, BorrowedStrPtr, FfiOption, RustAllocatedStrPtr},
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
    data_object::{DataObject, enum_data_object_format_ids, is_data_object_format_available},
    data_reader::DataReader,
    data_transfer::DataFormat,
    global_data::hglobal_writer,
    strings::copy_from_utf8_string,
};

pub(crate) type AutoDropByteArray = AutoDropArray<u8>;
pub(crate) type AutoDropUInt32Array = AutoDropArray<u32>;

pub(crate) trait IntoFfiOption<T> {
    fn into_ffi_option(self) -> anyhow::Result<FfiOption<T>>
    where
        T: desktop_common::logger::PanicDefault;
}

impl<T> IntoFfiOption<T> for anyhow::Result<T> {
    fn into_ffi_option(self) -> anyhow::Result<FfiOption<T>>
    where
        T: desktop_common::logger::PanicDefault,
    {
        match self {
            Ok(ok) => Ok(FfiOption::some(ok)),
            Err(err) => {
                log::trace!("failed to read data: {err}");
                Ok(FfiOption::none())
            }
        }
    }
}

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
    ffi_boundary("data_object_add_from_bytes", || {
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
pub extern "C" fn data_object_into_com(data_object_id: i64) -> ComInterfaceRawPtr {
    ffi_boundary("data_object_into_com", || {
        let registry = DATA_OBJECT_REGISTRY.pin();
        let data_object = registry.remove(&data_object_id).context("unknown data object id")?;
        Ok(ComInterfaceRawPtr::from_object(data_object)?)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn data_object_drop(data_object_id: i64) {
    ffi_boundary("data_object_drop", || {
        DATA_OBJECT_REGISTRY.pin().remove(&data_object_id);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn com_data_object_is_format_available(data_object_ptr: ComInterfaceRawPtr, data_format: u32) -> bool {
    ffi_boundary("com_data_object_is_format_available", || {
        let data_object = data_object_ptr.borrow::<IDataObject>()?;
        is_data_object_format_available(&data_object, DataFormat::Other(data_format))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn com_data_object_enum_formats(data_object_ptr: ComInterfaceRawPtr) -> AutoDropUInt32Array {
    ffi_boundary("com_data_object_enum_formats", || {
        let data_object = data_object_ptr.borrow::<IDataObject>()?;
        let formats = enum_data_object_format_ids(&data_object)?;
        Ok(AutoDropArray::new(formats))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn com_data_object_read_bytes(data_object_ptr: ComInterfaceRawPtr, data_format: u32) -> AutoDropByteArray {
    ffi_boundary("com_data_object_read_bytes", || {
        com_data_object_read_bytes_impl(&data_object_ptr, data_format)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn com_data_object_try_read_bytes(data_object_ptr: ComInterfaceRawPtr, data_format: u32) -> FfiOption<AutoDropByteArray> {
    ffi_boundary("com_data_object_try_read_bytes", || {
        com_data_object_read_bytes_impl(&data_object_ptr, data_format).into_ffi_option()
    })
}

fn com_data_object_read_bytes_impl(data_object_ptr: &ComInterfaceRawPtr, data_format: u32) -> anyhow::Result<AutoDropByteArray> {
    let data_object = data_object_ptr.borrow::<IDataObject>()?;
    DataReader::create(&data_object, DataFormat::Other(data_format))
        .and_then(|reader| reader.get_bytes())
        .map(|bytes| AutoDropArray::new(bytes.into_boxed_slice()))
}

#[unsafe(no_mangle)]
pub extern "C" fn com_data_object_read_file_list(data_object_ptr: ComInterfaceRawPtr) -> AutoDropArray<RustAllocatedStrPtr> {
    ffi_boundary("com_data_object_read_file_list", || {
        com_data_object_read_file_list_impl(&data_object_ptr)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn com_data_object_try_read_file_list(data_object_ptr: ComInterfaceRawPtr) -> FfiOption<AutoDropArray<RustAllocatedStrPtr>> {
    ffi_boundary("com_data_object_try_read_file_list", || {
        com_data_object_read_file_list_impl(&data_object_ptr).into_ffi_option()
    })
}

fn com_data_object_read_file_list_impl(data_object_ptr: &ComInterfaceRawPtr) -> anyhow::Result<AutoDropArray<RustAllocatedStrPtr>> {
    let data_object = data_object_ptr.borrow::<IDataObject>()?;
    DataReader::create(&data_object, DataFormat::FileList)
        .and_then(|reader| reader.get_file_list())
        .map(|file_list| file_list.into_iter().map(RustAllocatedStrPtr::from_c_string).collect())
        .map(AutoDropArray::new)
}

#[unsafe(no_mangle)]
pub extern "C" fn com_data_object_read_html_fragment(data_object_ptr: ComInterfaceRawPtr) -> RustAllocatedStrPtr {
    ffi_boundary("com_data_object_read_html_fragment", || {
        com_data_object_read_html_fragment_impl(&data_object_ptr)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn com_data_object_try_read_html_fragment(data_object_ptr: ComInterfaceRawPtr) -> FfiOption<RustAllocatedStrPtr> {
    ffi_boundary("com_data_object_try_read_html_fragment", || {
        com_data_object_read_html_fragment_impl(&data_object_ptr).into_ffi_option()
    })
}

fn com_data_object_read_html_fragment_impl(data_object_ptr: &ComInterfaceRawPtr) -> anyhow::Result<RustAllocatedStrPtr> {
    let data_object = data_object_ptr.borrow::<IDataObject>()?;
    DataReader::create(&data_object, DataFormat::HtmlFragment)
        .and_then(|reader| reader.get_html())
        .map(RustAllocatedStrPtr::from_c_string)
}

#[unsafe(no_mangle)]
pub extern "C" fn com_data_object_read_text(data_object_ptr: ComInterfaceRawPtr) -> RustAllocatedStrPtr {
    ffi_boundary("com_data_object_read_text", || com_data_object_read_text_impl(&data_object_ptr))
}

#[unsafe(no_mangle)]
pub extern "C" fn com_data_object_try_read_text(data_object_ptr: ComInterfaceRawPtr) -> FfiOption<RustAllocatedStrPtr> {
    ffi_boundary("com_data_object_try_read_text", || {
        com_data_object_read_text_impl(&data_object_ptr).into_ffi_option()
    })
}

fn com_data_object_read_text_impl(data_object_ptr: &ComInterfaceRawPtr) -> anyhow::Result<RustAllocatedStrPtr> {
    let data_object = data_object_ptr.borrow::<IDataObject>()?;
    DataReader::create(&data_object, DataFormat::Text)
        .and_then(|reader| reader.get_text())
        .map(RustAllocatedStrPtr::from_c_string)
}

#[unsafe(no_mangle)]
pub extern "C" fn com_data_object_release(data_object_ptr: ComInterfaceRawPtr) {
    ffi_boundary("com_data_object_release", || {
        drop(data_object_ptr);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn native_u32_array_drop(array: AutoDropUInt32Array) {
    ffi_boundary("native_u32_array_drop", || {
        drop(array);
        Ok(())
    });
}
