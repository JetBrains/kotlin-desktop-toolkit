#[cfg(target_os = "linux")]
mod linux_test_helper_http;

#[cfg(target_os = "linux")]
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    linux_test_helper_http::main()
}

#[cfg(not(target_os = "linux"))]
pub const fn main() {}
