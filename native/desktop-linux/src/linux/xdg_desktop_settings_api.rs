use desktop_common::ffi_utils::AutoDropArray;
use log::warn;

use super::xdg_desktop_settings::InternalXdgDesktopSetting;

#[repr(i32)]
#[derive(Copy, Clone, Debug)]
pub enum WindowButtonType {
    AppMenu,
    Icon,
    Spacer,
    Minimize,
    Maximize,
    Close,
}

impl WindowButtonType {
    pub(crate) fn parse(button_name: &str) -> Option<Self> {
        match button_name {
            "appmenu" => Some(Self::AppMenu),
            "icon" => Some(Self::Icon),
            "spacer" => Some(Self::Spacer),
            "minimize" => Some(Self::Minimize),
            "maximize" => Some(Self::Maximize),
            "close" => Some(Self::Close),
            _ => {
                warn!("Unknown button name {button_name}");
                None
            }
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct TitlebarButtonLayout {
    pub left_side: AutoDropArray<WindowButtonType>,
    pub right_side: AutoDropArray<WindowButtonType>,
}

#[repr(C)]
#[derive(Debug)]
pub enum XdgDesktopSetting {
    TitlebarLayout(TitlebarButtonLayout),
    DoubleClickIntervalMs(i32),
}

impl XdgDesktopSetting {
    pub(crate) fn with(s: InternalXdgDesktopSetting, f: impl FnOnce(Self)) {
        match s {
            InternalXdgDesktopSetting::TitlebarLayout(v) => {
                f(Self::TitlebarLayout(TitlebarButtonLayout {
                    left_side: AutoDropArray::new(v.left_side),
                    right_side: AutoDropArray::new(v.right_side),
                }));
            }
            InternalXdgDesktopSetting::DoubleClickIntervalMs(v) => f(Self::DoubleClickIntervalMs(v)),
        }
    }
}
