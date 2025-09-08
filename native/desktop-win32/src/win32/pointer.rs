use windows::{
    UI::Input::{PointerPoint, PointerUpdateKind},
    Win32::Foundation::WPARAM,
};

use super::{
    events::{PointerButtonEvent, Timestamp},
    geometry::LogicalPoint,
    utils::LOWORD,
};

#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::double_parens)]
#[inline]
pub(crate) fn get_pointer_point(wparam: WPARAM) -> Option<PointerPoint> {
    let pointer_id = u32::from(LOWORD!(wparam.0));
    PointerPoint::GetCurrentPoint(pointer_id)
        .inspect_err(|err| log::error!("failed to get PointerPoint for pointer ID {pointer_id}: {err}"))
        .ok()
}

#[inline]
pub(crate) fn get_pointer_location_in_window(pointer_point: &PointerPoint) -> Option<LogicalPoint> {
    let contact_rect = pointer_point
        .Properties()
        .and_then(|prop| prop.ContactRect())
        .inspect_err(|err| log::error!("failed to get PointerPoint.Properties.ContactRect property: {err}"))
        .ok()?;
    Some(LogicalPoint::new(contact_rect.X, contact_rect.Y))
}

#[inline]
pub(crate) fn get_pointer_state(pointer_point: &PointerPoint) -> Option<PointerState> {
    PointerState::try_from(pointer_point)
        .inspect_err(|err| log::error!("failed to get PointerState: {err}"))
        .ok()
}

#[inline]
pub(crate) fn get_pointer_event_timestamp(pointer_point: &PointerPoint) -> Option<Timestamp> {
    let timestamp = pointer_point
        .Timestamp()
        .inspect_err(|err| log::error!("failed to get PointerPoint.Timestamp property: {err}"))
        .ok()?;
    Some(Timestamp(timestamp))
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PointerState {
    pressed_buttons: PointerButtons,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum PointerButton {
    None = 0,
    Left = 1 << 0,
    Right = 1 << 1,
    Middle = 1 << 2,
    XButton1 = 1 << 3,
    XButton2 = 1 << 4,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PointerButtons(u32);

impl TryFrom<&PointerPoint> for PointerState {
    type Error = windows::core::Error;

    fn try_from(value: &PointerPoint) -> Result<Self, Self::Error> {
        Ok(Self {
            pressed_buttons: PointerButtons::try_from(value)?,
        })
    }
}

impl TryFrom<&PointerPoint> for PointerButtons {
    type Error = windows::core::Error;

    fn try_from(value: &PointerPoint) -> Result<Self, Self::Error> {
        let properties = value.Properties()?;
        let mut buttons = PointerButton::None as u32;
        if properties.IsLeftButtonPressed()? {
            buttons |= PointerButton::Left as u32;
        }
        if properties.IsRightButtonPressed()? {
            buttons |= PointerButton::Right as u32;
        }
        if properties.IsMiddleButtonPressed()? {
            buttons |= PointerButton::Middle as u32;
        }
        if properties.IsXButton1Pressed()? {
            buttons |= PointerButton::XButton1 as u32;
        }
        if properties.IsXButton2Pressed()? {
            buttons |= PointerButton::XButton2 as u32;
        }
        Ok(Self(buttons))
    }
}

impl TryFrom<&PointerPoint> for PointerButton {
    type Error = windows::core::Error;

    fn try_from(value: &PointerPoint) -> Result<Self, Self::Error> {
        let properties = value.Properties()?;
        let button = match properties.PointerUpdateKind()? {
            PointerUpdateKind::LeftButtonPressed | PointerUpdateKind::LeftButtonReleased => Self::Left,
            PointerUpdateKind::RightButtonPressed | PointerUpdateKind::RightButtonReleased => Self::Right,
            PointerUpdateKind::MiddleButtonPressed | PointerUpdateKind::MiddleButtonReleased => Self::Middle,
            PointerUpdateKind::XButton1Pressed | PointerUpdateKind::XButton1Released => Self::XButton1,
            PointerUpdateKind::XButton2Pressed | PointerUpdateKind::XButton2Released => Self::XButton2,
            _ => Self::None,
        };
        Ok(button)
    }
}

impl TryFrom<&PointerPoint> for PointerButtonEvent {
    type Error = windows::core::Error;

    fn try_from(value: &PointerPoint) -> Result<Self, Self::Error> {
        let position = value.Position()?;
        Ok(Self {
            button: PointerButton::try_from(value)?,
            location_in_window: LogicalPoint::new(position.X, position.Y),
            state: PointerState::try_from(value)?,
            timestamp: Timestamp(value.Timestamp()?),
        })
    }
}
