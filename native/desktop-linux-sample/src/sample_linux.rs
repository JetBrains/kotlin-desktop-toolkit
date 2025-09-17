use core::str;
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::{CStr, CString},
    str::FromStr,
};

use desktop_common::{
    ffi_utils::{ArraySize, BorrowedArray, BorrowedStrPtr},
    logger_api::{LogLevel, LoggerConfiguration, logger_init_impl},
};
use desktop_linux::linux::{
    application_api::{
        AppPtr, ApplicationCallbacks, DataSource, DragAction, DragAndDropQueryData, application_clipboard_put,
        application_get_egl_proc_func, application_init, application_is_event_loop_thread, application_run_event_loop,
        application_set_cursor_theme, application_shutdown, application_start_drag_and_drop, application_stop_event_loop,
        application_text_input_disable, application_text_input_enable, application_text_input_update,
    },
    events::{
        DataTransferContent, Event, KeyDownEvent, KeyModifier, KeyModifierBitflag, SoftwareDrawData, TextInputEvent, WindowDrawEvent,
        WindowId,
    },
    file_dialog_api::{CommonFileDialogParams, OpenFileDialogParams, SaveFileDialogParams},
    geometry::{LogicalPixels, LogicalPoint, LogicalRect, LogicalSize, PhysicalSize},
    text_input_api::{TextInputContentPurpose, TextInputContext},
    window_api::{
        WindowParams, window_clipboard_paste, window_close, window_create, window_show_open_file_dialog, window_show_save_file_dialog,
    },
    xdg_desktop_settings_api::XdgDesktopSetting,
};
use log::{debug, error, info};

use crate::gl_sys::{
    GL_COLOR_BUFFER_BIT, GL_COMPILE_STATUS, GL_DEPTH_BUFFER_BIT, GL_FALSE, GL_FLOAT, GL_FRAGMENT_SHADER, GL_LINK_STATUS, GL_TRIANGLES,
    GL_VERTEX_SHADER, GLchar, GLenum, GLint, GLuint, OpenGlFuncs,
};

extern "C" fn on_should_terminate() -> bool {
    println!("on_should_terminate");
    true
}

extern "C" fn on_will_terminate() {
    println!("on_will_terminate");
}

extern "C" fn on_display_configuration_change() {
    println!("on_display_configuration_change");
}

fn between(val: f64, min: f64, max: f64) -> bool {
    val > min && val < max
}

pub const TEXT_MIME_TYPE: &CStr = c"text/plain;charset=utf-8";
pub const URI_LIST_MIME_TYPE: &CStr = c"text/uri-list";

pub const ALL_MIMES: &CStr = c"text/uri-list,text/plain;charset=utf-8";

#[derive(Debug)]
struct OptionalAppPtr(Option<AppPtr<'static>>);

impl OptionalAppPtr {
    fn get(&self) -> AppPtr<'static> {
        self.0.as_ref().unwrap().clone()
    }
}

#[derive(Debug)]
struct OpenglState {
    funcs: OpenGlFuncs,
    programs: HashMap<WindowId, GLuint>,
}

#[derive(Debug, Default)]
struct Settings {
    cursor_theme_name: Option<CString>,
    cursor_theme_size: Option<u32>,
}

#[derive(Debug, Default)]
struct WindowState {
    text_input_available: bool,
    composed_text: String,
    text: String,
    key_modifiers: KeyModifierBitflag,
    animation_progress: u16,
}

#[derive(Debug)]
struct State {
    app_ptr: OptionalAppPtr,
    windows: HashMap<WindowId, WindowState>,
    opengl: Option<OpenglState>,
    settings: Settings,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
            app_ptr: OptionalAppPtr(None),
            windows: HashMap::new(),
            opengl: None,
            settings: Settings::default(),
        });
}

const V_POSITION: GLuint = 0;

fn load_shader(gl: &OpenGlFuncs, shader_type: GLenum, shader_src: *const GLchar) -> Option<GLuint> {
    // Create the shader object
    let shader = unsafe { (gl.glCreateShader)(shader_type) };
    if shader == 0 {
        return None;
    }
    // Load the shader source
    unsafe { (gl.glShaderSource)(shader, 1, &raw const shader_src, std::ptr::null()) };
    // Compile the shader
    unsafe { (gl.glCompileShader)(shader) };
    // Check the compile status
    {
        let mut compiled: GLint = 0;
        unsafe { (gl.glGetShaderiv)(shader, GL_COMPILE_STATUS, &raw mut compiled) };
        if compiled == 0 {
            unsafe { (gl.glDeleteShader)(shader) };
            return None;
        }
    }
    Some(shader)
}

/// Initialize the shader and program object
fn create_opengl_program(gl: &OpenGlFuncs) -> Option<GLuint> {
    const V_SHADER_STR: &CStr = c"attribute vec4 vPosition;
void main()
{
  gl_Position = vPosition;
}
";
    const F_SHADER_STR: &CStr = c"precision mediump float;
void main()
{
  gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
}
";
    // Load the vertex/fragment shaders
    let vertex_shader = load_shader(gl, GL_VERTEX_SHADER, V_SHADER_STR.as_ptr()).unwrap();
    let fragment_shader = load_shader(gl, GL_FRAGMENT_SHADER, F_SHADER_STR.as_ptr()).unwrap();
    // Create the program object
    unsafe {
        let program = (gl.glCreateProgram)();
        if program == 0 {
            return None;
        }
        (gl.glAttachShader)(program, vertex_shader);
        (gl.glAttachShader)(program, fragment_shader);
        // Bind vPosition to attribute 0
        (gl.glBindAttribLocation)(program, V_POSITION, c"vPosition".as_ptr());
        (gl.glLinkProgram)(program);
        // Check the link status
        {
            let mut linked: GLint = 0;
            (gl.glGetProgramiv)(program, GL_LINK_STATUS, &raw mut linked);
            if linked == 0 {
                (gl.glDeleteProgram)(program);
                return None;
            }
        }
        (gl.glClearColor)(0.0, 1.0, 0.0, 1.0);
        Some(program)
    }
}

/// Draw a triangle using the shader pair created in `Init()`
fn draw_opengl_triangle(gl: &OpenGlFuncs, program: GLuint, data: &WindowDrawEvent, animation_progress: f32) {
    //    debug!("draw_opengl_triangle, program = {program}, event = {data:?}");
    let v_vertices: [f32; 6] = [animation_progress, 1.0, -1.0, -1.0, 1.0, -1.0];
    unsafe {
        (gl.glViewport)(0, 0, data.physical_size.width.0, data.physical_size.height.0);
        (gl.glClear)(GL_DEPTH_BUFFER_BIT | GL_COLOR_BUFFER_BIT);
        (gl.glUseProgram)(program);
        //let v_position = (gl.glGetAttribLocation)(program, c"vPosition".as_ptr());
        //assert!(v_position != -1);
        // Load the vertex data
        (gl.glVertexAttribPointer)(V_POSITION, 2, GL_FLOAT, GL_FALSE, 0, v_vertices.as_ptr().cast());
        (gl.glEnableVertexAttribArray)(V_POSITION);
        (gl.glDrawArrays)(GL_TRIANGLES, 0, 3);
    }
}

fn draw_opengl_triangle_with_init(
    data: &WindowDrawEvent,
    app_ptr: AppPtr<'_>,
    window_id: WindowId,
    window_state: &mut WindowState,
    opengl_state: &mut Option<OpenglState>,
) {
    let opengl_state = opengl_state.get_or_insert_with(|| {
        let egl_lib = application_get_egl_proc_func(app_ptr);
        let funcs = OpenGlFuncs::new(&egl_lib).unwrap();
        let program = create_opengl_program(&funcs).unwrap();
        debug!("draw_opengl_triangle_with_init, program = {program}");
        let mut programs = HashMap::new();
        programs.insert(window_id, program);
        OpenglState { funcs, programs }
    });
    let program = opengl_state
        .programs
        .entry(window_id)
        .or_insert_with(|| create_opengl_program(&opengl_state.funcs).unwrap());
    if window_state.animation_progress == 200 {
        window_state.animation_progress = 0;
    } else {
        window_state.animation_progress += 1;
    }

    let animation_progress = if window_state.animation_progress < 100 {
        -1.0 + (f32::from(window_state.animation_progress) / 50.)
    } else {
        1.0 - (f32::from(window_state.animation_progress - 100) / 50.)
    };

    draw_opengl_triangle(&opengl_state.funcs, *program, data, animation_progress);
}

#[allow(clippy::many_single_char_names)]
fn draw_software(data: &SoftwareDrawData, physical_size: PhysicalSize, scale: f64) {
    const BYTES_PER_PIXEL: u8 = 4;
    let canvas = {
        let len = usize::try_from(physical_size.height.0 * data.stride).unwrap();
        unsafe { std::slice::from_raw_parts_mut(data.canvas, len) }
    };
    let w = f64::from(physical_size.width.0);
    let h = f64::from(physical_size.height.0);
    let line_thickness = 5.0 * scale;

    // Order of bytes in `pixel` is [b, g, r, a] (for the Argb8888 format)
    for (pixel, i) in canvas.chunks_exact_mut(BYTES_PER_PIXEL.into()).zip(1u32..) {
        let i = f64::from(i);
        let x = i % w;
        let y = (i / f64::from(data.stride)) * f64::from(BYTES_PER_PIXEL);
        if between(x, line_thickness,  line_thickness * 2.0)  // left border
           || between(y, line_thickness,  line_thickness * 2.0)  // top border
           || between(x, line_thickness.mul_add(-2.0, w), w - line_thickness)  // right border
           || between(y, line_thickness.mul_add(-2.0, h), h - line_thickness)  // bottom border
           || between(x, (i / h) - (line_thickness / 2.0), (i / h) + (line_thickness / 2.0))
        {
            pixel[0] = 0;
            pixel[1] = 0;
        } else {
            pixel[0] = 255;
            pixel[1] = 255;
        }
        pixel[2] = 255;
        pixel[3] = 255;
    }
}

fn create_text_input_context<'a>(text: &str, text_cstring: &'a CString, change_caused_by_input_method: bool) -> TextInputContext<'a> {
    let codepoints_count = u16::try_from(text.chars().count()).unwrap();
    TextInputContext {
        surrounding_text: BorrowedStrPtr::new(text_cstring),
        cursor_codepoint_offset: codepoints_count,
        selection_start_codepoint_offset: codepoints_count,
        is_multiline: true,
        content_purpose: TextInputContentPurpose::Normal,
        cursor_rectangle: LogicalRect {
            origin: LogicalPoint {
                x: LogicalPixels(f64::from(codepoints_count) * 10.0),
                y: LogicalPixels(100.0),
            },
            size: LogicalSize {
                width: LogicalPixels(5.0),
                height: LogicalPixels(10.0),
            },
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

const fn shortcut_modifiers(all_modifiers: KeyModifierBitflag) -> KeyModifierBitflag {
    KeyModifierBitflag(all_modifiers.0 & !(KeyModifier::CapsLock as u8) & !(KeyModifier::NumLock as u8))
}

fn on_keydown(event: &KeyDownEvent, app_ptr: AppPtr<'_>, window_id: WindowId, window_state: &mut WindowState) {
    const KEYCODE_BACKSPACE: u32 = 14;
    const KEYCODE_C: u32 = 46;
    const KEYCODE_O: u32 = 24;
    const KEYCODE_S: u32 = 31;
    const KEYCODE_V: u32 = 47;
    const KEY_MODIFIER_CTRL: u8 = KeyModifier::Ctrl as u8;

    let modifiers: KeyModifierBitflag = shortcut_modifiers(window_state.key_modifiers);
    let key_code: u32 = event.code.0;

    match (modifiers.0, key_code) {
        (0, KEYCODE_BACKSPACE) => {
            window_state.text.pop();
            if window_state.text_input_available {
                update_text_input_context(app_ptr, &window_state.text, false);
            }
            debug!("{window_id:?} : {} : {}", window_state.text.len(), window_state.text);
        }
        (KEY_MODIFIER_CTRL, KEYCODE_V) => {
            window_clipboard_paste(app_ptr, window_id, 0, BorrowedStrPtr::new(TEXT_MIME_TYPE));
        }
        (KEY_MODIFIER_CTRL, KEYCODE_C) => {
            application_clipboard_put(app_ptr, BorrowedStrPtr::new(ALL_MIMES));
        }
        (KEY_MODIFIER_CTRL, KEYCODE_O) => {
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
        }

        (KEY_MODIFIER_CTRL, KEYCODE_S) => {
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
        }
        (_, _) => {}
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
    if event.has_commit_string {
        if let Some(commit_string) = event.commit_string.as_optional_str().unwrap() {
            debug!("{window_id:?} commit_string: {commit_string}");
            window_state.text += commit_string;
        }
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
    if content
        .mime_types
        .as_str()
        .unwrap()
        .split(',')
        .any(|s| s == URI_LIST_MIME_TYPE.to_str().unwrap())
    {
        let list_str = str::from_utf8(content.data.as_slice().unwrap()).unwrap();
        let list = list_str.trim_ascii_end().split("\r\n").collect::<Vec<_>>();
        info!("Pasted file list: {list:?}");
    } else if content
        .mime_types
        .as_str()
        .unwrap()
        .split(',')
        .any(|s| s == TEXT_MIME_TYPE.to_str().unwrap())
    {
        let data_str = str::from_utf8(content.data.as_slice().unwrap()).unwrap();
        window_state.text += data_str;
    }
}

extern "C" fn event_handler(event: &Event, window_id: WindowId) -> bool {
    match event {
        Event::WindowDraw(_) | Event::MouseMoved(_) => {}
        _ => {
            debug!("event_handler: window_id={window_id:?} : {event:?}");
        }
    }

    STATE.with_borrow_mut(|state| {
        let app_ptr = state.app_ptr.get();
        let is_event_loop_thread = application_is_event_loop_thread(app_ptr.clone());
        assert!(is_event_loop_thread);

        match event {
            Event::WindowDraw(event) => {
                if event.software_draw_data.canvas.is_null() {
                    let window_state = state.windows.get_mut(&window_id).unwrap();
                    draw_opengl_triangle_with_init(event, app_ptr, window_id, window_state, &mut state.opengl);
                } else {
                    draw_software(&event.software_draw_data, event.physical_size, event.scale);
                }
                return true;
            }
            Event::WindowCloseRequest => {
                state.windows.retain(|&k, _v| k != window_id);
                window_close(app_ptr.clone(), window_id);
                if state.windows.is_empty() {
                    application_stop_event_loop(app_ptr);
                }
            }
            Event::MouseDown(_) => {
                application_start_drag_and_drop(app_ptr, window_id, BorrowedStrPtr::new(ALL_MIMES), DragAction::Copy);
            }
            Event::ModifiersChanged(data) => {
                let window_state = state.windows.get_mut(&window_id).unwrap();
                window_state.key_modifiers = data.modifiers;
            }
            Event::KeyDown(event) => {
                let window_state = state.windows.get_mut(&window_id).unwrap();
                on_keydown(event, app_ptr, window_id, window_state);
            }
            Event::FileChooserResponse(file_chooser_response) => match file_chooser_response.newline_separated_files.as_optional_str() {
                Ok(s) => {
                    let files = s.map(|s| s.trim_ascii_end().split("\r\n").collect::<Vec<_>>());
                    info!("Selected files: {files:?}");
                }
                Err(e) => error!("{e}"),
            },
            Event::DataTransfer(content) => {
                let window_state = state.windows.get_mut(&window_id).unwrap();
                on_data_transfer_received(content, window_state);
            }
            Event::TextInputAvailability(data) => {
                let window_state = state.windows.get_mut(&window_id).unwrap();
                on_text_input_availability_changed(data.available, app_ptr, window_state);
            }
            Event::TextInput(event) => {
                let window_state = state.windows.get_mut(&window_id).unwrap();
                on_text_input(event, app_ptr, window_id, window_state);
            }
            _ => {}
        }
        true
    })
}

extern "C" fn on_xdg_desktop_settings_change(s: &XdgDesktopSetting) {
    debug!("on_xdg_desktop_settings_change start: {s:?}");
    STATE.with_borrow_mut(|state| match s {
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
    });
    debug!("on_xdg_desktop_settings_change end");
}

extern "C" fn on_application_started() {
    const APP_ID: &CStr = c"org.jetbrains.desktop.linux.native.sample1";
    debug!("on_application_started start");
    STATE.with_borrow_mut(|state| {
        let window_1_id = WindowId(1);
        window_create(
            state.app_ptr.get(),
            WindowParams {
                window_id: window_1_id,
                size: LogicalSize {
                    width: LogicalPixels(200.),
                    height: LogicalPixels(300.),
                },
                title: BorrowedStrPtr::new(c"Window 1"),
                app_id: BorrowedStrPtr::new(APP_ID),
                prefer_client_side_decoration: false,
                force_software_rendering: true,
            },
        );
        state.windows.insert(window_1_id, WindowState::default());

        let window_2_id = WindowId(2);
        window_create(
            state.app_ptr.get(),
            WindowParams {
                window_id: window_2_id,
                size: LogicalSize {
                    width: LogicalPixels(300.),
                    height: LogicalPixels(200.),
                },
                title: BorrowedStrPtr::new(c"Window 2"),
                app_id: BorrowedStrPtr::new(APP_ID),
                prefer_client_side_decoration: true,
                force_software_rendering: false,
            },
        );
        state.windows.insert(window_2_id, WindowState::default());
    });
    debug!("on_application_started end");
}

extern "C" fn get_drag_and_drop_supported_mime_types(data: &DragAndDropQueryData) -> BorrowedStrPtr<'static> {
    if data.point.x.0 < 100. {
        BorrowedStrPtr::new(ALL_MIMES)
    } else {
        BorrowedStrPtr::new(c"")
    }
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
        }
    } else if mime_type_cstr == TEXT_MIME_TYPE {
        match source {
            DataSource::Clipboard => "/etc/hosts (from clipboard)",
            DataSource::DragAndDrop => "/boot/efi/ (from d&d)",
        }
    } else {
        mime_type_cstr.to_str().unwrap()
    };

    let mut a = BorrowedArray::from_slice(leaked_string_data(v));
    a.deinit = Some(deinit_u8_vec);
    a
}

extern "C" fn on_data_transfer_cancelled(source: DataSource) {
    debug!("on_data_transfer_cancelled: {source:?}");
}

pub fn main() {
    logger_init_impl(&LoggerConfiguration {
        file_path: BorrowedStrPtr::new(c"/tmp/a"),
        console_level: LogLevel::Debug,
        file_level: LogLevel::Error,
    });
    let app_ptr = application_init(ApplicationCallbacks {
        on_application_started,
        on_should_terminate,
        on_will_terminate,
        on_display_configuration_change,
        on_xdg_desktop_settings_change,
        event_handler,
        get_drag_and_drop_supported_mime_types,
        get_data_transfer_data,
        on_data_transfer_cancelled,
    });
    STATE.with_borrow_mut(|state| {
        state.app_ptr = OptionalAppPtr(Some(app_ptr.clone()));
    });
    application_run_event_loop(app_ptr.clone());
    application_shutdown(app_ptr);
}
