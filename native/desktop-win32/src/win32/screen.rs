use anyhow::Context;
use desktop_common::ffi_utils::{AutoDropStrPtr, RustAllocatedStrPtr};

use windows::{
    Win32::{
        Graphics::Gdi::{
            DEVMODEW, DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICEW, ENUM_CURRENT_SETTINGS, EnumDisplayDevicesW, EnumDisplayMonitors,
            EnumDisplaySettingsW, GetMonitorInfoW, HMONITOR, MONITORINFO, MONITORINFOEXW,
        },
        UI::{
            HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
            WindowsAndMessaging::USER_DEFAULT_SCREEN_DPI,
        },
    },
    core::HSTRING,
};

use super::geometry::{LogicalPoint, LogicalSize, PhysicalPoint, PhysicalSize};

#[repr(C)]
#[derive(Debug)]
pub struct ScreenInfo {
    pub is_primary: bool,
    pub name: AutoDropStrPtr,
    // relative to primary screen
    pub origin: LogicalPoint,
    pub size: LogicalSize,
    pub scale: f32,
    pub maximum_frames_per_second: u32,
    // todo color space?
    // todo stable uuid?
}

pub(crate) fn enumerate_screens() -> anyhow::Result<Box<[ScreenInfo]>> {
    let screens = Box::into_raw(Box::new(Vec::<ScreenInfo>::new()));
    let enum_result = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(monitor_enum_proc),
            windows::Win32::Foundation::LPARAM(screens as isize),
        )
    };
    let screens = unsafe { Box::from_raw(screens) };
    if !enum_result.as_bool() {
        anyhow::bail!("failed to enumerate screens");
    }
    Ok(screens.into_boxed_slice())
}

unsafe extern "system" fn monitor_enum_proc(
    hmonitor: windows::Win32::Graphics::Gdi::HMONITOR,
    _hdc: windows::Win32::Graphics::Gdi::HDC,
    _lprc: *mut windows::Win32::Foundation::RECT,
    dwdata: windows::Win32::Foundation::LPARAM,
) -> windows::core::BOOL {
    let screens = Box::leak(unsafe { Box::from_raw(dwdata.0 as *mut Vec<ScreenInfo>) });
    let screen_info = match get_screen_info(hmonitor) {
        Ok(screen_info) => screen_info,
        Err(err) => {
            log::error!("failed to get screen info: {err:?}");
            return windows::Win32::Foundation::FALSE;
        }
    };
    screens.push(screen_info);
    windows::Win32::Foundation::TRUE
}

#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_precision_loss)]
pub(crate) fn get_screen_info(hmonitor: HMONITOR) -> anyhow::Result<ScreenInfo> {
    let mut monitor_info = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: size_of::<MONITORINFOEXW>() as _,
            ..Default::default()
        },
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(hmonitor, (&raw mut monitor_info).cast()).as_bool() } {
        anyhow::bail!("failed to get monitor info");
    }
    let device_name = HSTRING::from_wide(&monitor_info.szDevice);
    let mut display_device = DISPLAY_DEVICEW {
        cb: size_of::<DISPLAY_DEVICEW>() as _,
        ..Default::default()
    };
    if !unsafe { EnumDisplayDevicesW(&device_name, 0, &raw mut display_device, 0).as_bool() } {
        anyhow::bail!("failed to get display device info");
    }
    if (display_device.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP) != DISPLAY_DEVICE_ATTACHED_TO_DESKTOP {
        anyhow::bail!("display device is not attached to the desktop");
    }
    let mut device_mode = DEVMODEW {
        dmSize: size_of::<DEVMODEW>() as u16,
        dmDriverExtra: 0,
        ..Default::default()
    };
    if !unsafe { EnumDisplaySettingsW(&device_name, ENUM_CURRENT_SETTINGS, &raw mut device_mode).as_bool() } {
        anyhow::bail!("failed to enum display's current settings");
    }
    let device_name = super::strings::copy_from_wide_string(&device_name).context("failed to copy the device name into a string")?;
    let device_position = unsafe { device_mode.Anonymous1.Anonymous2.dmPosition };
    let (mut dpi_x, mut dpi_y) = (USER_DEFAULT_SCREEN_DPI, USER_DEFAULT_SCREEN_DPI);
    unsafe { GetDpiForMonitor(hmonitor, MDT_EFFECTIVE_DPI, &raw mut dpi_x, &raw mut dpi_y).context("failed to get DPI for the monitor")? };
    let scale = (dpi_x as f32) / (USER_DEFAULT_SCREEN_DPI as f32);
    let screen_info = ScreenInfo {
        is_primary: (device_position.x == 0 && device_position.y == 0),
        name: RustAllocatedStrPtr::from_c_string(device_name).to_auto_drop(),
        origin: PhysicalPoint::new(device_position.x, device_position.y).to_logical(scale),
        size: PhysicalSize::new(device_mode.dmPelsWidth as _, device_mode.dmPelsHeight as _).to_logical(scale),
        scale,
        maximum_frames_per_second: device_mode.dmDisplayFrequency,
    };
    Ok(screen_info)
}
