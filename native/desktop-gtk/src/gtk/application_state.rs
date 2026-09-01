use crate::gtk::application::{send_event, with_app_state_mut};
use crate::gtk::application_api::{ApplicationCallbacks, FfiWindowCloseRequest, RenderingMode};
use crate::gtk::async_event_result::AsyncEventResult;
use crate::gtk::clipboard::{ClipboardContentProvider, KdtClipboard};
use crate::gtk::data_transfer_api::{DataSource, DragAndDropAction, DragAndDropActions};
use crate::gtk::desktop_settings::{DesktopSettings, InternalDesktopSetting};
use crate::gtk::desktop_settings_api::FfiDesktopSetting;
use crate::gtk::events::Event::DragIconFrameTick;
use crate::gtk::events::{
    DataTransferCancelledEvent, DragAndDropFeedbackFinishedEvent, DragAndDropFinishedEvent, DragIconDrawEvent, Event, EventHandler,
    ModifiersChangedEvent, NotificationClosedEvent, NotificationShownEvent, OpenGlDrawData, RequestId, WindowClosedEvent, WindowId,
};
use crate::gtk::ffi_return_conversions::{QueryDragAndDropTarget, RetrieveSurroundingText, TransferDataGetter};
use crate::gtk::file_dialog::show_file_dialog_impl;
use crate::gtk::file_dialog_api::{CommonFileDialogParams, OpenFileDialogParams, SaveFileDialogParams};
use crate::gtk::geometry::{LogicalSize, PhysicalSize, Scale};
use crate::gtk::gl_widget::GlWidget;
use crate::gtk::keyboard::key_modifiers_from_gdk;
use crate::gtk::mime_types::MimeTypes;
use crate::gtk::notifications::Notifications;
use crate::gtk::window::SimpleWindow;
use crate::gtk::window_api::WindowParams;
use anyhow::{Context, bail};
use gtk4::glib::translate::ToGlibPtr;
use gtk4::prelude::{
    ActionMapExtManual, AppInfoExt, AppLaunchContextExt, ApplicationExt, ApplicationExtManual, CancellableExt, DeviceExt, DisplayExt,
    DragExt, FileExt, GtkApplicationExt, GtkWindowExt, ObjectExt, ObjectType, SeatExt, WidgetExt, WidgetExtManual,
};
use gtk4::{gdk as gdk4, gio, glib};
use log::{debug, warn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::atomic::AtomicU32;
use std::sync::{LazyLock, OnceLock, atomic};

/// cbindgen:ignore
pub type EglInstance = khronos_egl::DynamicInstance<khronos_egl::EGL1_0>;

/// cbindgen:ignore
static EGL: LazyLock<Option<EglInstance>> = LazyLock::new(|| match unsafe { EglInstance::load() } {
    Ok(egl) => Some(egl),
    Err(e) => {
        warn!("Failed to load EGL: {e}");
        None
    }
});

/// cbindgen:ignore
pub static GL_INSTANCE: OnceLock<Option<libloading::Library>> = OnceLock::new();

pub fn get_gl(lib_path: impl AsRef<OsStr>) -> Option<&'static libloading::Library> {
    GL_INSTANCE
        .get_or_init(|| match unsafe { libloading::Library::new(lib_path.as_ref()) } {
            Ok(lib) => Some(lib),
            Err(e) => {
                warn!("Failed to load GL: {e}");
                None
            }
        })
        .as_ref()
}

pub fn get_egl() -> Option<&'static EglInstance> {
    match &*EGL {
        Some(v) => Some(v),
        None => None,
    }
}

pub struct ApplicationState {
    event_handler: EventHandler,
    window_close_request: FfiWindowCloseRequest,
    transfer_data_getter: TransferDataGetter,
    query_drag_and_drop_target: QueryDragAndDropTarget,
    retrieve_surrounding_text: RetrieveSurroundingText,
    _gtk_app_hold: gio::ApplicationHoldGuard,
    pub gtk_app: gtk4::Application,
    async_request_counter: AtomicU32,
    window_id_to_window: Rc<RefCell<HashMap<WindowId, SimpleWindow>>>,
    clipboard: KdtClipboard,
    primary_clipboard: KdtClipboard,
    current_drag: glib::WeakRef<gdk4::Drag>,
    drag_icon: RefCell<Option<GlWidget>>,
    drag_content_provider: RefCell<Option<ClipboardContentProvider>>,
    desktop_settings: DesktopSettings,
    notifications: Notifications,
    disconnect_modifier_state_notify_handler: Option<Box<dyn FnOnce()>>,
    cancellable: gio::Cancellable,
}

impl Drop for ApplicationState {
    fn drop(&mut self) {
        self.cancellable.cancel();
        for window in self.gtk_app.windows() {
            self.gtk_app.remove_window(&window);
            window.destroy();
        }
        if let Some(f) = self.disconnect_modifier_state_notify_handler.take() {
            f();
        }
        self.gtk_app.quit();
    }
}

fn create_gtk_app(display: &gdk4::Display) -> anyhow::Result<gtk4::Application> {
    let mut app_builder = gtk4::Application::builder().flags(gio::ApplicationFlags::NON_UNIQUE);

    let app_id = glib::program_name().context("glib::program_name not set")?;
    let app_id_str = app_id.as_str();
    if gio::Application::id_is_valid(app_id_str) {
        app_builder = app_builder.application_id(app_id);
    } else if gtk4::IconTheme::for_display(display).has_icon(app_id_str) {
        // In case the application ID is not valid for GTK, try manually configuring an application icon.
        gtk4::Window::set_default_icon_name(app_id_str);
    }

    Ok(app_builder.build())
}

impl ApplicationState {
    #[allow(clippy::too_many_lines)]
    pub fn new(callbacks: &ApplicationCallbacks) -> anyhow::Result<Self> {
        debug!("Initializing application state");

        let display = gdk4::Display::default().context("Unable to open display")?;
        let default_seat = display.default_seat();
        let gtk_app = create_gtk_app(&display)?;

        let application_wants_to_terminate = callbacks.application_wants_to_terminate;
        let event_handler = callbacks.event_handler;

        gtk_app.connect_activate(move |_gtk_app| {
            debug!("App Activate");
        });
        gtk_app.connect_startup(move |gtk_app| {
            debug!("App Startup");
            let quit = gio::ActionEntry::builder("quit")
                .activate(move |gtk_app: &gtk4::Application, _, _| {
                    if application_wants_to_terminate() {
                        gtk_app.quit();
                    }
                })
                .build();
            gtk_app.add_action_entries([quit]);

            let initial_settings = with_app_state_mut(|state| Ok(state.read_and_subscribe_to_desktop_settings())).unwrap();

            send_event(event_handler, Event::ApplicationStarted);
            debug!("After ApplicationStarted");

            for setting in initial_settings {
                FfiDesktopSetting::with(setting, move |setting| {
                    send_event(event_handler, Event::DesktopSettingChange(setting));
                });
            }

            if let Some(keyboard) = default_seat.as_ref().and_then(SeatExt::keyboard) {
                // Cannot use `EventControllerKey::modifiers` signal, see https://gitlab.gnome.org/GNOME/gtk/-/issues/5139
                let modifier_state_notify_handler_id = keyboard.connect_modifier_state_notify(move |keyboard| {
                    let modifiers = key_modifiers_from_gdk(keyboard.modifier_state());
                    let event = ModifiersChangedEvent { modifiers };
                    send_event(event_handler, event);
                });

                let modifiers = key_modifiers_from_gdk(keyboard.modifier_state());
                if !modifiers.is_empty() {
                    let event = ModifiersChangedEvent { modifiers };
                    send_event(event_handler, event);
                }

                with_app_state_mut(|s| {
                    s.disconnect_modifier_state_notify_handler = Some(Box::new(move || {
                        keyboard.disconnect(modifier_state_notify_handler_id);
                    }));
                    Ok(())
                })
                .unwrap();
            }
        });

        gtk_app.connect_shutdown(move |_gtk_app| {
            debug!("App Shutdown");
        });

        #[cfg(unix)]
        {
            let gtk_app = gtk_app.downgrade();
            // Handle SIGTERM
            glib_unix::unix_signal_add_local(15, move || {
                if let Some(gtk_app) = gtk_app.upgrade()
                    && application_wants_to_terminate()
                {
                    gtk_app.quit();
                }
                glib::ControlFlow::Continue
            });
        }

        let event_handler = callbacks.event_handler;
        let ffi_dealloc = callbacks.obj_dealloc;

        // If GSK decides to use Vulkan renderer, the OpenGL texture needs to be passed via DMA-BUF,
        // which is available only since GTK 4.14, and is an additional overhead.
        // Instead, force GTK to use OpenGL.
        // https://github.com/GNOME/gtk/blob/4.22.1/gsk/gskrenderer.c#L545
        unsafe {
            glib::gobject_ffi::g_object_set_qdata(
                display.as_object_ref().to_glib_none().0,
                glib::ffi::g_quark_from_string(c"gsk-renderer".as_ptr()),
                NonNull::from(c"opengl").as_ptr().cast(),
            );
        };

        let transfer_data_getter = TransferDataGetter {
            ffi_get: callbacks.get_data_transfer_data,
            ffi_dealloc,
        };

        let cancellable = gio::Cancellable::new();
        let clipboard = KdtClipboard::new(display.clipboard(), DataSource::Clipboard, event_handler, transfer_data_getter);
        let primary_clipboard = KdtClipboard::new(
            display.primary_clipboard(),
            DataSource::PrimarySelection,
            event_handler,
            transfer_data_getter,
        );
        let desktop_settings = DesktopSettings::new(display);
        let notifications = Notifications::new(cancellable.clone(), move |notification_data| {
            let e = NotificationClosedEvent::new(
                notification_data.id,
                notification_data.action.as_ref(),
                notification_data.activation_token.as_ref(),
            );
            send_event(event_handler, e);
        });

        Ok(Self {
            event_handler,
            window_close_request: callbacks.window_close_request,
            transfer_data_getter,
            query_drag_and_drop_target: QueryDragAndDropTarget {
                ffi_get: callbacks.query_drag_and_drop_target,
                ffi_dealloc,
            },
            retrieve_surrounding_text: RetrieveSurroundingText {
                ffi_get: callbacks.retrieve_surrounding_text,
                ffi_dealloc,
            },
            _gtk_app_hold: gtk_app.hold(),
            gtk_app,
            async_request_counter: AtomicU32::new(1),
            window_id_to_window: Rc::new(RefCell::new(HashMap::new())),
            clipboard,
            primary_clipboard,
            current_drag: glib::WeakRef::default(),
            drag_icon: RefCell::default(),
            drag_content_provider: RefCell::default(),
            desktop_settings,
            notifications,
            disconnect_modifier_state_notify_handler: None,
            cancellable,
        })
    }

    /// Executes the future produced by the provided function.
    /// Return value is the same as the one passed to the function, representing the request id,
    /// so that the response (optionally produced by the future) can be matched to the request.
    pub fn run_async<F>(&self, f: impl FnOnce(EventHandler, RequestId) -> F) -> RequestId
    where
        F: Future<Output = ()> + 'static,
        F::Output: 'static,
    {
        let raw_request_id = self.async_request_counter.fetch_add(1, atomic::Ordering::Relaxed);
        let request_id = RequestId(raw_request_id);
        let event_handler = self.event_handler;
        let future = f(event_handler, request_id);

        glib::spawn_future_local(future);
        request_id
    }

    pub fn read_and_subscribe_to_desktop_settings(&mut self) -> Vec<InternalDesktopSetting> {
        let event_handler = self.event_handler;
        self.desktop_settings.read_and_subscribe(move |setting| {
            FfiDesktopSetting::with(setting, move |setting| {
                send_event(event_handler, Event::DesktopSettingChange(setting));
            });
        })
    }

    pub fn new_window(&self, params: &WindowParams) -> anyhow::Result<()> {
        let window_id = params.window_id;
        let min_size = params.min_size.validate();

        if self.window_id_to_window.borrow().contains_key(&window_id) {
            bail!("Window with {window_id:?} already exists");
        }
        let event_handler = self.event_handler;
        let window_close_request = {
            let callback = self.window_close_request;
            move || {
                debug!("window_close_request {window_id:?} start");
                let close = callback(window_id);
                debug!("window_close_request {window_id:?} end ({close})");
                close
            }
        };
        let on_destroy = {
            let window_id_to_window_weak = Rc::downgrade(&self.window_id_to_window);
            move || {
                if let Some(window_id_to_window) = window_id_to_window_weak.upgrade() {
                    window_id_to_window.borrow_mut().remove(&window_id);
                }
                send_event(event_handler, WindowClosedEvent { window_id });
            }
        };
        let simple_window = SimpleWindow::new(
            &self.gtk_app,
            self.cancellable.clone(),
            window_id,
            params.size,
            params.rendering_mode,
            min_size,
            params.title.get_optional("WindowParams.title")?,
            params.decoration_mode,
            event_handler,
            self.query_drag_and_drop_target,
            self.retrieve_surrounding_text,
            window_close_request,
            on_destroy,
        );
        self.window_id_to_window.borrow_mut().insert(window_id, simple_window);
        debug!("Created window {window_id:?}");

        Ok(())
    }

    pub fn close_window(&self, window_id: WindowId) {
        if let Some(w) = self.window_id_to_window.borrow().get(&window_id) {
            w.destroy();
        }
    }

    pub fn with_window<T>(&self, window_id: WindowId, f: impl FnOnce(&SimpleWindow) -> anyhow::Result<T>) -> anyhow::Result<T> {
        let borrowed = self.window_id_to_window.borrow();
        let window = borrowed
            .get(&window_id)
            .with_context(|| format!("Couldn't find window for {window_id:?}"))?;
        f(window)
    }

    pub fn clipboard_put(&mut self, mime_types: &MimeTypes) -> anyhow::Result<()> {
        debug!("clipboard_put: mime_types={:?}", mime_types.val);
        self.clipboard.put(mime_types)
    }

    pub fn clipboard_get_available_mimetypes(&self) -> String {
        self.clipboard.mime_types_csv()
    }

    pub fn primary_selection_get_available_mimetypes(&self) -> String {
        self.primary_clipboard.mime_types_csv()
    }

    pub fn primary_selection_put(&mut self, mime_types: &MimeTypes) -> anyhow::Result<()> {
        self.primary_clipboard.put(mime_types)
    }

    pub fn primary_selection_paste(&self, serial: i32, supported_mime_types: &MimeTypes) {
        self.primary_clipboard.paste(serial, supported_mime_types, self.cancellable.clone());
    }

    pub fn clipboard_paste(&self, serial: i32, supported_mime_types: &MimeTypes) {
        self.clipboard.paste(serial, supported_mime_types, self.cancellable.clone());
    }

    pub fn start_drag(
        &self,
        window_id: WindowId,
        mime_types: &MimeTypes,
        actions: DragAndDropActions,
        drag_icon_rendering_mode: RenderingMode,
        drag_icon_size: LogicalSize,
    ) -> anyhow::Result<()> {
        let event_handler = self.event_handler;
        let provider = ClipboardContentProvider::new(mime_types, self.transfer_data_getter, DataSource::DragAndDrop, event_handler);
        let drag = self.with_window(window_id, |w| w.start_drag_operation(gdk4::DragAction::from(actions), &provider))?;
        {
            let drag_icon = self.drag_icon.clone();
            let drag_content_provider = self.drag_content_provider.clone();
            drag.add_weak_ref_notify_local(move || {
                debug!("drag object destroyed");
                *drag_icon.borrow_mut() = None;
                *drag_content_provider.borrow_mut() = None;
                send_event(event_handler, DragAndDropFeedbackFinishedEvent { window_id });
            });
        }
        drag.connect_dnd_finished(move |drag| {
            let action = drag.selected_action();
            send_event(
                event_handler,
                DragAndDropFinishedEvent {
                    window_id,
                    action: DragAndDropAction::from(action),
                },
            );
        });
        drag.connect_cancel(move |_drag, reason| {
            warn!("Drag&drop cancelled: {reason:?}");
            send_event(
                event_handler,
                DataTransferCancelledEvent {
                    data_source: DataSource::DragAndDrop,
                },
            );
        });
        *self.drag_icon.borrow_mut() = if let Some(drag_icon_size) = drag_icon_size.validate() {
            let on_draw = move |opengl_draw_data: OpenGlDrawData, physical_size: PhysicalSize, scale: Scale| {
                send_event(
                    event_handler,
                    DragIconDrawEvent {
                        opengl_draw_data,
                        physical_size,
                        scale,
                    },
                );
            };
            let gl_widget = GlWidget::new(drag_icon_rendering_mode, Some(drag_icon_size), on_draw, None);

            gl_widget.add_tick_callback(move |_gl_widget, _frame_clock| {
                send_event(event_handler, DragIconFrameTick);
                glib::ControlFlow::Continue
            });
            let icon = gtk4::DragIcon::for_drag(&drag);
            icon.set_child(Some(&gl_widget));
            Some(gl_widget)
        } else {
            None
        };
        self.current_drag.set(Some(&drag));
        *self.drag_content_provider.borrow_mut() = Some(provider);

        Ok(())
    }

    pub fn stop_drag(&self) {
        if let Some(drag) = self.current_drag.upgrade() {
            drag.emit_by_name::<()>("cancel", &[&gdk4::DragCancelReason::UserCancelled]);
        }
    }

    pub fn request_redraw_drag_icon(&self) {
        self.drag_icon.borrow().as_ref().map(GlWidget::queue_draw);
    }

    pub fn open_url(&self, url_string: &str, activation_token: Option<&str>) -> RequestId {
        debug!("application_open_url: {url_string}, activation_token={activation_token:?}");
        let raw_request_id = self.async_request_counter.fetch_add(1, atomic::Ordering::Relaxed);
        let request_id = RequestId(raw_request_id);
        gtk4::show_uri_full(
            gtk4::Window::NONE,
            url_string,
            gdk4::CURRENT_TIME,
            Some(&self.cancellable),
            move |res| {
                if let Err(e) = res {
                    warn!("Error trying to open URL for {request_id:?}: {e}");
                }
            },
        );

        request_id
    }

    fn open_file_manager_impl(path: &str, activation_token: Option<&str>, request_id: RequestId, cancellable: gio::Cancellable) {
        let file = gio::File::for_path(path);
        let uri = file.uri();
        let uris = vec![uri.as_str()];
        let file_type = file.query_file_type(gio::FileQueryInfoFlags::NOFOLLOW_SYMLINKS, Some(&cancellable));

        let app_launch_context = gdk4::DisplayManager::get()
            .default_display()
            .as_ref()
            .map(DisplayExt::app_launch_context);
        let app_info = gio::AppInfo::default_for_type("inode/directory", true);

        let args = if let Some(activation_token) = activation_token {
            glib::Variant::from((uris, activation_token))
        } else if let Some(app_info) = &app_info
            && let Some(app_launch_context) = &app_launch_context
        {
            let startup_id = app_launch_context
                .startup_notify_id(Some(app_info), std::slice::from_ref(&file))
                .unwrap_or_default();
            debug!("Opening file manager using startup ID: {startup_id}");
            glib::Variant::from((uris, startup_id.as_str()))
        } else {
            glib::Variant::from((uris, ""))
        };
        let command = if file_type == gio::FileType::Directory {
            "ShowFolders"
        } else {
            "ShowItems"
        };
        gio::bus_get(gio::BusType::Session, Some(&cancellable.clone()), move |result| match result {
            Ok(connection) => {
                connection.call(
                    Some("org.freedesktop.FileManager1"),
                    "/org/freedesktop/FileManager1",
                    "org.freedesktop.FileManager1",
                    command,
                    Some(&args),
                    None,
                    gio::DBusCallFlags::NONE,
                    -1,
                    Some(&cancellable),
                    move |result| {
                        if let Err(e) = result {
                            warn!("Error trying to open URL for {request_id:?}: {e}");
                            if let Some(app_info) = app_info {
                                _ = app_info.launch(&[file], app_launch_context.as_ref());
                            }
                        }
                    },
                );
            }
            Err(e) => {
                warn!("Error establishing DBus session connection: {e}");
                if let Some(app_info) = app_info {
                    _ = app_info.launch(&[file], app_launch_context.as_ref());
                }
            }
        });
    }

    pub fn open_file_manager(&self, path: &str, activation_token: Option<&str>) -> RequestId {
        debug!("application_open_file_manager: {path}, activation_token={activation_token:?}");
        let raw_request_id = self.async_request_counter.fetch_add(1, atomic::Ordering::Relaxed);
        let request_id = RequestId(raw_request_id);

        Self::open_file_manager_impl(path, activation_token, request_id, self.cancellable.clone());
        request_id
    }

    pub fn show_open_file_dialog(
        &self,
        window_id: WindowId,
        common_params: &CommonFileDialogParams,
        open_params: &OpenFileDialogParams,
    ) -> anyhow::Result<RequestId> {
        let Some(parent) = self.with_window(window_id, |w| Ok(w.window.upgrade()))? else {
            bail!("Window {window_id:?} not found");
        };
        let request = common_params.create_open_request(open_params, parent.as_ref())?;

        Ok(self.run_async(|event_handler, request_id| async move {
            show_file_dialog_impl(&request, move |result| {
                AsyncEventResult::FileChooserResponse(result).send_as_event(event_handler, request_id);
            });
        }))
    }

    pub fn show_save_file_dialog(
        &self,
        window_id: WindowId,
        common_params: &CommonFileDialogParams,
        save_params: &SaveFileDialogParams,
    ) -> anyhow::Result<RequestId> {
        let Some(parent) = self.with_window(window_id, |w| Ok(w.window.upgrade()))? else {
            bail!("Window {window_id:?} not found");
        };
        let request = common_params.create_save_request(save_params, parent.as_ref())?;

        Ok(self.run_async(|event_handler, request_id| async move {
            show_file_dialog_impl(&request, move |result| {
                AsyncEventResult::FileChooserResponse(result).send_as_event(event_handler, request_id);
            });
        }))
    }

    pub fn request_show_notification(&self, summary: &str, body: &str, sound_file_path: Option<&str>) -> RequestId {
        let raw_request_id = self.async_request_counter.fetch_add(1, atomic::Ordering::Relaxed);
        let request_id = RequestId(raw_request_id);
        let event_handler = self.event_handler;
        self.notifications.show_notification(summary, body, sound_file_path, move |result| {
            let notification_id = result.unwrap_or_else(|e| {
                warn!("{e}: {}", e.backtrace());
                0
            });
            let event = NotificationShownEvent {
                request_id,
                notification_id,
            };
            send_event(event_handler, event);
        });
        request_id
    }

    pub fn request_close_notification(&self, notification_id: u32) {
        self.notifications.close_notification(notification_id);
    }

    pub fn set_prefer_dark_theme(&self, value: bool) {
        self.desktop_settings.set_prefer_dark_theme(value);
    }
}
