use crate::gtk::application::send_event;
use crate::gtk::events::{
    EventHandler, MouseButton, MouseDownEvent, MouseEnteredEvent, MouseExitedEvent, MouseMovedEvent, MouseUpEvent, ScrollWheelEvent,
    Timestamp, WindowId,
};
use crate::gtk::geometry::{LogicalPixels, LogicalPoint};
use gtk4::prelude::{EventControllerExt, NativeExt, ObjectExt, WidgetExt};
use gtk4::{gdk as gdk4, glib};

fn translate_event_coordinates(e: &gdk4::Event, widget: &gtk4::Widget) -> Option<LogicalPoint> {
    let (mut event_x, mut event_y) = e.position()?;
    let surface = e.surface()?;
    let native = gtk4::Native::for_surface(&surface)?;
    let (nx, ny) = native.surface_transform();
    event_x -= nx;
    event_y -= ny;

    #[allow(clippy::cast_possible_truncation)]
    let p = native.compute_point(widget, &gtk4::graphene::Point::new(event_x as f32, event_y as f32))?;

    Some(LogicalPoint {
        x: LogicalPixels(f64::from(p.x())),
        y: LogicalPixels(f64::from(p.y())),
    })
}

fn set_motion_events_handler(widget: &gtk4::Widget, window_id: WindowId, event_handler: EventHandler) {
    let event_controller_motion = gtk4::EventControllerMotion::new();
    event_controller_motion.set_propagation_phase(gtk4::PropagationPhase::Capture);
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
            location_in_window: LogicalPoint::new(x, y),
        };
        send_event(event_handler, event);
    });
    event_controller_motion.connect_leave(move |_event_controller_motion| {
        let event = MouseExitedEvent { window_id };
        send_event(event_handler, event);
    });
    widget.add_controller(event_controller_motion);
}

fn set_mouse_button_events_handler(widget: &gtk4::Widget, window_id: WindowId, event_handler: EventHandler) {
    let event_controller_legacy = gtk4::EventControllerLegacy::new();
    event_controller_legacy.set_propagation_phase(gtk4::PropagationPhase::Capture);
    let widget_weak_ref = widget.downgrade();
    event_controller_legacy.connect_event(move |_event_controller_legacy, e| {
        let handled = match e.event_type() {
            gdk4::EventType::ButtonPress => {
                let button_event = e.downcast_ref::<gdk4::ButtonEvent>().unwrap();
                let button = button_event.button();
                if let Some(widget) = widget_weak_ref.upgrade()
                    && let Some(location_in_window) = translate_event_coordinates(e, &widget)
                {
                    let event = MouseDownEvent {
                        window_id,
                        button: MouseButton(button),
                        location_in_window,
                        timestamp: Timestamp(e.time()),
                    };
                    send_event(event_handler, event)
                } else {
                    false
                }
            }
            gdk4::EventType::ButtonRelease => {
                let button_event = e.downcast_ref::<gdk4::ButtonEvent>().unwrap();
                let button = button_event.button();
                if let Some(widget) = widget_weak_ref.upgrade()
                    && let Some(location_in_window) = translate_event_coordinates(e, &widget)
                {
                    let event = MouseUpEvent {
                        window_id,
                        button: MouseButton(button),
                        location_in_window,
                        timestamp: Timestamp(e.time()),
                    };
                    send_event(event_handler, event)
                } else {
                    false
                }
            }
            gdk4::EventType::Scroll => {
                let scroll_event = e.downcast_ref::<gdk4::ScrollEvent>().unwrap();

                let direction = scroll_event.direction();
                let is_smooth_scroll = direction == gdk4::ScrollDirection::Smooth;
                let (mut delta_x, mut delta_y) = scroll_event.deltas();

                if !is_smooth_scroll && delta_y == 0.0 && delta_x == 0.0 {
                    match direction {
                        gdk4::ScrollDirection::Up => delta_y = -1.0,
                        gdk4::ScrollDirection::Down => delta_y = 1.0,
                        gdk4::ScrollDirection::Left => delta_x = -1.0,
                        gdk4::ScrollDirection::Right => delta_x = 1.0,
                        _ => {}
                    }
                }

                let event = ScrollWheelEvent {
                    window_id,
                    timestamp: Timestamp(e.time()),
                    scroll_delta_x: LogicalPixels(delta_x),
                    scroll_delta_y: LogicalPixels(delta_y),
                    is_stop: scroll_event.is_stop(),
                    is_smooth_scroll,
                };
                send_event(event_handler, event)
            }
            gdk4::EventType::TouchBegin => false,  // TODO
            gdk4::EventType::TouchEnd => false,    // TODO
            gdk4::EventType::TouchUpdate => false, // TODO
            gdk4::EventType::TouchCancel => false, // TODO
            _ => false,
        };
        if handled {
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    widget.add_controller(event_controller_legacy);
}

pub fn set_mouse_event_handlers(widget: &gtk4::Widget, window_id: WindowId, event_handler: EventHandler) {
    set_motion_events_handler(widget, window_id, event_handler);
    set_mouse_button_events_handler(widget, window_id, event_handler);
}
