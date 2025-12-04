use anyhow::{Context, anyhow, bail};
use log::{debug, trace, warn};
use std::sync::{mpsc, Mutex};
use std::sync::mpsc::Sender;
use std::thread::ThreadId;
use sdl3_sys::events::{SDL_AddEventWatch, SDL_Event, SDL_EventType, SDL_PollEvent, SDL_EVENT_DISPLAY_CONTENT_SCALE_CHANGED, SDL_EVENT_DISPLAY_ORIENTATION, SDL_EVENT_WINDOW_HDR_STATE_CHANGED, SDL_EVENT_WINDOW_SHOWN};
use sdl3_sys::everything::SDL_HITTEST_NORMAL;
use sdl3_sys::init::{SDL_InitSubSystem, SDL_Quit, SDL_INIT_VIDEO};
// use sdl3_main::app::::event_loop::{ControlFlow, EventLoopProxy};
// use winit_x11::EventLoop;
// use tokio::spawn;

use crate::linux::application_state::ApplicationState;
use crate::linux::events::{DataTransferContent, DataTransferEvent};
use crate::linux::mime_types::MimeTypes;
use crate::linux::{
    application_api::ApplicationCallbacks,
    events::{Event, WindowId},
    geometry::LogicalPoint,
    window::SimpleWindow,
    window_api::WindowParams,
    window_resize_edge_api::WindowResizeEdge,
};
use desktop_common::logger::catch_panic;

pub struct Application {
    // pub app: GpuiApplication,
    // qh: QueueHandle<ApplicationState>,
    pub exit: bool,
    pub state: ApplicationState,
    // event_loop_proxy: Option<EventLoopProxy>,
    // pub run_on_event_loop: Option<Sender<extern "C" fn()>>,
    pub event_loop_thread_id: Option<ThreadId>,
    // rt: tokio::runtime::Runtime,
    // async_request_counter: u32,
    // run_async_sender: Sender<AsyncEventResult>,
}

impl ApplicationCallbacks {
    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    pub fn send_event<'a, T: Into<Event<'a>>>(&self, event_data: T) -> bool {
        let event: Event = event_data.into();
        match event {
            Event::MouseMoved(_) | Event::WindowDraw(_) | Event::DragIconDraw(_) => {}
            _ => debug!("Sending event: {event:?}"),
        }
        catch_panic(|| Ok((self.event_handler)(&event))).unwrap_or(false)
    }
}

//
// fn create_run_async_sender(event_loop: &EventLoop<'static, ApplicationState>) -> Sender<AsyncEventResult> {
//     let (sender, c) = channel::channel();
//
//     event_loop
//         .handle()
//         .insert_source(c, move |event: channel::Event<AsyncEventResult>, (), state| {
//             if let channel::Event::Msg(e) = event {
//                 e.send_as_event(state.callbacks.event_handler);
//             }
//         })
//         .unwrap();
//
//     sender
// }

impl Drop for Application {
    fn drop(&mut self) {
        unsafe { SDL_Quit() };
    }
}

impl Application {
    pub fn new(callbacks: ApplicationCallbacks) -> anyhow::Result<Self> {
        let state = ApplicationState::new(callbacks);

        // let rt = tokio::runtime::Builder::new_multi_thread().enable_io().worker_threads(1).build()?;
        // let run_async_sender = create_run_async_sender(&event_loop);

        Ok(Self {
            exit: false,
            state,
            // event_loop_proxy: None,
            // run_on_event_loop: None,
            event_loop_thread_id: None,
            // rt,
            // async_request_counter: 0,
            // run_async_sender,
        })
    }
    //
    // /// Executes the future produced by the provided function.
    // /// Return value is the same as the one passed to the function, representing the request id,
    // /// so that the response (optionally produced by the future) can be matched to the request.
    // pub fn run_async<F>(&mut self, f: impl FnOnce(RequestId) -> F) -> RequestId
    // where
    //     F: Future<Output = AsyncEventResult> + Send + 'static,
    //     F::Output: Send + 'static,
    // {
    //     self.async_request_counter = self.async_request_counter.wrapping_add(1);
    //     let request_id = RequestId(self.async_request_counter);
    //     let future = f(request_id);
    //     let sender = self.run_async_sender.clone();
    //
    //     self.rt.spawn(async move {
    //         sender.send(future.await).unwrap();
    //     });
    //     request_id
    // }
    //
    // fn init_xdg_desktop_settings_notifier(&self) {
    //     let (xdg_settings_sender, xdg_settings_channel) = channel::channel();
    //     self.rt.spawn(xdg_desktop_settings_notifier(move |s| {
    //         xdg_settings_sender.send(s).map_err(Into::into)
    //     }));
    //
    //     self.event_loop
    //         .handle()
    //         .insert_source(xdg_settings_channel, move |event, (), state| {
    //             if let channel::Event::Msg(e) = event {
    //                 XdgDesktopSetting::with(e, |s| {
    //                     state.send_event(Event::XdgDesktopSettingChange(s));
    //                 });
    //             }
    //         })
    //         .unwrap();
    // }

    pub fn run(&mut self) -> Result<(), anyhow::Error> {
        debug!("Application event loop: starting");

        if !unsafe { SDL_InitSubSystem(SDL_INIT_VIDEO) } {
            bail!("SDL_InitSubSystem failed");
        }
        // self.init_notifications();
        // self.init_xdg_desktop_settings_notifier();
        // let event_loop = EventLoop::new()?;
        // let active_event_loop = event_loop.window_target();
        // active_event_loop.set_control_flow(ControlFlow::Wait);

        // self.event_loop_proxy = Some(active_event_loop.create_proxy());
        self.event_loop_thread_id = Some(std::thread::current().id());

        self.state.send_event(Event::ApplicationStarted);
        while !self.exit {
            let mut event = SDL_Event::default();
            trace!("SDL_PollEvent");
            while unsafe { SDL_PollEvent(&raw mut event) } {
                self.handle_event(event);
            }
            trace!("Received SDL_Event");
        }

        debug!("Application event loop: stopped");
        Ok(())
    }

    pub fn handle_event(&mut self, event: SDL_Event) {
        let event_type = unsafe { event.r#type };
        debug!("Received event: {event_type:?}");
        match SDL_EventType(event_type) {
            SDL_EventType::QUIT => self.exit = true,
            SDL_EventType::TERMINATING => {}
            _ => self.state.handle_event(event),
        }
        // /// Display has been added to the system
        // pub const DISPLAY_ADDED: Self = Self(338);
        // /// Display has been removed from the system
        // pub const DISPLAY_REMOVED: Self = Self(339);
        // /// Display has changed position
        // pub const DISPLAY_MOVED: Self = Self(340);
        // /// Display has changed desktop mode
        // pub const DISPLAY_DESKTOP_MODE_CHANGED: Self = Self(341);
        // /// Display has changed current mode
        // pub const DISPLAY_CURRENT_MODE_CHANGED: Self = Self(342);
        // /// Display has changed content scale
        // pub const DISPLAY_CONTENT_SCALE_CHANGED: Self = Self(343);
        // /// Window has been shown
        // pub const WINDOW_SHOWN: Self = Self(0x202);
        // /// Window has been hidden
        // pub const WINDOW_HIDDEN: Self = Self(515);
        // /// Window has been exposed and should be redrawn, and can be redrawn directly from event watchers for this event
        // pub const WINDOW_EXPOSED: Self = Self(516);
        // /// Window has been moved to data1, data2
        // pub const WINDOW_MOVED: Self = Self(517);
        // /// Window has been resized to data1xdata2
        // pub const WINDOW_RESIZED: Self = Self(518);
        // /// The pixel size of the window has changed to data1xdata2
        // pub const WINDOW_PIXEL_SIZE_CHANGED: Self = Self(519);
        // /// The pixel size of a Metal view associated with the window has changed
        // pub const WINDOW_METAL_VIEW_RESIZED: Self = Self(520);
        // /// Window has been minimized
        // pub const WINDOW_MINIMIZED: Self = Self(521);
        // /// Window has been maximized
        // pub const WINDOW_MAXIMIZED: Self = Self(522);
        // /// Window has been restored to normal size and position
        // pub const WINDOW_RESTORED: Self = Self(523);
        // /// Window has gained mouse focus
        // pub const WINDOW_MOUSE_ENTER: Self = Self(524);
        // /// Window has lost mouse focus
        // pub const WINDOW_MOUSE_LEAVE: Self = Self(525);
        // /// Window has gained keyboard focus
        // pub const WINDOW_FOCUS_GAINED: Self = Self(526);
        // /// Window has lost keyboard focus
        // pub const WINDOW_FOCUS_LOST: Self = Self(527);
        // /// The window manager requests that the window be closed
        // pub const WINDOW_CLOSE_REQUESTED: Self = Self(528);
        // /// Window had a hit test that wasn't [`SDL_HITTEST_NORMAL`]
        // pub const WINDOW_HIT_TEST: Self = Self(529);
        // /// The ICC profile of the window's display has changed
        // pub const WINDOW_ICCPROF_CHANGED: Self = Self(530);
        // /// Window has been moved to display data1
        // pub const WINDOW_DISPLAY_CHANGED: Self = Self(531);
        // /// Window display scale has been changed
        // pub const WINDOW_DISPLAY_SCALE_CHANGED: Self = Self(532);
        // /// The window safe area has been changed
        // pub const WINDOW_SAFE_AREA_CHANGED: Self = Self(533);
        // /// The window has been occluded
        // pub const WINDOW_OCCLUDED: Self = Self(534);
        // /// The window has entered fullscreen mode
        // pub const WINDOW_ENTER_FULLSCREEN: Self = Self(535);
        // /// The window has left fullscreen mode
        // pub const WINDOW_LEAVE_FULLSCREEN: Self = Self(536);
        // /// The window with the associated ID is being or has been destroyed. If this message is being handled
        // /// in an event watcher, the window handle is still valid and can still be used to retrieve any properties
        // /// associated with the window. Otherwise, the handle has already been destroyed and all resources
        // /// associated with it are invalid
        // pub const WINDOW_DESTROYED: Self = Self(537);
        // /// Window HDR properties have changed
        // pub const WINDOW_HDR_STATE_CHANGED: Self = Self(538);
        // /// Key pressed
        // pub const KEY_DOWN: Self = Self(0x300);
        // /// Key released
        // pub const KEY_UP: Self = Self(769);
        // /// Keyboard text editing (composition)
        // pub const TEXT_EDITING: Self = Self(770);
        // /// Keyboard text input
        // pub const TEXT_INPUT: Self = Self(771);
        // /// Keymap changed due to a system event such as an
        // /// input language or keyboard layout change.
        // pub const KEYMAP_CHANGED: Self = Self(772);
        // /// A new keyboard has been inserted into the system
        // pub const KEYBOARD_ADDED: Self = Self(773);
        // /// A keyboard has been removed
        // pub const KEYBOARD_REMOVED: Self = Self(774);
        // /// Keyboard text editing candidates
        // pub const TEXT_EDITING_CANDIDATES: Self = Self(775);
        // /// Mouse moved
        // pub const MOUSE_MOTION: Self = Self(0x400);
        // /// Mouse button pressed
        // pub const MOUSE_BUTTON_DOWN: Self = Self(1025);
        // /// Mouse button released
        // pub const MOUSE_BUTTON_UP: Self = Self(1026);
        // /// Mouse wheel motion
        // pub const MOUSE_WHEEL: Self = Self(1027);
    }


    pub fn new_window(&mut self, params: &WindowParams) -> anyhow::Result<()> {
        let window_id = params.window_id;
        if self.state.window_id_to_sdl_window_id.contains_key(&window_id) {
            return Err(anyhow!("Window with ID {window_id:?} already exists"));
        }

        let min_size = if params.min_size.height.0 == 0.0 {
            None
        } else {
            Some(params.min_size)
        };

        self.state.create_window(
            window_id,
            params.rect,
            min_size,
            params.title.as_optional_cstr(),
            params.app_id.as_str()?.to_owned(),
            params.prefer_client_side_decoration,
            params.rendering_mode,
        )
    }

    pub fn close_window(&mut self, window_id: WindowId) {
        if let Some(winit_window_id) = self.state.window_id_to_sdl_window_id.remove(&window_id) {
            self.state.windows.remove(&winit_window_id);
        }
    }

    pub fn get_window(&self, window_id: WindowId) -> anyhow::Result<&SimpleWindow> {
        self.state
            .window_id_to_sdl_window_id
            .get(&window_id)
            .and_then(|surface_id| self.state.windows.get(surface_id))
            .with_context(|| format!("Couldn't find window for {window_id:?}"))
    }

    pub fn get_window_mut(&mut self, window_id: WindowId) -> anyhow::Result<&mut SimpleWindow> {
        self.state
            .window_id_to_sdl_window_id
            .get(&window_id)
            .and_then(|surface_id| self.state.windows.get_mut(surface_id))
            .with_context(|| format!("Couldn't find window for {window_id:?}"))
    }
    //
    // pub fn set_cursor_theme(&mut self, name: &str, size: u32) -> anyhow::Result<()> {
    //     self.state.set_cursor_theme(&self.qh, name, size)
    // }

    // fn clipboard_put_impl(&mut self, mime_types: MimeTypes, kind: LinuxClipboardKind) {
    //     let Some(clipboard) = &mut self.state.clipboard else {
    //         warn!("application_clipboard_put: clipboard not initialized");
    //         return;
    //     };
    //     let mut clipboard_setter = clipboard.set(kind);
    //     for mime_type in mime_types.val {
    //         clipboard_setter.custom_format(mime_type);
    //     }
    //     if let Err(e) = clipboard_setter.commit() {
    //         warn!("application_clipboard_put: {e}");
    //     }
    // }

    pub fn clipboard_put(&mut self, mime_types: MimeTypes) {
        // self.clipboard_put_impl(mime_types, LinuxClipboardKind::Clipboard)
    }

    pub fn clipboard_get_available_mimetypes(&self) -> Option<String> {
        // let clipboard = self.state.clipboard.as_ref()?;
        // let mime_types = match clipboard.available_formats() {
        //     Ok(mime_types) => mime_types,
        //     Err(err) => {
        //         warn!("application_clipboard_get_available_mimetypes: {err}");
        //         return None;
        //     }
        // };
        // Some(mime_types.join(","))
        None
    }

    // pub fn primary_selection_get_available_mimetypes(&self) -> Option<String> {
    //     let Some(device) = self.state.primary_selection_device.as_ref() else {
    //         warn!("application_primary_selection_get_available_mimetypes: No primary selection device");
    //         return None;
    //     };
    //     let Some(selection_offer) = device.data().selection_offer() else {
    //         debug!("application_primary_selection_get_available_mimetypes: No selection offer found");
    //         return None;
    //     };
    //     selection_offer.with_mime_types(|mime_types| Some(mime_types.join(",")))
    // }

    pub fn primary_selection_put(&mut self, mime_types: MimeTypes) {
        // self.clipboard_put_impl(mime_types, LinuxClipboardKind::Primary)
    }
    //
    // fn clipboard_paste_impl(&self, serial: i32, supported_mime_types: &str, kind: LinuxClipboardKind) -> bool {
    //     let Some(clipboard) = self.state.clipboard.as_ref() else {
    //         warn!("application_clipboard_paste: clipboard not initialized");
    //         return false;
    //     };
    //     for mime_type in supported_mime_types.split(',') {
    //         if let Ok(content) = clipboard.get(kind).custom_format(mime_type) {
    //             if let Err(e) = self.user_event(UserEvents::ClipboardReceived {
    //                 serial,
    //                 mime_type: mime_type.to_owned(),
    //                 content,
    //             }) {
    //                 warn!("application_clipboard_paste: {e}");
    //                 return false;
    //             }
    //             return true;
    //         }
    //     }
    //     false
    // }

    pub fn primary_selection_paste(&self, serial: i32, supported_mime_types: &str) -> bool {
        false
        // self.clipboard_paste_impl(serial, supported_mime_types, LinuxClipboardKind::Primary)
    }

    pub fn clipboard_paste(&self, serial: i32, supported_mime_types: &str) -> bool {
        false
        // self.clipboard_paste_impl(serial, supported_mime_types, LinuxClipboardKind::Clipboard)
    }

    // pub fn start_drag(
    //     &mut self,
    //     window_id: WindowId,
    //     mime_types: MimeTypes,
    //     action: DndAction,
    //     drag_icon_rendering_mode: RenderingMode,
    //     drag_icon_size: LogicalSize,
    // ) -> anyhow::Result<()> {
    //     if mime_types.val.is_empty() {
    //         self.state.drag_source = None;
    //         return Ok(());
    //     }
    //
    //     let mut drag_icon = if drag_icon_size.width > 0 && drag_icon_size.height > 0 {
    //         let egl = match drag_icon_rendering_mode {
    //             RenderingMode::Auto | RenderingMode::EGL => get_egl(),
    //             RenderingMode::Software => None,
    //         };
    //
    //         Some(DragIcon::new(
    //             &self.state,
    //             &self.qh,
    //             &self.state.shm_state,
    //             &self.state.wl_display,
    //             drag_icon_size,
    //             egl,
    //         )?)
    //     } else {
    //         None
    //     };
    //
    //     let origin = self
    //         .get_window(window_id)
    //         .with_context(|| format!("No window found {window_id:?}"))?
    //         .window
    //         .wl_surface();
    //
    //     let drag_source = self
    //         .state
    //         .data_device_manager_state
    //         .create_drag_and_drop_source(&self.qh, mime_types.val, action);
    //
    //     let device = self.state.data_device.as_ref().context("No data device found")?;
    //
    //     // Required to have a mouse button pressed serial, e.g.:
    //     // https://gitlab.gnome.org/GNOME/mutter/-/blob/607a7aef5f02d3213b5e436d11440997478a4ecc/src/wayland/meta-wayland-data-device.c#L894
    //     let (_seat, serial) = self
    //         .state
    //         .get_latest_pointer_button_seat_and_serial()
    //         .context("Called start_drag without an implicit grab")?;
    //
    //     let wl_surface = drag_icon.as_ref().map(|drag_icon| drag_icon.surface.wl_surface());
    //     drag_source.start_drag(device, origin, wl_surface, serial);
    //     if let Some(drag_icon) = &mut drag_icon {
    //         drag_icon.draw(&self.qh, &|_e| true);
    //     }
    //
    //     self.state.drag_icon = drag_icon;
    //     self.state.current_drag_source_window_id = Some(window_id);
    //     self.state.drag_source = Some(drag_source);
    //
    //     Ok(())
    // }

    // pub fn request_internal_activation_token(&self, source_window_id: WindowId) -> anyhow::Result<u32> {
    //     let source_w: &SimpleWindow = self
    //         .get_window(source_window_id)
    //         .with_context(|| format!("No window found {source_window_id:?}"))?;
    //     let Some(xdg_activation) = &self.state.xdg_activation else {
    //         warn!("xdg_activation not found");
    //         return Ok(0);
    //     };
    //     // Serial should be from the latest keyboard or mouse button event, e.g.:
    //     // https://gitlab.gnome.org/GNOME/mutter/-/blob/607a7aef5f02d3213b5e436d11440997478a4ecc/src/wayland/meta-wayland-activation.c#L302
    //     // https://invent.kde.org/plasma/kwin/-/blob/271eae7f21326b48e67de1ed218caf3bdf96a9c5/src/activation.cpp#L640
    //     let Some((seat, serial)) = self.state.get_latest_event_seat_and_serial() else {
    //         return Ok(0);
    //     };
    //     let request_id = serial + 1; // aligned with `impl ActivationHandler for ApplicationState`
    //     xdg_activation.request_token(
    //         &self.qh,
    //         smithay_client_toolkit::activation::RequestData {
    //             app_id: Some(source_w.app_id.clone()),
    //             seat_and_serial: Some((seat.clone(), serial)),
    //             surface: Some(source_w.window.wl_surface().clone()),
    //         },
    //     );
    //     Ok(request_id)
    // }

    pub fn window_activate(&self, window_id: WindowId, token: String) -> anyhow::Result<()> {
        debug!("window_activate: {window_id:?}, token={token}");
        let w = self
            .get_window(window_id)
            .with_context(|| format!("No window found {window_id:?}"))?;
        // let Some(xdg_activation) = &self.state.xdg_activation else {
        //     warn!("xdg_activation not found");
        //     return Ok(());
        // };
        // xdg_activation.activate::<ApplicationState>(w.window.wl_surface(), token);
        Ok(())
    }

    pub fn window_start_move(&self, window_id: WindowId) -> anyhow::Result<()> {
        let w = self
            .get_window(window_id)
            .with_context(|| format!("No window found {window_id:?}"))?;
        w.start_move()?;
        Ok(())
    }

    pub fn window_start_resize(&self, window_id: WindowId, edge: WindowResizeEdge) -> anyhow::Result<()> {
        let w = self
            .get_window(window_id)
            .with_context(|| format!("No window found {window_id:?}"))?;
        w.start_resize(edge)?;
        Ok(())
    }

    pub fn window_show_menu(&self, window_id: WindowId, position: LogicalPoint) -> anyhow::Result<()> {
        let w = self
            .get_window(window_id)
            .with_context(|| format!("No window found {window_id:?}"))?;
        w.show_menu(position);
        Ok(())
    }

    // pub fn open_url(&mut self, url_string: &str, activation_token: Option<&str>) -> anyhow::Result<RequestId> {
    //     debug!("application_open_url: {url_string}, activation_token={activation_token:?}");
    //     let uri = ashpd::url::Url::parse(url_string)?;
    //     let request = ashpd::desktop::open_uri::OpenFileRequest::default()
    //         .ask(false)
    //         .activation_token(activation_token.map(Into::into));
    //
    //     Ok(self.run_async(|request_id| async move {
    //         let error = request.send_uri(&uri).await.err().map(Into::into);
    //         AsyncEventResult::UrlOpenResponse { request_id, error }
    //     }))
    // }
    //
    // async fn open_file_manager_impl(path: &str, request: ashpd::desktop::open_uri::OpenDirectoryRequest) -> anyhow::Result<()> {
    //     let f = tokio::fs::File::open(path).await?;
    //     request.send(&f).await?;
    //     Ok(())
    // }
    //
    // pub fn open_file_manager(&mut self, path: String, activation_token: Option<&str>) -> RequestId {
    //     debug!("application_open_file_manager: {path}, activation_token={activation_token:?}");
    //     let request = ashpd::desktop::open_uri::OpenDirectoryRequest::default().activation_token(activation_token.map(Into::into));
    //     self.run_async(|request_id| async move {
    //         let error = Self::open_file_manager_impl(&path, request).await.err();
    //         AsyncEventResult::UrlOpenResponse { request_id, error }
    //     })
    // }
    //
    // fn get_wl_surface(&self, window_id: WindowId) -> anyhow::Result<WlSurface> {
    //     Ok(self.get_window(window_id)?.window.wl_surface().clone())
    // }
    //
    // pub fn show_open_file_dialog(
    //     &mut self,
    //     window_id: WindowId,
    //     common_params: &CommonFileDialogParams,
    //     open_params: &OpenFileDialogParams,
    // ) -> anyhow::Result<RequestId> {
    //     let wl_surface = self.get_wl_surface(window_id)?;
    //     let request = common_params.create_open_request(open_params)?;
    //     Ok(self.run_async(|request_id| async move {
    //         let identifier = ashpd::WindowIdentifier::from_wayland(&wl_surface).await;
    //         let result = show_open_file_dialog_impl(identifier, request).await;
    //         AsyncEventResult::FileChooserResponse { request_id, result }
    //     }))
    // }
    //
    // pub fn show_save_file_dialog(
    //     &mut self,
    //     window_id: WindowId,
    //     common_params: &CommonFileDialogParams,
    //     save_params: &SaveFileDialogParams,
    // ) -> anyhow::Result<RequestId> {
    //     let wl_surface = self.get_wl_surface(window_id)?;
    //     let request = common_params.create_save_request(save_params)?;
    //     Ok(self.run_async(|request_id| async move {
    //         let identifier: Option<ashpd::WindowIdentifier> = ashpd::WindowIdentifier::from_wayland(&wl_surface).await;
    //         let result = show_save_file_dialog_impl(identifier, request).await;
    //         AsyncEventResult::FileChooserResponse { request_id, result }
    //     }))
    // }
    //
    // pub fn init_notifications(&self) {
    //     let (sender, c) = channel::channel();
    //
    //     let dispatcher = Dispatcher::new(
    //         c,
    //         move |event: channel::Event<zbus::Connection>, (), state: &mut ApplicationState| {
    //             if let channel::Event::Msg(c) = event {
    //                 state.notifications_connection = Some(c);
    //             }
    //         },
    //     );
    //     self.event_loop.handle().register_dispatcher(dispatcher).unwrap();
    //
    //     let (event_sender, event_c) = channel::channel();
    //     self.event_loop
    //         .handle()
    //         .insert_source(event_c, move |event: channel::Event<NotificationData>, (), state| {
    //             if let channel::Event::Msg(notification_data) = event {
    //                 let activation_token_cstring = notification_data.activation_token.map(|v| CString::new(v).unwrap());
    //                 let e = NotificationClosedEvent::new(notification_data.id, activation_token_cstring.as_ref());
    //                 state.send_event(e);
    //             }
    //         })
    //         .unwrap();
    //     self.rt.spawn(async move {
    //         match zbus::Connection::session().await {
    //             Ok(c) => {
    //                 sender.send(c.clone()).unwrap();
    //                 spawn(notifications_receiver(c, move |s| event_sender.send(s).map_err(Into::into)));
    //             }
    //             Err(e) => warn!("Error initializing notifications: {e}"),
    //         }
    //     });
    // }

    // pub fn request_show_notification(
    //     &mut self,
    //     summary: String,
    //     body: String,
    //     sound_file_path: Option<String>,
    // ) -> anyhow::Result<RequestId> {
    //     let Some(conn) = &self.state.notifications_connection else {
    //         return Err(anyhow!("Cannot interact with notifications"));
    //     };
    //     let conn = conn.clone();
    //
    //     Ok(self.run_async(|request_id| async move {
    //         let result = show_notification_async(&conn, &summary, &body, sound_file_path).await;
    //         AsyncEventResult::NotificationShown { request_id, result }
    //     }))
    // }
    //
    // pub fn request_close_notification(&mut self, notification_id: u32) -> anyhow::Result<()> {
    //     let Some(conn) = &self.state.notifications_connection else {
    //         return Err(anyhow!("Cannot interact with notifications"));
    //     };
    //     let conn = conn.clone();
    //
    //     self.run_async(|_request_id| async move {
    //         if let Err(e) = close_notification_async(&conn, notification_id).await {
    //             warn!("Error closing notification {notification_id}: {e}");
    //         }
    //         AsyncEventResult::NotificationClosed {}
    //     });
    //     Ok(())
    // }
}
