#[cfg(feature = "enabled")]
mod sample_gtk_draw;

#[cfg(feature = "enabled")]
mod sample_gtk_impl;

#[cfg(feature = "enabled")]
pub fn main() {
    sample_gtk_impl::main_impl();
}

#[cfg(not(feature = "enabled"))]
pub fn main() -> Result<(), anyhow::Error> {
    anyhow::bail!(
        "Trying to run GTK example without enabling it. Try running `cargo run --features desktop-gtk/enabled --example sample_gtk`"
    )
}
