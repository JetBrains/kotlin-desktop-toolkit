use core::str;
use std::{
    cell::RefCell,
    collections::{HashSet, VecDeque},
    ffi::{CStr, OsString},
    io::{Read, Write},
    os::unix::net::UnixStream,
    panic::Location,
    sync::{Arc, Barrier},
    time::Duration,
};

use crate::linux::{
    application_api::{
        AppPtr, ApplicationCallbacks, DataSource, DragAndDropQueryData, application_get_key_mapping, application_init,
        application_is_event_loop_thread, application_run_event_loop, application_shutdown, application_stop_event_loop,
        application_text_input_disable, application_text_input_enable,
    },
    events::{
        Event, KeyDownEvent, KeyModifier, KeyModifierBitflag, KeyUpEvent, ModifiersChangedEvent, ScreenId, TextInputAvailabilityEvent,
        WindowCapabilities, WindowCloseRequestEvent, WindowConfigureEvent, WindowDecorationMode, WindowDrawEvent, WindowId,
        WindowKeyboardEnterEvent, WindowScaleChangedEvent, WindowScreenChangeEvent,
    },
    geometry::{LogicalPixels, LogicalPoint, LogicalRect, LogicalSize},
    text_input_api::{TextInputContentPurpose, TextInputContext},
    virtual_keys::{KeyCodes, MappingResult, VirtualKey},
    window_api::{WindowParams, window_close, window_create, window_set_fullscreen},
};

use desktop_common::{
    ffi_utils::{BorrowedArray, BorrowedStrPtr},
    logger_api::{LogLevel, LoggerConfiguration, logger_init_impl},
};
use desktop_linux_test_helper::{RawKeyCommandData, TestHelper, TestHelperCommand};
use log::{debug, error};

trait SystemInteraction {
    fn close_windows(&mut self, app_id: &str);
    fn client_side_decorations_for_active_window(&mut self);
    fn server_side_decorations_for_active_window(&mut self);
    fn change_keyboard_layout(&mut self, new_layout: &str);
    fn raw_key_press(&self, keycode: KeyCodes);
    fn raw_key_release(&self, keycode: KeyCodes);
}

struct SwaySystemInteraction {
    _sway_config: tempfile::NamedTempFile,
    sway_child: std::process::Child,
    connection: swayipc::Connection,
    command_sender: Box<dyn Fn(TestHelperCommand)>,
    test_helper_handle: Option<std::thread::JoinHandle<()>>,
}

impl SwaySystemInteraction {
    fn read_file(file_path: &str) -> std::io::Result<String> {
        let mut buf = String::new();
        let mut i = 0;
        loop {
            match std::fs::File::open(file_path) {
                Ok(mut file) => {
                    file.read_to_string(&mut buf)?;
                    return Ok(buf);
                }
                Err(e) => {
                    if i == 1000 {
                        return Err(e);
                    }
                    i += 1;
                    debug!("{e}");
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }

    fn get_random_tmpfile_path() -> std::path::PathBuf {
        tempfile::NamedTempFile::new().unwrap().path().to_owned()
    }

    fn new() -> Result<Self, TestError> {
        debug!("SwaySystemInteraction::new");
        let sway_wayland_display_name = Self::get_random_tmpfile_path().to_str().unwrap().to_owned();
        let sway_socket = Self::get_random_tmpfile_path();

        let mut sway_config = tempfile::NamedTempFile::new()?;
        sway_config.write_all(
            format!(
                "
output HEADLESS-1 {{
    pos 0,0
    mode 3000x1500@75Hz
    scale 1.5
}}
exec echo -n \"$WAYLAND_DISPLAY\" > {sway_wayland_display_name}.tmp && mv {sway_wayland_display_name}.tmp {sway_wayland_display_name}
"
            )
            .as_bytes(),
        )?;

        let sway_child = std::process::Command::new("sway")
            .env("WLR_BACKENDS", "headless")
            .env("SWAYSOCK", &sway_socket)
            .arg("--config")
            .arg(sway_config.path())
            .spawn()?;

        let wayland_display: OsString = Self::read_file(&sway_wayland_display_name)?.into();
        debug!("WAYLAND_DISPLAY={}", wayland_display.to_str().unwrap());
        unsafe {
            std::env::set_var("WAYLAND_DISPLAY", &wayland_display);
        };
        let socket = UnixStream::connect(&sway_socket)?;
        let mut connection: swayipc::Connection = socket.try_clone().unwrap().into();
        let outputs = connection.get_outputs()?;
        debug!("outputs = {outputs:?}");
        let mut test_helper = TestHelper::new();
        let command_sender = test_helper.get_sender();

        let barrier = Arc::new(Barrier::new(2));
        let barrier2 = barrier.clone();
        let test_helper_handle = std::thread::spawn(move || {
            test_helper
                .run(Box::new(move || {
                    debug!("Test helper run callback called");
                    barrier2.wait();
                }))
                .unwrap();
        });
        barrier.wait();

        Ok(Self {
            _sway_config: sway_config,
            sway_child,
            connection,
            command_sender,
            test_helper_handle: Some(test_helper_handle),
        })
    }

    fn do_command(&mut self, payload: &str) {
        for outcome in self.connection.run_command(payload).unwrap() {
            if let Err(error) = outcome {
                error!("failure '{error}'");
            }
        }
    }

    fn raw_key(&self, keycode: KeyCodes, down: bool) {
        let barrier = Arc::new(Barrier::new(2));
        let barrier2 = barrier.clone();
        (*self.command_sender)(TestHelperCommand::RawKey(
            RawKeyCommandData {
                keycode: keycode.to_keycode().0,
                down,
            },
            Box::new(move |_success| {
                barrier2.wait();
            }),
        ));
        barrier.wait();
    }
}

impl Drop for SwaySystemInteraction {
    fn drop(&mut self) {
        (*self.command_sender)(TestHelperCommand::Exit(Box::new(|_| {})));
        self.test_helper_handle.take().unwrap().join().unwrap();
        self.sway_child.kill().unwrap();
    }
}

impl SystemInteraction for SwaySystemInteraction {
    fn close_windows(&mut self, app_id: &str) {
        debug!("SwaySystemInteraction::close_windows");
        let payload = format!("[app_id=\"{app_id}\"] kill");
        self.do_command(&payload);
    }

    fn client_side_decorations_for_active_window(&mut self) {
        let payload = "floating enable";
        self.do_command(payload);
    }

    fn server_side_decorations_for_active_window(&mut self) {
        let payload = "floating disable";
        self.do_command(payload);
    }

    fn change_keyboard_layout(&mut self, new_layout: &str) {
        let barrier = Arc::new(Barrier::new(2));
        let barrier2 = barrier.clone();
        (*self.command_sender)(TestHelperCommand::SetKeyboardLayout(
            new_layout.to_owned(),
            Box::new(move |_success| {
                barrier2.wait();
            }),
        ));
        barrier.wait();
        // let payload = format!("input type:keyboard xkb_layout {new_layout}");
        // self.do_command(&payload);
    }

    fn raw_key_press(&self, keycode: KeyCodes) {
        self.raw_key(keycode, true);
    }

    fn raw_key_release(&self, keycode: KeyCodes) {
        self.raw_key(keycode, false);
    }
}

const APP_ID: &CStr = c"org.jetbrains.desktop.linux.native.smoke_test_1";
const INITIAL_WINDOW_1_SIZE: LogicalSize = LogicalSize {
    width: LogicalPixels(200.),
    height: LogicalPixels(300.),
};
const SWAY_WINDOW_CAPABILITIES: WindowCapabilities = WindowCapabilities {
    window_menu: true,
    maximixe: true,    // sway bug
    fullscreen: false, // sway bug
    minimize: false,
};
const WINDOW_1_ID: WindowId = WindowId(1);

struct State {
    app_ptr: Option<AppPtr<'static>>,
    data_transfer_data: &'static str,
    system_integration: Result<SwaySystemInteraction, TestError>,
    open_windows: HashSet<WindowId>,
    test_cases: VecDeque<TestCase>,
    test_error: Option<TestError>,
}

#[derive(Clone)]
pub struct TestError(Vec<String>, &'static Location<'static>);

impl TestError {
    #[track_caller]
    fn unexpected<T: Into<String>>(msg: T) -> Self {
        Self(vec![msg.into()], Location::caller())
    }

    fn new(errors: Vec<String>, location: &'static Location<'static>) -> Result<(), Self> {
        if errors.is_empty() { Ok(()) } else { Err(Self(errors, location)) }
    }
}

impl std::fmt::Debug for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.1, f)?;
        for msg in &self.0 {
            std::fmt::Write::write_str(f, "\n    ")?;
            f.write_str(msg)?;
        }
        Ok(())
    }
}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.1, f)?;
        for msg in &self.0 {
            std::fmt::Write::write_str(f, "\n    ")?;
            f.write_str(msg)?;
        }
        Ok(())
    }
}

impl<E> From<E> for TestError
where
    E: core::error::Error + Send + Sync + 'static,
{
    #[track_caller]
    fn from(error: E) -> Self {
        Self(vec![error.to_string()], Location::caller())
    }
}

fn assert_eq<T>(a: T, b: T) -> Result<(), String>
where
    T: std::cmp::PartialEq + std::fmt::Debug + Copy,
{
    if a == b { Ok(()) } else { Err(format!("{a:?} != {b:?}")) }
}

impl State {
    fn get_app_ptr(&self) -> AppPtr<'static> {
        self.app_ptr.as_ref().unwrap().clone()
    }

    fn stop_app(&self) {
        application_stop_event_loop(self.get_app_ptr());
    }
}

trait Verify<T> {
    fn verify(&self, expected: T) -> Vec<String>;
}

impl Verify<KeyDownEvent<'_>> for KeyDownEvent<'_> {
    fn verify(&self, expected: KeyDownEvent<'_>) -> Vec<String> {
        let mut errors = Vec::new();

        if let Err(e) = assert_eq(self.code, expected.code) {
            errors.push(format!("KeyDown::code mismatch: {e}"));
        }

        if let Err(e) = assert_eq(self.key, expected.key) {
            errors.push(format!("KeyDown::key mismatch: {e}"));
        }

        if let Err(e) = assert_eq(self.is_repeat, expected.is_repeat) {
            errors.push(format!("KeyDown::is_repeat mismatch: {e}"));
        }

        if let Err(e) = assert_eq(self.vk, expected.vk) {
            errors.push(format!("KeyDown::vk mismatch: {e}"));
        }

        match self.characters.as_optional_str() {
            Ok(actual_chars) => match expected.characters.as_optional_str() {
                Ok(expected_chars) => {
                    if let Err(e) = assert_eq(actual_chars, expected_chars) {
                        errors.push(format!("KeyDown::characters mismatch: {e}"));
                    }
                }
                Err(e) => errors.push(format!("Invalid unicode expected.characters: {e}")),
            },
            Err(e) => errors.push(format!("Invalid unicode self.characters: {e}")),
        }

        errors
    }
}

impl Verify<Self> for KeyUpEvent {
    fn verify(&self, expected: Self) -> Vec<String> {
        let mut errors = Vec::new();

        if let Err(e) = assert_eq(self.code, expected.code) {
            errors.push(format!("KeyUp::code mismatch: {e}"));
        }

        if let Err(e) = assert_eq(self.key, expected.key) {
            errors.push(format!("KeyUp::key mismatch: {e}"));
        }

        errors
    }
}

impl Verify<Self> for WindowConfigureEvent {
    fn verify(&self, expected: Self) -> Vec<String> {
        let mut errors = Vec::new();

        if let Err(e) = assert_eq(self.window_id, expected.window_id) {
            errors.push(format!("WindowConfigure::window_id mismatch: {e}"));
        }
        if let Err(e) = assert_eq(self.active, expected.active) {
            errors.push(format!("WindowConfigure::active mismatch: {e}"));
        }
        if let Err(e) = assert_eq(self.maximized, expected.maximized) {
            errors.push(format!("WindowConfigure::maximized mismatch: {e}"));
        }
        if let Err(e) = assert_eq(self.fullscreen, expected.fullscreen) {
            errors.push(format!("WindowConfigure::fullscreen mismatch: {e}"));
        }
        if let Err(e) = assert_eq(self.decoration_mode, expected.decoration_mode) {
            errors.push(format!("WindowConfigure::decoration_mode mismatch: {e}"));
        }
        if let Err(e) = assert_eq(self.capabilities.window_menu, expected.capabilities.window_menu) {
            errors.push(format!("WindowConfigure::capabilities.window_menu mismatch: {e}"));
        }
        if let Err(e) = assert_eq(self.capabilities.maximixe, expected.capabilities.maximixe) {
            errors.push(format!("WindowConfigure::capabilities.maximixe mismatch: {e}"));
        }
        if let Err(e) = assert_eq(self.capabilities.fullscreen, expected.capabilities.fullscreen) {
            errors.push(format!("WindowConfigure::capabilities.fullscreen mismatch: {e}"));
        }
        if let Err(e) = assert_eq(self.capabilities.minimize, expected.capabilities.minimize) {
            errors.push(format!("WindowConfigure::capabilities.minimize mismatch: {e}"));
        }
        if let Err(e) = assert_eq(self.size.height.0, expected.size.height.0) {
            errors.push(format!("WindowConfigure::size.height mismatch: {e}"));
        }
        if let Err(e) = assert_eq(self.size.width.0, expected.size.width.0) {
            errors.push(format!("WindowConfigure::size.width mismatch: {e}"));
        }

        errors
    }
}

impl Verify<Self> for WindowScaleChangedEvent {
    fn verify(&self, expected: Self) -> Vec<String> {
        let mut errors = Vec::new();

        if let Err(e) = assert_eq(self.window_id, expected.window_id) {
            errors.push(format!("WindowConfigure::window_id mismatch: {e}"));
        }
        if let Err(e) = assert_eq(self.new_scale, expected.new_scale) {
            errors.push(format!("WindowScaleChanged::new_scale mismatch: {e}"));
        }

        errors
    }
}

impl Verify<Self> for ModifiersChangedEvent {
    fn verify(&self, expected: Self) -> Vec<String> {
        let mut errors = Vec::new();

        if let Err(e) = assert_eq(self.modifiers, expected.modifiers) {
            errors.push(format!("ModifiersChanged::modifiers mismatch: {e}"));
        }

        errors
    }
}

impl Verify<Self> for TextInputAvailabilityEvent {
    fn verify(&self, expected: Self) -> Vec<String> {
        let mut errors = Vec::new();

        if let Err(e) = assert_eq(self.window_id, expected.window_id) {
            errors.push(format!("WindowConfigure::window_id mismatch: {e}"));
        }
        if let Err(e) = assert_eq(self.available, expected.available) {
            errors.push(format!("TextInputAvailability::available mismatch: {e}"));
        }

        errors
    }
}

impl Verify<Event<'_>> for Event<'_> {
    fn verify(&self, expected: Event<'_>) -> Vec<String> {
        match expected {
            Event::KeyDown(expected_event) => {
                if let Event::KeyDown(actual_event) = self {
                    actual_event.verify(expected_event)
                } else {
                    vec![format!("Expected {expected_event:?}, received: {self:?}")]
                }
            }
            Event::KeyUp(expected_event) => {
                if let Event::KeyUp(actual_event) = self {
                    actual_event.verify(expected_event)
                } else {
                    vec![format!("Expected {expected_event:?}, received: {self:?}")]
                }
            }
            Event::WindowConfigure(expected_event) => {
                if let Event::WindowConfigure(actual_event) = self {
                    actual_event.verify(expected_event)
                } else {
                    vec![format!("Expected {expected_event:?}, received: {self:?}")]
                }
            }
            Event::WindowScaleChanged(expected_event) => {
                if let Event::WindowScaleChanged(actual_event) = self {
                    actual_event.verify(expected_event)
                } else {
                    vec![format!("Expected {expected_event:?}, received: {self:?}")]
                }
            }
            Event::WindowKeyboardEnter(expected_event) => {
                if let Event::WindowKeyboardEnter(_actual_event) = self {
                    Vec::new() // TODO?
                } else {
                    vec![format!("Expected {expected_event:?}, received: {self:?}")]
                }
            }
            Event::ModifiersChanged(expected_event) => {
                if let Event::ModifiersChanged(actual_event) = self {
                    actual_event.verify(expected_event)
                } else {
                    vec![format!("Expected {expected_event:?}, received: {self:?}")]
                }
            }
            Event::TextInputAvailability(expected_event) => {
                if let Event::TextInputAvailability(actual_event) = self {
                    actual_event.verify(expected_event)
                } else {
                    vec![format!("Expected {expected_event:?}, received: {self:?}")]
                }
            }
            Event::WindowScreenChange(expected_event) => {
                if let Event::WindowScreenChange(actual_event) = self {
                    if let Err(e) = assert_eq(actual_event.window_id, expected_event.window_id) {
                        // TODO: rest?
                        vec![format!("WindowCloseRequest::window_id mismatch: {e}")]
                    } else {
                        Vec::new()
                    }
                } else {
                    vec![format!("Expected {expected_event:?}, received: {self:?}")]
                }
            }
            Event::WindowCloseRequest(expected_event) => {
                if let Event::WindowCloseRequest(actual_event) = self {
                    if let Err(e) = assert_eq(actual_event.window_id, expected_event.window_id) {
                        vec![format!("WindowCloseRequest::window_id mismatch: {e}")]
                    } else {
                        Vec::new()
                    }
                } else {
                    vec![format!("Expected {expected_event:?}, received: {self:?}")]
                }
            }
            _ => vec![format!("Unexpected event type: {self:?}")],
        }
    }
}

impl Verify<Self> for MappingResult {
    fn verify(&self, expected: Self) -> Vec<String> {
        let mut errors = Vec::new();
        if let Err(e) = assert_eq(self.vk_to_listen, expected.vk_to_listen) {
            errors.push(format!("MappingResult::vk_to_listen mismatch: {e}"));
        }
        if let Err(_e) = assert_eq(self.char_to_display, expected.char_to_display) {
            errors.push(format!(
                "MappingResult::char_to_display: {:?} != {:?}",
                char::from_u32(self.char_to_display),
                char::from_u32(expected.char_to_display)
            ));
        }
        if let Err(e) = assert_eq(self.modifiers_to_listen, expected.modifiers_to_listen) {
            errors.push(format!("MappingResult::modifiers_to_listen mismatch: {e}"));
        }

        errors
    }
}

#[track_caller]
fn check_mapping_result(actual: MappingResult, expected: MappingResult) -> Result<(), TestError> {
    TestError::new(actual.verify(expected), Location::caller())
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
            app_ptr: None,
            data_transfer_data: "some transfer data",
            system_integration: SwaySystemInteraction::new(),
            open_windows: HashSet::new(),
            test_cases: VecDeque::new(),
            test_error: None,
        });
}

fn draw(event: &WindowDrawEvent) {
    let stride = event.software_draw_data.stride;
    let canvas = {
        let canvas_ptr = event.software_draw_data.canvas;
        assert!(!canvas_ptr.is_null());
        let len = usize::try_from(event.physical_size.height.0 * stride).unwrap();
        unsafe { std::slice::from_raw_parts_mut(canvas_ptr, len) }
    };
    canvas.fill(255);
}

fn create_text_input_context<'a>(text: &str, text_cstring: &'a CStr, change_caused_by_input_method: bool) -> TextInputContext<'a> {
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

#[allow(clippy::type_complexity)]
#[derive(Default)]
struct TestCase {
    perform: Option<Box<dyn FnOnce(&mut dyn SystemInteraction, AppPtr<'static>)>>,
    expects: VecDeque<(
        Event<'static>,
        Option<Box<dyn Fn(&State) -> Result<(), TestError>>>,
        &'static Location<'static>,
    )>,
}

impl TestCase {
    fn perform<F: Fn(&mut dyn SystemInteraction, AppPtr<'static>) + 'static>(mut self, f: F) -> Self {
        self.perform = Some(Box::new(f));
        self
    }

    #[track_caller]
    fn expect<E: Into<Event<'static>>>(mut self, e: E) -> Self {
        self.expects.push_back((e.into(), None, Location::caller()));
        self
    }

    #[track_caller]
    fn expect_event_with_check<E: Into<Event<'static>>, F: Fn(&State) -> Result<(), TestError> + 'static>(mut self, e: E, f: F) -> Self {
        self.expects.push_back((e.into(), Some(Box::new(f)), Location::caller()));
        self
    }
}

#[allow(clippy::too_many_lines)]
fn event_handler_impl(event: &Event) -> Result<bool, TestError> {
    match event {
        Event::WindowDraw(_) | Event::MouseMoved(_) => {}
        _ => {
            debug!("event_handler: {event:?}");
        }
    }
    STATE.with_borrow_mut(|state| {
        let is_event_loop_thread = application_is_event_loop_thread(state.get_app_ptr());
        if let Err(e) = assert_eq(is_event_loop_thread, true) {
            return Err(TestError::unexpected(e));
        }
        if let Event::WindowDraw(data) = event {
            draw(data);
            return Ok(true);
        }

        let expect = state.test_cases.front_mut().and_then(|test_case| test_case.expects.pop_front());

        let start_new_test_case = if let Some((expect, f, location)) = expect {
            TestError::new(event.verify(expect), location)?;
            if let Some(f) = f {
                f(state)?;
            }
            if state.test_cases.front().is_some_and(|test_case| test_case.expects.is_empty()) {
                state.test_cases.pop_front();
                true
            } else {
                false
            }
        } else {
            return Err(TestError::unexpected(format!("Unexpected event: {event:?}")));
        };

        let handled = match event {
            Event::WindowConfigure(data) => {
                state.open_windows.insert(data.window_id);
                true
            }
            Event::WindowDraw(data) => {
                draw(data);
                true
            }
            Event::WindowCloseRequest(data) => {
                window_close(state.get_app_ptr(), data.window_id);
                if !state.open_windows.remove(&data.window_id) {
                    return Err(TestError::unexpected(format!(
                        "Tried to close non-existing window {:?}",
                        data.window_id
                    )));
                }
                if state.open_windows.is_empty() {
                    application_stop_event_loop(state.get_app_ptr());
                }
                true
            }
            Event::TextInputAvailability(data) => {
                let app_ptr = state.get_app_ptr();
                if data.available {
                    application_text_input_enable(app_ptr, create_text_input_context("", c"", false));
                } else {
                    application_text_input_disable(app_ptr);
                }
                true
            }
            _ => false,
        };

        if start_new_test_case && let Some(perform) = state.test_cases.front_mut().and_then(|test_case| test_case.perform.take()) {
            let app_ptr = state.get_app_ptr();
            if let Ok(system_integration) = &mut state.system_integration {
                perform(system_integration, app_ptr);
            } else {
                return Err(TestError::unexpected("System integration failed"));
            }
        }

        Ok(handled)
    })
}

extern "C" fn event_handler(event: &Event) -> bool {
    if let Err(e) = event_handler_impl(event) {
        STATE.with_borrow_mut(|state| {
            if state.test_error.is_none() {
                state.test_error = Some(e);
                state.stop_app();
            }
        });
        false
    } else {
        true
    }
}

extern "C" fn get_drag_and_drop_supported_mime_types(_data: &DragAndDropQueryData) -> BorrowedStrPtr<'static> {
    BorrowedStrPtr::new(c"")
}

extern "C" fn get_data_transfer_data(_source: DataSource, _mime_type: BorrowedStrPtr) -> BorrowedArray<'static, u8> {
    STATE.with_borrow(|state| BorrowedArray::from_slice(state.data_transfer_data.as_bytes()))
}

#[allow(clippy::too_many_lines)]
#[test]
pub fn test1() -> Result<(), TestError> {
    logger_init_impl(&LoggerConfiguration {
        file_path: BorrowedStrPtr::null(),
        console_level: LogLevel::Debug,
        file_level: LogLevel::Error,
    });

    if let Some(e) = STATE.with_borrow_mut(|state| {
        if state.system_integration.is_err() {
            let mut res = Err(TestError::unexpected(""));
            std::mem::swap(&mut state.system_integration, &mut res);
            let e = res.err().unwrap();
            Some(e)
        } else {
            None
        }
    }) {
        return Err(e);
    }

    let app_ptr = STATE.with_borrow_mut(|state| {
        let app_ptr = application_init(ApplicationCallbacks {
            event_handler,
            get_drag_and_drop_supported_mime_types,
            get_data_transfer_data,
        });
        state.app_ptr = Some(app_ptr.clone());
        state.test_cases = VecDeque::from([
            TestCase::default().expect(Event::ApplicationStarted),
            TestCase::default()
                .perform(|_sys, app_ptr| {
                    window_create(
                        app_ptr,
                        WindowParams {
                            window_id: WINDOW_1_ID,
                            size: INITIAL_WINDOW_1_SIZE,
                            title: BorrowedStrPtr::new(c"Window 1"),
                            app_id: BorrowedStrPtr::new(APP_ID),
                            prefer_client_side_decoration: true,
                            force_software_rendering: true,
                        },
                    );
                })
                .expect(WindowConfigureEvent {
                    window_id: WINDOW_1_ID,
                    size: INITIAL_WINDOW_1_SIZE,
                    active: false,
                    maximized: false,
                    fullscreen: false,
                    decoration_mode: WindowDecorationMode::Server,
                    capabilities: SWAY_WINDOW_CAPABILITIES,
                })
                .expect(WindowScaleChangedEvent {
                    window_id: WINDOW_1_ID,
                    new_scale: 1.5,
                })
                .expect(WindowKeyboardEnterEvent {
                    window_id: WINDOW_1_ID,
                    raw: BorrowedArray::from_slice(&[]),
                    keysyms: BorrowedArray::from_slice(&[]),
                })
                .expect(ModifiersChangedEvent {
                    modifiers: KeyModifierBitflag::EMPTY,
                })
                .expect(TextInputAvailabilityEvent {
                    window_id: WINDOW_1_ID,
                    available: true,
                })
                .expect(WindowConfigureEvent {
                    window_id: WINDOW_1_ID,
                    // some space for the server-side decorations
                    size: LogicalSize {
                        width: LogicalPixels(1996.),
                        height: LogicalPixels(973.),
                    },
                    active: true,
                    maximized: false,
                    fullscreen: false,
                    decoration_mode: WindowDecorationMode::Server,
                    capabilities: SWAY_WINDOW_CAPABILITIES,
                })
                .expect(WindowScreenChangeEvent {
                    window_id: WINDOW_1_ID,
                    new_screen_id: ScreenId(4),
                }),
            TestCase::default()
                .perform(|sys, _app_ptr| {
                    sys.change_keyboard_layout("fr");
                    sys.raw_key_press(KeyCodes::W);
                    sys.raw_key_release(KeyCodes::W);
                })
                .expect_event_with_check(
                    KeyDownEvent {
                        code: KeyCodes::W.to_keycode(),
                        characters: BorrowedStrPtr::new(c"z"),
                        key: xkeysym::key::z,
                        is_repeat: false,
                        vk: VirtualKey::Z,
                    },
                    move |state| {
                        let mapping = application_get_key_mapping(state.get_app_ptr(), KeyModifier::Ctrl.into(), 'w'.into());
                        check_mapping_result(
                            mapping,
                            MappingResult {
                                modifiers_to_listen: KeyModifier::Ctrl.into(),
                                vk_to_listen: VirtualKey::W,
                                char_to_display: 'w'.into(),
                            },
                        )
                    },
                )
                .expect(KeyUpEvent {
                    code: KeyCodes::W.to_keycode(),
                    key: xkeysym::key::z,
                }),
            TestCase::default()
                .perform(|sys, _app_ptr| {
                    sys.change_keyboard_layout("fr");
                    sys.raw_key_press(KeyCodes::LEFTSHIFT);
                    sys.raw_key_press(KeyCodes::W);
                    sys.raw_key_release(KeyCodes::W);
                    sys.raw_key_release(KeyCodes::LEFTSHIFT);
                })
                .expect(ModifiersChangedEvent {
                    modifiers: KeyModifier::Shift.into(),
                })
                .expect(KeyDownEvent {
                    code: KeyCodes::W.to_keycode(),
                    characters: BorrowedStrPtr::new(c"Z"),
                    key: xkeysym::key::Z,
                    is_repeat: false,
                    vk: VirtualKey::Z,
                })
                .expect(KeyUpEvent {
                    code: KeyCodes::W.to_keycode(),
                    key: xkeysym::key::Z,
                })
                .expect(ModifiersChangedEvent {
                    modifiers: KeyModifierBitflag::EMPTY,
                }),
            TestCase::default()
                .perform(|sys, _app_ptr| {
                    sys.change_keyboard_layout("fr");
                    sys.raw_key_press(KeyCodes::CAPSLOCK);
                    sys.raw_key_release(KeyCodes::CAPSLOCK);
                    sys.raw_key_press(KeyCodes::W);
                    sys.raw_key_release(KeyCodes::W);
                    sys.raw_key_press(KeyCodes::CAPSLOCK);
                    sys.raw_key_release(KeyCodes::CAPSLOCK);
                })
                .expect(ModifiersChangedEvent {
                    modifiers: KeyModifier::CapsLock.into(),
                })
                .expect(ModifiersChangedEvent {
                    modifiers: KeyModifier::CapsLock.into(),
                })
                .expect(KeyDownEvent {
                    code: KeyCodes::W.to_keycode(),
                    characters: BorrowedStrPtr::new(c"Z"),
                    key: xkeysym::key::Z,
                    is_repeat: false,
                    vk: VirtualKey::Z,
                })
                .expect(KeyUpEvent {
                    code: KeyCodes::W.to_keycode(),
                    key: xkeysym::key::Z,
                })
                .expect(ModifiersChangedEvent {
                    modifiers: KeyModifier::CapsLock.into(),
                })
                .expect(ModifiersChangedEvent {
                    modifiers: KeyModifierBitflag::EMPTY,
                }),
            TestCase::default()
                .perform(|sys, _app_ptr| {
                    sys.change_keyboard_layout("fr");
                    sys.raw_key_press(KeyCodes::LEFTCTRL);
                    sys.raw_key_press(KeyCodes::W);
                    sys.raw_key_release(KeyCodes::W);
                    sys.raw_key_release(KeyCodes::LEFTCTRL);
                })
                .expect(ModifiersChangedEvent {
                    modifiers: KeyModifier::Ctrl.into(),
                })
                .expect(KeyDownEvent {
                    code: KeyCodes::W.to_keycode(),
                    characters: BorrowedStrPtr::new(c"\u{1a}"),
                    key: xkeysym::key::z,
                    is_repeat: false,
                    vk: VirtualKey::Z,
                })
                .expect(KeyUpEvent {
                    code: KeyCodes::W.to_keycode(),
                    key: xkeysym::key::z,
                })
                .expect(ModifiersChangedEvent {
                    modifiers: KeyModifierBitflag::EMPTY,
                }),
            TestCase::default()
                .perform(|sys, _app_ptr| {
                    sys.change_keyboard_layout("az");
                    sys.raw_key_press(KeyCodes::W);
                    sys.raw_key_release(KeyCodes::W);
                })
                .expect_event_with_check(
                    KeyDownEvent {
                        code: KeyCodes::W.to_keycode(),
                        characters: BorrowedStrPtr::new(c"ü"),
                        key: xkeysym::key::udiaeresis,
                        is_repeat: false,
                        vk: VirtualKey::W,
                    },
                    move |state| {
                        let mapping = application_get_key_mapping(state.get_app_ptr(), KeyModifier::Ctrl.into(), 'w'.into());
                        check_mapping_result(
                            mapping,
                            MappingResult {
                                modifiers_to_listen: KeyModifier::Ctrl.into(),
                                vk_to_listen: VirtualKey::W,
                                char_to_display: 'ü'.into(),
                            },
                        )
                    },
                )
                .expect(KeyUpEvent {
                    code: KeyCodes::W.to_keycode(),
                    key: xkeysym::key::udiaeresis,
                }),
            TestCase::default()
                .perform(|sys, _app_ptr| {
                    sys.change_keyboard_layout("lv");
                    sys.raw_key_press(KeyCodes::W);
                    sys.raw_key_release(KeyCodes::W);
                })
                .expect_event_with_check(
                    ModifiersChangedEvent {
                        modifiers: KeyModifierBitflag::EMPTY,
                    },
                    move |state| {
                        let mapping = application_get_key_mapping(state.get_app_ptr(), KeyModifier::Ctrl.into(), 'w'.into());
                        check_mapping_result(
                            mapping,
                            MappingResult {
                                modifiers_to_listen: (KeyModifier::Ctrl | KeyModifier::Alt).into(),
                                vk_to_listen: VirtualKey::V,
                                char_to_display: 'v'.into(),
                            },
                        )
                    },
                )
                .expect(KeyDownEvent {
                    code: KeyCodes::W.to_keycode(),
                    characters: BorrowedStrPtr::new(c"g"),
                    key: xkeysym::key::g,
                    is_repeat: false,
                    vk: VirtualKey::G,
                })
                .expect(KeyUpEvent {
                    code: KeyCodes::W.to_keycode(),
                    key: xkeysym::key::g,
                }),
            TestCase::default()
                .perform(|sys, _app_ptr| {
                    sys.change_keyboard_layout("lv");
                    sys.raw_key_press(KeyCodes::G);
                    sys.raw_key_release(KeyCodes::G);
                })
                .expect_event_with_check(
                    ModifiersChangedEvent {
                        modifiers: KeyModifierBitflag::EMPTY,
                    },
                    move |state| {
                        let mapping = application_get_key_mapping(state.get_app_ptr(), KeyModifier::Ctrl.into(), 'g'.into());
                        check_mapping_result(
                            mapping,
                            MappingResult {
                                modifiers_to_listen: KeyModifier::Ctrl.into(),
                                vk_to_listen: VirtualKey::G, // ???
                                char_to_display: 'g'.into(), // ???
                            },
                        )
                    },
                )
                .expect(KeyDownEvent {
                    code: KeyCodes::G.to_keycode(),
                    characters: BorrowedStrPtr::new(c"l"),
                    key: xkeysym::key::l,
                    is_repeat: false,
                    vk: VirtualKey::L,
                })
                .expect(KeyUpEvent {
                    code: KeyCodes::G.to_keycode(),
                    key: xkeysym::key::l,
                }),
            TestCase::default()
                .perform(|sys, _app_ptr| {
                    sys.change_keyboard_layout("en");
                    sys.raw_key_press(KeyCodes::W);
                    sys.raw_key_release(KeyCodes::W);
                })
                .expect_event_with_check(
                    KeyDownEvent {
                        code: KeyCodes::W.to_keycode(),
                        characters: BorrowedStrPtr::new(c"w"),
                        key: xkeysym::key::w,
                        is_repeat: false,
                        vk: VirtualKey::W,
                    },
                    move |state| {
                        let mapping = application_get_key_mapping(state.get_app_ptr(), KeyModifier::Ctrl.into(), '+'.into());
                        check_mapping_result(
                            mapping,
                            MappingResult {
                                modifiers_to_listen: (KeyModifier::Ctrl | KeyModifier::Shift).into(),
                                vk_to_listen: VirtualKey::Equals,
                                char_to_display: '='.into(),
                            },
                        )
                    },
                )
                .expect(KeyUpEvent {
                    code: KeyCodes::W.to_keycode(),
                    key: xkeysym::key::w,
                }),
            TestCase::default()
                .perform(|sys, _app_ptr| {
                    sys.change_keyboard_layout("rs");
                    sys.raw_key_press(KeyCodes::W);
                    sys.raw_key_release(KeyCodes::W);
                })
                .expect_event_with_check(
                    KeyDownEvent {
                        code: KeyCodes::W.to_keycode(),
                        characters: BorrowedStrPtr::new(c"њ"),
                        key: xkeysym::key::Cyrillic_nje,
                        is_repeat: false,
                        vk: VirtualKey::W,
                    },
                    move |state| {
                        let mapping = application_get_key_mapping(state.get_app_ptr(), KeyModifier::Ctrl.into(), '+'.into());
                        check_mapping_result(
                            mapping,
                            MappingResult {
                                modifiers_to_listen: KeyModifier::Ctrl.into(),
                                vk_to_listen: VirtualKey::Equals,
                                char_to_display: '+'.into(),
                            },
                        )
                    },
                )
                .expect(KeyUpEvent {
                    code: KeyCodes::W.to_keycode(),
                    key: xkeysym::key::Cyrillic_nje,
                }),
            TestCase::default()
                .perform(|sys, _app_ptr| {
                    sys.change_keyboard_layout("de");
                    sys.raw_key_press(KeyCodes::Y);
                    sys.raw_key_release(KeyCodes::Y);
                })
                .expect_event_with_check(
                    ModifiersChangedEvent {
                        modifiers: KeyModifierBitflag::EMPTY,
                    },
                    move |state| {
                        let mapping = application_get_key_mapping(state.get_app_ptr(), KeyModifier::Ctrl.into(), '+'.into());
                        check_mapping_result(
                            mapping,
                            MappingResult {
                                modifiers_to_listen: KeyModifier::Ctrl.into(),
                                vk_to_listen: VirtualKey::RightBracket,
                                char_to_display: '+'.into(),
                            },
                        )
                    },
                )
                .expect(KeyDownEvent {
                    code: KeyCodes::Y.to_keycode(),
                    characters: BorrowedStrPtr::new(c"z"),
                    key: xkeysym::key::z,
                    is_repeat: false,
                    vk: VirtualKey::Z,
                })
                .expect(KeyUpEvent {
                    code: KeyCodes::Y.to_keycode(),
                    key: xkeysym::key::z,
                }),
            TestCase::default()
                .perform(|sys, _app_ptr| {
                    sys.change_keyboard_layout("de");
                    sys.raw_key_press(KeyCodes::RIGHTBRACE);
                    sys.raw_key_release(KeyCodes::RIGHTBRACE);
                })
                .expect(ModifiersChangedEvent {
                    modifiers: KeyModifierBitflag::EMPTY,
                })
                .expect(KeyDownEvent {
                    code: KeyCodes::RIGHTBRACE.to_keycode(),
                    characters: BorrowedStrPtr::new(c"+"),
                    key: xkeysym::key::plus,
                    is_repeat: false,
                    vk: VirtualKey::RightBracket,
                })
                .expect(KeyUpEvent {
                    code: KeyCodes::RIGHTBRACE.to_keycode(),
                    key: xkeysym::key::plus,
                }),
            TestCase::default()
                .perform(|sys, _app_ptr| {
                    sys.change_keyboard_layout("tr");
                    sys.raw_key_press(KeyCodes::RIGHTBRACE);
                    sys.raw_key_release(KeyCodes::RIGHTBRACE);
                })
                .expect(ModifiersChangedEvent {
                    modifiers: KeyModifierBitflag::EMPTY,
                })
                .expect(KeyDownEvent {
                    code: KeyCodes::RIGHTBRACE.to_keycode(),
                    characters: BorrowedStrPtr::new(c"w"),
                    key: xkeysym::key::w,
                    is_repeat: false,
                    vk: VirtualKey::W,
                })
                .expect_event_with_check(
                    KeyUpEvent {
                        code: KeyCodes::RIGHTBRACE.to_keycode(),
                        key: xkeysym::key::w,
                    },
                    move |state| {
                        let mapping = application_get_key_mapping(state.get_app_ptr(), KeyModifier::Ctrl.into(), ']'.into());
                        check_mapping_result(
                            mapping,
                            MappingResult {
                                modifiers_to_listen: (KeyModifier::Ctrl | KeyModifier::Alt).into(),
                                vk_to_listen: VirtualKey::_9,
                                char_to_display: '9'.into(),
                            },
                        )
                    },
                ),
            TestCase::default()
                .perform(|sys, _app_ptr| {
                    sys.change_keyboard_layout("de");
                    sys.raw_key_press(KeyCodes::MINUS);
                    sys.raw_key_release(KeyCodes::MINUS);
                })
                .expect(ModifiersChangedEvent {
                    modifiers: KeyModifierBitflag::EMPTY,
                })
                .expect(KeyDownEvent {
                    code: KeyCodes::MINUS.to_keycode(),
                    characters: BorrowedStrPtr::new(c"ß"),
                    key: xkeysym::key::ssharp,
                    is_repeat: false,
                    vk: VirtualKey::Minus,
                })
                .expect(KeyUpEvent {
                    code: KeyCodes::MINUS.to_keycode(),
                    key: xkeysym::key::ssharp,
                }),
            TestCase::default()
                .perform(|sys, _app_ptr| {
                    sys.change_keyboard_layout("de");
                    sys.raw_key_press(KeyCodes::SLASH);
                    sys.raw_key_release(KeyCodes::SLASH);
                })
                .expect_event_with_check(
                    ModifiersChangedEvent {
                        modifiers: KeyModifierBitflag::EMPTY,
                    },
                    move |state| {
                        let mapping = application_get_key_mapping(state.get_app_ptr(), KeyModifier::Ctrl.into(), '/'.into());
                        check_mapping_result(
                            mapping,
                            MappingResult {
                                modifiers_to_listen: (KeyModifier::Ctrl | KeyModifier::Shift).into(),
                                vk_to_listen: VirtualKey::_7,
                                char_to_display: '7'.into(),
                            },
                        )
                    },
                )
                .expect(KeyDownEvent {
                    code: KeyCodes::SLASH.to_keycode(),
                    characters: BorrowedStrPtr::new(c"-"),
                    key: xkeysym::key::minus,
                    is_repeat: false,
                    vk: VirtualKey::Slash,
                })
                .expect(KeyUpEvent {
                    code: KeyCodes::SLASH.to_keycode(),
                    key: xkeysym::key::minus,
                }),
            TestCase::default()
                .perform(|sys, _app_ptr| {
                    sys.client_side_decorations_for_active_window();
                })
                .expect(WindowConfigureEvent {
                    window_id: WINDOW_1_ID,
                    size: INITIAL_WINDOW_1_SIZE,
                    active: true,
                    maximized: false,
                    fullscreen: false,
                    decoration_mode: WindowDecorationMode::Client,
                    capabilities: SWAY_WINDOW_CAPABILITIES,
                }),
            TestCase::default()
                .perform(|sys, _app_ptr| {
                    sys.server_side_decorations_for_active_window();
                })
                .expect(WindowConfigureEvent {
                    window_id: WINDOW_1_ID,
                    size: INITIAL_WINDOW_1_SIZE,
                    active: true,
                    maximized: false,
                    fullscreen: false,
                    decoration_mode: WindowDecorationMode::Client,
                    capabilities: SWAY_WINDOW_CAPABILITIES,
                })
                .expect(WindowConfigureEvent {
                    window_id: WINDOW_1_ID,
                    // some space for the server-side decorations
                    size: LogicalSize {
                        width: LogicalPixels(1996.),
                        height: LogicalPixels(973.),
                    },
                    active: true,
                    maximized: false,
                    fullscreen: false,
                    decoration_mode: WindowDecorationMode::Server,
                    capabilities: SWAY_WINDOW_CAPABILITIES,
                }),
            TestCase::default()
                .perform(|_sys, app_ptr| {
                    debug!("window_set_fullscreen");
                    window_set_fullscreen(app_ptr, WINDOW_1_ID);
                })
                .expect(WindowConfigureEvent {
                    window_id: WINDOW_1_ID,
                    // some space for the server-side decorations
                    size: LogicalSize {
                        width: LogicalPixels(1996.),
                        height: LogicalPixels(973.),
                    },
                    active: true,
                    maximized: false,
                    fullscreen: true,
                    decoration_mode: WindowDecorationMode::Server,
                    capabilities: SWAY_WINDOW_CAPABILITIES,
                })
                .expect(WindowConfigureEvent {
                    window_id: WINDOW_1_ID,
                    // display size divided by scale (1.5)
                    size: LogicalSize {
                        width: LogicalPixels(2000.),
                        height: LogicalPixels(1000.),
                    },
                    active: true,
                    maximized: false,
                    fullscreen: true,
                    decoration_mode: WindowDecorationMode::Server,
                    capabilities: SWAY_WINDOW_CAPABILITIES,
                }),
            TestCase::default()
                .perform(|sys, _app_ptr| {
                    sys.close_windows(APP_ID.to_str().unwrap());
                })
                .expect(WindowCloseRequestEvent { window_id: WINDOW_1_ID }),
        ]);
        app_ptr
    });
    application_run_event_loop(app_ptr.clone());
    application_shutdown(app_ptr);
    STATE.with_borrow(|state| if let Some(e) = &state.test_error { Err(e.clone()) } else { Ok(()) })
}
