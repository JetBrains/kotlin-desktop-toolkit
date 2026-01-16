use crate::linux::application::send_event;
use crate::linux::application_api::ApplicationCallbacks;
use crate::linux::drag_icon::DragIcon;
use crate::linux::events::{Event, WindowId};
use crate::linux::geometry::LogicalSize;
use crate::linux::window::SimpleWindow;
use crate::linux::xdg_desktop_settings::DesktopSettings;
use crate::linux::xdg_desktop_settings_api::XdgDesktopSetting;
use anyhow::bail;
use gtk4::prelude::{DisplayExt, GtkWindowExt};
use gtk4::{gdk as gdk4, glib};
use khronos_egl;
use log::{debug, warn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, LazyLock, RwLock};

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

pub fn get_egl() -> Option<&'static EglInstance> {
    match &*EGL {
        Some(v) => Some(v),
        None => None,
    }
}

pub struct ApplicationState {
    pub callbacks: ApplicationCallbacks,
    pub window_id_to_window: RefCell<HashMap<WindowId, SimpleWindow>>,
    pub clipboard: Option<gdk4::Clipboard>,
    pub primary_clipboard: Option<gdk4::Clipboard>,
    pub current_drag: glib::WeakRef<gdk4::Drag>,
    pub drag_icon: Option<DragIcon>,
    pub desktop_settings: Option<DesktopSettings>,
    pub display_manager: Option<gdk4::DisplayManager>,
    pub notifications_connection: Arc<RwLock<Option<zbus::Connection>>>,
    pub windows_to_close: Rc<RefCell<Vec<WindowId>>>,
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
            window_id_to_window: RefCell::new(HashMap::new()),
            display_manager: None,
            clipboard: None,
            primary_clipboard: None,
            current_drag: glib::WeakRef::default(),
            drag_icon: None,
            desktop_settings: None,
            notifications_connection: Arc::new(RwLock::new(None)),
            windows_to_close: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn init(&mut self) -> anyhow::Result<()> {
        let display_manager = gdk4::DisplayManager::get();
        let (clipboard, primary_clipboard) =
            with_default_display(&display_manager, |display| (display.clipboard(), display.primary_clipboard()));
        self.clipboard = Some(clipboard);
        self.primary_clipboard = Some(primary_clipboard);
        {
            let event_sender = self.callbacks.event_handler;
            let desktop_settings = DesktopSettings::new(&display_manager);
            desktop_settings.read_and_subscribe(move |e| {
                XdgDesktopSetting::with(e, |s| {
                    send_event(event_sender, Event::XdgDesktopSettingChange(s));
                });
            })?;
            self.desktop_settings = Some(desktop_settings);
        }
        self.display_manager = Some(display_manager);
        Ok(())
    }

    pub fn send_event<'a, T: Into<Event<'a>>>(&self, event_data: T) {
        let event: Event = event_data.into();
        send_event(self.callbacks.event_handler, event);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_window(
        &self,
        window_id: WindowId,
        size: LogicalSize,
        min_size: Option<LogicalSize>,
        title: Option<&str>,
        prefer_client_side_decoration: bool,
    ) -> anyhow::Result<()> {
        if let Some(w) = self.window_id_to_window.borrow().get(&window_id) {
            if w.is_unrealized() {
                w.window.destroy();
            } else {
                bail!("Window with ID {window_id:?} already exists");
            }
        }

        let window_close_request = {
            let callback = self.callbacks.window_close_request;
            let windows_to_close = self.windows_to_close.clone();
            move || {
                debug!("window_close_request {window_id:?} start");
                let close = callback(window_id);
                if close {
                    windows_to_close.borrow_mut().push(window_id);
                }
                debug!("window_close_request {window_id:?} end ({close})");
                close
            }
        };
        let simple_window = SimpleWindow::new(
            window_id,
            size,
            min_size,
            title,
            prefer_client_side_decoration,
            self.callbacks.event_handler,
            self.callbacks.query_drag_and_drop_target,
            window_close_request,
        );
        self.window_id_to_window.borrow_mut().insert(window_id, simple_window);
        debug!("Created window {window_id:?}");

        // if let Some(screen_change_event) = screen_change_event {
        //     self.send_event(screen_change_event);
        // }

        // let xdg_titlebar_event =
        //     XdgDesktopSettingChange(XdgDesktopSetting::TitlebarLayout(BorrowedStrPtr::new(c":minimize,maximize,close")));
        // self.send_event(xdg_titlebar_event);
        Ok(())
    }
}
