use crate::gtk::application::send_event;
use crate::gtk::events::{
    EventHandler, MouseButton, MouseDownEvent, MouseEnteredEvent, MouseExitedEvent, MouseMovedEvent, MouseUpEvent, ScrollWheelEvent,
    Timestamp, WindowId,
};
use crate::gtk::geometry::{LogicalPixels, LogicalPoint};
use gtk4::prelude::{EventControllerExt, GestureSingleExt, IsA, WidgetExt};
use gtk4::{gdk as gdk4, glib};
use std::cell::RefCell;
use std::rc::Rc;

#[allow(clippy::too_many_lines)]
pub fn set_mouse_event_handlers(
    widget: &impl IsA<gtk4::Widget>,
    window_id: WindowId,
    event_handler: EventHandler,
    last_pointer_down_event: Rc<RefCell<Option<(gdk4::Event, u32)>>>,
) {
    {
        let event_controller_motion = gtk4::EventControllerMotion::new();
        event_controller_motion.connect_motion(move |event_controller_motion, x, y| {
            let current_event_time = event_controller_motion.current_event_time();
            // Ignore dummy mouse move events, we report the mouse entered event anyway.
            // https://github.com/GNOME/gtk/blob/60d01c5af12282a7ef1517b651c30b5fbe003f37/gdk/gdksurface.c#L2381
            if current_event_time > 0 {
                let event = MouseMovedEvent {
                    window_id,
                    location_in_window: LogicalPoint {
                        x: LogicalPixels(x),
                        y: LogicalPixels(y),
                    },
                    timestamp: Timestamp(current_event_time),
                };
                send_event(event_handler, event);
            }
        });
        event_controller_motion.connect_enter(move |_event_controller_motion, x, y| {
            let event = MouseEnteredEvent {
                window_id,
                location_in_window: (x, y).into(),
            };
            send_event(event_handler, event);
        });
        event_controller_motion.connect_leave(move |_event_controller_motion| {
            let event = MouseExitedEvent { window_id };
            send_event(event_handler, event);
        });
        widget.add_controller(event_controller_motion);
    }

    {
        let click_gesture = gtk4::GestureClick::new();
        click_gesture.set_button(0);
        click_gesture.connect_pressed(move |c, _n_press, x, y| {
            let button = c.current_button();
            *last_pointer_down_event.borrow_mut() = c.current_event().map(|e| (e, button));
            let event = MouseDownEvent {
                window_id,
                button: MouseButton(button),
                location_in_window: LogicalPoint {
                    x: LogicalPixels(x),
                    y: LogicalPixels(y),
                },
                timestamp: Timestamp(c.current_event_time()),
            };
            send_event(event_handler, event);
        });
        click_gesture.connect_released(move |c, _n_press, x, y| {
            let event = MouseUpEvent {
                window_id,
                button: MouseButton(c.current_button()),
                location_in_window: LogicalPoint {
                    x: LogicalPixels(x),
                    y: LogicalPixels(y),
                },
                timestamp: Timestamp(c.current_event_time()),
            };
            send_event(event_handler, event);
        });
        widget.add_controller(click_gesture);
    }

    {
        let event_controller_scroll = gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::BOTH_AXES);
        event_controller_scroll.connect_scroll(move |event_controller_scroll, delta_x, delta_y| {
            let timestamp = Timestamp(event_controller_scroll.current_event_time());
            let event = ScrollWheelEvent {
                window_id,
                timestamp,
                scroll_delta_x: LogicalPixels(delta_x),
                scroll_delta_y: LogicalPixels(delta_y),
            };
            send_event(event_handler, event);
            glib::signal::Propagation::Stop
        });
        widget.add_controller(event_controller_scroll);
    }
}
