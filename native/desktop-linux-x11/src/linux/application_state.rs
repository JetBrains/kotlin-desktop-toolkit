use crate::linux::application_api::{ApplicationCallbacks, DataSource, DragAndDropAction, DragAndDropQueryData, RenderingMode};
use crate::linux::events::{
    DataTransferContent, DataTransferEvent, DragAndDropFinishedEvent, DragAndDropLeaveEvent, DropPerformedEvent, Event, KeyCode,
    KeyDownEvent, KeyModifierBitflag, KeyUpEvent, ModifiersChangedEvent, MouseDownEvent, MouseEnteredEvent, MouseExitedEvent,
    MouseMovedEvent, MouseUpEvent, ScreenId, ScrollData, ScrollWheelEvent, TextInputAvailabilityEvent, TextInputDeleteSurroundingTextData,
    TextInputEvent, TextInputPreeditStringData, WindowCapabilities, WindowCloseRequestEvent, WindowConfigureEvent, WindowDecorationMode,
    WindowId, WindowKeyboardEnterEvent, WindowKeyboardLeaveEvent, WindowScaleChangedEvent, WindowScreenChangeEvent,
};
use crate::linux::geometry::{LogicalRect, LogicalSize, round_to_u32, LogicalPixels};
use crate::linux::window::{get_logical_window_size, RenderingData, SimpleWindow};
use desktop_common::ffi_utils::BorrowedStrPtr;
use khronos_egl;
use log::{debug, info, warn};
use sdl3_sys::events::SDL_Event;
use sdl3_sys::video::{SDL_CreateWindow, SDL_WINDOW_OPENGL, SDL_Window, SDL_WindowFlags, SDL_WindowID, SDL_GetWindowDisplayScale, SDL_GetWindowID, SDL_GetWindowFlags, SDL_WINDOW_FULLSCREEN, SDL_WINDOW_MAXIMIZED, SDL_GetWindowSize};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::rc::Rc;
use std::sync::LazyLock;
use std::sync::mpsc::Receiver;
use std::time::Instant;
use anyhow::bail;
// use winit::platform::scancode::PhysicalKeyExtScancode;
use crate::linux::events::Event::XdgDesktopSettingChange;
use crate::linux::pointer_shapes_api::PointerShape;
use crate::linux::xdg_desktop_settings_api::XdgDesktopSetting;

/// cbindgen:ignore
pub type EglInstance = khronos_egl::DynamicInstance<khronos_egl::EGL1_0>;

/// cbindgen:ignore
static EGL: LazyLock<Option<EglInstance>> = LazyLock::new(|| match unsafe { libloading::Library::new("libEGL.so.1") } {
    Ok(egl_lib) => match unsafe { EglInstance::load_required_from(egl_lib) } {
        Ok(egl) => Some(egl),
        Err(e) => {
            warn!("Failed to load the required symbols from the EGL library: {e}");
            None
        }
    },
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
    pub window_id_to_sdl_window_id: HashMap<WindowId, SDL_WindowID>,
    pub windows: HashMap<SDL_WindowID, SimpleWindow>,
    // pub clipboard: Option<Clipboard>,
}

impl ApplicationState {
    pub fn new(callbacks: ApplicationCallbacks) -> Self {
        Self {
            callbacks,
            window_id_to_sdl_window_id: HashMap::new(),
            windows: HashMap::new(),
        }
    }

    pub fn send_event<'a, T: Into<Event<'a>>>(&self, event_data: T) -> bool {
        let event: Event = event_data.into();
        self.callbacks.send_event(event)
    }

    pub fn create_window(
        &mut self,
        window_id: WindowId,
        rect: LogicalRect,
        min_size: Option<LogicalSize>,
        title: Option<&CStr>,
        app_id: String,
        prefer_client_side_decoration: bool,
        rendering_mode: RenderingMode,
    ) -> anyhow::Result<()> {
        let egl = match rendering_mode {
            RenderingMode::Auto | RenderingMode::EGL => get_egl(),
            RenderingMode::Software => None,
        };
        let mut window_flags: SDL_WindowFlags = 0;
        if egl.is_some() {
            window_flags |= SDL_WINDOW_OPENGL;
        }

        // .with_decorations(!prefer_client_side_decoration)
        // .with_position(rect.as_winit_position())
        // .with_surface_size(rect.as_winit_size());
        // if let Some(min_size) = min_size {
        //     window_attributes = window_attributes.with_min_surface_size(min_size.as_winit_size());
        // }
        let window = unsafe { SDL_CreateWindow(title.unwrap_or(c"").as_ptr(), rect.width.0 as i32, rect.height.0 as i32, window_flags) };
        if window.is_null() {
            bail!("Failed to create winit window");
        }
        let sdl_window_id = unsafe { SDL_GetWindowID(window) };
        let current_scale = unsafe { SDL_GetWindowDisplayScale(window) };
        let rendering_data = RenderingData::new(window, egl).expect("Failed to create rendering data");
        //
        // let decoration_mode = if window.is_decorated() {
        //     WindowDecorationMode::Server
        // } else {
        //     WindowDecorationMode::Client
        // };
        // let fullscreen = unsafe { SDL_WindowGet
        let actual_flags = unsafe { SDL_GetWindowFlags(window) };
        let configure_event = WindowConfigureEvent {
            window_id,
            size: get_logical_window_size(window),
            active: false,
            maximized: actual_flags & SDL_WINDOW_MAXIMIZED != 0,
            fullscreen: actual_flags & SDL_WINDOW_FULLSCREEN != 0,
            decoration_mode: WindowDecorationMode::Server,
            capabilities: WindowCapabilities {
                window_menu: true,
                maximize: true,
                fullscreen: true,
                minimize: true,
            },
        };

        // let screen_change_event = window.current_monitor().map(|monitor| WindowScreenChangeEvent {
        //     window_id,
        //     new_screen_id: ScreenId(monitor.native_id()),
        // });

        let simple_window = SimpleWindow {
            window_id,
            window,
            current_scale,
            rendering_data,
            last_draw_measure_time: Instant::now(),
            draw_call_count: 0,
        };
        self.window_id_to_sdl_window_id.insert(window_id, sdl_window_id);
        self.windows.insert(sdl_window_id, simple_window);
        // self.send_event(configure_event);
        //
        // if let Some(screen_change_event) = screen_change_event {
        //     self.send_event(screen_change_event);
        // }

        // let xdg_titlebar_event =
        //     XdgDesktopSettingChange(XdgDesktopSetting::TitlebarLayout(BorrowedStrPtr::new(c":minimize,maximize,close")));
        // self.send_event(xdg_titlebar_event);
        //
        // let ime_available_event = TextInputAvailabilityEvent {
        //     window_id,
        //     available: true,
        // };
        // self.send_event(ime_available_event);
        Ok(())
    }

    // fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
    //     _ = event_loop;
    //     self.send_event(Event::ApplicationStarted);
    // }
    //
    // fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
    //     while let Ok(event) = self.receiver.try_recv() {
    //         match event {
    //             UserEvents::Exit => {
    //                 if !self.send_event(Event::ApplicationWantsToTerminate) {
    //                     event_loop.exit();
    //                     self.send_event(Event::ApplicationWillTerminate);
    //                 }
    //             }
    //             UserEvents::CreateWindow {
    //                 window_id,
    //                 rect,
    //                 min_size,
    //                 title,
    //                 app_id,
    //                 prefer_client_side_decoration,
    //                 rendering_mode,
    //             } => self.create_window(
    //                 event_loop,
    //                 window_id,
    //                 rect,
    //                 min_size,
    //                 title,
    //                 app_id,
    //                 prefer_client_side_decoration,
    //                 rendering_mode,
    //             ),
    //             UserEvents::RunOnEventLoop(f) => f(),
    //             UserEvents::ClipboardReceived {
    //                 serial,
    //                 mime_type,
    //                 content,
    //             } => {
    //                 let mime_type_cstr = CString::new(mime_type).unwrap();
    //                 let content = DataTransferContent::new(&mime_type_cstr, &content);
    //                 self.send_event(DataTransferEvent { serial, content });
    //             }
    //         }
    //     }
    // }
    pub fn handle_event(&mut self, event: SDL_Event) {

    }

        #[allow(clippy::too_many_lines)]
    pub fn window_event(&mut self, sdl_window: SDL_WindowID, event: SDL_Event) {
        if let Some(w) = self.windows.get_mut(&sdl_window) {
            // match event {
            //     WindowEvent::ActivationTokenDone { serial, token } => {
            //         // let token_cstr = CString::new(token.into_raw()).expect("Invalid activation token string");
            //         // let event = ActivationTokenResponse { request_id: serial, token: BorrowedStrPtr::new(&token_cstr) };
            //         // self.send_event(event);
            //     }
            //     WindowEvent::SurfaceResized(new_physical_size) => {
            //         w.on_resize(new_physical_size.into());
            //         let outer_size = w.window.outer_size();
            //         debug!(
            //             "WindowEvent::Resized: {:?}, {new_physical_size:?}, outer_size={outer_size:?}",
            //             w.window_id
            //         );
            //         let decoration_mode = if w.window.is_decorated() {
            //             WindowDecorationMode::Server
            //         } else {
            //             WindowDecorationMode::Client
            //         };
            //         let event = WindowConfigureEvent {
            //             window_id: w.window_id,
            //             size: w.window.surface_size().to_logical(w.current_scale).into(),
            //             active: false,
            //             maximized: w.window.is_maximized(),
            //             fullscreen: w.window.fullscreen().is_some(),
            //             decoration_mode,
            //             capabilities: WindowCapabilities {
            //                 window_menu: true,
            //                 maximize: true,
            //                 fullscreen: true,
            //                 minimize: true,
            //             },
            //         };
            //         self.send_event(event);
            //     }
            //     WindowEvent::Moved(physical_position) => {
            //         //w.window.current_monitor()
            //     }
            //     WindowEvent::CloseRequested => {
            //         let event = WindowCloseRequestEvent { window_id: w.window_id };
            //         self.send_event(event);
            //     }
            //     WindowEvent::Destroyed => {}
            //     WindowEvent::Focused(is_focused) => {
            //         if is_focused {
            //             let event = WindowKeyboardEnterEvent { window_id: w.window_id };
            //             self.send_event(event);
            //         } else {
            //             let event = WindowKeyboardLeaveEvent { window_id: w.window_id };
            //             self.send_event(event);
            //         }
            //     }
            //     WindowEvent::KeyboardInput {
            //         device_id: _,
            //         event,
            //         is_synthetic: _,
            //     } => {
            //         if let Some(scancode) = physicalkey_to_scancode(event.physical_key) {
            //             let code = KeyCode(scancode);
            //             let key_without_modifiers = winit_key_to_keysym(event.key_without_modifiers, event.location);
            //             let key = winit_key_to_keysym(event.logical_key, event.location);
            //             match event.state {
            //                 ElementState::Pressed => {
            //                     let text_cstr = event.text.map(|t| CString::new(t.as_str()).unwrap());
            //                     let event = KeyDownEvent {
            //                         window_id: w.window_id,
            //                         code,
            //                         characters: BorrowedStrPtr::new_optional(text_cstr.as_ref()),
            //                         key,
            //                         key_without_modifiers,
            //                         is_repeat: event.repeat,
            //                     };
            //                     self.send_event(event);
            //                 }
            //                 ElementState::Released => {
            //                     let event = KeyUpEvent {
            //                         window_id: w.window_id,
            //                         code,
            //                         key,
            //                         key_without_modifiers,
            //                     };
            //                     self.send_event(event);
            //                 }
            //             }
            //         }
            //     }
            //     WindowEvent::ModifiersChanged(modifiers) => {
            //         debug!("WindowEvent::ModifiersChanged for {:?}: {modifiers:?}", w.window_id);
            //         let event = ModifiersChangedEvent {
            //             modifiers: KeyModifierBitflag::from_winit(modifiers),
            //         };
            //         self.send_event(event);
            //     }
            //     WindowEvent::Ime(ime) => match ime {
            //         Ime::Enabled => {
            //             debug!("WindowEvent::Ime::Enabled for {:?}", w.window_id);
            //         }
            //         Ime::Disabled => {
            //             debug!("WindowEvent::Ime::Disabled for {:?}", w.window_id);
            //             // let event = TextInputAvailabilityEvent {
            //             //     window_id: w.window_id,
            //             //     available: false,
            //             // };
            //             // self.send_event(event);
            //         }
            //         Ime::Preedit(text, pos) => {
            //             debug!("WindowEvent::Ime::Preedit for {:?}: text={text}, pos={pos:?}", w.window_id);
            //             let text_cstr = CString::new(text).unwrap();
            //             #[allow(clippy::cast_possible_truncation)]
            //             let (cursor_begin_byte_pos, cursor_end_byte_pos) = pos.map_or((-1, -1), |(a, b)| (a as i32, b as i32));
            //             let preedit_string = TextInputPreeditStringData {
            //                 text: BorrowedStrPtr::new(&text_cstr),
            //                 cursor_begin_byte_pos,
            //                 cursor_end_byte_pos,
            //             };
            //             let event = TextInputEvent {
            //                 window_id: w.window_id,
            //                 has_preedit_string: true,
            //                 preedit_string,
            //                 has_commit_string: false,
            //                 commit_string: BorrowedStrPtr::null(),
            //                 has_delete_surrounding_text: false,
            //                 delete_surrounding_text: TextInputDeleteSurroundingTextData::default(),
            //             };
            //             self.send_event(event);
            //         }
            //         Ime::Commit(commit_string) => {
            //             debug!("WindowEvent::Ime::Commit for {:?}: commit_string={commit_string}", w.window_id);
            //             let commit_string_cstr = CString::new(commit_string).unwrap();
            //             let event = TextInputEvent {
            //                 window_id: w.window_id,
            //                 has_preedit_string: false,
            //                 preedit_string: TextInputPreeditStringData::default(),
            //                 has_commit_string: true,
            //                 commit_string: BorrowedStrPtr::new(&commit_string_cstr),
            //                 has_delete_surrounding_text: false,
            //                 delete_surrounding_text: TextInputDeleteSurroundingTextData::default(),
            //             };
            //             self.send_event(event);
            //         }
            //     },
            //     WindowEvent::PointerMoved {
            //         device_id: _,
            //         position,
            //         primary: _,
            //         source: _,
            //     } => {
            //         // debug!("PointerMoved {:?}: {position:?}", w.window_id);
            //         let event = MouseMovedEvent {
            //             window_id: w.window_id,
            //             location_in_window: position.to_logical(w.current_scale).into(),
            //         };
            //         self.send_event(event);
            //     }
            //     WindowEvent::PointerEntered {
            //         device_id: _,
            //         position,
            //         primary: _,
            //         kind: _,
            //     } => {
            //         let event = MouseEnteredEvent {
            //             window_id: w.window_id,
            //             location_in_window: position.to_logical(w.current_scale).into(),
            //         };
            //         self.send_event(event);
            //     }
            //     WindowEvent::PointerLeft {
            //         device_id: _,
            //         position: _,
            //         primary: _,
            //         kind: _,
            //     } => {
            //         let event = MouseExitedEvent { window_id: w.window_id };
            //         self.send_event(event);
            //     }
            //     WindowEvent::MouseWheel {
            //         device_id: _,
            //         delta,
            //         phase,
            //     } => {
            //         let (horizontal_scroll, vertical_scroll) = ScrollData::from_winit(delta, phase);
            //         let event = ScrollWheelEvent {
            //             window_id: w.window_id,
            //             horizontal_scroll,
            //             vertical_scroll,
            //         };
            //         self.send_event(event);
            //     }
            //     WindowEvent::PointerButton {
            //         device_id: _,
            //         state,
            //         position,
            //         primary: _,
            //         button,
            //     } => {
            //         debug!("PointerButton {:?}: {button:?}@{position:?}", w.window_id);
            //         if let Ok(button) = button.try_into() {
            //             let event: Event = match state {
            //                 ElementState::Pressed => MouseDownEvent {
            //                     window_id: w.window_id,
            //                     button,
            //                     location_in_window: position.to_logical(w.current_scale).into(),
            //                 }
            //                 .into(),
            //                 ElementState::Released => MouseUpEvent {
            //                     window_id: w.window_id,
            //                     button,
            //                     location_in_window: position.to_logical(w.current_scale).into(),
            //                 }
            //                 .into(),
            //             };
            //             self.send_event(event);
            //         }
            //     }
            //     WindowEvent::PinchGesture {
            //         device_id: _,
            //         delta: _,
            //         phase: _,
            //     } => {
            //         // TODO?
            //     }
            //     WindowEvent::PanGesture {
            //         device_id: _,
            //         delta: _,
            //         phase: _,
            //     } => {
            //         // TODO?
            //     }
            //     WindowEvent::DoubleTapGesture { device_id: _ } => {
            //         // TODO?
            //     }
            //     WindowEvent::RotationGesture {
            //         device_id: _,
            //         delta: _,
            //         phase: _,
            //     } => {
            //         // TODO?
            //     }
            //     WindowEvent::TouchpadPressure {
            //         device_id: _,
            //         pressure: _,
            //         stage: _,
            //     } => {
            //         // TODO?
            //     }
            //     WindowEvent::ScaleFactorChanged {
            //         scale_factor,
            //         mut surface_size_writer,
            //     } => {
            //         let old_scale = w.current_scale;
            //         debug!(
            //             "WindowEvent::ScaleFactorChanged {:?}: old_scale={old_scale}, new_scale={scale_factor}",
            //             w.window_id
            //         );
            //         w.scale_changed(scale_factor);
            //         let event = WindowScaleChangedEvent {
            //             window_id: w.window_id,
            //             new_scale: scale_factor,
            //         };
            //         let new_size = {
            //             let physical_size = w.window.surface_size();
            //             let new_w = f64::from(physical_size.width) / old_scale * scale_factor;
            //             let new_h = f64::from(physical_size.height) / old_scale * scale_factor;
            //             debug!(
            //                 "Automatically adjusting window {:?} size after the scale change: \
            //                 physical_size={physical_size:?}, new_w={new_w}, new_h={new_h}",
            //                 w.window_id
            //             );
            //             PhysicalSize {
            //                 width: round_to_u32(new_w),
            //                 height: round_to_u32(new_h),
            //             }
            //         };
            //         if let Err(e) = surface_size_writer.request_surface_size(new_size) {
            //             warn!("Error adjusting window {:?} size after the scale change: {e}", w.window_id);
            //         }
            //         self.send_event(event);
            //     }
            //     WindowEvent::ThemeChanged(_) => {
            //         // Unsupported on X11
            //     }
            //     WindowEvent::Occluded(_is_occluded) => {
            //         // TODO?
            //     }
            //     WindowEvent::RedrawRequested => {
            //         w.draw(&|e| self.callbacks.send_event(e));
            //     }
            //     WindowEvent::DragEntered { paths, position } => {
            //         info!("DragEntered {:?}", paths);
            //         let query = DragAndDropQueryData{ window_id: w.window_id, location_in_window: position.to_logical(w.current_scale).into() };
            //         // Unsupported:
            //         // * d&d of anything else than files (so, e.g., text)
            //         // * rejecting d&d
            //         // * choosing whether to copy or move
            //         _ = (self.callbacks.query_drag_and_drop_target)(&query);
            //     }
            //     WindowEvent::DragMoved { position } => {
            //         info!("DragMoved {:?}", position);
            //         w.set_cursor_icon(PointerShape::Move);
            //         let query = DragAndDropQueryData{ window_id: w.window_id, location_in_window: position.to_logical(w.current_scale).into() };
            //         _ = (self.callbacks.query_drag_and_drop_target)(&query);
            //     }
            //     WindowEvent::DragDropped { paths, position } => {
            //         info!("DragDropped {:?}", paths);
            //     }
            //     WindowEvent::DragLeft { position } => {
            //         info!("DragLeft {:?}", position);
            //         let event = DragAndDropLeaveEvent { window_id: w.window_id };
            //         self.send_event(event);
            //     }
            // }
        }
    }
}
