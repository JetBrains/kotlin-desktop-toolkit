use crate::gtk::application::send_event;
use crate::gtk::application_api::{DataSource, FfiTransferDataGetter, QueryDragAndDropTarget};
use crate::gtk::clipboard::ClipboardContentProvider;
use crate::gtk::data_transfer::set_drag_and_drop_event_handlers;
use crate::gtk::events::{
    Event, EventHandler, OpenGlDrawData, ScreenId, ShouldRedraw, WindowCapabilities, WindowConfigureEvent, WindowDecorationMode,
    WindowDrawEvent, WindowId, WindowKeyboardEnterEvent, WindowKeyboardLeaveEvent, WindowScaleChangedEvent, WindowScreenChangeEvent,
};
use crate::gtk::geometry::{LogicalSize, PhysicalSize};
use crate::gtk::gl_widget::GlWidget;
use crate::gtk::kdt_application::KdtApplication;
use crate::gtk::keyboard::set_keyboard_event_handlers;
use crate::gtk::mime_types::MimeTypes;
use crate::gtk::mouse::set_mouse_event_handlers;
use crate::gtk::pointer_shapes_api::PointerShape;
use crate::gtk::text_input::create_im_context;
use crate::gtk::text_input_api::TextInputContext;
use crate::gtk::window_resize_edge_api::WindowResizeEdge;
use anyhow::{Context, anyhow, bail};
use gtk4::gdk as gdk4;
use gtk4::glib;
use gtk4::prelude::{Cast, DisplayExt, IMContextExt, NativeExt, ObjectExt, ObjectType, SurfaceExt, ToplevelExt, WidgetExtManual};
use gtk4::prelude::{GtkWindowExt, WidgetExt};
use log::debug;
use std::cell::RefCell;
use std::rc::Rc;

fn get_toplevel(window: &gtk4::ApplicationWindow) -> anyhow::Result<gdk4::Toplevel> {
    let surface = window.surface().context("Cannot get window surface")?;
    surface.downcast().ok().context("Cannot cast surface to toplevel")
}

fn create_gl_widget(
    window_id: WindowId,
    event_handler: EventHandler,
    min_size: Option<LogicalSize>,
    last_window_configure_event: Rc<RefCell<Option<WindowConfigureEvent>>>,
) -> GlWidget {
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
    let on_resize = Box::new(move |logical_size| {
        let event = &mut last_window_configure_event.borrow_mut();
        let event = event.as_mut().unwrap();
        if event.size != logical_size {
            event.size = logical_size;
            if let Ok(event) = event.clone().try_into() {
                send_event::<Event>(event_handler, event);
            }
        }
    });

    let gl_widget = GlWidget::new(do_draw, on_resize, min_size);

    gl_widget.add_tick_callback(move |_gl_widget, _frame_clock| {
        send_event(event_handler, ShouldRedraw { window_id });
        glib::ControlFlow::Continue
    });

    gl_widget
}

fn do_async<T: ObjectType>(obj: glib::WeakRef<T>, f: impl FnOnce(T) + 'static) {
    glib::idle_add_local_once(move || {
        if let Some(obj) = obj.upgrade() {
            f(obj);
        }
    });
}

impl WindowConfigureEvent {
    fn new(window: &gtk4::ApplicationWindow, window_id: WindowId) -> Self {
        let active = {
            let toplevel = get_toplevel(window).unwrap();
            let state = toplevel.state();
            state.contains(gdk4::ToplevelState::FOCUSED)
        };

        let decoration_mode = if window.is_decorated() {
            WindowDecorationMode::Server
        } else {
            WindowDecorationMode::Client
        };

        let maximized = window.is_maximized();
        let fullscreen = window.is_fullscreen();
        let width = window.width();
        let height = window.height();

        Self {
            window_id,
            size: LogicalSize { width, height },
            active,
            maximized,
            fullscreen,
            decoration_mode,
            capabilities: WindowCapabilities::all(),
        }
    }
}

fn set_window_configure_event_handlers(
    window: &gtk4::ApplicationWindow,
    event_handler: EventHandler,
    last_window_configure_event: Rc<RefCell<Option<WindowConfigureEvent>>>,
) {
    {
        let event = last_window_configure_event.clone();
        window.connect_decorated_notify(move |window| {
            debug!("decorated_notify");
            let event = &mut event.borrow_mut();
            let event = event.as_mut().unwrap();
            event.decoration_mode = if window.is_decorated() {
                WindowDecorationMode::Server
            } else {
                WindowDecorationMode::Client
            };
            if let Ok(event) = event.clone().try_into() {
                send_event::<Event>(event_handler, event);
            }
        });
    }

    {
        let event = last_window_configure_event.clone();
        window.connect_maximized_notify(move |window| {
            let event = &mut event.borrow_mut();
            let event = event.as_mut().unwrap();
            event.maximized = window.is_maximized();
            if let Ok(event) = event.clone().try_into() {
                send_event::<Event>(event_handler, event);
            }
        });
    }

    window.connect_fullscreened_notify(move |window| {
        let event = &mut last_window_configure_event.borrow_mut();
        let event = event.as_mut().unwrap();
        event.fullscreen = window.is_fullscreen();
        if let Ok(event) = event.clone().try_into() {
            send_event::<Event>(event_handler, event);
        }
    });
}

pub struct SimpleWindow {
    pub window_id: WindowId,
    pub window: glib::WeakRef<gtk4::ApplicationWindow>,
    gl_widget: glib::WeakRef<GlWidget>,
    event_controller_key_weak: glib::WeakRef<gtk4::EventControllerKey>,
    im_context: gtk4::IMMulticontext,
    last_pointer_down_event: Rc<RefCell<Option<(gdk4::Event, u32)>>>,
}

impl SimpleWindow {
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn new(
        gtk_application: &KdtApplication,
        window_id: WindowId,
        size: LogicalSize,
        min_size: Option<LogicalSize>,
        title: Option<&str>,
        prefer_client_side_decoration: bool,
        event_handler: EventHandler,
        query_drag_and_drop_target: QueryDragAndDropTarget,
        window_close_request: impl Fn() -> bool + 'static,
        on_destroy: impl Fn() + 'static,
    ) -> Self {
        let last_pointer_down_event = Rc::new(RefCell::new(None));

        let last_window_configure_event = Rc::new(RefCell::new(None));

        let gl_widget = create_gl_widget(window_id, event_handler, min_size, last_window_configure_event.clone());

        let mut window_builder = gtk4::ApplicationWindow::builder()
            .application(gtk_application)
            .resizable(true)
            .default_width(size.width)
            .default_height(size.height)
            .decorated(!prefer_client_side_decoration);
        if let Some(title) = title {
            window_builder = window_builder.title(title);
        }
        let window = window_builder.build();

        window.set_child(Some(&gl_widget));

        set_drag_and_drop_event_handlers(&gl_widget, window_id, event_handler, query_drag_and_drop_target);
        let event_controller_key = set_keyboard_event_handlers(window_id, event_handler);
        let event_controller_key_weak = event_controller_key.downgrade();
        window.add_controller(event_controller_key);

        let im_context = create_im_context(window_id, event_handler);
        let im_context_weak_ref = im_context.downgrade();
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

        set_mouse_event_handlers(&gl_widget, window_id, event_handler, last_pointer_down_event.clone());
        set_window_configure_event_handlers(&window, event_handler, last_window_configure_event.clone());

        window.connect_scale_factor_notify(move |window| {
            let scale = window.scale_factor();
            let event = WindowScaleChangedEvent {
                window_id,
                new_scale: scale.into(),
            };
            send_event(event_handler, event);
        });

        let im_context_weak_ref = im_context.downgrade();
        gl_widget.connect_realize(move |gl_widget| {
            if let Some(im_context) = im_context_weak_ref.upgrade() {
                im_context.set_client_widget(Some(gl_widget));
            }
        });

        window.connect_realize(move |window| {
            let scale = window.scale_factor();
            let scale_event = WindowScaleChangedEvent {
                window_id,
                new_scale: scale.into(),
            };
            send_event(event_handler, scale_event);

            let last_window_configure_event = last_window_configure_event.clone();
            *last_window_configure_event.borrow_mut() = Some(WindowConfigureEvent::new(window, window_id));
            // Don't send the event yet, because we don't have a proper size in `realize`

            let toplevel = get_toplevel(window).unwrap();

            if let Some(monitor) = toplevel.display().monitor_at_surface(&toplevel) {
                let new_screen_id = ScreenId::new(&monitor);
                let event = WindowScreenChangeEvent { window_id, new_screen_id };
                send_event(event_handler, event);
            }

            toplevel.connect_state_notify(move |toplevel| {
                let state = toplevel.state();

                let event = &mut last_window_configure_event.borrow_mut();
                let event = event.as_mut().unwrap();
                let active = state.contains(gdk4::ToplevelState::FOCUSED);
                if active != event.active {
                    event.active = active;
                    if let Ok(event) = event.clone().try_into() {
                        send_event::<Event>(event_handler, event);
                    }
                }
            });
            toplevel.connect_enter_monitor(move |_toplevel, monitor| {
                let new_screen_id = ScreenId::new(monitor);
                let event = WindowScreenChangeEvent { window_id, new_screen_id };
                send_event(event_handler, event);
            });
        });

        window.connect_close_request(move |_window| {
            if window_close_request() {
                // All the references to the window have to be destroyed before returning this
                glib::Propagation::Proceed
            } else {
                glib::Propagation::Stop
            }
        });

        window.connect_destroy(move |_w| {
            debug!("destroy for {window_id:?}");
            on_destroy();
        });

        // Don't call `gtk4::Window::present` synchronously, because that can trigger events,
        // which then can call another `window_` function, and we want this window ID to be valid before that and to avoid
        // reentrancy in the client code.
        do_async(window.downgrade(), move |window| {
            debug!("Before present");
            window.present();
            debug!("After present");
        });

        Self {
            window_id,
            window: window.downgrade(),
            gl_widget: gl_widget.downgrade(),
            event_controller_key_weak,
            im_context,
            last_pointer_down_event,
        }
    }

    pub fn destroy(&self) {
        // Close the window asynchronously, because closing can trigger events, which then can cause reentrancy issues in the client code.
        let window_id = self.window_id;
        do_async(self.window.clone(), move |window| {
            debug!("Calling GTKWindow.destroy for {window_id:?}");
            window.destroy();
        });
    }

    pub fn request_redraw(&self) {
        do_async(self.gl_widget.clone(), move |gl_widget| {
            gl_widget.queue_draw();
        });
    }

    pub fn maximize(&self) {
        do_async(self.window.clone(), move |window| {
            window.maximize();
        });
    }

    pub fn unmaximize(&self) {
        do_async(self.window.clone(), move |window| {
            window.unmaximize();
        });
    }

    pub fn minimize(&self) {
        do_async(self.window.clone(), move |window| {
            window.minimize();
        });
    }

    pub fn set_min_size(&self, size: LogicalSize) {
        if let Some(gl_widget) = self.gl_widget.upgrade() {
            gl_widget.set_size_request(size.width, size.height);
        }
    }

    pub fn set_title(&self, new_title: &str) {
        if let Some(w) = self.window.upgrade() {
            w.set_title(Some(new_title));
        }
    }

    pub fn set_cursor_icon(&self, pointer_shape: PointerShape) {
        if let Some(w) = self.window.upgrade() {
            if pointer_shape == PointerShape::Hidden {
                w.set_cursor(None);
            } else {
                let cursor: Option<gdk4::Cursor> = pointer_shape.into();
                if let Some(cursor) = cursor {
                    w.set_cursor(Some(&cursor));
                }
            }
        }
    }

    pub fn start_drag_operation(
        &self,
        mime_types: &MimeTypes,
        action: gdk4::DragAction,
        get_data_transfer_data: FfiTransferDataGetter,
    ) -> anyhow::Result<gdk4::Drag> {
        let w = self.window.upgrade().context("Window destroyed")?;
        let last_pointer_down_event = self.last_pointer_down_event.borrow();
        let (event, _button) = last_pointer_down_event.as_ref().context("No pointer event")?;
        let device = event.device().context("Pointer event has no device")?;
        let surface = w.surface().context("Cannot get window surface")?;
        let (x, y, _modifiers) = surface.device_position(&device).context("Pointer event has no position")?;
        gdk4::Drag::begin(
            &surface,
            &device,
            &ClipboardContentProvider::new(mime_types, get_data_transfer_data, DataSource::DragAndDrop),
            action,
            x,
            y,
        )
        .context("Cannot begin drag")
    }

    pub fn start_move(&self) -> anyhow::Result<()> {
        let Some(w) = self.window.upgrade() else { return Ok(()) };
        let toplevel = get_toplevel(&w)?;

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
        let Some(w) = self.window.upgrade() else { return Ok(()) };
        let toplevel = get_toplevel(&w)?;

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
        let Some(w) = self.window.upgrade() else { return Ok(()) };
        let toplevel = get_toplevel(&w)?;
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
        do_async(self.window.clone(), move |window| {
            window.set_fullscreened(true);
        });
    }

    pub fn unset_fullscreen(&self) {
        do_async(self.window.clone(), move |window| {
            window.set_fullscreened(false);
        });
    }

    pub fn set_decoration_mode(&self, decoration_mode: WindowDecorationMode) {
        do_async(self.window.clone(), move |window| {
            window.set_decorated(decoration_mode == WindowDecorationMode::Server);
        });
    }

    pub fn focus(&self, activation_token: Option<&str>) {
        let activation_token = activation_token.map(ToOwned::to_owned);
        // Don't call `gtk4::Window::present` synchronously, because that can trigger events,
        // which then can cause reentrancy issues in the client code.
        do_async(self.window.clone(), move |window| {
            if let Some(activation_token) = &activation_token {
                window.set_startup_id(activation_token);
            }
            window.present();
        });
    }

    fn text_input_update_impl(&self, context: &TextInputContext) -> anyhow::Result<()> {
        let surrounding_text = context.surrounding_text.as_str()?;
        let cursor_index = context.cursor_codepoint_offset;
        let anchor_index = context.selection_start_codepoint_offset;
        self.im_context
            .set_surrounding_with_selection(surrounding_text, cursor_index.into(), anchor_index.into());

        self.im_context.set_input_hints(context.hints.try_into()?);
        self.im_context.set_input_purpose(context.content_purpose.into());
        self.im_context.set_cursor_location(&context.cursor_rectangle.into());
        Ok(())
    }

    pub fn text_input_enable(&self, context: &TextInputContext) -> anyhow::Result<()> {
        debug!("{:?}: text_input_enable: {context:?}", self.window_id);
        self.im_context.focus_in();
        if let Some(event_controller_key) = self.event_controller_key_weak.upgrade() {
            event_controller_key.set_im_context(Some(&self.im_context));
        }
        self.text_input_update_impl(context)
    }

    pub fn text_input_disable(&self) {
        debug!("{:?}: text_input_disable", self.window_id);
        if let Some(event_controller_key) = self.event_controller_key_weak.upgrade() {
            event_controller_key.set_im_context(gtk4::IMContext::NONE);
        }
        self.im_context.focus_out();
    }

    pub fn text_input_update(&self, context: &TextInputContext) -> anyhow::Result<()> {
        debug!("{:?}: text_input_update: {context:?}", self.window_id);
        self.text_input_update_impl(context)
    }
}
