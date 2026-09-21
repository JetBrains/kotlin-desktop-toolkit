#[cfg(target_os = "linux")]
mod wayland_virtual_devices_lib;

#[cfg(target_os = "linux")]
mod wayland_virtual_devices;

#[cfg(target_os = "linux")]
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    wayland_virtual_devices::main()
}

#[cfg(not(target_os = "linux"))]
pub const fn main() {}
