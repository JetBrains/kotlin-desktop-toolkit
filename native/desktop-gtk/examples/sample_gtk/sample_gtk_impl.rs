//noinspection DuplicatedCode

#[cfg(not(feature = "skia"))]
use crate::sample_gtk_draw::{OpenglState, draw};
#[cfg(feature = "skia")]
use crate::sample_gtk_draw_skia::{OpenglState, draw};

use crate::sample_gtk_actions::Action;
use core::str;
use desktop_common::{
    ffi_utils::{BorrowedArray, BorrowedStrPtr},
    logger_api::{LogLevel, LoggerConfiguration, logger_init_impl},
};
use desktop_gtk::gtk::application_api::{
    FfiTextInputSurroundingText, FfiTransferDataResponse, application_close_notification, application_open_file_manager,
    application_open_url, application_request_redraw_drag_icon, application_request_show_notification,
};
use desktop_gtk::gtk::desktop_settings_api::{FfiDesktopSetting, XdgDesktopColorScheme};
use desktop_gtk::gtk::events::{OpenGlDrawData, WindowDecorationMode};
use desktop_gtk::gtk::file_dialog_api::{CommonFileDialogParams, OpenFileDialogParams, SaveFileDialogParams};
use desktop_gtk::gtk::text_input_api::{TextInputContentPurpose, TextInputContext, TextInputContextHint, TextInputContextHintBitflag};
use desktop_gtk::gtk::{
    application_api::{
        ApplicationCallbacks,
        DataSource,
        DragAndDropAction,
        DragAndDropActions,
        DragAndDropQueryData,
        FfiDragAndDropQueryResponse,
        FfiSupportedActionsForMime,
        RenderingMode,
        application_init,
        application_is_event_loop_thread,
        application_run_event_loop,
        //
    },
    events::{DataTransferContent, Event, KeyDownEvent, KeyModifier, KeyModifierBitflag, RequestId, TextInputEvent, WindowId},
    geometry::{LogicalRect, LogicalSize, PhysicalSize},
    window_api::{
        window_request_redraw,
        window_show_open_file_dialog,
        window_show_save_file_dialog,
        window_start_drag_and_drop,
        window_text_input_update,
        //
    },
};
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

const DRAG_WIDTH_NEXT_TO_INSETS: i32 = 50;
const DRAG_AND_DROP_LEFT_OF: f64 = 100.;

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
    scale: f64,
    size: LogicalSize,
    inset_start: LogicalSize,
    inset_end: LogicalSize,
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
    key_modifiers: KeyModifierBitflag,
    windows: HashMap<WindowId, WindowState>,
    drag_icon: Option<WindowState>,
    request_sources: HashMap<RequestId, WindowId>,
    notification_sources: HashMap<u32, WindowId>,
    key_window_id: Option<WindowId>,
}

impl State {
    fn with_mut_window_state<T>(&mut self, window_id: WindowId, f: impl FnOnce(&mut WindowState) -> T) -> Option<T> {
        if let Some(window) = self.windows.get_mut(&window_id) {
            Some(f(window))
        } else {
            warn!("with_mut_window_state: cannot find {window_id:?}");
            None
        }
    }

    fn next_window_id(&self) -> WindowId {
        let r = self.windows.keys().max_by_key(|e| e.0).map(|v| v.0 + 1).unwrap_or_default();
        WindowId(r)
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::default();
    static OBJ_ID_TO_DEALLOC: RefCell<HashMap<i64, Box<dyn FnOnce()>>> = RefCell::default();
}

fn with_borrow_mut_state<T>(f: impl FnOnce(&mut State) -> T) -> T {
    STATE.with_borrow_mut(f)
}

fn draw_with_init(draw_data: &OpenGlDrawData, physical_size: PhysicalSize, scale: f64, window_state: &mut WindowState) {
    let opengl_state = window_state.opengl.get_or_insert_with(|| OpenglState::new(draw_data));
    #[allow(clippy::cast_possible_truncation)]
    draw(opengl_state, physical_size, scale as f32, window_state.animation_progress);
}

fn create_text_input_context(text: &str) -> TextInputContext {
    let codepoints_count = u16::try_from(text.chars().count()).unwrap();
    TextInputContext {
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
fn on_keydown(event: &KeyDownEvent, state: &mut State, window_id: WindowId) -> Option<Action> {
    const KEY_MODIFIER_CTRL: u8 = KeyModifier::Ctrl as u8;

    let modifiers: KeyModifierBitflag = shortcut_modifiers(state.key_modifiers);
    let key_code = decode_key_code(event.code.0)?;

    let window_state = state.windows.get_mut(&window_id).unwrap();

    #[allow(clippy::single_match_else)]
    match (modifiers.0, key_code) {
        (0, keycode::KeyMappingCode::Escape) => Some(Action::ApplicationStopDragAndDrop),
        (0, keycode::KeyMappingCode::Backspace) => {
            window_state.text.pop();
            if window_state.text_input_available {
                window_text_input_update(window_id, create_text_input_context(&window_state.text));
            }
            debug!("{window_id:?} : {} : {}", window_state.text.len(), window_state.text);
            Some(Action::Dummy)
        }
        (0, keycode::KeyMappingCode::F11) => {
            if window_state.fullscreen {
                Some(Action::WindowUnsetFullscreen(window_id))
            } else {
                Some(Action::WindowSetFullscreen(window_id))
            }
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::Tab) => {
            state
                .windows
                .keys()
                .find(|&&w| Some(w) != state.key_window_id)
                .map(|window_id| Action::WindowActivate {
                    window_id: *window_id,
                    token: None,
                })
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyQ) => Some(Action::WindowClose(window_id)),
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyM) => {
            if window_state.maximized {
                Some(Action::WindowUnmaximize(window_id))
            } else {
                Some(Action::WindowMaximize(window_id))
            }
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyV) => Some(Action::ApplicationClipboardPaste {
            serial: 0,
            supported_mime_types: TEXT_MIME_TYPE,
        }),
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyC) => Some(Action::ApplicationClipboardPut(ALL_MIMES)),
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyF) => Some(Action::ApplicationClipboardPut(c"")),
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyP) => {
            let title = format!("Notification from window {}", window_id.0);
            let body = format!("Clicking this notification will activate window {}", window_id.0);
            let title_cstr = CString::new(title).unwrap();
            let body_cstr = CString::new(body).unwrap();
            let request_id = application_request_show_notification(
                BorrowedStrPtr::new(&title_cstr),
                BorrowedStrPtr::new(&body_cstr),
                BorrowedStrPtr::null(),
            );
            if request_id.0 != 0 {
                state.request_sources.insert(request_id, window_id);
            }
            Some(Action::Dummy)
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
            let request_id = window_show_open_file_dialog(window_id, &common_params, &open_params);
            debug!("Requested open file dialog for {window_id:?}, request_id = {request_id:?}");
            Some(Action::Dummy)
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
            let request_id = window_show_save_file_dialog(window_id, &common_params, &save_params);
            debug!("Requested open file dialog for {window_id:?}, request_id = {request_id:?}");
            Some(Action::Dummy)
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyN) => {
            let new_window_id = state.next_window_id();
            state.windows.insert(new_window_id, WindowState::default());
            Some(Action::WindowCreate {
                window_id: new_window_id,
                size: LogicalSize { width: 300, height: 200 },
                min_size: LogicalSize { width: 0, height: 0 },
                title: c"Window N".to_owned(),
                decoration_mode: WindowDecorationMode::Server,
                rendering_mode: RenderingMode::Auto,
            })
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyL) => {
            application_open_url(BorrowedStrPtr::new(c"https://jetbrains.com"), BorrowedStrPtr::null());
            Some(Action::Dummy)
        }
        (KEY_MODIFIER_CTRL, keycode::KeyMappingCode::KeyU) => {
            if let Some(path) = window_state.last_received_path.clone() {
                application_open_file_manager(BorrowedStrPtr::new(&path), BorrowedStrPtr::null());
            }
            Some(Action::Dummy)
        }
        (_, _) => {
            if event.has_character {
                window_state.text.push(event.character);
                if window_state.text_input_available {
                    window_text_input_update(window_id, create_text_input_context(&window_state.text));
                }
                Some(Action::Dummy)
            } else {
                None
            }
        }
    }
}

fn on_text_input_availability_changed(available: bool, window_id: WindowId, window_state: &mut WindowState) -> Action {
    window_state.text_input_available = available;
    if available && window_id != WindowId(1) {
        let context = create_text_input_context(&window_state.text);
        Action::WindowTextInputEnable(window_id, context)
    } else {
        Action::WindowTextInputDisable(window_id)
    }
}

fn on_text_input(event: &TextInputEvent, window_id: WindowId, window_state: &mut WindowState) {
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
        window_text_input_update(window_id, create_text_input_context(&window_state.text));
    }

    if event.has_preedit_string
        && event.preedit_string.cursor_byte_pos != -1
        && let Some(preedit_string) = event.preedit_string.text.as_optional_str().unwrap()
    {
        window_state.composed_text.push_str(preedit_string);
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

fn on_application_started(state: &mut State) -> Vec<Action> {
    state.windows.insert(WindowId(1), WindowState::default());
    state.windows.insert(WindowId(2), WindowState::default());

    vec![
        Action::WindowCreate {
            window_id: WindowId(1),
            size: LogicalSize { width: 200, height: 300 },
            min_size: LogicalSize { width: 200, height: 100 },
            title: c"Window 1".to_owned(),
            decoration_mode: WindowDecorationMode::Server,
            rendering_mode: RenderingMode::Auto,
        },
        Action::WindowCreate {
            window_id: WindowId(2),
            size: LogicalSize { width: 600, height: 200 },
            min_size: LogicalSize { width: 200, height: 100 },
            title: c"Window 2".to_owned(),
            decoration_mode: WindowDecorationMode::CustomTitlebar(40),
            rendering_mode: RenderingMode::Auto,
        },
    ]
}

#[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
fn event_handler_impl(event: &Event) -> Vec<Action> {
    const MOUSE_BUTTON_LEFT: u32 = 1;
    const MOUSE_BUTTON_MIDDLE: u32 = 2;

    let is_event_loop_thread = application_is_event_loop_thread();
    assert!(is_event_loop_thread);

    let mut actions = Vec::new();
    match event {
        Event::WindowFrameTick(_) | Event::DragIconFrameTick | Event::WindowDraw(_) | Event::MouseMoved(_) => {}
        Event::DataTransferAvailable(_) => return actions,
        _ => {
            debug!("event_handler: {event:?}");
        }
    }

    with_borrow_mut_state(|state| match event {
        Event::ApplicationStarted => {
            actions.append(&mut on_application_started(state));
        }
        Event::DesktopSettingChange(FfiDesktopSetting::ColorScheme(color_scheme)) => {
            actions.push(Action::ApplicationSetPreferDarkTheme(matches!(
                color_scheme,
                XdgDesktopColorScheme::PreferDark
            )));
        }
        Event::WindowClosed(data) => {
            state.windows.retain(|&k, _v| k != data.window_id);
            state.request_sources.retain(|_k, &mut v| v != data.window_id);
            for (notification_id, _window_id) in state.notification_sources.extract_if(|_k, &mut v| v == data.window_id) {
                application_close_notification(notification_id);
            }
            if state.windows.is_empty() {
                actions.push(Action::ApplicationStopEventLoop);
            }
        }
        Event::WindowConfigure(data) => {
            if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                window_state.active = data.active;
                window_state.fullscreen = data.fullscreen;
                window_state.maximized = data.maximized;
                window_state.size = data.size;
                window_state.inset_start = data.inset_start;
                window_state.inset_end = data.inset_end;
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

                draw_with_init(&data.opengl_draw_data, data.physical_size, window_state.scale, window_state);
            }
        }
        Event::DragIconDraw(data) => {
            debug!("Event::DragIconDraw");
            let window_state = state.drag_icon.get_or_insert_default();
            window_state.animation_tick();

            draw_with_init(&data.opengl_draw_data, data.physical_size, data.scale, window_state);
        }
        Event::MouseDown(data) => {
            if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                let x = data.location_in_window.x.0;
                let y = data.location_in_window.y.0;
                match data.button.0 {
                    MOUSE_BUTTON_LEFT => {
                        if x < DRAG_AND_DROP_LEFT_OF && y > f64::from(window_state.inset_start.height) {
                            let mime_types = if state.key_modifiers.0 == KeyModifier::Shift as u8 {
                                ALL_MIMES
                            } else {
                                TEXT_MIME_TYPE
                            };
                            let dnd_actions = DragAndDropActions(DragAndDropAction::Copy as u8 | DragAndDropAction::Move as u8);
                            let drag_icon_size = LogicalSize { width: 300, height: 300 };
                            window_start_drag_and_drop(
                                data.window_id,
                                BorrowedStrPtr::new(mime_types),
                                dnd_actions,
                                RenderingMode::Auto,
                                drag_icon_size,
                            );
                            if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                                window_state.drag_and_drop_source = true;
                            }
                            actions.push(Action::Dummy);
                        } else if y <= f64::from(window_state.inset_start.height)
                            && x > f64::from(window_state.inset_start.width + DRAG_WIDTH_NEXT_TO_INSETS)
                            && x < f64::from(window_state.size.width - window_state.inset_end.width - DRAG_WIDTH_NEXT_TO_INSETS)
                        {
                            actions.push(Action::Dummy);
                        }
                    }
                    MOUSE_BUTTON_MIDDLE => {
                        actions.push(Action::ApplicationPrimarySelectionPaste {
                            serial: 1,
                            supported_mime_types: TEXT_MIME_TYPE,
                        });
                    }
                    _ => {}
                }
            }
        }
        Event::ModifiersChanged(data) => {
            state.key_modifiers = data.modifiers;
        }
        Event::WindowKeyboardEnter(data) => {
            state.key_window_id = Some(data.window_id);
            state.with_mut_window_state(data.window_id, |window_state| {
                actions.push(on_text_input_availability_changed(true, data.window_id, window_state));
            });
        }
        Event::WindowKeyboardLeave(data) => {
            if state.key_window_id == Some(data.window_id) {
                state.key_window_id = None;
            }
            state.with_mut_window_state(data.window_id, |window_state| {
                actions.push(on_text_input_availability_changed(false, data.window_id, window_state));
            });
        }
        Event::KeyDown(data) => {
            if let Some(action) = on_keydown(data, state, data.window_id) {
                actions.push(action);
            }
        }
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
        Event::DropPerformed(data) => {
            state.with_mut_window_state(data.window_id, |window_state| {
                on_data_transfer_received(&data.content, window_state);
            });
        }
        Event::DragAndDropLeave(data) => {
            state.with_mut_window_state(data.window_id, |window_state| {
                window_state.drag_and_drop_target = false;
            });
        }
        Event::DragAndDropFinished(data) => {
            state.with_mut_window_state(data.window_id, |window_state| {
                window_state.drag_and_drop_source = false;
                info!("Finished initiated drag and drop with action {:?}", data.action);
            });
        }
        Event::DataTransferCancelled(data) => {
            if data.data_source == DataSource::DragAndDrop {
                for window_state in state.windows.values_mut() {
                    window_state.drag_and_drop_source = false;
                }
            }
        }
        Event::TextInput(data) => {
            state.with_mut_window_state(data.window_id, |window_state| {
                on_text_input(data, data.window_id, window_state);
            });
        }
        Event::NotificationShown(data) => {
            if data.notification_id > 0 {
                if let Some(requester) = state.request_sources.remove(&data.request_id) {
                    state.notification_sources.insert(data.notification_id, requester);
                } else {
                    application_close_notification(data.notification_id);
                }
            }
        }
        Event::NotificationClosed(data) => {
            if data.action.as_optional_cstr().is_some()
                && let Some(window_id_to_activate) = state.notification_sources.remove(&data.notification_id)
            {
                let activation_token = data.activation_token.as_optional_cstr().map(ToOwned::to_owned);
                actions.push(Action::WindowActivate {
                    window_id: window_id_to_activate,
                    token: activation_token,
                });
            }
        }
        Event::WindowFrameTick(data) => {
            window_request_redraw(data.window_id);
        }
        Event::DragIconFrameTick => {
            application_request_redraw_drag_icon();
        }
        _ => {}
    });
    actions
}

#[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
extern "C" fn event_handler(event: &Event) -> bool {
    let actions = event_handler_impl(event);
    let handled = !actions.is_empty();
    for action in actions {
        action.perform();
    }
    handled
}

extern "C" fn query_drag_and_drop_target(data: &DragAndDropQueryData) -> FfiDragAndDropQueryResponse {
    let inset_height = with_borrow_mut_state(|state| {
        state.with_mut_window_state(data.window_id, |window_state| {
            window_state.drag_and_drop_target = true;
            f64::from(window_state.inset_start.height)
        })
    });
    if data.location_in_window.x.0 < DRAG_AND_DROP_LEFT_OF && data.location_in_window.y.0 > inset_height.unwrap_or_default() {
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

const extern "C" fn window_close_request(_window_id: WindowId) -> bool {
    true
}

const extern "C" fn application_wants_to_terminate() -> bool {
    true
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
    } else if mime_type_cstr == HTML_TEXT_MIME_TYPE {
        match source {
            DataSource::Clipboard => "<b>some html text</b>",
            DataSource::DragAndDrop => "<b>some html d&d text</b>",
            DataSource::PrimarySelection => "<b>some html primary selection text</b>",
        }
    } else {
        mime_type_cstr.to_str().unwrap()
    };

    let (static_str, obj_id) = leak_string_data(v.to_owned());
    let data = BorrowedArray::from_slice(static_str);
    FfiTransferDataResponse { obj_id, data }
}

extern "C" fn retrieve_surrounding_text(window_id: WindowId) -> FfiTextInputSurroundingText {
    let text = with_borrow_mut_state(|state| {
        state
            .with_mut_window_state(window_id, |window_state| window_state.text.clone())
            .unwrap()
    });
    let codepoints_count = u16::try_from(text.chars().count()).unwrap();
    let (static_str, obj_id) = leak_string_data(text);
    let surrounding_text = BorrowedArray::from_slice(static_str);
    FfiTextInputSurroundingText {
        obj_id,
        surrounding_text,
        cursor_codepoint_offset: codepoints_count,
        selection_start_codepoint_offset: codepoints_count,
    }
}

pub fn main_impl() {
    logger_init_impl(&LoggerConfiguration {
        file_path: BorrowedStrPtr::new(c"/tmp/a"),
        console_level: LogLevel::Debug,
        file_level: LogLevel::Error,
    });
    application_init(BorrowedStrPtr::new(APP_ID));
    application_run_event_loop(ApplicationCallbacks {
        obj_dealloc,
        event_handler,
        query_drag_and_drop_target,
        get_data_transfer_data,
        retrieve_surrounding_text,
        window_close_request,
        application_wants_to_terminate,
    });
}
