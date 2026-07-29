use crate::sample_linux_actions::Action;
use crate::sample_linux_draw::OpenglState;
use crate::sample_linux_draw_software::draw_software;
use core::str;
use desktop_common::ffi_utils::BorrowedStrPtr;
use desktop_common::{
    ffi_utils::{BorrowedArray, BorrowedUtf8},
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
        application_close_notification,
        application_init,
        application_is_event_loop_thread,
        application_open_file_manager,
        application_open_url,
        application_request_show_notification,
        application_run_event_loop,
        application_set_cursor_theme,
        application_shutdown,
        application_text_input_disable,
        application_text_input_enable,
        application_text_input_update,
        //
    },
    desktop_settings_api::FfiDesktopSetting,
    events::{DataTransferContent, Event, KeyDownEvent, KeyModifiers, RequestId, SoftwareDrawData, TextInputEvent, WindowId},
    file_dialog_api::{CommonFileDialogParams, OpenFileDialogParams, SaveFileDialogParams},
    geometry::{LogicalPixelsInt, LogicalRect, LogicalSize, PhysicalSize, Scale},
    text_input_api::{TextInputContentHints, TextInputContentPurpose, TextInputContext},
    window_api::{
        window_request_internal_activation_token,
        window_show_open_file_dialog,
        window_show_save_file_dialog,
        window_start_drag_and_drop,
        //
    },
};
use log::{debug, info, warn};
use std::{cell::RefCell, collections::HashMap, env, str::FromStr};
use url::Url;

const APP_ID: &str = "org.jetbrains.desktop.linux.native.sample1";
const TEXT_MIME_TYPE: &str = "text/plain;charset=utf-8";
const URI_LIST_MIME_TYPE: &str = "text/uri-list";

const ALL_MIMES: &str = "text/uri-list,text/plain;charset=utf-8";

#[derive(Debug, Default)]
struct OptionalAppPtr(Option<AppPtr<'static>>);

impl OptionalAppPtr {
    fn get(&self) -> AppPtr<'static> {
        self.0.as_ref().unwrap().clone()
    }
}

#[derive(Debug, Default)]
struct Settings {
    cursor_theme_name: Option<String>,
    cursor_theme_size: Option<u32>,
}

pub trait Drawable {
    fn draw(&mut self, physical_size: PhysicalSize, window_state: &WindowState);
}

#[derive(Default)]
pub struct WindowState {
    pub active: bool,
    maximized: bool,
    fullscreen: bool,
    pub scale: Scale,
    text_input_available: bool,
    composed_text: String,
    text: String,
    pub animation_progress: f32,
    pub drag_and_drop_target: bool,
    pub drag_and_drop_source: bool,
    drawable: Option<Box<dyn Drawable>>,
    last_received_path: Option<String>,
    redraw: bool,
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
    ActivateWindow(WindowId),
    OpenUrl(String),
    OpenFileManager(String),
}

#[derive(Default)]
struct State {
    app_ptr: OptionalAppPtr,
    key_window_id: Option<WindowId>,
    key_modifiers: KeyModifiers,
    windows: HashMap<WindowId, WindowState>,
    drag_icon: Option<WindowState>,
    settings: Settings,
    request_sources: HashMap<RequestId, WindowId>,
    notification_sources: HashMap<u32, WindowId>,
    activation_token_action: HashMap<RequestId, ActivationTokenAction>,
    data_request_sources: HashMap<i32, WindowId>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
    static OBJ_ID_TO_DEALLOC: RefCell<HashMap<i64, Box<dyn FnOnce()>>> = RefCell::default();
}

pub const DRAG_AND_DROP_LEFT_OF: LogicalPixelsInt = LogicalPixelsInt::new(100);

impl State {
    fn add_data_request_source(&mut self, window_id: WindowId) -> i32 {
        let v = self.data_request_sources.keys().max().unwrap_or(&0) + 1;
        self.data_request_sources.insert(v, window_id);
        v
    }

    fn get_window_for_request(&mut self, serial: i32) -> Option<WindowId> {
        self.data_request_sources.remove(&serial)
    }
}

fn create_text_input_context(text: &str, change_caused_by_input_method: bool) -> TextInputContext<'_> {
    let codepoints_count = u16::try_from(text.chars().count()).unwrap();
    TextInputContext {
        surrounding_text: BorrowedUtf8::new(text),
        cursor_codepoint_offset: codepoints_count,
        selection_start_codepoint_offset: codepoints_count,
        hints: TextInputContentHints::Multiline,
        content_purpose: TextInputContentPurpose::Normal,
        cursor_rectangle: LogicalRect {
            x: LogicalPixelsInt::new((codepoints_count * 10).into()),
            y: LogicalPixelsInt::new(100),
            width: LogicalPixelsInt::new(5),
            height: LogicalPixelsInt::new(10),
        },
        change_caused_by_input_method,
    }
}

fn update_text_input_context(app_ptr: AppPtr<'_>, text: &str, change_caused_by_input_method: bool) {
    application_text_input_update(app_ptr, create_text_input_context(text, change_caused_by_input_method));
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

const fn shortcut_modifiers(all_modifiers: KeyModifiers) -> KeyModifiers {
    all_modifiers.and(KeyModifiers::CapsLock.not()).and(KeyModifiers::NumLock.not())
}

#[allow(clippy::too_many_lines)]
fn on_keydown(event: &KeyDownEvent, app_ptr: AppPtr<'_>, state: &mut State) -> Option<Action> {
    const KEY_MODIFIER_NONE: KeyModifiers = KeyModifiers::empty();
    const KEY_MODIFIER_CTRL: KeyModifiers = KeyModifiers::Ctrl;

    let modifiers = shortcut_modifiers(state.key_modifiers);
    let window_id = state.key_window_id.expect("Key window not found");
    let key_code = decode_key_code(event.code.0)?;
    let window_state = state.windows.get_mut(&window_id).unwrap();

    match (modifiers, key_code) {
        (KEY_MODIFIER_NONE, keycode::KeyMappingCode::Backspace) => {
            window_state.text.pop();
            if window_state.text_input_available {
                update_text_input_context(app_ptr, &window_state.text, false);
            }
            debug!("{window_id:?} : {} : {}", window_state.text.len(), window_state.text);
            Some(Action::Dummy)
        }
        (KEY_MODIFIER_NONE, keycode::KeyMappingCode::F11) => {
            if window_state.fullscreen {
                Some(Action::WindowUnsetFullscreen(window_id))
            } else {
                Some(Action::WindowSetFullscreen(window_id))
            }
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::Tab) => {
            if let Some(&window_id) = state.windows.keys().find(|&&w| Some(w) != state.key_window_id) {
                let request_id = window_request_internal_activation_token(app_ptr, state.key_window_id.unwrap());
                if request_id.0 > 0 {
                    state
                        .activation_token_action
                        .insert(request_id, ActivationTokenAction::ActivateWindow(window_id));
                }
            }
            Some(Action::Dummy)
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyQ) => Some(Action::WindowClose(window_id)),
        (KeyModifiers::Ctrl, keycode::KeyMappingCode::KeyM) => {
            if window_state.maximized {
                Some(Action::WindowUnmaximize(window_id))
            } else {
                Some(Action::WindowMaximize(window_id))
            }
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyH) => Some(Action::WindowMinimize(window_id)),
        (KeyModifiers::Ctrl, keycode::KeyMappingCode::KeyV) => Some(Action::ApplicationClipboardPaste {
            serial: state.add_data_request_source(window_id),
            supported_mime_types: TEXT_MIME_TYPE,
        }),
        (KeyModifiers::Ctrl, keycode::KeyMappingCode::KeyC) => Some(Action::ApplicationClipboardPut(ALL_MIMES)),
        (KeyModifiers::Ctrl, keycode::KeyMappingCode::KeyF) => Some(Action::ApplicationClipboardPut("")),
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyP) => {
            let title = format!("Notification from window {}", window_id.0);
            let body = format!("Clicking this notification will activate window {}", window_id.0);
            let request_id =
                application_request_show_notification(app_ptr, BorrowedUtf8::new(&title), BorrowedUtf8::new(&body), BorrowedUtf8::null());
            if request_id.0 != 0 {
                state.request_sources.insert(request_id, window_id);
            }
            Some(Action::Dummy)
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyO) => {
            let common_params = CommonFileDialogParams {
                modal: false,
                title: BorrowedUtf8::new("Open File for Linux Native Sample App test"),
                accept_label: BorrowedUtf8::new("Let's go!"),
                current_folder: BorrowedUtf8::new("/etc"),
            };
            let open_params = OpenFileDialogParams {
                select_directories: false,
                allows_multiple_selection: true,
            };
            let request_id = window_show_open_file_dialog(app_ptr, window_id, &common_params, &open_params);
            debug!("Requested open file dialog for {window_id:?}, request_id = {request_id:?}");
            Some(Action::Dummy)
        }

        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyS) => {
            let common_params = CommonFileDialogParams {
                modal: false,
                title: BorrowedUtf8::new("Save File for Linux Native Sample App test"),
                accept_label: BorrowedUtf8::new("Let's go!"),
                current_folder: BorrowedUtf8::new("/tmp"),
            };
            let save_params = SaveFileDialogParams {
                name_field_string_value: BorrowedUtf8::new("file from Linux Native Sample App.txt"),
            };
            let request_id = window_show_save_file_dialog(app_ptr, window_id, &common_params, &save_params);
            debug!("Requested open file dialog for {window_id:?}, request_id = {request_id:?}");
            Some(Action::Dummy)
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyN) => {
            let new_window_id = WindowId(state.windows.len() as i64 + 1);
            state.windows.insert(new_window_id, WindowState::default());
            Some(Action::WindowCreate {
                window_id: new_window_id,
                size: LogicalSize::wh(300, 200),
                min_size: LogicalSize::wh(0, 0),
                title: "Window N".to_owned(),
                app_id: APP_ID.to_owned(),
                prefer_client_side_decoration: false,
                rendering_mode: RenderingMode::Auto,
            })
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyL) => {
            let request_id = window_request_internal_activation_token(app_ptr, window_id);
            if request_id.0 > 0 {
                state
                    .activation_token_action
                    .insert(request_id, ActivationTokenAction::OpenUrl("https://jetbrains.com".to_owned()));
            }
            Some(Action::Dummy)
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyU) => {
            let window_state = state.windows.get_mut(&window_id).unwrap();
            if let Some(path) = window_state.last_received_path.clone() {
                let request_id = window_request_internal_activation_token(app_ptr, window_id);
                if request_id.0 > 0 {
                    state
                        .activation_token_action
                        .insert(request_id, ActivationTokenAction::OpenFileManager(path));
                }
            }
            Some(Action::Dummy)
        }
        (_, _) => {
            if let Some(s) = event.characters.get_optional("KeyDownEvent: characters").unwrap() {
                let window_state = state.windows.get_mut(&window_id).unwrap();
                window_state.text += s;
            }
            Some(Action::Dummy)
        }
    }
}

fn on_text_input_availability_changed(available: bool, app_ptr: AppPtr<'_>, window_state: &mut WindowState) {
    if available {
        let context = create_text_input_context(&window_state.text, false);
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
        && let Some(commit_string) = event.commit_string.get_optional("TextInputEvent.commit_string").unwrap()
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
        } else if let Some(preedit_string) = event.preedit_string.text.get_optional("TextInputEvent.preedit_string").unwrap() {
            window_state.composed_text.push_str(preedit_string);
        }
    }

    debug!("{window_id:?} : {}", window_state.text);
}

fn on_data_transfer_received(content: &DataTransferContent, window_state: &mut WindowState) {
    if let Some(mime_type) = content.mime_type.get_optional("DataTransferContent.mime_type").unwrap() {
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
                String::from_utf8(path_bytes).unwrap()
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

fn on_application_started(state: &mut State) -> Vec<Action> {
    let window_1_id = WindowId(1);
    state.windows.insert(window_1_id, WindowState::default());

    let window_2_id = WindowId(2);
    state.windows.insert(window_2_id, WindowState::default());

    let mut actions = vec![
        Action::WindowCreate {
            window_id: window_1_id,
            size: LogicalSize::wh(200, 300),
            min_size: LogicalSize::wh(100, 200),
            title: "Window 1".to_owned(),
            app_id: APP_ID.to_owned(),
            prefer_client_side_decoration: true,
            rendering_mode: RenderingMode::Software,
        },
        Action::WindowCreate {
            window_id: window_2_id,
            size: LogicalSize::wh(300, 200),
            min_size: LogicalSize::wh(200, 100),
            title: "Window 2".to_owned(),
            app_id: APP_ID.to_owned(),
            prefer_client_side_decoration: false,
            rendering_mode: RenderingMode::Auto,
        },
    ];

    if let Ok(activation_token) = env::var("XDG_ACTIVATION_TOKEN") {
        actions.push(Action::WindowActivate {
            window_id: window_2_id,
            token: Some(activation_token),
        });
    }

    actions
}

fn new_opengl(app_ptr: AppPtr, prefer_skia: bool) -> Box<dyn Drawable> {
    if prefer_skia {
        #[cfg(feature = "skia")]
        return Box::new(super::sample_linux_draw_skia::SkiaOpenglState::new(app_ptr));
    }
    Box::new(OpenglState::new(app_ptr))
}

fn draw_with_init(app_ptr: AppPtr, software_draw_data: &SoftwareDrawData, physical_size: PhysicalSize, window_state: &mut WindowState) {
    if software_draw_data.canvas.is_null() {
        let mut drawable = window_state.drawable.take().unwrap_or_else(|| new_opengl(app_ptr, true));
        drawable.draw(physical_size, window_state);
        window_state.drawable = Some(drawable);
    } else {
        draw_software(software_draw_data, physical_size, window_state);
    }
}

#[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
fn event_handler_impl(event: &Event) -> (Vec<Action>, AppPtr<'static>) {
    const MOUSE_BUTTON_LEFT: u32 = 0x110;
    const MOUSE_BUTTON_MIDDLE: u32 = 0x112;

    let mut actions = Vec::new();

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
                actions.append(&mut on_application_started(state));
            }
            Event::DisplayConfigurationChange => {
                let ffi_screens = screen_list(app_ptr);
                let screen_infos = unsafe { std::slice::from_raw_parts_mut(ffi_screens.ptr.cast_mut(), ffi_screens.len) };
                println!("DisplayConfigurationChange: {screen_infos:?}");
            }
            Event::DesktopSettingChange(data) => {
                on_desktop_settings_change(data, state);
            }
            Event::WindowScaleChanged(data) => {
                if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                    window_state.scale = data.new_scale;
                }
            }
            Event::WindowConfigure(data) => {
                if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                    window_state.active = data.active;
                    window_state.maximized = data.maximized;
                    window_state.fullscreen = data.fullscreen;
                    window_state.redraw = true;
                }
            }
            Event::WindowDraw(data) => {
                if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                    window_state.animation_tick();

                    draw_with_init(app_ptr, &data.software_draw_data, data.physical_size, window_state);
                    actions.push(Action::Dummy);
                    window_state.redraw = false;
                }
            }
            Event::DragIconDraw(data) => {
                let window_state = state.drag_icon.get_or_insert_default();
                window_state.scale = data.scale;
                window_state.animation_tick();

                draw_with_init(app_ptr, &data.software_draw_data, data.physical_size, window_state);
                actions.push(Action::Dummy);
                window_state.redraw = false;
            }
            Event::WindowCloseRequest(data) => {
                actions.push(Action::WindowClose(data.window_id));
            }
            Event::WindowClosed(data) => {
                state.windows.retain(|&k, _v| k != data.window_id);
                state.request_sources.retain(|_k, &mut v| v != data.window_id);
                for (notification_id, _window_id) in state.notification_sources.extract_if(|_k, &mut v| v != data.window_id) {
                    application_close_notification(app_ptr.clone(), notification_id);
                }
                if state.windows.is_empty() {
                    actions.push(Action::ApplicationStopEventLoop);
                }
            }
            Event::MouseDown(data) => match data.button.0 {
                MOUSE_BUTTON_LEFT => {
                    if let Some(window_state) = state.windows.get_mut(&data.window_id)
                        && data.location_in_window.x < DRAG_AND_DROP_LEFT_OF
                    {
                        let mime_types = if state.key_modifiers == KeyModifiers::Shift {
                            ALL_MIMES
                        } else {
                            TEXT_MIME_TYPE
                        };
                        let dnd_actions = DragAndDropActions(DragAndDropAction::Copy as u32 | DragAndDropAction::Move as u32);
                        let drag_icon_size = LogicalSize::wh(300, 300);
                        window_start_drag_and_drop(
                            app_ptr,
                            data.window_id,
                            BorrowedUtf8::new(mime_types),
                            dnd_actions,
                            RenderingMode::Auto,
                            drag_icon_size,
                        );
                        window_state.drag_and_drop_source = true;
                        window_state.redraw = true;
                        actions.push(Action::Dummy);
                    }
                }
                MOUSE_BUTTON_MIDDLE => {
                    actions.push(Action::ApplicationPrimarySelectionPaste {
                        serial: state.add_data_request_source(data.window_id),
                        supported_mime_types: TEXT_MIME_TYPE,
                    });
                }
                _ => {}
            },
            Event::ModifiersChanged(data) => {
                state.key_modifiers = data.modifiers;
            }
            Event::WindowKeyboardEnter(event) => {
                state.key_window_id = Some(event.window_id);
            }
            Event::WindowKeyboardLeave(event) => {
                assert_eq!(state.key_window_id, Some(event.window_id));
                state.key_window_id = None;
            }
            Event::KeyDown(event) => {
                if let Some(action) = on_keydown(event, app_ptr, state) {
                    actions.push(action);
                }
            }
            Event::FileChooserResponse(file_chooser_response) => {
                if let Some(s) = file_chooser_response
                    .newline_separated_files
                    .get_optional("FileChooserResponse.newline_separated_files")
                    .unwrap()
                {
                    let files = s.trim_ascii_end().split("\r\n").collect::<Vec<_>>();
                    info!("Selected files: {files:?}");
                }
            }
            Event::DataTransfer(data) => {
                if let Some(window_id) = state.get_window_for_request(data.serial)
                    && let Some(window_state) = state.windows.get_mut(&window_id)
                {
                    on_data_transfer_received(&data.content, window_state);
                }
            }
            Event::DropPerformed(data) => {
                if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                    on_data_transfer_received(&data.content, window_state);
                }
            }
            Event::DragAndDropLeave(data) => {
                if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                    window_state.drag_and_drop_target = false;
                    window_state.redraw = true;
                }
            }
            Event::DragAndDropFinished(data) => {
                state.drag_icon = None;
                if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                    window_state.drag_and_drop_source = false;
                    window_state.redraw = true;
                    info!("Finished initiated drag and drop with action {:?}", data.action);
                }
            }
            Event::DataTransferCancelled(data) => {
                if data.data_source == DataSource::DragAndDrop {
                    for window_state in state.windows.values_mut() {
                        window_state.drag_and_drop_source = false;
                        window_state.redraw = true;
                    }
                    state.drag_icon = None;
                }
            }
            Event::TextInputAvailability(data) => {
                if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                    on_text_input_availability_changed(data.available, app_ptr, window_state);
                }
            }
            Event::TextInput(event) => {
                if let Some(key_window_id) = state.key_window_id
                    && let Some(window_state) = state.windows.get_mut(&key_window_id)
                {
                    on_text_input(event, app_ptr, key_window_id, window_state);
                }
            }
            Event::ActivationTokenResponse(data) => {
                let token = data.token.get("ActivationTokenResponse.token").unwrap();
                match state.activation_token_action.remove(&data.request_id).unwrap() {
                    ActivationTokenAction::ActivateWindow(window_id) => {
                        actions.push(Action::WindowActivate {
                            window_id,
                            token: Some(token.to_owned()),
                        });
                    }
                    ActivationTokenAction::OpenFileManager(path) => {
                        application_open_file_manager(app_ptr, BorrowedUtf8::new(&path), BorrowedUtf8::new(token));
                    }
                    ActivationTokenAction::OpenUrl(url) => {
                        application_open_url(app_ptr, BorrowedUtf8::new(&url), BorrowedUtf8::new(token));
                    }
                }
            }
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
                if data.action.get_optional("NotificationClosedEvent.action").unwrap().is_some()
                    && let Some(window_id_to_activate) = state.notification_sources.remove(&data.notification_id)
                {
                    let activation_token = data
                        .activation_token
                        .get_optional("NotificationClosedEvent.activation_token")
                        .unwrap()
                        .map(ToOwned::to_owned);
                    actions.push(Action::WindowActivate {
                        window_id: window_id_to_activate,
                        token: activation_token,
                    });
                }
            }
            _ => {}
        }
        (actions, state.app_ptr.get())
    })
}

extern "C" fn event_handler(event: &Event) -> bool {
    let (actions, app_ptr) = event_handler_impl(event);
    let handled = !actions.is_empty();
    for action in actions {
        action.perform(app_ptr.clone());
    }
    handled
}

fn on_desktop_settings_change(s: &FfiDesktopSetting, state: &mut State) {
    match s {
        FfiDesktopSetting::CursorSize(v) => {
            let size = (*v).try_into().unwrap();
            if let Some(name) = &state.settings.cursor_theme_name {
                application_set_cursor_theme(state.app_ptr.get(), BorrowedUtf8::new(name), size);
            }
            state.settings.cursor_theme_size = Some(size);
        }
        FfiDesktopSetting::CursorTheme(v) => {
            let name = v.get("FfiDesktopSetting::CursorTheme").unwrap().to_owned();
            if let Some(size) = state.settings.cursor_theme_size {
                application_set_cursor_theme(state.app_ptr.get(), BorrowedUtf8::new(&name), size);
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
    if data.location_in_window.x < DRAG_AND_DROP_LEFT_OF {
        const SUPPORTED_ACTIONS_PER_MIME: [FfiSupportedActionsForMime; 2] = [
            FfiSupportedActionsForMime {
                supported_mime_type: BorrowedUtf8::new(URI_LIST_MIME_TYPE),
                supported_actions: DragAndDropActions(DragAndDropAction::Copy as u32),
                preferred_action: DragAndDropAction::Copy,
            },
            FfiSupportedActionsForMime {
                supported_mime_type: BorrowedUtf8::new(TEXT_MIME_TYPE),
                supported_actions: DragAndDropActions(DragAndDropAction::Move as u32 | DragAndDropAction::Copy as u32),
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

extern "C" fn get_data_transfer_data(source: DataSource, mime_type: BorrowedUtf8) -> FfiTransferDataResponse {
    let mime_type_str = mime_type.get("get_data_transfer_data: mime_type").unwrap();
    let v = if mime_type_str == URI_LIST_MIME_TYPE {
        match source {
            DataSource::Clipboard => "file:///etc/hosts",
            DataSource::DragAndDrop => "file:///boot/efi/",
            DataSource::PrimarySelection => "file:///etc/environment",
        }
    } else if mime_type_str == TEXT_MIME_TYPE {
        match source {
            DataSource::Clipboard => "clipboard text",
            DataSource::DragAndDrop => "d&d text",
            DataSource::PrimarySelection => "primary selection text",
        }
    } else {
        mime_type_str
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
