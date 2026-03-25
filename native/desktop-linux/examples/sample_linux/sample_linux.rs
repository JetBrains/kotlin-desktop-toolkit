use core::str;
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::{CStr, CString},
    str::FromStr,
};

use crate::sample_linux_draw::{OpenglState, draw_opengl_triangle_with_init, draw_software, draw_software_drag_icon};
use desktop_common::{
    ffi_utils::{BorrowedArray, BorrowedStrPtr},
    logger_api::{LogLevel, LoggerConfiguration, logger_init_impl},
};
use desktop_linux::linux::application_api::{FfiDragAndDropQueryResponse, FfiSupportedActionsForMime, FfiTransferDataResponse};
use desktop_linux::linux::screen::screen_list;
use desktop_linux::linux::{
    application_api::{
        AppPtr,
        ApplicationCallbacks,
        DataSource,
        DragAndDropAction,
        DragAndDropActions,
        DragAndDropQueryData,
        RenderingMode,
        application_clipboard_paste,
        application_clipboard_put,
        application_close_notification,
        application_init,
        application_is_event_loop_thread,
        application_open_file_manager,
        application_open_url,
        application_primary_selection_paste,
        application_request_show_notification,
        application_run_event_loop,
        application_set_cursor_theme,
        application_shutdown,
        application_stop_event_loop,
        application_text_input_disable,
        application_text_input_enable,
        application_text_input_update,
        //
    },
    events::{DataTransferContent, Event, KeyDownEvent, KeyModifier, KeyModifierBitflag, RequestId, TextInputEvent, WindowId},
    file_dialog_api::{CommonFileDialogParams, OpenFileDialogParams, SaveFileDialogParams},
    geometry::{LogicalRect, LogicalSize},
    text_input_api::{TextInputContentHint, TextInputContentPurpose, TextInputContext},
    window_api::{
        WindowParams,
        window_activate,
        window_close,
        window_create,
        window_request_internal_activation_token,
        window_show_open_file_dialog,
        window_show_save_file_dialog,
        window_start_drag_and_drop,
        //
    },
    xdg_desktop_settings_api::XdgDesktopSetting,
};
use log::{debug, error, info, warn};
use url::Url;

const APP_ID: &CStr = c"org.jetbrains.desktop.linux.native.sample1";
const TEXT_MIME_TYPE: &CStr = c"text/plain;charset=utf-8";
const URI_LIST_MIME_TYPE: &CStr = c"text/uri-list";

const ALL_MIMES: &CStr = c"text/uri-list,text/plain;charset=utf-8";
const DRAG_ICON_WINDOW_ID: WindowId = WindowId(-1);

#[derive(Debug, Default)]
struct OptionalAppPtr(Option<AppPtr<'static>>);

impl OptionalAppPtr {
    fn get(&self) -> AppPtr<'static> {
        self.0.as_ref().unwrap().clone()
    }
}

#[derive(Debug, Default)]
struct Settings {
    cursor_theme_name: Option<CString>,
    cursor_theme_size: Option<u32>,
}

#[derive(Debug, Default)]
pub struct WindowState {
    pub active: bool,
    text_input_available: bool,
    composed_text: String,
    text: String,
    pub animation_progress: f32,
    pub drag_and_drop_target: bool,
    pub drag_and_drop_source: bool,
    pub opengl: Option<OpenglState>,
    last_received_path: Option<CString>,
}

impl WindowState {
    fn animation_tick(&mut self) {
        if self.animation_progress >= 200. {
            self.animation_progress = 0.;
        } else {
            self.animation_progress += if self.active { 1. } else { 0.2 };
        }
    }
}

#[derive(Debug)]
enum ActivationTokenAction {
    ActivateWindow,
    OpenUrl(CString),
    OpenFileManager(CString),
}

#[derive(Debug, Default)]
struct State {
    app_ptr: OptionalAppPtr,
    key_window_id: Option<WindowId>,
    key_modifiers: KeyModifierBitflag,
    windows: HashMap<WindowId, WindowState>,
    settings: Settings,
    request_sources: HashMap<RequestId, WindowId>,
    notification_sources: HashMap<u32, WindowId>,
    activation_token_action: HashMap<u32, ActivationTokenAction>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
    static OBJ_ID_TO_DEALLOC: RefCell<HashMap<i64, Box<dyn FnOnce()>>> = RefCell::default();
}

const DRAG_AND_DROP_LEFT_OF: f64 = 100.;

fn create_text_input_context<'a>(text: &str, text_cstring: &'a CString, change_caused_by_input_method: bool) -> TextInputContext<'a> {
    let codepoints_count = u16::try_from(text.chars().count()).unwrap();
    TextInputContext {
        surrounding_text: BorrowedStrPtr::new(text_cstring),
        cursor_codepoint_offset: codepoints_count,
        selection_start_codepoint_offset: codepoints_count,
        hints: TextInputContentHint::Multiline.into(),
        content_purpose: TextInputContentPurpose::Normal,
        cursor_rectangle: LogicalRect {
            x: (codepoints_count * 10).into(),
            y: 100,
            width: 5,
            height: 10,
        },
        change_caused_by_input_method,
    }
}

fn update_text_input_context(app_ptr: AppPtr<'_>, text: &str, change_caused_by_input_method: bool) {
    let surrounding_text_cstring = CString::from_str(text).unwrap();
    application_text_input_update(
        app_ptr,
        create_text_input_context(text, &surrounding_text_cstring, change_caused_by_input_method),
    );
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

const fn shortcut_modifiers(all_modifiers: KeyModifierBitflag) -> KeyModifierBitflag {
    KeyModifierBitflag(all_modifiers.0 & !(KeyModifier::CapsLock as u8) & !(KeyModifier::NumLock as u8))
}

#[allow(clippy::too_many_lines)]
fn on_keydown(event: &KeyDownEvent, app_ptr: AppPtr<'_>, state: &mut State) -> bool {
    const KEY_MODIFIER_CTRL: u8 = KeyModifier::Ctrl as u8;

    let modifiers: KeyModifierBitflag = shortcut_modifiers(state.key_modifiers);
    let window_id = state.key_window_id.expect("Key window not found");
    let Some(key_code) = decode_key_code(event.code.0) else {
        return false;
    };

    match (modifiers.0, key_code) {
        (0, keycode::KeyMappingCode::Backspace) => {
            let window_state = state.windows.get_mut(&window_id).unwrap();
            window_state.text.pop();
            if window_state.text_input_available {
                update_text_input_context(app_ptr, &window_state.text, false);
            }
            debug!("{window_id:?} : {} : {}", window_state.text.len(), window_state.text);
            true
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::Tab) => {
            let request_id = window_request_internal_activation_token(app_ptr, state.key_window_id.unwrap());
            if request_id > 0 {
                state
                    .activation_token_action
                    .insert(request_id, ActivationTokenAction::ActivateWindow);
            }
            true
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyV) => {
            application_clipboard_paste(app_ptr, 0, BorrowedStrPtr::new(TEXT_MIME_TYPE));
            true
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyC) => {
            application_clipboard_put(app_ptr, BorrowedStrPtr::new(ALL_MIMES));
            true
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
            true
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyO) => {
            let common_params = CommonFileDialogParams {
                modal: false,
                title: c"Open File for Linux Native Sample App test".into(),
                accept_label: c"Let's go!".into(),
                current_folder: c"/etc".into(),
            };
            let open_params = OpenFileDialogParams {
                select_directories: false,
                allows_multiple_selection: true,
            };
            let request_id = window_show_open_file_dialog(app_ptr, window_id, &common_params, &open_params);
            debug!("Requested open file dialog for {window_id:?}, request_id = {request_id:?}");
            true
        }

        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyS) => {
            let common_params = CommonFileDialogParams {
                modal: false,
                title: c"Save File for Linux Native Sample App test".into(),
                accept_label: c"Let's go!".into(),
                current_folder: c"/tmp".into(),
            };
            let save_params = SaveFileDialogParams {
                name_field_string_value: c"file from Linux Native Sample App.txt".into(),
            };
            let request_id = window_show_save_file_dialog(app_ptr, window_id, &common_params, &save_params);
            debug!("Requested open file dialog for {window_id:?}, request_id = {request_id:?}");
            true
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyN) => {
            let new_window_id = WindowId(state.windows.len() as i64 + 1);
            window_create(
                state.app_ptr.get(),
                WindowParams {
                    window_id: new_window_id,
                    size: LogicalSize { width: 300, height: 200 },
                    min_size: LogicalSize { width: 0, height: 0 },
                    title: BorrowedStrPtr::new(c"Window N"),
                    app_id: BorrowedStrPtr::new(APP_ID),
                    prefer_client_side_decoration: true,
                    rendering_mode: RenderingMode::Auto,
                },
            );
            state.windows.insert(new_window_id, WindowState::default());
            true
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyL) => {
            let request_id = window_request_internal_activation_token(app_ptr, state.key_window_id.unwrap());
            if request_id > 0 {
                state
                    .activation_token_action
                    .insert(request_id, ActivationTokenAction::OpenUrl(c"https://jetbrains.com".to_owned()));
            }
            true
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyU) => {
            let window_state = state.windows.get_mut(&window_id).unwrap();
            if let Some(path) = window_state.last_received_path.clone() {
                let request_id = window_request_internal_activation_token(app_ptr, state.key_window_id.unwrap());
                if request_id > 0 {
                    state
                        .activation_token_action
                        .insert(request_id, ActivationTokenAction::OpenFileManager(path));
                }
            }
            true
        }
        (_, _) => {
            if let Some(s) = event.characters.as_optional_slice() {
                let window_state = state.windows.get_mut(&window_id).unwrap();
                window_state.text += str::from_utf8(s).unwrap();
            }
            false
        }
    }
}

fn on_text_input_availability_changed(available: bool, app_ptr: AppPtr<'_>, window_state: &mut WindowState) {
    if available {
        let surrounding_text_cstring = CString::from_str(&window_state.text).unwrap();
        let context = create_text_input_context(&window_state.text, &surrounding_text_cstring, false);
        application_text_input_enable(app_ptr, context);
    } else {
        application_text_input_disable(app_ptr);
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
        update_text_input_context(app_ptr, &window_state.text, true);
    }

    if event.has_preedit_string {
        if event.preedit_string.cursor_begin_byte_pos == -1 && event.preedit_string.cursor_end_byte_pos == -1 {
            // TODO: hide cursor
        } else if let Some(preedit_string) = event.preedit_string.text.as_optional_str().unwrap() {
            window_state.composed_text.push_str(preedit_string);
        }
    }

    debug!("{window_id:?} : {} : {:?}", window_state.text.len(), window_state);
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

fn on_application_started(state: &mut State) {
    let window_1_id = WindowId(1);
    window_create(
        state.app_ptr.get(),
        WindowParams {
            window_id: window_1_id,
            size: LogicalSize { width: 200, height: 300 },
            min_size: LogicalSize { width: 100, height: 200 },
            title: BorrowedStrPtr::new(c"Window 1"),
            app_id: BorrowedStrPtr::new(APP_ID),
            prefer_client_side_decoration: false,
            rendering_mode: RenderingMode::Software,
        },
    );
    state.windows.insert(window_1_id, WindowState::default());

    let window_2_id = WindowId(2);
    window_create(
        state.app_ptr.get(),
        WindowParams {
            window_id: window_2_id,
            size: LogicalSize { width: 300, height: 200 },
            min_size: LogicalSize { width: 200, height: 100 },
            title: BorrowedStrPtr::new(c"Window 2"),
            app_id: BorrowedStrPtr::new(APP_ID),
            prefer_client_side_decoration: true,
            rendering_mode: RenderingMode::Auto,
        },
    );
    state.windows.insert(window_2_id, WindowState::default());
}

#[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
extern "C" fn event_handler(event: &Event) -> bool {
    const MOUSE_BUTTON_LEFT: u32 = 0x110;
    const MOUSE_BUTTON_MIDDLE: u32 = 0x112;

    match event {
        Event::WindowDraw(_) | Event::MouseMoved(_) => {}
        _ => {
            debug!("event_handler: {event:?}");
        }
    }

    STATE.with_borrow_mut(|state| {
        let app_ptr = state.app_ptr.get();
        let is_event_loop_thread = application_is_event_loop_thread(app_ptr.clone());
        assert!(is_event_loop_thread);

        match event {
            Event::ApplicationStarted => {
                on_application_started(state);
                true
            }
            Event::DisplayConfigurationChange => {
                let ffi_screens = screen_list(app_ptr);
                let screen_infos = unsafe { std::slice::from_raw_parts_mut(ffi_screens.ptr.cast_mut(), ffi_screens.len) };
                println!("DisplayConfigurationChange: {screen_infos:?}");
                false
            }
            Event::XdgDesktopSettingChange(data) => {
                on_xdg_desktop_settings_change(data, state);
                true
            }
            Event::WindowConfigure(data) => {
                if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                    window_state.active = data.active;
                }
                true
            }
            Event::WindowDraw(data) => {
                if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                    window_state.animation_tick();

                    if data.software_draw_data.canvas.is_null() {
                        draw_opengl_triangle_with_init(data.physical_size, data.window_id, window_state);
                    } else {
                        draw_software(&data.software_draw_data, data.physical_size, data.scale, window_state);
                    }
                    true
                } else {
                    false
                }
            }
            Event::DragIconDraw(data) => {
                let window_id = DRAG_ICON_WINDOW_ID;
                let window_state = state.windows.entry(window_id).or_insert_with(WindowState::default);
                window_state.animation_tick();

                if data.software_draw_data.canvas.is_null() {
                    draw_opengl_triangle_with_init(data.physical_size, window_id, window_state);
                } else {
                    draw_software_drag_icon(&data.software_draw_data, data.physical_size, data.scale);
                }
                true
            }
            Event::WindowCloseRequest(data) => {
                window_close(app_ptr.clone(), data.window_id);
                true
            }
            Event::WindowClosed(data) => {
                state.windows.retain(|&k, _v| k != data.window_id);
                state.request_sources.retain(|_k, &mut v| v != data.window_id);
                for (notification_id, _window_id) in state.notification_sources.extract_if(|_k, &mut v| v != data.window_id) {
                    application_close_notification(app_ptr.clone(), notification_id);
                }
                if state.windows.is_empty() {
                    application_stop_event_loop(app_ptr);
                }
                true
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
                            RenderingMode::Auto,
                            drag_icon_size,
                        );
                        if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                            window_state.drag_and_drop_source = true;
                        }
                    }
                    true
                }
                MOUSE_BUTTON_MIDDLE => {
                    application_primary_selection_paste(app_ptr, 1, BorrowedStrPtr::new(TEXT_MIME_TYPE));
                    true
                }
                _ => false,
            },
            Event::ModifiersChanged(data) => {
                state.key_modifiers = data.modifiers;
                true
            }
            Event::WindowKeyboardEnter(event) => {
                state.key_window_id = Some(event.window_id);
                true
            }
            Event::WindowKeyboardLeave(event) => {
                assert_eq!(state.key_window_id, Some(event.window_id));
                state.key_window_id = None;
                true
            }
            Event::KeyDown(event) => on_keydown(event, app_ptr, state),
            Event::FileChooserResponse(file_chooser_response) => {
                match file_chooser_response.newline_separated_files.as_optional_str() {
                    Ok(s) => {
                        let files = s.map(|s| s.trim_ascii_end().split("\r\n").collect::<Vec<_>>());
                        info!("Selected files: {files:?}");
                    }
                    Err(e) => {
                        error!("{e}");
                    }
                }
                true
            }
            Event::DataTransfer(data) => {
                if let Some(key_window_id) = state.key_window_id
                    && let Some(window_state) = state.windows.get_mut(&key_window_id)
                {
                    on_data_transfer_received(&data.content, window_state);
                    true
                } else {
                    false
                }
            }
            Event::DropPerformed(data) => {
                if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                    on_data_transfer_received(&data.content, window_state);
                    true
                } else {
                    false
                }
            }
            Event::DragAndDropLeave(data) => {
                if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                    window_state.drag_and_drop_target = false;
                    true
                } else {
                    false
                }
            }
            Event::DragAndDropFinished(data) => {
                state.windows.remove(&DRAG_ICON_WINDOW_ID);
                if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                    window_state.drag_and_drop_source = false;
                    info!("Finished initiated drag and drop with action {:?}", data.action);
                    true
                } else {
                    false
                }
            }
            Event::DataTransferCancelled(data) => {
                if data.data_source == DataSource::DragAndDrop {
                    for window_state in state.windows.values_mut() {
                        window_state.drag_and_drop_source = false;
                    }
                    state.windows.remove(&DRAG_ICON_WINDOW_ID);
                    true
                } else {
                    false
                }
            }
            Event::TextInputAvailability(data) => {
                if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                    on_text_input_availability_changed(data.available, app_ptr, window_state);
                    true
                } else {
                    false
                }
            }
            Event::TextInput(event) => {
                if let Some(key_window_id) = state.key_window_id
                    && let Some(window_state) = state.windows.get_mut(&key_window_id)
                {
                    on_text_input(event, app_ptr, key_window_id, window_state);
                    true
                } else {
                    false
                }
            }
            Event::ActivationTokenResponse(data) => {
                let token = BorrowedStrPtr::new(data.token.as_optional_cstr().unwrap());
                match state.activation_token_action.remove(&data.request_id).unwrap() {
                    ActivationTokenAction::ActivateWindow => {
                        if let Some(window_id) = state.windows.keys().find(|&&w| Some(w) != state.key_window_id) {
                            window_activate(app_ptr, *window_id, token);
                        }
                    }
                    ActivationTokenAction::OpenFileManager(path) => {
                        application_open_file_manager(app_ptr, BorrowedStrPtr::new(&path), token);
                    }
                    ActivationTokenAction::OpenUrl(url) => {
                        application_open_url(app_ptr, BorrowedStrPtr::new(&url), token);
                    }
                }
                true
            }
            Event::NotificationShown(data) => {
                if data.notification_id > 0 {
                    if let Some(requester) = state.request_sources.remove(&data.request_id) {
                        state.notification_sources.insert(data.notification_id, requester);
                    } else {
                        application_close_notification(app_ptr, data.notification_id);
                    }
                }
                true
            }
            Event::NotificationClosed(data) => {
                if let Some(window_id_to_activate) = state.notification_sources.remove(&data.notification_id)
                    && let Some(activation_token) = data.activation_token.as_optional_cstr()
                {
                    window_activate(app_ptr, window_id_to_activate, BorrowedStrPtr::new(activation_token));
                }
                true
            }
            _ => false,
        }
    })
}

fn on_xdg_desktop_settings_change(s: &XdgDesktopSetting, state: &mut State) {
    match s {
        XdgDesktopSetting::CursorSize(v) => {
            let size = (*v).try_into().unwrap();
            if let Some(name) = &state.settings.cursor_theme_name {
                application_set_cursor_theme(state.app_ptr.get(), BorrowedStrPtr::new(name), size);
            }
            state.settings.cursor_theme_size = Some(size);
        }
        XdgDesktopSetting::CursorTheme(v) => {
            let name = CString::new(v.as_str().unwrap()).unwrap();
            if let Some(size) = state.settings.cursor_theme_size {
                application_set_cursor_theme(state.app_ptr.get(), BorrowedStrPtr::new(&name), size);
            }
            state.settings.cursor_theme_name = Some(name);
        }
        _ => {}
    }
}

extern "C" fn query_drag_and_drop_target(data: &DragAndDropQueryData) -> FfiDragAndDropQueryResponse {
    STATE.with_borrow_mut(|state| {
        let window_state = state.windows.get_mut(&data.window_id).unwrap();
        window_state.drag_and_drop_target = true;
    });
    if data.location_in_window.x.0 < DRAG_AND_DROP_LEFT_OF {
        const SUPPORTED_ACTIONS_PER_MIME: [FfiSupportedActionsForMime; 2] = [
            FfiSupportedActionsForMime {
                supported_mime_type: BorrowedStrPtr::new(URI_LIST_MIME_TYPE),
                supported_actions: DragAndDropActions(DragAndDropAction::Copy as u8),
                preferred_action: DragAndDropAction::Copy,
            },
            FfiSupportedActionsForMime {
                supported_mime_type: BorrowedStrPtr::new(TEXT_MIME_TYPE),
                supported_actions: DragAndDropActions(DragAndDropAction::Move as u8 | DragAndDropAction::Copy as u8),
                preferred_action: DragAndDropAction::Copy,
            },
        ];

        FfiDragAndDropQueryResponse {
            obj_id: 0,
            supported_actions_per_mime: BorrowedArray::from_slice(&SUPPORTED_ACTIONS_PER_MIME),
        }
    } else {
        FfiDragAndDropQueryResponse {
            obj_id: 0,
            supported_actions_per_mime: BorrowedArray::from_slice(&[]),
        }
    }
}

extern "C" fn obj_dealloc(obj_id: i64) {
    if obj_id != 0 {
        OBJ_ID_TO_DEALLOC.with_borrow_mut(|cache| {
            let f = cache.remove(&obj_id).unwrap();
            f();
        });
    }
}

fn new_dealloc(f: impl FnOnce() + 'static) -> i64 {
    OBJ_ID_TO_DEALLOC.with_borrow_mut(|cache| {
        let obj_id = cache.keys().max().copied().unwrap_or_default() + 1;
        cache.insert(obj_id, Box::new(f));
        obj_id
    })
}

fn leak_string_data(s: String) -> (&'static [u8], i64) {
    let static_str = Box::leak(s.into_boxed_str().into_boxed_bytes());
    let ptr = static_str.as_ptr();
    let len = static_str.len();
    let obj_id = new_dealloc(move || unsafe {
        let s = std::slice::from_raw_parts_mut(ptr.cast_mut(), len);
        drop(Box::from_raw(s));
    });
    (static_str, obj_id)
}

extern "C" fn get_data_transfer_data(source: DataSource, mime_type: BorrowedStrPtr) -> FfiTransferDataResponse {
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
    } else {
        mime_type_cstr.to_str().unwrap()
    };

    let (static_str, obj_id) = leak_string_data(v.to_owned());
    let data = BorrowedArray::from_slice(static_str);
    FfiTransferDataResponse { obj_id, data }
}

pub fn main() {
    logger_init_impl(&LoggerConfiguration {
        file_path: BorrowedStrPtr::new(c"/tmp/a"),
        console_level: LogLevel::Debug,
        file_level: LogLevel::Error,
    });
    let app_ptr = application_init(ApplicationCallbacks {
        obj_dealloc,
        event_handler,
        query_drag_and_drop_target,
        get_data_transfer_data,
    });
    STATE.with_borrow_mut(|state| {
        state.app_ptr = OptionalAppPtr(Some(app_ptr.clone()));
    });
    application_run_event_loop(app_ptr.clone());
    application_shutdown(app_ptr);
}
