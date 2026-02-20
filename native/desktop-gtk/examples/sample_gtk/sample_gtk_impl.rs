//noinspection DuplicatedCode

use crate::sample_gtk_draw::{OpenglState, draw_opengl_triangle, init_gl};
use core::str;
use desktop_common::{
    ffi_utils::{ArraySize, BorrowedArray, BorrowedStrPtr},
    logger_api::{LogLevel, LoggerConfiguration, logger_init_impl},
};
use desktop_gtk::gtk::application_api::{
    application_clipboard_paste, application_clipboard_put, application_close_notification, application_open_file_manager,
    application_open_url, application_primary_selection_paste, application_request_redraw_drag_icon, application_request_show_notification,
    application_stop_drag_and_drop,
};
use desktop_gtk::gtk::events::WindowDecorationMode;
use desktop_gtk::gtk::file_dialog_api::{CommonFileDialogParams, OpenFileDialogParams, SaveFileDialogParams};
use desktop_gtk::gtk::text_input_api::{TextInputContentPurpose, TextInputContext, TextInputContextHint, TextInputContextHintBitflag};
use desktop_gtk::gtk::window_api::{
    window_close, window_maximize, window_request_decoration_mode, window_set_fullscreen, window_show_open_file_dialog,
    window_show_save_file_dialog, window_start_drag_and_drop, window_text_input_disable, window_text_input_enable,
    window_text_input_update, window_unmaximize, window_unset_fullscreen,
};
use desktop_gtk::gtk::{
    application_api::{
        AppPtr,
        ApplicationCallbacks,
        DataSource,
        DragAndDropAction,
        DragAndDropActions,
        DragAndDropQueryData,
        DragAndDropQueryResponse,
        RenderingMode,
        SupportedActionsForMime,
        application_init,
        application_is_event_loop_thread,
        application_run_event_loop,
        application_shutdown,
        application_stop_event_loop,
        //
    },
    events::{DataTransferContent, Event, KeyDownEvent, KeyModifier, KeyModifierBitflag, RequestId, TextInputEvent, WindowId},
    geometry::{LogicalRect, LogicalSize, PhysicalSize},
    window_api::{
        WindowParams,
        window_activate,
        window_create,
        window_request_redraw,
        //
    },
};
use gles30::GlFns;
use log::{debug, error, info, warn};
use std::cell::RefCell;
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    str::FromStr,
};
use url::Url;

const APP_ID: &CStr = c"org.jetbrains.desktop.gtk.native.sample-1";
const HTML_TEXT_MIME_TYPE: &CStr = c"text/html";
const TEXT_MIME_TYPE: &CStr = c"text/plain;charset=utf-8";
const URI_LIST_MIME_TYPE: &CStr = c"text/uri-list";

const ALL_MIMES: &CStr = c"text/html,text/plain;charset=utf-8";

const DRAG_AND_DROP_LEFT_OF: f64 = 100.;

#[derive(Debug, Default, Clone)]
struct OptionalAppPtr(Option<AppPtr<'static>>);

impl OptionalAppPtr {
    fn get(&self) -> AppPtr<'static> {
        self.0.as_ref().unwrap().clone()
    }
}

#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for OptionalAppPtr {}
unsafe impl Sync for OptionalAppPtr {}

#[derive(Default)]
struct WindowState {
    active: bool,
    text_input_available: bool,
    composed_text: String,
    text: String,
    animation_progress: f32,
    drag_and_drop_target: bool,
    drag_and_drop_source: bool,
    opengl: Option<OpenglState>,
    last_received_path: Option<CString>,
    fullscreen: bool,
    maximized: bool,
    decoration: Option<WindowDecorationMode>,
    scale: f64,
}

impl WindowState {
    fn animation_tick(&mut self) {
        if self.animation_progress >= 360. {
            self.animation_progress = 0.;
        } else {
            self.animation_progress += if self.active { 1. } else { 0.2 };
        }
    }
}

#[derive(Default)]
struct State {
    gl: Option<GlFns>,
    app_ptr: OptionalAppPtr,
    key_modifiers: KeyModifierBitflag,
    windows: HashMap<WindowId, WindowState>,
    drag_icon: Option<WindowState>,
    request_sources: HashMap<RequestId, WindowId>,
    notification_sources: HashMap<u32, WindowId>,
    key_window_id: Option<WindowId>,
}

impl State {
    fn with_mut_window_state(&mut self, window_id: WindowId, f: impl FnOnce(&mut WindowState)) {
        if let Some(window) = self.windows.get_mut(&window_id) {
            f(window);
            window_request_redraw(self.app_ptr.get(), window_id);
        } else {
            warn!("with_mut_window_state: cannot find {window_id:?}");
        }
    }

    fn next_window_id(&self) -> WindowId {
        let r = self.windows.keys().max_by_key(|e| e.0).map(|v| v.0 + 1).unwrap_or_default();
        WindowId(r)
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn with_borrow_mut_state<T>(f: impl FnOnce(&mut State) -> T) -> T {
    STATE.with_borrow_mut(f)
}

fn draw_opengl_triangle_with_init(gl: &GlFns, is_es: bool, physical_size: PhysicalSize, scale: f64, window_state: &mut WindowState) {
    let opengl_state = window_state.opengl.get_or_insert_with(|| OpenglState::new(gl, is_es));
    draw_opengl_triangle(gl, opengl_state, physical_size, scale, window_state.animation_progress);
}

fn create_text_input_context<'a>(text: &str, text_cstring: &'a CString) -> TextInputContext<'a> {
    let codepoints_count = u16::try_from(text.chars().count()).unwrap();
    TextInputContext {
        surrounding_text: BorrowedStrPtr::new(text_cstring),
        cursor_codepoint_offset: codepoints_count,
        selection_start_codepoint_offset: codepoints_count,
        hints: TextInputContextHintBitflag((TextInputContextHint::WordCompletion | TextInputContextHint::Spellcheck).bits()),
        content_purpose: TextInputContentPurpose::Normal,
        cursor_rectangle: LogicalRect {
            x: (codepoints_count * 10).into(),
            y: 100,
            width: 5,
            height: 10,
        },
    }
}

fn update_text_input_context(app_ptr: AppPtr<'_>, window_id: WindowId, text: &str) {
    let surrounding_text_cstring = CString::from_str(text).unwrap();
    window_text_input_update(app_ptr, window_id, create_text_input_context(text, &surrounding_text_cstring));
}

const fn shortcut_modifiers(all_modifiers: KeyModifierBitflag) -> KeyModifierBitflag {
    KeyModifierBitflag(all_modifiers.0 & !(KeyModifier::CapsLock as u8) & !(KeyModifier::NumLock as u8))
}

fn decode_key_code(raw: u32) -> Option<keycode::KeyMappingCode> {
    let Ok(raw) = u16::try_from(raw) else {
        warn!("decode_key_code: raw value too large ({raw})");
        return None;
    };
    if let Ok(keymap) = keycode::KeyMap::from_key_mapping(keycode::KeyMapping::Xkb(raw)) {
        if let Some(code) = keymap.code {
            Some(code)
        } else {
            warn!("decode_key_code returning None for {raw}");
            None
        }
    } else {
        warn!("decode_key_code error for {raw}");
        None
    }
}

#[allow(clippy::too_many_lines)]
fn on_keydown(event: &KeyDownEvent, app_ptr: AppPtr<'_>, state: &mut State, window_id: WindowId) {
    const KEY_MODIFIER_CTRL: u8 = KeyModifier::Ctrl as u8;

    let modifiers: KeyModifierBitflag = shortcut_modifiers(state.key_modifiers);
    let Some(key_code) = decode_key_code(event.code.0) else {
        return;
    };

    #[allow(clippy::single_match_else)]
    match (modifiers.0, key_code) {
        (0, keycode::KeyMappingCode::Escape) => {
            application_stop_drag_and_drop(app_ptr);
        }
        (0, keycode::KeyMappingCode::Backspace) => {
            let window_state = state.windows.get_mut(&window_id).unwrap();
            window_state.text.pop();
            if window_state.text_input_available {
                update_text_input_context(app_ptr, window_id, &window_state.text);
            }
            debug!("{window_id:?} : {} : {}", window_state.text.len(), window_state.text);
        }
        (0, keycode::KeyMappingCode::F11) => {
            let window_state = state.windows.get_mut(&window_id).unwrap();
            if window_state.fullscreen {
                window_unset_fullscreen(app_ptr, window_id);
            } else {
                window_set_fullscreen(app_ptr, window_id);
            }
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::Tab) => {
            if let Some(window_id) = state.windows.keys().find(|&&w| Some(w) != state.key_window_id) {
                window_activate(app_ptr, *window_id, BorrowedStrPtr::null());
            }
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyQ) => {
            window_close(app_ptr, window_id);
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyM) => {
            let window_state = state.windows.get_mut(&window_id).unwrap();
            if window_state.maximized {
                window_unmaximize(app_ptr, window_id);
            } else {
                window_maximize(app_ptr, window_id);
            }
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyD) => {
            let window_state = state.windows.get_mut(&window_id).unwrap();
            let new_decoration = match window_state.decoration {
                None => WindowDecorationMode::Client,
                Some(WindowDecorationMode::Client) => WindowDecorationMode::Server,
                Some(WindowDecorationMode::Server) => WindowDecorationMode::Client,
            };
            window_request_decoration_mode(app_ptr, window_id, new_decoration);
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyV) => {
            application_clipboard_paste(app_ptr, 0, BorrowedStrPtr::new(TEXT_MIME_TYPE));
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyC) => {
            application_clipboard_put(app_ptr, BorrowedStrPtr::new(ALL_MIMES));
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyF) => {
            application_clipboard_put(app_ptr, BorrowedStrPtr::new(c""));
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyP) => {
            let title = format!("Notification from window {}", window_id.0);
            let body = format!("Clicking this notification will activate window {}", window_id.0);
            let title_cstr = CString::new(title).unwrap();
            let body_cstr = CString::new(body).unwrap();
            let request_id = application_request_show_notification(
                app_ptr,
                BorrowedStrPtr::new(&title_cstr),
                BorrowedStrPtr::new(&body_cstr),
                BorrowedStrPtr::null(),
            );
            if request_id.0 != 0 {
                state.request_sources.insert(request_id, window_id);
            }
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyO) => {
            let common_params = CommonFileDialogParams {
                modal: false,
                title: c"Open File for GTK Native Sample App test".into(),
                accept_label: c"Let's go!".into(),
                current_folder: c"/etc".into(),
            };
            let open_params = OpenFileDialogParams {
                select_directories: false,
                allows_multiple_selection: true,
            };
            let request_id = window_show_open_file_dialog(app_ptr, window_id, &common_params, &open_params);
            debug!("Requested open file dialog for {window_id:?}, request_id = {request_id:?}");
        }

        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyS) => {
            let common_params = CommonFileDialogParams {
                modal: false,
                title: c"Save File for GTK Native Sample App test".into(),
                accept_label: c"Let's go!".into(),
                current_folder: c"/tmp".into(),
            };
            let save_params = SaveFileDialogParams {
                name_field_string_value: c"file from GTK Native Sample App.txt".into(),
            };
            let request_id = window_show_save_file_dialog(app_ptr, window_id, &common_params, &save_params);
            debug!("Requested open file dialog for {window_id:?}, request_id = {request_id:?}");
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyN) => {
            let new_window_id = state.next_window_id();
            window_create(
                state.app_ptr.get(),
                WindowParams {
                    window_id: new_window_id,
                    size: LogicalSize { width: 300, height: 200 },
                    min_size: LogicalSize { width: 0, height: 0 },
                    title: BorrowedStrPtr::new(c"Window N"),
                    prefer_client_side_decoration: false,
                    rendering_mode: RenderingMode::Auto,
                },
            );
            state.windows.insert(new_window_id, WindowState::default());
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyL) => {
            application_open_url(app_ptr, BorrowedStrPtr::new(c"https://jetbrains.com"), BorrowedStrPtr::null());
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyU) => {
            let window_state = state.windows.get_mut(&window_id).unwrap();
            if let Some(path) = window_state.last_received_path.clone() {
                application_open_file_manager(app_ptr, BorrowedStrPtr::new(&path), BorrowedStrPtr::null());
            }
        }
        (_, _) => {
            if event.has_character {
                let window_state = state.windows.get_mut(&window_id).unwrap();
                window_state.text.push(event.character);
                if window_state.text_input_available {
                    update_text_input_context(app_ptr, window_id, &window_state.text);
                }
            }
        }
    }
}

fn on_text_input_availability_changed(available: bool, app_ptr: AppPtr<'_>, window_id: WindowId, window_state: &mut WindowState) {
    if available && window_id != WindowId(1) {
        let surrounding_text_cstring = CString::from_str(&window_state.text).unwrap();
        let context = create_text_input_context(&window_state.text, &surrounding_text_cstring);
        window_text_input_enable(app_ptr, window_id, context);
    } else {
        window_text_input_disable(app_ptr, window_id);
    }
    window_state.text_input_available = available;
}

fn on_text_input(event: &TextInputEvent, app_ptr: AppPtr<'_>, window_id: WindowId, window_state: &mut WindowState) {
    window_state.composed_text.clear();
    if event.has_delete_surrounding_text {
        let cursor_pos = window_state.text.len();
        let range = (cursor_pos - event.delete_surrounding_text.before_length_in_bytes as usize)
            ..(cursor_pos + event.delete_surrounding_text.after_length_in_bytes as usize);
        window_state.text.drain(range);
    }
    if event.has_commit_string
        && let Some(commit_string) = event.commit_string.as_optional_str().unwrap()
    {
        debug!("{window_id:?} commit_string: {commit_string}");
        window_state.text += commit_string;
    }
    if event.has_delete_surrounding_text || event.has_commit_string {
        update_text_input_context(app_ptr, window_id, &window_state.text);
    }

    if event.has_preedit_string {
        if event.preedit_string.cursor_begin_byte_pos == -1 && event.preedit_string.cursor_end_byte_pos == -1 {
            // TODO: hide cursor
        } else if let Some(preedit_string) = event.preedit_string.text.as_optional_str().unwrap() {
            window_state.composed_text.push_str(preedit_string);
        }
    }

    debug!("{window_id:?} : {}", window_state.text);
}

fn on_data_transfer_received(content: &DataTransferContent, window_state: &mut WindowState) {
    if let Some(mime_type) = content.mime_type.as_optional_cstr() {
        let data = content.data.as_slice().unwrap();
        if mime_type == URI_LIST_MIME_TYPE {
            let list_str = str::from_utf8(data).unwrap();
            assert!(list_str.ends_with("\r\n"), "{list_str} doesn't end with CRLF");
            let list = {
                let mut v = list_str.split("\r\n").collect::<Vec<_>>();
                let last = v.pop();
                assert_eq!(last, Some(""));
                v
            };
            info!("Pasted file list: {list:?}");
            let first_path = {
                let first_uri_str = *list.first().unwrap();
                let first_uri = Url::from_str(first_uri_str).unwrap();
                let path_buf = first_uri.to_file_path().unwrap();
                let path_bytes = path_buf.into_os_string().into_encoded_bytes();
                CString::new(path_bytes).unwrap()
            };
            window_state.last_received_path = Some(first_path);
            for e in list {
                assert!(e.starts_with("file:///"), "\"{e}\" doesn't start with \"file:///\"");
                assert_eq!(e, e.trim_ascii_end());
            }
        } else if mime_type == TEXT_MIME_TYPE {
            let data_str = str::from_utf8(data).unwrap();
            window_state.text += data_str;
            window_state.last_received_path = None;
        } else {
            warn!("Mime type {mime_type:?} is not supported");
            window_state.last_received_path = None;
        }
    }
    window_state.drag_and_drop_target = false;
}

extern "C" fn on_application_started(state: &mut State) {
    window_create(
        state.app_ptr.get(),
        WindowParams {
            window_id: WindowId(1),
            size: LogicalSize { width: 200, height: 300 },
            min_size: LogicalSize { width: 200, height: 100 },
            title: BorrowedStrPtr::new(c"Window 1"),
            prefer_client_side_decoration: false,
            rendering_mode: RenderingMode::Auto,
        },
    );

    window_create(
        state.app_ptr.get(),
        WindowParams {
            window_id: WindowId(2),
            size: LogicalSize { width: 600, height: 200 },
            min_size: LogicalSize { width: 200, height: 100 },
            title: BorrowedStrPtr::new(c"Window 2"),
            prefer_client_side_decoration: true,
            rendering_mode: RenderingMode::Auto,
        },
    );

    state.windows.insert(WindowId(1), WindowState::default());
    state.windows.insert(WindowId(2), WindowState::default());
}

#[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
extern "C" fn event_handler(event: &Event) {
    const MOUSE_BUTTON_LEFT: u32 = 1;
    const MOUSE_BUTTON_MIDDLE: u32 = 2;

    match event {
        Event::ShouldRedraw(_) | Event::ShouldRedrawDragIcon | Event::WindowDraw(_) | Event::MouseMoved(_) => {}
        Event::DataTransferAvailable(_) => return,
        _ => {
            debug!("event_handler: {event:?}");
        }
    }

    with_borrow_mut_state(|state| {
        let app_ptr = state.app_ptr.get();
        let is_event_loop_thread = application_is_event_loop_thread(app_ptr.clone());
        assert!(is_event_loop_thread);

        match event {
            Event::ApplicationStarted => {
                on_application_started(state);
            }
            Event::WindowClosed(data) => {
                state.windows.retain(|&k, _v| k != data.window_id);
                state.request_sources.retain(|_k, &mut v| v != data.window_id);
                for (notification_id, _window_id) in state.notification_sources.extract_if(|_k, &mut v| v == data.window_id) {
                    application_close_notification(state.app_ptr.get(), notification_id);
                }
                if state.windows.is_empty() {
                    application_stop_event_loop(state.app_ptr.get());
                }
            }
            Event::WindowConfigure(data) => {
                if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                    window_state.active = data.active;
                    window_state.fullscreen = data.fullscreen;
                    window_state.maximized = data.maximized;
                    window_state.decoration = Some(data.decoration_mode);
                }
            }
            Event::WindowScaleChanged(data) => {
                if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                    window_state.scale = data.new_scale;
                }
            }
            Event::WindowDraw(data) => {
                if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                    window_state.animation_tick();

                    draw_opengl_triangle_with_init(
                        state.gl.as_ref().unwrap(),
                        data.opengl_draw_data.is_es,
                        data.physical_size,
                        window_state.scale,
                        window_state,
                    );
                }
            }
            Event::DragIconDraw(data) => {
                debug!("Event::DragIconDraw");
                let window_state = state.drag_icon.get_or_insert_default();
                window_state.animation_tick();

                draw_opengl_triangle_with_init(
                    state.gl.as_ref().unwrap(),
                    data.opengl_draw_data.is_es,
                    data.physical_size,
                    data.scale,
                    window_state,
                );
            }
            Event::MouseDown(data) => match data.button.0 {
                MOUSE_BUTTON_LEFT => {
                    if data.location_in_window.x.0 < DRAG_AND_DROP_LEFT_OF {
                        let mime_types = if state.key_modifiers.0 == KeyModifier::Shift as u8 {
                            ALL_MIMES
                        } else {
                            TEXT_MIME_TYPE
                        };
                        let actions = DragAndDropActions(DragAndDropAction::Copy as u8 | DragAndDropAction::Move as u8);
                        let drag_icon_size = LogicalSize { width: 300, height: 300 };
                        window_start_drag_and_drop(
                            app_ptr,
                            data.window_id,
                            BorrowedStrPtr::new(mime_types),
                            actions,
                            RenderingMode::EGL,
                            drag_icon_size,
                        );
                        if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                            window_state.drag_and_drop_source = true;
                        }
                    }
                }
                MOUSE_BUTTON_MIDDLE => {
                    application_primary_selection_paste(app_ptr, 1, BorrowedStrPtr::new(TEXT_MIME_TYPE));
                }
                _ => {}
            },
            Event::ModifiersChanged(data) => {
                state.key_modifiers = data.modifiers;
            }
            Event::WindowKeyboardEnter(data) => {
                state.key_window_id = Some(data.window_id);
                state.with_mut_window_state(data.window_id, |window_state| {
                    on_text_input_availability_changed(true, app_ptr, data.window_id, window_state);
                });
            }
            Event::WindowKeyboardLeave(data) => {
                if state.key_window_id == Some(data.window_id) {
                    state.key_window_id = None;
                }
                state.with_mut_window_state(data.window_id, |window_state| {
                    on_text_input_availability_changed(false, app_ptr, data.window_id, window_state);
                });
            }
            Event::KeyDown(event) => on_keydown(event, app_ptr, state, event.window_id),
            Event::FileChooserResponse(file_chooser_response) => match file_chooser_response.newline_separated_files.as_optional_str() {
                Ok(s) => {
                    let files = s.map(|s| s.trim_ascii_end().split("\r\n").collect::<Vec<_>>());
                    info!("Selected files: {files:?}");
                }
                Err(e) => {
                    error!("{e}");
                }
            },
            Event::DataTransfer(data) => {
                if let Some(key_window_id) = state.key_window_id
                    && let Some(window_state) = state.windows.get_mut(&key_window_id)
                {
                    on_data_transfer_received(&data.content, window_state);
                }
            }
            Event::DropPerformed(data) => state.with_mut_window_state(data.window_id, |window_state| {
                on_data_transfer_received(&data.content, window_state);
            }),
            Event::DragAndDropLeave(data) => state.with_mut_window_state(data.window_id, |window_state| {
                window_state.drag_and_drop_target = false;
            }),
            Event::DragAndDropFinished(data) => state.with_mut_window_state(data.window_id, |window_state| {
                window_state.drag_and_drop_source = false;
                info!("Finished initiated drag and drop with action {:?}", data.action);
            }),
            Event::DataTransferCancelled(data) => {
                if data.data_source == DataSource::DragAndDrop {
                    for window_state in state.windows.values_mut() {
                        window_state.drag_and_drop_source = false;
                    }
                }
            }
            Event::TextInput(data) => state.with_mut_window_state(data.window_id, |window_state| {
                on_text_input(data, app_ptr, data.window_id, window_state);
            }),
            Event::NotificationShown(data) => {
                if data.notification_id > 0 {
                    if let Some(requester) = state.request_sources.remove(&data.request_id) {
                        state.notification_sources.insert(data.notification_id, requester);
                    } else {
                        application_close_notification(app_ptr, data.notification_id);
                    }
                }
            }
            Event::NotificationClosed(data) => {
                if data.action.as_optional_cstr().is_some()
                    && let Some(window_id_to_activate) = state.notification_sources.remove(&data.notification_id)
                {
                    let activation_token = data.activation_token.as_optional_cstr().map(ToOwned::to_owned);
                    window_activate(
                        app_ptr,
                        window_id_to_activate,
                        BorrowedStrPtr::new_optional(activation_token.as_ref()),
                    );
                }
            }
            Event::ShouldRedraw(data) => {
                window_request_redraw(app_ptr, data.window_id);
            }
            Event::ShouldRedrawDragIcon => {
                application_request_redraw_drag_icon(app_ptr);
            }
            _ => {}
        }
    });
}

extern "C" fn query_drag_and_drop_target(data: &DragAndDropQueryData) -> DragAndDropQueryResponse<'_> {
    with_borrow_mut_state(|state| {
        state.with_mut_window_state(data.window_id, |window_state| {
            window_state.drag_and_drop_target = true;
        });
    });
    if data.location_in_window.x.0 < DRAG_AND_DROP_LEFT_OF {
        const SUPPORTED_ACTIONS_PER_MIME: [SupportedActionsForMime; 2] = [
            SupportedActionsForMime {
                supported_mime_type: BorrowedStrPtr::new(URI_LIST_MIME_TYPE),
                supported_actions: DragAndDropActions(DragAndDropAction::Copy as u8),
                preferred_action: DragAndDropAction::Copy,
            },
            SupportedActionsForMime {
                supported_mime_type: BorrowedStrPtr::new(TEXT_MIME_TYPE),
                supported_actions: DragAndDropActions(DragAndDropAction::Move as u8 | DragAndDropAction::Copy as u8),
                preferred_action: DragAndDropAction::Copy,
            },
        ];

        DragAndDropQueryResponse {
            supported_actions_per_mime: BorrowedArray::from_slice(&SUPPORTED_ACTIONS_PER_MIME),
        }
    } else {
        DragAndDropQueryResponse {
            supported_actions_per_mime: BorrowedArray::from_slice(&[]),
        }
    }
}

const extern "C" fn window_close_request(_window_id: WindowId) -> bool {
    true
}

const extern "C" fn application_wants_to_terminate() -> bool {
    true
}

extern "C" fn deinit_u8_vec(ptr: *const u8, len: ArraySize) {
    let _ = unsafe {
        let s = std::slice::from_raw_parts_mut(ptr.cast_mut(), len);
        Box::from_raw(s)
    };
}

fn leaked_string_data(s: &str) -> &'static [u8] {
    Box::leak(s.to_owned().into_boxed_str().into_boxed_bytes())
}

extern "C" fn get_data_transfer_data(source: DataSource, mime_type: BorrowedStrPtr) -> BorrowedArray<'static, u8> {
    let mime_type_cstr = mime_type.as_optional_cstr().unwrap();
    let v = if mime_type_cstr == URI_LIST_MIME_TYPE {
        match source {
            DataSource::Clipboard => "file:///etc/hosts",
            DataSource::DragAndDrop => "file:///boot/efi/",
            DataSource::PrimarySelection => "file:///etc/environment",
        }
    } else if mime_type_cstr == TEXT_MIME_TYPE {
        match source {
            DataSource::Clipboard => "clipboard text",
            DataSource::DragAndDrop => "d&d text",
            DataSource::PrimarySelection => "primary selection text",
        }
    } else if mime_type_cstr == HTML_TEXT_MIME_TYPE {
        match source {
            DataSource::Clipboard => "<b>some html text</b>",
            DataSource::DragAndDrop => "<b>some html d&d text</b>",
            DataSource::PrimarySelection => "<b>some html primary selection text</b>",
        }
    } else {
        mime_type_cstr.to_str().unwrap()
    };

    let mut a = BorrowedArray::from_slice(leaked_string_data(v));
    a.deinit = Some(deinit_u8_vec);
    a
}

pub fn main_impl() {
    let gl = init_gl();
    logger_init_impl(&LoggerConfiguration {
        file_path: BorrowedStrPtr::new(c"/tmp/a"),
        console_level: LogLevel::Debug,
        file_level: LogLevel::Error,
    });
    let app_ptr = application_init(ApplicationCallbacks {
        event_handler,
        query_drag_and_drop_target,
        get_data_transfer_data,
        window_close_request,
        application_wants_to_terminate,
    });
    with_borrow_mut_state(|state| {
        state.gl = gl;
        state.app_ptr = OptionalAppPtr(Some(app_ptr.clone()));
    });
    application_run_event_loop(app_ptr.clone(), BorrowedStrPtr::new(APP_ID));
    application_shutdown(app_ptr);
}
