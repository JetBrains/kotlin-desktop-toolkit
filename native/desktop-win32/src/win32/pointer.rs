use windows::Win32::{
    Foundation::{POINT, RECT, WPARAM},
    Graphics::Gdi::{InflateRect, MapWindowPoints, PtInRect, SetRect},
    UI::{
        HiDpi::GetDpiForWindow,
        Input::{
            KeyboardAndMouse::GetDoubleClickTime,
            Pointer::{
                GetPointerInfo, GetPointerPenInfo, GetPointerTouchInfo, GetPointerType, POINTER_FLAG_FIFTHBUTTON, POINTER_FLAG_FIRSTBUTTON,
                POINTER_FLAG_FOURTHBUTTON, POINTER_FLAG_SECONDBUTTON, POINTER_FLAG_THIRDBUTTON, POINTER_INFO, POINTER_PEN_INFO,
                POINTER_TOUCH_INFO,
            },
        },
        WindowsAndMessaging::{
            GetMessageTime, GetSystemMetrics, POINTER_INPUT_TYPE, POINTER_MESSAGE_FLAG_FIFTHBUTTON, POINTER_MESSAGE_FLAG_FIRSTBUTTON,
            POINTER_MESSAGE_FLAG_FOURTHBUTTON, POINTER_MESSAGE_FLAG_SECONDBUTTON, POINTER_MESSAGE_FLAG_THIRDBUTTON, PT_PEN, PT_TOUCH,
            SM_CXDOUBLECLK, SM_CYDOUBLECLK, USER_DEFAULT_SCREEN_DPI,
        },
    },
};

use super::{
    events::Timestamp,
    geometry::{LogicalPoint, PhysicalPoint},
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

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerButton {
    None = 0,
    Left = 1 << 0,
    Right = 1 << 1,
    Middle = 1 << 2,
    XButton1 = 1 << 3,
    XButton2 = 1 << 4,
}

impl PointerButton {
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::double_parens)]
    pub(crate) fn from_message_flags(value: WPARAM) -> Self {
        let flags = u32::from(HIWORD!(value.0));
        if (flags & POINTER_MESSAGE_FLAG_FIRSTBUTTON) == POINTER_MESSAGE_FLAG_FIRSTBUTTON {
            Self::Left
        } else if (flags & POINTER_MESSAGE_FLAG_SECONDBUTTON) == POINTER_MESSAGE_FLAG_SECONDBUTTON {
            Self::Right
        } else if (flags & POINTER_MESSAGE_FLAG_THIRDBUTTON) == POINTER_MESSAGE_FLAG_THIRDBUTTON {
            Self::Middle
        } else if (flags & POINTER_MESSAGE_FLAG_FOURTHBUTTON) == POINTER_MESSAGE_FLAG_FOURTHBUTTON {
            Self::XButton1
        } else if (flags & POINTER_MESSAGE_FLAG_FIFTHBUTTON) == POINTER_MESSAGE_FLAG_FIFTHBUTTON {
            Self::XButton2
        } else {
            Self::None
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PointerButtons(u32);

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
                buttons |= PointerButton::Left as u32;
            }
            if (pointer_flags & POINTER_FLAG_SECONDBUTTON) == POINTER_FLAG_SECONDBUTTON {
                buttons |= PointerButton::Right as u32;
            }
            if (pointer_flags & POINTER_FLAG_THIRDBUTTON) == POINTER_FLAG_THIRDBUTTON {
                buttons |= PointerButton::Middle as u32;
            }
            if (pointer_flags & POINTER_FLAG_FOURTHBUTTON) == POINTER_FLAG_FOURTHBUTTON {
                buttons |= PointerButton::XButton1 as u32;
            }
            if (pointer_flags & POINTER_FLAG_FIFTHBUTTON) == POINTER_FLAG_FIFTHBUTTON {
                buttons |= PointerButton::XButton2 as u32;
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

    pub(crate) const fn get_physical_location(&self) -> PhysicalPoint {
        let native_pointer_info = self.get_native_pointer_info();
        PhysicalPoint::new(native_pointer_info.ptPixelLocation.x, native_pointer_info.ptPixelLocation.y)
    }
}

pub(crate) struct PointerClickCounter {
    button: PointerButton,
    clicks: u32,
    last_click_time: u32,
    last_click_rect: RECT,
}

impl PointerClickCounter {
    pub fn new() -> Self {
        Self {
            button: PointerButton::None,
            clicks: 0,
            last_click_time: 0,
            last_click_rect: RECT::default(),
        }
    }

    // See https://devblogs.microsoft.com/oldnewthing/20041018-00/?p=37543
    #[allow(clippy::cast_sign_loss)]
    pub fn register_click(&mut self, button: PointerButton, physical_point: PhysicalPoint) -> u32 {
        let (x, y) = (physical_point.x.0, physical_point.y.0);
        let pt = POINT { x, y };
        let tm_click = unsafe { GetMessageTime() } as u32;

        if button != self.button
            || !unsafe { PtInRect(&raw const self.last_click_rect, pt) }.as_bool()
            || tm_click - self.last_click_time > unsafe { GetDoubleClickTime() }
        {
            self.clicks = 0;
        }

        self.clicks += 1;
        self.last_click_time = tm_click;

        unsafe {
            let _ = SetRect(&raw mut self.last_click_rect, x, y, x, y);
            let _ = InflateRect(
                &raw mut self.last_click_rect,
                GetSystemMetrics(SM_CXDOUBLECLK) / 2,
                GetSystemMetrics(SM_CYDOUBLECLK) / 2,
            );
        }

        self.clicks
    }

    pub const fn reset(&mut self) {
        self.clicks = 0;
    }
}
