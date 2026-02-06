use crate::gtk::application::send_event;
use crate::gtk::application_api::{ApplicationCallbacks, DataSource};
use crate::gtk::clipboard::KdtClipboard;
use crate::gtk::drag_icon::DragIcon;
use crate::gtk::events::{Event, WindowClosedEvent, WindowId};
use crate::gtk::geometry::LogicalSize;
use crate::gtk::kdt_application::KdtApplication;
use crate::gtk::window::SimpleWindow;
use crate::gtk::xdg_desktop_settings::DesktopSettings;
use crate::gtk::xdg_desktop_settings_api::XdgDesktopSetting;
use gtk4::prelude::DisplayExt;
use gtk4::{gdk as gdk4, glib};
use log::{debug, warn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::rc::Rc;
use std::sync::{Arc, LazyLock, OnceLock, RwLock};

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
        .get_or_init(|| match unsafe { libloading::Library::new(lib_path) } {
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
    pub callbacks: ApplicationCallbacks,
    pub window_id_to_window: Rc<RefCell<HashMap<WindowId, SimpleWindow>>>,
    pub clipboard: Option<KdtClipboard>,
    pub primary_clipboard: Option<KdtClipboard>,
    pub current_drag: glib::WeakRef<gdk4::Drag>,
    pub drag_icon: Option<DragIcon>,
    pub desktop_settings: Option<DesktopSettings>,
    pub display_manager: Option<gdk4::DisplayManager>,
    pub notifications_connection: Arc<RwLock<Option<zbus::Connection>>>,
}

pub fn with_default_display<T>(display_manager: &gdk4::DisplayManager, f: impl FnOnce(&gdk4::Display) -> T) -> T {
    if let Some(display) = display_manager.default_display() {
        f(&display)
    } else {
        let all_displays = display_manager.list_displays();
        let display = all_displays.first().unwrap();
        f(display)
    }
}

impl ApplicationState {
    pub fn new(callbacks: ApplicationCallbacks) -> Self {
        Self {
            callbacks,
            window_id_to_window: Rc::new(RefCell::new(HashMap::new())),
            display_manager: None,
            clipboard: None,
            primary_clipboard: None,
            current_drag: glib::WeakRef::default(),
            drag_icon: None,
            desktop_settings: None,
            notifications_connection: Arc::new(RwLock::new(None)),
        }
    }

    pub fn init(&mut self) -> anyhow::Result<()> {
        let event_handler = self.callbacks.event_handler;
        let get_data_transfer_data = self.callbacks.get_data_transfer_data;
        let display_manager = gdk4::DisplayManager::get();
        let (clipboard, primary_clipboard) =
            with_default_display(&display_manager, |display| (display.clipboard(), display.primary_clipboard()));
        self.clipboard = Some(KdtClipboard::new(
            clipboard,
            DataSource::Clipboard,
            event_handler,
            get_data_transfer_data,
        ));
        self.primary_clipboard = Some(KdtClipboard::new(
            primary_clipboard,
            DataSource::PrimarySelection,
            event_handler,
            get_data_transfer_data,
        ));
        {
            let desktop_settings = DesktopSettings::new(&display_manager);
            desktop_settings.read_and_subscribe(move |e| {
                XdgDesktopSetting::with(e, |s| {
                    send_event(event_handler, Event::XdgDesktopSettingChange(s));
                });
            })?;
            self.desktop_settings = Some(desktop_settings);
        }
        self.display_manager = Some(display_manager);
        Ok(())
    }

    pub fn create_window(
        &self,
        gtk_application: &KdtApplication,
        window_id: WindowId,
        size: LogicalSize,
        min_size: Option<LogicalSize>,
        title: Option<&str>,
        prefer_client_side_decoration: bool,
    ) {
        let event_handler = self.callbacks.event_handler;
        let window_close_request = {
            let callback = self.callbacks.window_close_request;
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
            gtk_application,
            window_id,
            size,
            min_size,
            title,
            prefer_client_side_decoration,
            event_handler,
            self.callbacks.query_drag_and_drop_target,
            window_close_request,
            on_destroy,
        );
        self.window_id_to_window.borrow_mut().insert(window_id, simple_window);
        debug!("Created window {window_id:?}");

        // if let Some(screen_change_event) = screen_change_event {
        //     self.send_event(screen_change_event);
        // }

        // let xdg_titlebar_event =
        //     XdgDesktopSettingChange(XdgDesktopSetting::TitlebarLayout(BorrowedStrPtr::new(c":minimize,maximize,close")));
        // self.send_event(xdg_titlebar_event);
    }
}
