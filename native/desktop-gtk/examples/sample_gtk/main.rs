#[cfg(all(target_os = "linux", not(feature = "skia")))]
mod sample_gtk_draw;
#[cfg(all(target_os = "linux", feature = "skia"))]
mod sample_gtk_draw_skia;

#[cfg(target_os = "linux")]
mod sample_gtk_impl;

#[cfg(target_os = "linux")]
mod sample_gtk_actions;

#[cfg(target_os = "linux")]
pub fn main() {
    sample_gtk_impl::main_impl();
}

#[cfg(not(target_os = "linux"))]
pub const fn main() {}
