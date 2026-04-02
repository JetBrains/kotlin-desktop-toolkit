use crate::gtk::application::send_event;
use crate::gtk::application_api::RenderingMode;
use crate::gtk::clipboard::ClipboardContentProvider;
use crate::gtk::data_transfer::set_drag_and_drop_event_handlers;
use crate::gtk::events::{
    Event, EventHandler, OpenGlDrawData, ScreenId, WindowConfigureEvent, WindowDecorationMode, WindowDrawEvent, WindowFrameTick, WindowId,
    WindowKeyboardEnterEvent, WindowKeyboardLeaveEvent, WindowScaleChangedEvent, WindowScreenChangeEvent,
};
use crate::gtk::ffi_return_conversions::{QueryDragAndDropTarget, RetrieveSurroundingText};
use crate::gtk::geometry::{LogicalRect, LogicalSize, PhysicalSize};
use crate::gtk::gl_widget::GlWidget;
use crate::gtk::kdt_application::KdtApplication;
use crate::gtk::keyboard::set_keyboard_event_handlers;
use crate::gtk::layout_manager_wrapper::LayoutManagerWrapper;
use crate::gtk::mouse::set_mouse_event_handlers;
use crate::gtk::pointer_shapes_api::PointerShape;
use crate::gtk::text_input::create_im_context;
use crate::gtk::text_input_api::TextInputContext;
use anyhow::Context;
use gtk4::gdk as gdk4;
use gtk4::glib;
use gtk4::prelude::{
    Cast, DisplayExt, IMContextExt, NativeExt, ObjectExt, ObjectType, RootExt, SeatExt, SurfaceExt, ToplevelExt, WidgetExtManual,
};
use gtk4::prelude::{GtkWindowExt, WidgetExt};
use log::{debug, warn};
use std::cell::RefCell;
use std::rc::Rc;

impl From<LogicalRect> for gdk4::Rectangle {
    fn from(value: LogicalRect) -> Self {
        Self::new(value.x, value.y, value.width, value.height)
    }
}

fn get_toplevel(window: &gtk4::ApplicationWindow) -> anyhow::Result<gdk4::Toplevel> {
    let surface = window.surface().context("Cannot get window surface")?;
    surface.downcast().ok().context("Cannot cast surface to toplevel")
}

fn create_gl_widget(
    window_id: WindowId,
    event_handler: EventHandler,
    rendering_mode: RenderingMode,
    min_size: Option<LogicalSize>,
    last_window_configure_event: Rc<RefCell<WindowConfigureEvent>>,
) -> GlWidget {
    let on_allocate = move |logical_size: LogicalSize| {
        update_window_configure(event_handler, &last_window_configure_event, |event| {
            logical_size != event.size && {
                event.size = logical_size;
                true
            }
        });
    };

    let on_draw = move |opengl_draw_data: OpenGlDrawData, physical_size: PhysicalSize, _scale: f64| {
        send_event(
            event_handler,
            WindowDrawEvent {
                window_id,
                opengl_draw_data,
                physical_size,
            },
        );
    };
    let gl_widget = GlWidget::new(rendering_mode, min_size, on_draw, Some(Box::new(on_allocate)));

    gl_widget.add_tick_callback(move |_gl_widget, frame_clock| {
        let frame_time_microseconds = frame_clock.frame_time();
        send_event(
            event_handler,
            WindowFrameTick {
                window_id,
                frame_time_microseconds,
            },
        );
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
    fn new(window: &gtk4::ApplicationWindow, window_id: WindowId, decoration_mode: WindowDecorationMode) -> Self {
        let maximized = window.is_maximized();
        let fullscreen = window.is_fullscreen();
        let width = window.width();
        let height = window.height();

        Self {
            window_id,
            size: LogicalSize { width, height },
            active: false,
            maximized,
            fullscreen,
            decoration_mode,
            inset_start: LogicalSize::default(),
            inset_end: LogicalSize::default(),
        }
    }
}

fn set_window_configure_event_handlers(
    window: &gtk4::ApplicationWindow,
    event_handler: EventHandler,
    last_window_configure_event: Rc<RefCell<WindowConfigureEvent>>,
    overlay_controls: Rc<RefCell<Vec<gtk4::Widget>>>,
) {
    {
        let event = last_window_configure_event.clone();
        window.connect_maximized_notify(move |window| {
            update_window_configure(event_handler, &event, |event| {
                event.maximized = window.is_maximized();
                true
            });
        });
    }

    window.connect_fullscreened_notify(move |window| {
        update_window_configure(event_handler, &last_window_configure_event, |event| {
            event.fullscreen = window.is_fullscreen();
            let overlay_controls_borrow = overlay_controls.borrow();
            let controls: &Vec<_> = overlay_controls_borrow.as_ref();
            for control in controls {
                control.set_visible(!event.fullscreen);
            }
            true
        });
    });
}

fn update_window_configure(
    event_handler: EventHandler,
    last_window_configure_event: &Rc<RefCell<WindowConfigureEvent>>,
    f: impl FnOnce(&mut WindowConfigureEvent) -> bool,
) {
    if let Some(event) = {
        let mut last_configure_event = last_window_configure_event.borrow_mut();
        if f(&mut last_configure_event) {
            let inset_height = last_configure_event.inset_start.height.max(last_configure_event.inset_end.height);
            if inset_height > 0 {
                last_configure_event.decoration_mode = WindowDecorationMode::CustomTitlebar(inset_height);
            }
            let mut value = last_configure_event.clone();
            if value.size.width == 0 || value.size.height == 0 {
                None
            } else {
                if value.fullscreen {
                    value.inset_start = LogicalSize {
                        width: 0,
                        height: inset_height,
                    };
                    value.inset_end = LogicalSize {
                        width: 0,
                        height: inset_height,
                    };
                }
                Some(Event::WindowConfigure(value))
            }
        } else {
            None
        }
    } {
        send_event(event_handler, event);
    }
}

fn report_alloc_size(
    widget: &gtk4::Widget,
    event_handler: EventHandler,
    last_window_configure_event: Rc<RefCell<WindowConfigureEvent>>,
    f: impl Fn(&mut WindowConfigureEvent, LogicalSize) -> bool + 'static,
) {
    LayoutManagerWrapper::wrap(widget, move |alloc_size| {
        update_window_configure(event_handler, &last_window_configure_event, |e| f(e, alloc_size));
    });
}

fn on_realize(
    window: &gtk4::ApplicationWindow,
    window_id: WindowId,
    event_handler: EventHandler,
    last_window_configure_event: Rc<RefCell<WindowConfigureEvent>>,
) {
    let scale = window.scale_factor();
    let scale_event = WindowScaleChangedEvent {
        window_id,
        new_scale: scale.into(),
    };
    send_event(event_handler, scale_event);

    update_window_configure(event_handler, &last_window_configure_event, |event| {
        let toplevel = get_toplevel(window).unwrap();
        let toplevel_state = toplevel.state();
        debug!("window state={:b}", toplevel_state.bits());

        event.active = toplevel_state.contains(gdk4::ToplevelState::FOCUSED);
        true
    });

    let toplevel = get_toplevel(window).unwrap();

    if let Some(monitor) = toplevel.display().monitor_at_surface(&toplevel) {
        let new_screen_id = ScreenId::new(&monitor);
        let event = WindowScreenChangeEvent { window_id, new_screen_id };
        send_event(event_handler, event);
    }

    toplevel.connect_state_notify(move |toplevel| {
        let state = toplevel.state();
        let active = state.contains(gdk4::ToplevelState::FOCUSED);

        update_window_configure(event_handler, &last_window_configure_event, |event| {
            active != event.active && {
                event.active = active;
                true
            }
        });
    });
    toplevel.connect_enter_monitor(move |_toplevel, monitor| {
        let new_screen_id = ScreenId::new(monitor);
        let event = WindowScreenChangeEvent { window_id, new_screen_id };
        send_event(event_handler, event);
    });
}

fn set_custom_titlebar(
    window: &gtk4::ApplicationWindow,
    height: i32,
    event_handler: EventHandler,
    overlay: gtk4::Overlay,
    last_window_configure_event: &Rc<RefCell<WindowConfigureEvent>>,
    overlay_controls: &mut Vec<gtk4::Widget>,
) {
    let fake_titlebar = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    window.set_titlebar(Some(&fake_titlebar));
    fake_titlebar.set_visible(false);
    fake_titlebar.remove_css_class("titlebar");
    overlay_controls.push(fake_titlebar.upcast());

    let window_controls_start = gtk4::WindowControls::new(gtk4::PackType::Start);
    report_alloc_size(
        window_controls_start.upcast_ref(),
        event_handler,
        last_window_configure_event.clone(),
        move |event, alloc_size| {
            event.inset_start != alloc_size && {
                event.inset_start = alloc_size;
                true
            }
        },
    );

    let window_controls_end = gtk4::WindowControls::new(gtk4::PackType::End);
    report_alloc_size(
        window_controls_end.upcast_ref(),
        event_handler,
        last_window_configure_event.clone(),
        move |event, alloc_size| {
            event.inset_end != alloc_size && {
                event.inset_end = alloc_size;
                true
            }
        },
    );

    let titlebar_layout = gtk4::CenterBox::new();
    titlebar_layout.set_start_widget(Some(&window_controls_start));
    titlebar_layout.set_end_widget(Some(&window_controls_end));
    overlay_controls.push(window_controls_start.upcast());
    overlay_controls.push(window_controls_end.upcast());
    titlebar_layout.set_size_request(height, height);

    let window_handle = gtk4::WindowHandle::builder().child(&titlebar_layout).build();
    window_handle.set_valign(gtk4::Align::Start);
    window_handle.set_halign(gtk4::Align::Fill);
    window_handle.set_hexpand(true);
    overlay_controls.push(titlebar_layout.upcast());

    overlay.add_overlay(&window_handle);
    overlay_controls.push(window_handle.upcast());
    overlay_controls.push(overlay.upcast());
}

pub struct SimpleWindow {
    pub window_id: WindowId,
    pub window: glib::WeakRef<gtk4::ApplicationWindow>,
    _overlay_controls: Rc<RefCell<Vec<gtk4::Widget>>>,
    gl_widget: glib::WeakRef<GlWidget>,
    event_controller_key_weak: glib::WeakRef<gtk4::EventControllerKey>,
    im_context: gtk4::IMMulticontext,
}

impl SimpleWindow {
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn new(
        gtk_application: &KdtApplication,
        window_id: WindowId,
        size: LogicalSize,
        rendering_mode: RenderingMode,
        min_size: Option<LogicalSize>,
        title: Option<&str>,
        decoration_mode: WindowDecorationMode,
        event_handler: EventHandler,
        query_drag_and_drop_target: QueryDragAndDropTarget,
        retrieve_surrounding_text: RetrieveSurroundingText,
        window_close_request: impl Fn() -> bool + 'static,
        on_destroy: impl Fn() + 'static,
    ) -> Self {
        let mut window_builder = gtk4::ApplicationWindow::builder()
            .application(gtk_application)
            .resizable(true)
            .default_width(size.width)
            .default_height(size.height);
        if let Some(title) = title {
            window_builder = window_builder.title(title);
        }
        let window = window_builder.build();

        let last_window_configure_event = Rc::new(RefCell::new(WindowConfigureEvent::new(&window, window_id, decoration_mode)));
        let overlay_controls = Rc::new(RefCell::new(Vec::new()));

        let gl_widget = create_gl_widget(
            window_id,
            event_handler,
            rendering_mode,
            min_size,
            last_window_configure_event.clone(),
        );

        match decoration_mode {
            WindowDecorationMode::Server => {
                window.set_child(Some(&gl_widget));
                set_mouse_event_handlers(gl_widget.upcast_ref(), window_id, event_handler);
            }
            WindowDecorationMode::CustomTitlebar(height) => {
                let overlay = gtk4::Overlay::builder().child(&gl_widget).build();
                window.set_child(Some(&overlay));
                set_mouse_event_handlers(overlay.upcast_ref(), window_id, event_handler);
                set_custom_titlebar(
                    &window,
                    height,
                    event_handler,
                    overlay,
                    &last_window_configure_event,
                    overlay_controls.borrow_mut().as_mut(),
                );
            }
        }

        set_drag_and_drop_event_handlers(&gl_widget, window_id, event_handler, query_drag_and_drop_target);
        let event_controller_key = set_keyboard_event_handlers(window_id, event_handler);
        let event_controller_key_weak = event_controller_key.downgrade();
        window.add_controller(event_controller_key);

        let im_context = create_im_context(window_id, event_handler, retrieve_surrounding_text);
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

        set_window_configure_event_handlers(
            &window,
            event_handler,
            last_window_configure_event.clone(),
            overlay_controls.clone(),
        );

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

        window.connect_realize(move |window| on_realize(window, window_id, event_handler, last_window_configure_event.clone()));

        window.connect_close_request(move |_window| {
            if window_close_request() {
                // All the references to the window have to be destroyed before returning this
                glib::Propagation::Proceed
            } else {
                glib::Propagation::Stop
            }
        });

        {
            let im_context_weak_ref = im_context.downgrade();
            window.connect_unrealize(move |_window| {
                if let Some(im_context) = im_context_weak_ref.upgrade() {
                    im_context.set_client_widget(gtk4::Widget::NONE);
                }
            });
        }

        window.add_weak_ref_notify_local(move || {
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
            _overlay_controls: overlay_controls,
            gl_widget: gl_widget.downgrade(),
            event_controller_key_weak,
            im_context,
        }
    }

    pub fn destroy(&self) {
        // Close the window asynchronously, because closing can trigger events, which then can cause reentrancy issues.
        let window_id = self.window_id;
        do_async(self.window.clone(), move |window| {
            debug!("Calling GTKWindow.destroy for {window_id:?}");
            window.destroy();
        });
    }

    pub fn request_redraw(&self) {
        // Do this async; otherwise, in some cases, it prevents `application_run_on_event_loop_async` from working properly.
        do_async(self.gl_widget.clone(), move |gl_widget| {
            gl_widget.queue_draw();
        });
    }

    pub fn maximize(&self) {
        if let Some(window) = self.window.upgrade() {
            window.maximize();
        }
    }

    pub fn unmaximize(&self) {
        if let Some(window) = self.window.upgrade() {
            window.unmaximize();
        }
    }

    pub fn minimize(&self) {
        if let Some(window) = self.window.upgrade() {
            window.minimize();
        }
    }

    pub fn set_min_size(&self, size: LogicalSize) {
        if let Some(gl_widget) = self.gl_widget.upgrade() {
            gl_widget.set_size_request(size.width, size.height);
        }
    }

    pub fn set_title(&self, new_title: &str) {
        if let Some(window) = self.window.upgrade() {
            window.set_title(Some(new_title));
        }
    }

    pub fn set_cursor_icon(&self, pointer_shape: PointerShape) {
        if let Some(w) = self.window.upgrade() {
            let cursor: Option<gdk4::Cursor> = pointer_shape.into();
            if let Some(cursor) = cursor {
                w.set_cursor(Some(&cursor));
            } else {
                warn!("Failed to set pointer shape {pointer_shape:?}");
            }
        }
    }

    pub fn start_drag_operation(&self, action: gdk4::DragAction, provider: &ClipboardContentProvider) -> anyhow::Result<gdk4::Drag> {
        let w = self.window.upgrade().context("Window destroyed")?;
        let surface = w.surface().context("Cannot get window surface")?;
        let seat = RootExt::display(&w).default_seat().context("No default seat")?;
        let device = seat.pointer().context("Seat doesn't have pointer")?;
        gdk4::Drag::begin(&surface, &device, provider, action, 0., 0.).context("Cannot begin drag")
    }

    pub fn set_fullscreen(&self) {
        if let Some(window) = self.window.upgrade() {
            window.set_fullscreened(true);
        }
    }

    pub fn unset_fullscreen(&self) {
        if let Some(window) = self.window.upgrade() {
            window.set_fullscreened(false);
        }
    }

    pub fn focus(&self, activation_token: Option<&str>) {
        if let Some(window) = self.window.upgrade() {
            if let Some(activation_token) = activation_token {
                window.set_startup_id(activation_token);
            }
            window.present();
        }
    }

    fn text_input_update_impl(&self, context: &TextInputContext) -> anyhow::Result<()> {
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
