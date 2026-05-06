#[cfg(target_os = "linux")]
mod test_app_data_source_impl;

#[cfg(target_os = "linux")]
pub fn main() {
    test_app_data_source_impl::main_impl();
}

#[cfg(not(target_os = "linux"))]
pub const fn main() {}
