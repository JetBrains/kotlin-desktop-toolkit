use anyhow::bail;
use log::{info, warn};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use objc2_app_kit::{NSEvent, NSEventSubtype, NSEventType};

pub(crate) trait NSMouseEventExt {
    fn me(&self) -> &NSEvent;

    fn mouse_button(&self) -> Option<MouseButton> {
        let me = self.me();

        match unsafe { me.r#type() } {
            NSEventType::LeftMouseDown
            | NSEventType::RightMouseDown
            | NSEventType::OtherMouseDown
            | NSEventType::LeftMouseUp
            | NSEventType::RightMouseUp
            | NSEventType::OtherMouseUp
            | NSEventType::LeftMouseDragged
            | NSEventType::RightMouseDragged
            | NSEventType::OtherMouseDragged => {
                let button_number = unsafe { me.buttonNumber() };
                let button = button_number.try_into().map(|button| MouseButton(button)).ok();
                if button.is_none() {
                    warn!("Ignored mouse button: {me:?}");
                }
                button
            }

            _ => None,
        }
    }

    fn pressed_mouse_buttons() -> MouseButtonsSet {
        let pressed_buttons = unsafe { NSEvent::pressedMouseButtons() };
        MouseButtonsSet(pressed_buttons.try_into().unwrap())
    }
}

impl NSMouseEventExt for NSEvent {
    fn me(&self) -> &NSEvent {
        self
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct MouseButton(u32);

#[derive(Debug)]
#[repr(transparent)]
pub struct MouseButtonsSet(u32);

#[allow(non_upper_case_globals)]
pub(crate) const EmptyMouseButtonsSet: MouseButtonsSet = MouseButtonsSet(0);

//#[derive(Debug)]
//pub(crate) enum MouseEventType {
//    Down,
//    Up,
//    Move,
//    Drag
//}
//
//
//#[derive(Debug)]
//pub(crate) enum MouseEventSource {
//    Mouse,
//    Touchpad,
//    Tablet
//}
//
//#[derive(Debug)]
//struct MouseEventInfo {
//    event_type: MouseEventType,
//    event_source: MouseEventSource,
//    mouse_button: Option<MouseButton>,
//    pressed_buttons: MouseButtonsSet
//}

//pub(crate) fn unpack_mouse_event(ns_event: &NSEvent) -> anyhow::Result<()> {
//    let (event_type, button) = match unsafe { ns_event.r#type() } {
//        NSEventType::LeftMouseDown |
//        NSEventType::RightMouseDown |
//        NSEventType::OtherMouseDown => {
//            (MouseEventType::Down, Some(unsafe { ns_event.buttonNumber() }))
//        },
//
//        NSEventType::LeftMouseUp |
//        NSEventType::RightMouseUp |
//        NSEventType::OtherMouseUp => {
//            (MouseEventType::Up, Some(unsafe { ns_event.buttonNumber() }))
//        },
//
//        NSEventType::LeftMouseDragged |
//        NSEventType::RightMouseDragged |
//        NSEventType::OtherMouseDragged => {
//            (MouseEventType::Drag, Some(unsafe { ns_event.buttonNumber() }))
//        }
//
//        NSEventType::MouseMoved => {
//            (MouseEventType::Move, None)
//        }
//
//        _ => bail!("Unexpected type of event: {ns_event:?}")
//    };
//
//    let event_source = match unsafe { ns_event.subtype() } {
//        NSEventSubtype::MouseEvent => MouseEventSource::Mouse,
//
//        NSEventSubtype::Touch => MouseEventSource::Touchpad,
//
//        NSEventSubtype::TabletPoint |
//        NSEventSubtype::TabletProximity => MouseEventSource::Tablet,
//
//        _ => bail!("Unexpected event subtype: {ns_event:?}")
//    };
//
//    let click_count = unsafe { ns_event.clickCount() };
//
//    info!("{event_type:?} {event_source:?} button: {button:?} click_count: {click_count:?}");
//    Ok(())
//}
