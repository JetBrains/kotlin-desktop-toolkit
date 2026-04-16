use desktop_common::{ffi_utils::BorrowedStrPtr, logger::ffi_boundary};

use super::{data_transfer, strings::copy_from_utf8_string};

#[unsafe(no_mangle)]
pub extern "C" fn data_transfer_register_format(name: BorrowedStrPtr) -> u32 {
    ffi_boundary("data_transfer_register_format", || {
        let format_name = copy_from_utf8_string(&name)?;
        Ok(data_transfer::register_format(&format_name))
    })
}
