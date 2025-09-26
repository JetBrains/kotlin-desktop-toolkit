use windows::Win32::{
    Foundation::WPARAM,
    Graphics::Gdi::MapWindowPoints,
    UI::{
        HiDpi::GetDpiForWindow,
        Input::Pointer::{
            GetPointerInfo, GetPointerPenInfo, GetPointerTouchInfo, GetPointerType, POINTER_FLAG_FIFTHBUTTON, POINTER_FLAG_FIRSTBUTTON,
            POINTER_FLAG_FOURTHBUTTON, POINTER_FLAG_SECONDBUTTON, POINTER_FLAG_THIRDBUTTON, POINTER_INFO, POINTER_PEN_INFO,
            POINTER_TOUCH_INFO,
        },
        WindowsAndMessaging::{
            POINTER_INPUT_TYPE, POINTER_MESSAGE_FLAG_FIFTHBUTTON, POINTER_MESSAGE_FLAG_FIRSTBUTTON, POINTER_MESSAGE_FLAG_FOURTHBUTTON,
            POINTER_MESSAGE_FLAG_SECONDBUTTON, POINTER_MESSAGE_FLAG_THIRDBUTTON, PT_PEN, PT_TOUCH, USER_DEFAULT_SCREEN_DPI,
        },
    },
};

use super::{
    events::Timestamp,
    geometry::LogicalPoint,
    utils::{HIWORD, LOWORD},
};

pub(crate) enum PointerInfo {
    Touch(POINTER_TOUCH_INFO),
    Pen(POINTER_PEN_INFO),
    Common(POINTER_INFO),
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PointerState {
    pressed_buttons: PointerButtons,
    modifiers: PointerModifiers,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PointerButtons(u32);

/// cbindgen:ignore
const POINTER_BUTTON_LEFT: u32 = 1 << 0;
/// cbindgen:ignore
const POINTER_BUTTON_RIGHT: u32 = 1 << 1;
/// cbindgen:ignore
const POINTER_BUTTON_MIDDLE: u32 = 1 << 2;
/// cbindgen:ignore
const POINTER_BUTTON_XBUTTON1: u32 = 1 << 3;
/// cbindgen:ignore
const POINTER_BUTTON_XBUTTON2: u32 = 1 << 4;

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PointerModifiers(u32);

impl PointerInfo {
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::double_parens)]
    pub(crate) fn try_from_message(wparam: WPARAM) -> windows::core::Result<Self> {
        let pointer_id = u32::from(LOWORD!(wparam.0));

        let pointer_type = unsafe {
            let mut pointer_type = POINTER_INPUT_TYPE::default();
            GetPointerType(pointer_id, &raw mut pointer_type)
                .inspect_err(|err| log::error!("failed to get pointer type for {pointer_id}: {err}"))
                .map(|()| pointer_type)?
        };

        match pointer_type {
            PT_TOUCH => unsafe {
                let mut touch_info = POINTER_TOUCH_INFO::default();
                GetPointerTouchInfo(pointer_id, &raw mut touch_info)
                    .inspect_err(|err| log::error!("failed to get pointer touch info for {pointer_id}: {err}"))
                    .map(|()| Self::Touch(touch_info))
            },
            PT_PEN => unsafe {
                let mut pen_info = POINTER_PEN_INFO::default();
                GetPointerPenInfo(pointer_id, &raw mut pen_info)
                    .inspect_err(|err| log::error!("failed to get pointer pen info for {pointer_id}: {err}"))
                    .map(|()| Self::Pen(pen_info))
            },
            _ => unsafe {
                let mut pointer_info = POINTER_INFO::default();
                GetPointerInfo(pointer_id, &raw mut pointer_info)
                    .inspect_err(|err| log::error!("failed to get pointer info for {pointer_id}: {err}"))
                    .map(|()| Self::Common(pointer_info))
            },
        }
    }

    const fn get_native_pointer_info(&self) -> &POINTER_INFO {
        match self {
            Self::Touch(touch_info) => &touch_info.pointerInfo,
            Self::Pen(pen_info) => &pen_info.pointerInfo,
            Self::Common(pointer_info) => pointer_info,
        }
    }

    pub(crate) fn get_pointer_state(&self) -> PointerState {
        let native_pointer_info = self.get_native_pointer_info();
        let pointer_flags = native_pointer_info.pointerFlags;
        let pressed_buttons = {
            let mut buttons = 0_u32;
            if (pointer_flags & POINTER_FLAG_FIRSTBUTTON) == POINTER_FLAG_FIRSTBUTTON {
                buttons |= POINTER_BUTTON_LEFT;
            }
            if (pointer_flags & POINTER_FLAG_SECONDBUTTON) == POINTER_FLAG_SECONDBUTTON {
                buttons |= POINTER_BUTTON_RIGHT;
            }
            if (pointer_flags & POINTER_FLAG_THIRDBUTTON) == POINTER_FLAG_THIRDBUTTON {
                buttons |= POINTER_BUTTON_MIDDLE;
            }
            if (pointer_flags & POINTER_FLAG_FOURTHBUTTON) == POINTER_FLAG_FOURTHBUTTON {
                buttons |= POINTER_BUTTON_XBUTTON1;
            }
            if (pointer_flags & POINTER_FLAG_FIFTHBUTTON) == POINTER_FLAG_FIFTHBUTTON {
                buttons |= POINTER_BUTTON_XBUTTON2;
            }
            PointerButtons(buttons)
        };
        PointerState {
            pressed_buttons,
            modifiers: unsafe { core::mem::transmute::<u32, PointerModifiers>(native_pointer_info.dwKeyStates) },
        }
    }

    pub(crate) fn get_timestamp(&self) -> Timestamp {
        let native_pointer_info = self.get_native_pointer_info();
        Timestamp(u64::from(native_pointer_info.dwTime) * 1000)
    }

    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn get_location_in_window(&self) -> LogicalPoint {
        let native_pointer_info = self.get_native_pointer_info();
        let window_dpi = unsafe { GetDpiForWindow(native_pointer_info.hwndTarget) };
        let mut points = [native_pointer_info.ptPixelLocation];
        unsafe { MapWindowPoints(None, Some(native_pointer_info.hwndTarget), &mut points) };
        let x = ((points[0].x * USER_DEFAULT_SCREEN_DPI as i32) as f32) / (window_dpi as f32);
        let y = ((points[0].y * USER_DEFAULT_SCREEN_DPI as i32) as f32) / (window_dpi as f32);
        LogicalPoint::new(x, y)
    }
}

impl PointerButtons {
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::double_parens)]
    pub(crate) fn from_message_flags(value: WPARAM) -> Self {
        let flags = u32::from(HIWORD!(value.0));
        if (flags & POINTER_MESSAGE_FLAG_FIRSTBUTTON) == POINTER_MESSAGE_FLAG_FIRSTBUTTON {
            Self(POINTER_BUTTON_LEFT)
        } else if (flags & POINTER_MESSAGE_FLAG_SECONDBUTTON) == POINTER_MESSAGE_FLAG_SECONDBUTTON {
            Self(POINTER_BUTTON_RIGHT)
        } else if (flags & POINTER_MESSAGE_FLAG_THIRDBUTTON) == POINTER_MESSAGE_FLAG_THIRDBUTTON {
            Self(POINTER_BUTTON_MIDDLE)
        } else if (flags & POINTER_MESSAGE_FLAG_FOURTHBUTTON) == POINTER_MESSAGE_FLAG_FOURTHBUTTON {
            Self(POINTER_BUTTON_XBUTTON1)
        } else if (flags & POINTER_MESSAGE_FLAG_FIFTHBUTTON) == POINTER_MESSAGE_FLAG_FIFTHBUTTON {
            Self(POINTER_BUTTON_XBUTTON2)
        } else {
            Self(0_u32)
        }
    }
}
