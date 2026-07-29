#[cfg(target_os = "linux")]
mod sample_linux;

#[cfg(target_os = "linux")]
mod sample_linux_actions;

#[cfg(target_os = "linux")]
mod sample_linux_draw;

#[cfg(all(target_os = "linux", feature = "skia"))]
mod sample_linux_draw_skia;

#[cfg(target_os = "linux")]
mod sample_linux_draw_software;

#[cfg(target_os = "linux")]
pub fn main() {
    sample_linux::main();
}

#[cfg(not(target_os = "linux"))]
pub const fn main() {}
