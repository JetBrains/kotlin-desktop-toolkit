use crate::linux::events::{MouseButton, ScrollData};
use crate::linux::geometry::LogicalPixels;

impl TryFrom<winit_core::event::MouseButton> for MouseButton {
    type Error = ();
    fn try_from(value: winit_core::event::MouseButton) -> Result<Self, Self::Error> {
        use winit_core::event::MouseButton;
        match value {
            MouseButton::Left => Ok(Self(0x110)),
            MouseButton::Right => Ok(Self(0x111)),
            MouseButton::Middle => Ok(Self(0x112)),
            MouseButton::Back => Ok(Self(0x116)),
            MouseButton::Forward => Ok(Self(0x115)),
            _ => Err(()), // TODO
        }
    }
}

impl TryFrom<winit_core::event::ButtonSource> for MouseButton {
    type Error = ();

    fn try_from(value: winit_core::event::ButtonSource) -> Result<Self, Self::Error> {
        use winit_core::event::ButtonSource;
        match value {
            ButtonSource::Mouse(mouse_button) => mouse_button.try_into(),
            ButtonSource::Touch { .. } => Err(()),
            ButtonSource::TabletTool { .. } => Err(()),
            ButtonSource::Unknown(raw_button) => Ok(Self(raw_button.into())),
        }
    }
}

impl ScrollData {
    #[must_use]
    pub fn from_winit(value: winit_core::event::MouseScrollDelta, touch_phase: winit_core::event::TouchPhase) -> (Self, Self) {
        let is_stop = touch_phase == winit_core::event::TouchPhase::Cancelled || touch_phase == winit_core::event::TouchPhase::Ended;
        match value {
            winit_core::event::MouseScrollDelta::LineDelta(h, v) => (
                Self {
                    delta: LogicalPixels(h.into()),
                    wheel_value120: LogicalPixels(f64::from(h) * 120.).round(),
                    is_stop,
                },
                Self {
                    delta: LogicalPixels(v.into()),
                    wheel_value120: LogicalPixels(f64::from(v) * 120.).round(),
                    is_stop,
                },
            ),
            winit_core::event::MouseScrollDelta::PixelDelta(phys_pos) => {
                (
                    Self {
                        delta: LogicalPixels(phys_pos.x),                  // TODO
                        wheel_value120: LogicalPixels(phys_pos.x).round(), // TODO
                        is_stop,
                    },
                    Self {
                        delta: LogicalPixels(phys_pos.y),                  // TODO
                        wheel_value120: LogicalPixels(phys_pos.y).round(), // TODO
                        is_stop,
                    },
                )
            }
        }
    }
}
