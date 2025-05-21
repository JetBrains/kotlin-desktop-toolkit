#[cfg(target_os = "linux")]
mod sample_linux;

#[cfg(target_os = "linux")]
mod gl_sys;

#[cfg(target_os = "linux")]
pub fn main() {
    sample_linux::main();
}

#[cfg(not(target_os = "linux"))]
pub const fn main() {}
