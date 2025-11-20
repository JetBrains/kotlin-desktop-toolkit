//noinspection DuplicatedCode

use crate::gl_sys::{
    GL_COLOR_BUFFER_BIT, GL_COMPILE_STATUS, GL_DEPTH_BUFFER_BIT, GL_FALSE, GL_FLOAT, GL_FRAGMENT_SHADER, GL_LINK_STATUS, GL_TRIANGLES,
    GL_VERTEX_SHADER, GLchar, GLenum, GLint, GLuint, OpenGlFuncs,
};
use core::str;
use desktop_common::{
    ffi_utils::{ArraySize, BorrowedArray, BorrowedStrPtr},
    logger_api::{LogLevel, LoggerConfiguration, logger_init_impl},
};
use desktop_linux_x11::linux::geometry::LogicalPixels;
use desktop_linux_x11::linux::{
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
        application_get_egl_proc_func,
        application_init,
        application_is_event_loop_thread,
        application_run_event_loop,
        application_shutdown,
        application_stop_event_loop,
        //
    },
    events::{
        DataTransferContent, Event, KeyDownEvent, KeyModifier, KeyModifierBitflag, RequestId, SoftwareDrawData, TextInputEvent, WindowId,
    },
    geometry::{LogicalRect, LogicalSize, PhysicalSize},
    window_api::{
        WindowParams,
        window_activate,
        window_close,
        window_create,
        window_request_redraw,
        //
    },
};
use log::{debug, error, info, warn};
use std::collections::hash_map::Entry;
use std::sync::{LazyLock, Mutex};
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    str::FromStr,
};
use url::Url;

fn between(val: f64, min: f64, max: f64) -> bool {
    val > min && val < max
}

const APP_ID: &CStr = c"org.jetbrains.desktop.linux.native.sample1";
const TEXT_MIME_TYPE: &CStr = c"text/plain;charset=utf-8";
const URI_LIST_MIME_TYPE: &CStr = c"text/uri-list";

const ALL_MIMES: &CStr = c"text/uri-list,text/plain;charset=utf-8";
const DRAG_ICON_WINDOW_ID: WindowId = WindowId(-1);

#[derive(Debug, Default, Clone)]
struct OptionalAppPtr(Option<AppPtr<'static>>);

impl OptionalAppPtr {
    fn get(&self) -> AppPtr<'static> {
        self.0.as_ref().unwrap().clone()
    }
}

unsafe impl Send for OptionalAppPtr {}
unsafe impl Sync for OptionalAppPtr {}

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
    active: bool,
    text_input_available: bool,
    composed_text: String,
    text: String,
    animation_progress: f32,
    drag_and_drop_target: bool,
    drag_and_drop_source: bool,
    opengl: Option<OpenglState>,
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

#[derive(Debug, Default)]
struct State {
    app_ptr: OptionalAppPtr,
    key_modifiers: KeyModifierBitflag,
    windows: HashMap<WindowId, WindowState>,
    settings: Settings,
    request_sources: HashMap<RequestId, WindowId>,
    notification_sources: HashMap<u32, WindowId>,
    // activation_token_action: HashMap<u32, ActivationTokenAction>,
    redraw_requester: Option<std::thread::JoinHandle<()>>,
}

impl State {
    fn with_mut_window_state(&mut self, window_id: WindowId, f: impl FnOnce(&mut WindowState)) -> bool {
        if let Some(window) = self.windows.get_mut(&window_id) {
            f(window);
            window_request_redraw(self.app_ptr.get(), window_id);
            true
        } else {
            false
        }
    }
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| Mutex::new(State::default()));

fn with_borrow_mut_state<T>(f: impl FnOnce(&mut State) -> T) -> T {
    let mut state = STATE.lock().unwrap();
    f(&mut *state)
}

const V_POSITION: GLuint = 0;
const DRAG_AND_DROP_LEFT_OF: f64 = 100.;

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
fn draw_opengl_triangle(gl: &OpenGlFuncs, program: GLuint, physical_size: PhysicalSize, scale: f64, animation_progress: f32) {
    //    debug!("draw_opengl_triangle, program = {program}, event = {data:?}");
    let rect_tip_x = (animation_progress + 1.0) * ((200. / f64::from(physical_size.width.0)) * scale) as f32;
    let rect_bottom_right_x = ((400. / f64::from(physical_size.width.0)) * scale) as f32;
    let rect_bottom_y = ((200. / f64::from(physical_size.height.0)) * scale) as f32;
    let v_vertices: [f32; 6] = [
        rect_tip_x - 1.0,           // rectangle tip x
        1.0,                        // rectangle tip y
        -1.0,                       // rectangle bottom-left x
        1.0 - rect_bottom_y,        // rectangle bottom-left y
        -1.0 + rect_bottom_right_x, // rectangle bottom-right x
        1.0 - rect_bottom_y,        // rectangle bottom-right y
    ];
    unsafe {
        (gl.glViewport)(0, 0, physical_size.width.0, physical_size.height.0);
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

fn draw_opengl_triangle_with_init(physical_size: PhysicalSize, scale: f64, window_id: WindowId, window_state: &mut WindowState) {
    let opengl_state = window_state.opengl.get_or_insert_with(|| {
        let egl_lib = application_get_egl_proc_func();
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
    let animation_progress = if window_state.animation_progress < 100. {
        -1.0 + (window_state.animation_progress / 50.)
    } else {
        1.0 - ((window_state.animation_progress - 100.) / 50.)
    };

    draw_opengl_triangle(&opengl_state.funcs, *program, physical_size, scale, animation_progress);
}

#[allow(clippy::many_single_char_names)]
fn draw_software(data: &SoftwareDrawData, physical_size: PhysicalSize, scale: f64, window_state: &WindowState) {
    const BYTES_PER_PIXEL: u8 = 4;
    let drag_source_indicator_heigh = 100. * scale;
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
        if between(
            x,
            DRAG_AND_DROP_LEFT_OF * scale,
            DRAG_AND_DROP_LEFT_OF.mul_add(scale, line_thickness),
        ) {
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 0;
        } else if between(x, line_thickness,  line_thickness * 2.0)  // left border
           || between(y, line_thickness,  line_thickness * 2.0)  // top border
           || between(x, line_thickness.mul_add(-2.0, w), w - line_thickness)  // right border
           || between(y, line_thickness.mul_add(-2.0, h), h - line_thickness)  // bottom border
           || between(x, (i / h) - (line_thickness / 2.0), (i / h) + (line_thickness / 2.0))
        {
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 255;
        } else if x < DRAG_AND_DROP_LEFT_OF
            && window_state.drag_and_drop_source
            && between(y, drag_source_indicator_heigh, drag_source_indicator_heigh + line_thickness)
        {
            pixel[0] = 255;
            pixel[1] = 0;
            pixel[2] = 0;
        } else if x < DRAG_AND_DROP_LEFT_OF && window_state.drag_and_drop_target {
            pixel[0] = 128;
            pixel[1] = 0;
            pixel[2] = 0;
        } else if window_state.active {
            pixel[0] = 255;
            pixel[1] = 255;
            pixel[2] = 255;
        } else {
            pixel[0] = 128;
            pixel[1] = 128;
            pixel[2] = 128;
        }
        pixel[3] = 255;
    }
}

#[allow(clippy::many_single_char_names)]
fn draw_software_drag_icon(data: &SoftwareDrawData, physical_size: PhysicalSize, scale: f64) {
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
            pixel[0] = 128;
            pixel[1] = 128;
        }
        pixel[2] = 128;
        pixel[3] = 128;
    }
}

const fn shortcut_modifiers(all_modifiers: KeyModifierBitflag) -> KeyModifierBitflag {
    KeyModifierBitflag(all_modifiers.0 & !(KeyModifier::CapsLock as u8) & !(KeyModifier::NumLock as u8))
}

#[allow(clippy::too_many_lines)]
fn on_keydown(event: &KeyDownEvent, app_ptr: AppPtr<'_>, state: &mut State) -> bool {
    // const KEYCODE_BACKSPACE: u32 = 14;
    // const KEYCODE_TAB: u32 = 15;
    // const KEYCODE_C: u32 = 46;
    // const KEYCODE_L: u32 = 38;
    const KEYCODE_N: u32 = 49;
    // const KEYCODE_O: u32 = 24;
    // const KEYCODE_P: u32 = 25;
    // const KEYCODE_S: u32 = 31;
    // const KEYCODE_U: u32 = 22;
    // const KEYCODE_V: u32 = 47;
    const KEY_MODIFIER_CTRL: u8 = KeyModifier::Ctrl as u8;

    let modifiers: KeyModifierBitflag = shortcut_modifiers(state.key_modifiers);
    let key_code: u32 = event.code.0;

    #[allow(clippy::single_match_else)]
    match (modifiers.0, key_code) {
        (KEY_MODIFIER_CTRL, KEYCODE_N) => {
            let new_window_id = WindowId(state.windows.len() as i64 + 1);
            window_create(
                state.app_ptr.get(),
                WindowParams {
                    window_id: new_window_id,
                    rect: LogicalRect {
                        x: LogicalPixels(0.),
                        y: LogicalPixels(0.),
                        width: LogicalPixels(300.),
                        height: LogicalPixels(200.),
                    },
                    min_size: LogicalSize {
                        width: LogicalPixels::default(),
                        height: LogicalPixels::default(),
                    },
                    title: BorrowedStrPtr::new(c"Window N"),
                    app_id: BorrowedStrPtr::new(APP_ID),
                    prefer_client_side_decoration: true,
                    rendering_mode: RenderingMode::Auto,
                },
            );
            state.windows.insert(new_window_id, WindowState::default());
            true
        }
        (_, _) => {
            if let Some(s) = event.characters.as_optional_str().unwrap() {
                state.with_mut_window_state(event.window_id, |window_state| {
                    window_state.text += s;
                });
            }
            false
        }
    }
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
    state.redraw_requester = Some(std::thread::spawn(move || {
        loop {
            with_borrow_mut_state(|state| {
                for window_id in state.windows.keys() {
                    window_request_redraw(state.app_ptr.get(), *window_id);
                }
            });
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }));
    window_create(
        state.app_ptr.get(),
        WindowParams {
            window_id: WindowId(1),
            rect: LogicalRect {
                x: LogicalPixels(0.),
                y: LogicalPixels(0.),
                width: LogicalPixels(200.),
                height: LogicalPixels(300.),
            },
            min_size: LogicalSize {
                width: LogicalPixels::default(),
                height: LogicalPixels::default(),
            },
            title: BorrowedStrPtr::new(c"Window 1"),
            app_id: BorrowedStrPtr::new(APP_ID),
            prefer_client_side_decoration: false,
            rendering_mode: RenderingMode::Software,
        },
    );

    window_create(
        state.app_ptr.get(),
        WindowParams {
            window_id: WindowId(2),
            rect: LogicalRect {
                x: LogicalPixels(0.),
                y: LogicalPixels(0.),
                width: LogicalPixels(300.),
                height: LogicalPixels(200.),
            },
            min_size: LogicalSize {
                width: LogicalPixels::default(),
                height: LogicalPixels::default(),
            },
            title: BorrowedStrPtr::new(c"Window 2"),
            app_id: BorrowedStrPtr::new(APP_ID),
            prefer_client_side_decoration: true,
            rendering_mode: RenderingMode::Auto,
        },
    );
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

    with_borrow_mut_state(|state| {
        let app_ptr = state.app_ptr.get();
        let is_event_loop_thread = application_is_event_loop_thread(app_ptr.clone());
        assert!(is_event_loop_thread);

        match event {
            Event::ApplicationStarted => {
                on_application_started(state);
                true
            }
            // Event::XdgDesktopSettingChange(data) => {
            //     on_xdg_desktop_settings_change(data, state);
            //     true
            // }
            Event::WindowConfigure(data) => {
                if let Entry::Vacant(e) = state.windows.entry(data.window_id) {
                    let mut window_state = WindowState::default();
                    window_state.active = data.active;
                    e.insert(window_state);
                }
                true
            }
            Event::WindowDraw(data) => {
                if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                    window_state.animation_tick();

                    if data.software_draw_data.canvas.is_null() {
                        draw_opengl_triangle_with_init(data.physical_size, data.scale, data.window_id, window_state);
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
                    draw_opengl_triangle_with_init(data.physical_size, data.scale, window_id, window_state);
                } else {
                    draw_software_drag_icon(&data.software_draw_data, data.physical_size, data.scale);
                }
                true
            }
            Event::WindowCloseRequest(data) => {
                window_close(app_ptr.clone(), data.window_id);
                state.windows.retain(|&k, _v| k != data.window_id);
                state.request_sources.retain(|_k, &mut v| v != data.window_id);
                for (notification_id, _window_id) in state.notification_sources.extract_if(|_k, &mut v| v != data.window_id) {
                    // application_close_notification(app_ptr.clone(), notification_id);
                }
                if state.windows.is_empty() {
                    application_stop_event_loop(app_ptr);
                }
                true
            }
            Event::MouseDown(data) => match data.button.0 {
                MOUSE_BUTTON_LEFT => {
                    // if data.location_in_window.x.0 < DRAG_AND_DROP_LEFT_OF {
                    //     let mime_types = if state.key_modifiers.0 == KeyModifier::Shift as u8 {
                    //         ALL_MIMES
                    //     } else {
                    //         TEXT_MIME_TYPE
                    //     };
                    //     let actions = DragAndDropActions(DragAndDropAction::Copy as u8 | DragAndDropAction::Move as u8);
                    //     let drag_icon_size = LogicalSize { width: 300, height: 300 };
                    //     window_start_drag_and_drop(
                    //         app_ptr,
                    //         data.window_id,
                    //         BorrowedStrPtr::new(mime_types),
                    //         actions,
                    //         RenderingMode::Auto,
                    //         drag_icon_size,
                    //     );
                    //     if let Some(window_state) = state.windows.get_mut(&data.window_id) {
                    //         window_state.drag_and_drop_source = true;
                    //     }
                    // }
                    true
                }
                MOUSE_BUTTON_MIDDLE => {
                    // application_primary_selection_paste(app_ptr, 1, BorrowedStrPtr::new(TEXT_MIME_TYPE));
                    true
                }
                _ => false,
            },
            Event::ModifiersChanged(data) => {
                state.key_modifiers = data.modifiers;
                true
            }
            Event::WindowKeyboardEnter(data) => {
                state.with_mut_window_state(data.window_id, |window_state| {
                    window_state.active = true;
                });
                true
            }
            Event::WindowKeyboardLeave(data) => {
                state.with_mut_window_state(data.window_id, |window_state| {
                    window_state.active = false;
                });

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
            // Event::DataTransfer(data) => {
            //     if let Some(key_window_id) = state.key_window_id
            //         && let Some(window_state) = state.windows.get_mut(&key_window_id)
            //     {
            //         on_data_transfer_received(&data.content, window_state);
            //         true
            //     } else {
            //         false
            //     }
            // }
            Event::DropPerformed(data) => state.with_mut_window_state(data.window_id, |window_state| {
                on_data_transfer_received(&data.content, window_state);
            }),
            Event::DragAndDropLeave(data) => state.with_mut_window_state(data.window_id, |window_state| {
                window_state.drag_and_drop_target = false;
            }),
            Event::DragAndDropFinished(data) => {
                state.windows.remove(&DRAG_ICON_WINDOW_ID);
                state.with_mut_window_state(data.window_id, |window_state| {
                    window_state.drag_and_drop_source = false;
                    info!("Finished initiated drag and drop with action {:?}", data.action);
                })
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
                state.with_mut_window_state(data.window_id, |window_state| {
                    // on_text_input_availability_changed(data.available, app_ptr, window_state);
                })
            }
            Event::TextInput(data) => {
                state.with_mut_window_state(data.window_id, |window_state| {
                    // on_text_input(event, app_ptr, key_window_id, window_state);
                })
            }
            Event::ActivationTokenResponse(data) => {
                let _token = BorrowedStrPtr::new(data.token.as_optional_cstr().unwrap());
                true
            }
            Event::NotificationShown(data) => {
                if data.notification_id > 0 {
                    if let Some(requester) = state.request_sources.remove(&data.request_id) {
                        state.notification_sources.insert(data.notification_id, requester);
                    } else {
                        // application_close_notification(app_ptr, data.notification_id);
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
//
// fn on_xdg_desktop_settings_change(s: &XdgDesktopSetting, state: &mut State) {
//     match s {
//         XdgDesktopSetting::CursorSize(v) => {
//             let size = (*v).try_into().unwrap();
//             if let Some(name) = &state.settings.cursor_theme_name {
//                 application_set_cursor_theme(state.app_ptr.get(), BorrowedStrPtr::new(name), size);
//             }
//             state.settings.cursor_theme_size = Some(size);
//         }
//         XdgDesktopSetting::CursorTheme(v) => {
//             let name = CString::new(v.as_str().unwrap()).unwrap();
//             if let Some(size) = state.settings.cursor_theme_size {
//                 application_set_cursor_theme(state.app_ptr.get(), BorrowedStrPtr::new(&name), size);
//             }
//             state.settings.cursor_theme_name = Some(name);
//         }
//         _ => {}
//     }
// }

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
    } else {
        mime_type_cstr.to_str().unwrap()
    };

    let mut a = BorrowedArray::from_slice(leaked_string_data(v));
    a.deinit = Some(deinit_u8_vec);
    a
}

pub fn main() {
    logger_init_impl(&LoggerConfiguration {
        file_path: BorrowedStrPtr::new(c"/tmp/a"),
        console_level: LogLevel::Debug,
        file_level: LogLevel::Error,
    });
    let app_ptr = application_init(ApplicationCallbacks {
        event_handler,
        query_drag_and_drop_target,
        get_data_transfer_data,
    });
    with_borrow_mut_state(|state| {
        state.app_ptr = OptionalAppPtr(Some(app_ptr.clone()));
    });
    application_run_event_loop(app_ptr.clone());
    application_shutdown(app_ptr);
}
