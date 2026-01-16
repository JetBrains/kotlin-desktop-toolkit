use crate::linux::application::send_event;
use crate::linux::application_api::{FfiTransferDataGetter, QueryDragAndDropTarget};
use crate::linux::clipboard::ClipboardContentProvider;
use crate::linux::data_transfer::{get_drag_offer_actions, handle_drop_start};
use crate::linux::events::{
    DragAndDropLeaveEvent, EventHandler, KeyCode, KeyDownEvent, KeyModifierBitflag, KeyUpEvent, ModifiersChangedEvent, MouseButton,
    MouseDownEvent, MouseEnteredEvent, MouseExitedEvent, MouseMovedEvent, MouseUpEvent, OpenGlDrawData, ScrollData, ScrollWheelEvent,
    ShouldRedraw, Timestamp, WindowCapabilities, WindowClosedEvent, WindowConfigureEvent, WindowDecorationMode, WindowDrawEvent, WindowId,
    WindowKeyboardEnterEvent, WindowKeyboardLeaveEvent, WindowScaleChangedEvent,
};
use crate::linux::geometry::{LogicalPixels, LogicalPoint, LogicalSize, PhysicalSize, round_to_i32};
use crate::linux::gl_widget::GlWidget;
use crate::linux::keyboard::{key_modifier_from_gdk, key_modifiers_from_gdk};
use crate::linux::mime_types::MimeTypes;
use crate::linux::pointer_shapes_api::PointerShape;
use crate::linux::text_input::{im_commit_handler, im_preedit_changed_handler};
use crate::linux::text_input_api::TextInputContext;
use crate::linux::window_resize_edge_api::WindowResizeEdge;
use anyhow::{Context, anyhow, bail};
use desktop_common::ffi_utils::BorrowedStrPtr;
use gdk4::glib::translate::IntoGlib;
use gtk4::gdk as gdk4;
use gtk4::glib;
use gtk4::prelude::{Cast, EventControllerExt, GestureSingleExt, IMContextExt, NativeExt, SurfaceExt, ToplevelExt, WidgetExtManual};
use gtk4::prelude::{GtkWindowExt, WidgetExt};
use log::debug;
use std::cell::{Cell, RefCell};
use std::ffi::CString;
use std::rc::Rc;

fn get_toplevel(window: &gtk4::Window) -> anyhow::Result<gdk4::Toplevel> {
    let surface = window.surface().context("Cannot get window surface")?;
    surface.downcast().ok().context("Cannot cast surface to toplevel")
}

pub struct SimpleWindow {
    pub window_id: WindowId,
    pub window: gtk4::Window,
    gl_widget: GlWidget,
    im_context: gtk4::IMMulticontext,
    event_handler: EventHandler,
    last_pointer_down_event: Rc<RefCell<Option<(gdk4::Event, u32)>>>,
    unrealized: Rc<Cell<bool>>,
}

impl Drop for SimpleWindow {
    fn drop(&mut self) {
        send_event(self.event_handler, WindowClosedEvent { window_id: self.window_id });
    }
}

impl SimpleWindow {
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn new(
        window_id: WindowId,
        size: LogicalSize,
        min_size: Option<LogicalSize>,
        title: Option<&str>,
        prefer_client_side_decoration: bool,
        event_handler: EventHandler,
        query_drag_and_drop_target: QueryDragAndDropTarget,
        window_close_request: impl Fn() -> bool + 'static,
    ) -> Self {
        let last_pointer_down_event = Rc::new(RefCell::new(None));
        let unrealized = Rc::new(Cell::new(false));

        let last_window_configure_event = {
            Rc::new(RefCell::new(WindowConfigureEvent {
                window_id,
                size: LogicalSize { width: 0, height: 0 },
                active: false,
                maximized: false,
                fullscreen: false,
                decoration_mode: if prefer_client_side_decoration {
                    WindowDecorationMode::Client
                } else {
                    WindowDecorationMode::Server
                },
                capabilities: WindowCapabilities::all(),
            }))
        };

        let do_draw = Box::new(move |opengl_draw_data: OpenGlDrawData, physical_size: PhysicalSize| {
            // debug!("do_draw");
            send_event(
                event_handler,
                WindowDrawEvent {
                    window_id,
                    opengl_draw_data,
                    physical_size,
                },
            );
        });
        let gl_widget = {
            let last_window_configure_event = last_window_configure_event.clone();
            let on_resize = Box::new(move |logical_size| {
                let event = &mut last_window_configure_event.borrow_mut();
                event.size = logical_size;
                send_event(event_handler, event.clone());
            });

            GlWidget::new(do_draw, on_resize)
        };

        if let Some(min_size) = min_size {
            gl_widget.set_size_request(min_size.width, min_size.height);
        }

        gl_widget.add_tick_callback(move |_gl_widget, _frame_clock| {
            send_event(event_handler, ShouldRedraw { window_id });
            glib::ControlFlow::Continue
        });

        let mut window_builder = gtk4::Window::builder()
            .resizable(true)
            .default_width(size.width)
            .default_height(size.height)
            .decorated(!prefer_client_side_decoration);
        if let Some(title) = title {
            window_builder = window_builder.title(title);
        }
        let window = window_builder.build();

        window.set_child(Some(&gl_widget));

        let drop_target = gtk4::DropTargetAsync::new(None, gdk4::DragAction::COPY);
        drop_target.connect_accept(move |drop_target, drop| {
            // let mime_types = drop.formats().mime_types();
            // debug!("drop_target_accept: {mime_types:?}");
            drop_target.set_formats(Some(&drop.formats()));
            drop_target.set_actions(drop.actions());
            true
        });
        drop_target.connect_drag_enter(move |_drop_target, drop, x, y| {
            // debug!("drop_target drag_enter");
            let mime_type_and_actions = get_drag_offer_actions(query_drag_and_drop_target, drop, x, y, window_id);
            drop.status(mime_type_and_actions.supported_actions, mime_type_and_actions.preferred_action);
            mime_type_and_actions.preferred_action
        });
        drop_target.connect_drag_motion(move |_drop_target, drop, x, y| {
            // debug!("drop_target drag_motion");
            let mime_type_and_actions = get_drag_offer_actions(query_drag_and_drop_target, drop, x, y, window_id);
            drop.status(mime_type_and_actions.supported_actions, mime_type_and_actions.preferred_action);
            mime_type_and_actions.preferred_action
        });
        drop_target.connect_drag_leave(move |_drop_target, _drop| {
            send_event(event_handler, DragAndDropLeaveEvent { window_id });
        });
        drop_target.connect_drop(move |_drop_target, drop, x, y| {
            handle_drop_start(event_handler, window_id, query_drag_and_drop_target, drop, x, y)
        });
        gl_widget.add_controller(drop_target);

        let im_context = gtk4::IMMulticontext::new();
        im_context.connect_commit(move |_im_context, commit_string| {
            im_commit_handler(event_handler, window_id, commit_string);
        });

        im_context.connect_preedit_changed(move |im_context| {
            im_preedit_changed_handler(event_handler, window_id, im_context);
        });

        let event_controller_key = gtk4::EventControllerKey::new();
        {
            event_controller_key.connect_key_pressed(move |_event_controller_key, keyval, keycode, modifiers| {
                debug!("connect_key_pressed: {keycode}, modifiers={modifiers:?}");

                // Cannot use the "modifiers" signal, see https://gitlab.gnome.org/GNOME/gtk/-/issues/5139
                if let Some(modifier) = key_modifier_from_gdk(keyval) {
                    let mut key_modifiers = key_modifiers_from_gdk(modifiers);
                    key_modifiers.set(modifier, true);
                    let event = ModifiersChangedEvent {
                        window_id,
                        modifiers: KeyModifierBitflag::new(key_modifiers),
                    };
                    send_event(event_handler, event);
                }

                let code = KeyCode(keycode - 8);
                let key = keyval.into_glib();
                let char = keyval.to_unicode();
                let c_char = char.map(|c| CString::new(c.to_string()).unwrap());
                let event = KeyDownEvent {
                    window_id,
                    code,
                    characters: BorrowedStrPtr::new_optional(c_char.as_ref()),
                    key,
                    // key_without_modifiers,
                    is_repeat: false, //event.repeat, // TODO
                };
                send_event(event_handler, event);
                glib::Propagation::Proceed
            });

            event_controller_key.connect_key_released(move |_event_controller_key, keyval, keycode, modifiers| {
                debug!("connect_key_released: {keycode}");

                // Cannot use the "modifiers" signal, see https://gitlab.gnome.org/GNOME/gtk/-/issues/5139
                if let Some(modifier) = key_modifier_from_gdk(keyval) {
                    let mut key_modifiers = key_modifiers_from_gdk(modifiers);
                    key_modifiers.set(modifier, false);
                    let event = ModifiersChangedEvent {
                        window_id,
                        modifiers: KeyModifierBitflag::new(key_modifiers),
                    };
                    send_event(event_handler, event);
                }

                let code = KeyCode(keycode - 8);
                let key = keyval.into_glib();
                let event = KeyUpEvent {
                    window_id,
                    code,
                    key,
                    // key_without_modifiers,
                };
                send_event(event_handler, event);
            });

            im_context.set_client_widget(Some(&gl_widget));
            event_controller_key.set_im_context(Some(&im_context));
            window.add_controller(event_controller_key);
        }

        {
            let event_controller_motion = gtk4::EventControllerMotion::new();
            event_controller_motion.connect_motion(move |event_controller_motion, x, y| {
                let event = MouseMovedEvent {
                    window_id,
                    location_in_window: LogicalPoint {
                        x: LogicalPixels(x),
                        y: LogicalPixels(y),
                    },
                    timestamp: Timestamp(event_controller_motion.current_event_time()),
                };
                send_event(event_handler, event);
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
            gl_widget.add_controller(event_controller_motion);
        }

        {
            let last_pointer_down_event = last_pointer_down_event.clone();
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
            gl_widget.add_controller(click_gesture);
        }

        {
            let event_controller_scroll = gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::BOTH_AXES);
            event_controller_scroll.connect_scroll(move |event_controller_scroll, delta_x, delta_y| {
                let timestamp = Timestamp(event_controller_scroll.current_event_time());
                let event = ScrollWheelEvent {
                    window_id,
                    timestamp,
                    horizontal_scroll: ScrollData {
                        delta: LogicalPixels(delta_x),
                        wheel_value120: round_to_i32(delta_x * 120.),
                        is_stop: false,
                    },
                    vertical_scroll: ScrollData {
                        delta: LogicalPixels(delta_y),
                        wheel_value120: round_to_i32(delta_y * 120.),
                        is_stop: false,
                    },
                };
                send_event(event_handler, event);
                glib::signal::Propagation::Stop
            });
            event_controller_scroll.connect_scroll_end(move |event_controller_scroll| {
                let timestamp = Timestamp(event_controller_scroll.current_event_time());
                let event = ScrollWheelEvent {
                    window_id,
                    timestamp,
                    horizontal_scroll: ScrollData {
                        delta: LogicalPixels(0.),
                        wheel_value120: 0,
                        is_stop: true,
                    },
                    vertical_scroll: ScrollData {
                        delta: LogicalPixels(0.),
                        wheel_value120: 0,
                        is_stop: true,
                    },
                };
                send_event(event_handler, event);
            });
            gl_widget.add_controller(event_controller_scroll);
        }

        {
            let im_context_weak_ref = glib::WeakRef::new();
            im_context_weak_ref.set(Some(&im_context));
            window.connect_is_active_notify(move |window| {
                let active = window.is_active();
                // Do this first to prevent a deadlock if `text_input_disable` is called as a result of the following events.
                if let Some(im_context) = im_context_weak_ref.upgrade() {
                    if active {
                        im_context.focus_in();
                    } else {
                        im_context.focus_out();
                    }
                }

                if active {
                    send_event(event_handler, WindowKeyboardEnterEvent { window_id });
                } else {
                    send_event(event_handler, WindowKeyboardLeaveEvent { window_id });
                }
            });
        }

        window.connect_close_request(move |_window| {
            if window_close_request() {
                // All the references to the window have to be destroyed before returning this
                glib::Propagation::Proceed
            } else {
                glib::Propagation::Stop
            }
        });

        // {
        //     let event = last_window_configure_event.clone();
        //     window.connect_realize(move |window| {
        //         let event = &mut event.borrow_mut();
        //         event.active = window.is_active();
        //         event.size = get_logical_window_size(window);
        //         send_event(event_handler, event.clone());
        //     });
        // }

        {
            let event = last_window_configure_event.clone();
            window.connect_decorated_notify(move |window| {
                debug!("decorated_notify");
                let event = &mut event.borrow_mut();
                event.decoration_mode = if window.is_decorated() {
                    WindowDecorationMode::Server
                } else {
                    WindowDecorationMode::Client
                };
                send_event(event_handler, event.clone());
            });
        }

        {
            let event = last_window_configure_event.clone();
            window.connect_maximized_notify(move |window| {
                let event = &mut event.borrow_mut();
                event.maximized = window.is_maximized();
                send_event(event_handler, event.clone());
            });
        }

        {
            let last_window_configure_event = last_window_configure_event.clone();
            window.connect_fullscreened_notify(move |window| {
                let event = &mut last_window_configure_event.borrow_mut();
                event.fullscreen = window.is_fullscreen();
                send_event(event_handler, event.clone());
            });
        }

        window.connect_scale_factor_notify(move |window| {
            let scale = window.scale_factor();
            send_event(
                event_handler,
                WindowScaleChangedEvent {
                    window_id,
                    new_scale: scale.into(),
                },
            );
        });

        window.connect_realize(move |window| {
            let scale = window.scale_factor();
            send_event(
                event_handler,
                WindowScaleChangedEvent {
                    window_id,
                    new_scale: scale.into(),
                },
            );

            let last_window_configure_event = last_window_configure_event.clone();
            let toplevel = get_toplevel(window).unwrap();
            toplevel.connect_state_notify(move |toplevel| {
                let state = toplevel.state();

                let event = &mut last_window_configure_event.borrow_mut();
                let active = state.contains(gdk4::ToplevelState::FOCUSED);
                if active != event.active {
                    event.active = active;
                    send_event(event_handler, event.clone());
                }
            });
        });

        {
            let unrealized = unrealized.clone();
            window.connect_unrealize(move |_window| {
                debug!("connect_unrealize {window_id:?}");
                unrealized.set(true);
            });
        }

        {
            // Don't call `gtk4::Window::present` synchronously, because that can trigger events,
            // which then can call another `window_` function, and we want this window ID to be valid before that.
            let window = window.clone();
            glib::idle_add_local_once(move || {
                debug!("Before present");
                window.present();
                debug!("After present");
            });
        }

        Self {
            window_id,
            window,
            gl_widget,
            im_context,
            event_handler,
            last_pointer_down_event,
            unrealized,
        }
    }

    pub fn is_unrealized(&self) -> bool {
        self.unrealized.get()
    }

    pub fn request_redraw(&self) {
        self.gl_widget.queue_draw();
    }

    pub fn maximize(&self) {
        self.window.maximize();
    }

    pub fn unmaximize(&self) {
        self.window.unmaximize();
    }

    pub fn minimize(&self) {
        self.window.minimize();
    }

    pub fn set_min_size(&self, size: LogicalSize) {
        self.gl_widget.set_size_request(size.width, size.height);
    }

    pub fn set_title(&self, new_title: &str) {
        self.window.set_title(Some(new_title));
    }

    pub fn set_cursor_icon(&self, pointer_shape: PointerShape) {
        if pointer_shape == PointerShape::Hidden {
            self.window.set_cursor(None);
        } else {
            let cursor: Option<gdk4::Cursor> = pointer_shape.into();
            if let Some(cursor) = cursor {
                self.window.set_cursor(Some(&cursor));
            }
        }
    }

    pub fn start_drag_operation(
        &self,
        mime_types: &MimeTypes,
        action: gdk4::DragAction,
        get_data_transfer_data: FfiTransferDataGetter,
    ) -> anyhow::Result<gdk4::Drag> {
        let last_pointer_down_event = self.last_pointer_down_event.borrow();
        let (event, _button) = last_pointer_down_event.as_ref().context("No pointer event")?;
        let device = event.device().context("Pointer event has no device")?;
        let surface = self.window.surface().context("Cannot get window surface")?;
        let (x, y, _modifiers) = surface.device_position(&device).context("Pointer event has no position")?;
        gdk4::Drag::begin(
            &surface,
            &device,
            &ClipboardContentProvider::new(mime_types, get_data_transfer_data),
            action,
            x,
            y,
        )
        .context("Cannot begin drag")
    }

    pub fn start_move(&self) -> anyhow::Result<()> {
        let toplevel = get_toplevel(&self.window)?;

        let last_pointer_down_event = self.last_pointer_down_event.borrow();
        let Some((event, button)) = last_pointer_down_event.as_ref() else {
            bail!(
                "Attempting to start moving window {:?} without a pointer down event",
                self.window_id
            )
        };

        let device = event.device();
        let Some(device) = device.as_ref() else {
            bail!(
                "Error moving window {:?}: cannot get device from the pointer down event",
                self.window_id
            )
        };

        let (x, y) = if let Some((x, y, _modifiers)) = toplevel.device_position(device) {
            (x, y)
        } else if let Some(position) = event.position() {
            position
        } else {
            bail!("Error moving window {:?}: cannot get pointer position", self.window_id)
        };

        toplevel.begin_move(device, *button as i32, x, y, event.time());
        Ok(())
    }

    pub fn start_resize(&self, edge: WindowResizeEdge) -> anyhow::Result<()> {
        let toplevel = get_toplevel(&self.window)?;

        let last_pointer_down_event = self.last_pointer_down_event.borrow();
        let Some((event, button)) = last_pointer_down_event.as_ref() else {
            bail!(
                "Attempting to start resizing window {:?} without a pointer down event",
                self.window_id
            )
        };

        let device = event.device();
        let Some(device) = device.as_ref() else {
            bail!(
                "Error resizing window {:?}: cannot get device from the pointer down event",
                self.window_id
            )
        };

        let (x, y) = if let Some((x, y, _modifiers)) = toplevel.device_position(device) {
            (x, y)
        } else if let Some(position) = event.position() {
            position
        } else {
            bail!("Error resizing window {:?}: cannot get pointer position", self.window_id)
        };

        toplevel.begin_resize(edge.into(), Some(device), *button as i32, x, y, event.time());
        Ok(())
    }

    pub fn show_menu(&self) -> anyhow::Result<()> {
        let toplevel = get_toplevel(&self.window)?;
        if let Some((event, _button)) = self.last_pointer_down_event.borrow().as_ref() {
            toplevel.show_window_menu(event);
            Ok(())
        } else {
            Err(anyhow!(
                "Attempting to show menu for window {:?} without a pointer down event",
                self.window_id
            ))
        }
    }

    pub fn set_fullscreen(&self) {
        self.window.set_fullscreened(true);
    }

    pub fn unset_fullscreen(&self) {
        self.window.set_fullscreened(false);
    }

    pub fn set_decoration_mode(&self, decoration_mode: WindowDecorationMode) {
        self.window.set_decorated(decoration_mode == WindowDecorationMode::Server);
    }

    pub fn focus(&self, activation_token: Option<&str>) {
        if let Some(activation_token) = activation_token {
            self.window.set_startup_id(activation_token);
        }
        self.window.present();
    }

    fn text_input_update_impl(&self, context: &TextInputContext) {
        let surrounding_text = context.surrounding_text.as_str().unwrap();
        let cursor_index = context.cursor_codepoint_offset;
        let anchor_index = context.selection_start_codepoint_offset;
        self.im_context
            .set_surrounding_with_selection(surrounding_text, cursor_index.into(), anchor_index.into());

        self.im_context.set_cursor_location(&context.cursor_rectangle.into());
        self.im_context.set_input_purpose(context.content_purpose.into());
    }

    pub fn text_input_enable(&self, context: &TextInputContext) {
        debug!("{:?}: text_input_enable: {context:?}", self.window_id);
        self.im_context.focus_in();
        self.text_input_update_impl(context);
    }

    pub fn text_input_disable(&self) {
        debug!("{:?}: text_input_disable", self.window_id);
        self.im_context.focus_out();
    }

    pub fn text_input_update(&self, context: &TextInputContext) {
        debug!("{:?}: text_input_update: {context:?}", self.window_id);
        self.text_input_update_impl(context);
    }
}
