use windows::Win32::{
    Foundation::{LPARAM, WPARAM},
    UI::WindowsAndMessaging::{
        WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDBLCLK, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_NCLBUTTONDBLCLK,
        WM_NCLBUTTONDOWN, WM_NCLBUTTONUP, WM_NCMBUTTONDBLCLK, WM_NCMBUTTONDOWN, WM_NCMBUTTONUP, WM_NCRBUTTONDBLCLK, WM_NCRBUTTONDOWN,
        WM_NCRBUTTONUP, WM_NCXBUTTONDBLCLK, WM_NCXBUTTONDOWN, WM_NCXBUTTONUP, WM_RBUTTONDBLCLK, WM_RBUTTONDOWN, WM_RBUTTONUP,
        WM_XBUTTONDBLCLK, WM_XBUTTONDOWN, WM_XBUTTONUP, XBUTTON1, XBUTTON2,
    },
};

use super::{
    geometry::{LogicalPoint, PhysicalPoint},
    utils::{GET_X_LPARAM, GET_Y_LPARAM, HIWORD, LOWORD},
};

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct MouseKeyState(u16);

impl MouseKeyState {
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::double_parens)]
    #[must_use]
    pub const fn get(wparam: WPARAM) -> Self {
        Self(LOWORD!(wparam.0))
    }
}

#[repr(C)]
#[derive(Debug)]
pub enum MouseButton {
    None,
    Left,
    Right,
    Middle,
    XButton1,
    XButton2,
}

impl MouseButton {
    #[allow(clippy::cast_possible_truncation)]
    #[must_use]
    pub const fn from_message(msg: u32, wparam: WPARAM) -> Self {
        match msg {
            WM_LBUTTONDOWN | WM_LBUTTONUP | WM_LBUTTONDBLCLK | WM_NCLBUTTONDOWN | WM_NCLBUTTONUP | WM_NCLBUTTONDBLCLK => Self::Left,
            WM_RBUTTONDOWN | WM_RBUTTONUP | WM_RBUTTONDBLCLK | WM_NCRBUTTONDOWN | WM_NCRBUTTONUP | WM_NCRBUTTONDBLCLK => Self::Right,
            WM_MBUTTONDOWN | WM_MBUTTONUP | WM_MBUTTONDBLCLK | WM_NCMBUTTONDOWN | WM_NCMBUTTONUP | WM_NCMBUTTONDBLCLK => Self::Middle,
            WM_XBUTTONDOWN | WM_XBUTTONUP | WM_XBUTTONDBLCLK | WM_NCXBUTTONDOWN | WM_NCXBUTTONUP | WM_NCXBUTTONDBLCLK => {
                match HIWORD!(wparam.0) {
                    XBUTTON1 => Self::XButton1,
                    XBUTTON2 => Self::XButton2,
                    _ => Self::None,
                }
            }
            _ => Self::None,
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_sign_loss)]
#[inline]
pub(crate) const fn get_mouse_position(lparam: LPARAM, scale: f32) -> LogicalPoint {
    let x_pos = GET_X_LPARAM!(lparam.0);
    let y_pos = GET_Y_LPARAM!(lparam.0);
    PhysicalPoint::new(x_pos, y_pos).to_logical(scale)
}
