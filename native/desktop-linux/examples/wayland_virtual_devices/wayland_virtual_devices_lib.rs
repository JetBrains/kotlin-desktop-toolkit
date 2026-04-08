#![cfg(target_os = "linux")]

use anyhow::{anyhow, bail};
use log::{debug, error};
use smithay_client_toolkit::{
    delegate_input_method, delegate_registry, delegate_seat,
    reexports::{
        calloop::{self, EventLoop},
        calloop_wayland_source::WaylandSource,
        client::{
            Connection, Proxy, QueueHandle, delegate_noop,
            globals::registry_queue_init,
            protocol::{wl_pointer, wl_seat::WlSeat},
        },
        protocols::wp::text_input::zv3::client::zwp_text_input_v3::ContentHint,
        protocols_misc::zwp_virtual_keyboard_v1::client::{zwp_virtual_keyboard_manager_v1, zwp_virtual_keyboard_v1},
        protocols_wlr::virtual_pointer::v1::client::{zwlr_virtual_pointer_manager_v1, zwlr_virtual_pointer_v1},
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        input_method::{Active, InputMethod, InputMethodEventState, InputMethodHandler, InputMethodManager, ZwpInputMethodV2},
    },
};
use std::cell::RefCell;
use std::sync::{Arc, Condvar, Mutex};
use std::{io::Write, os::fd::AsFd, time::Instant};
use xkbcommon::xkb;

pub type CursorPosition = smithay_client_toolkit::seat::input_method::CursorPosition;

pub struct TestHelper {
    sender: Option<calloop::channel::Sender<TestHelperCommand>>,
    channel: calloop::channel::Channel<TestHelperCommand>,
}

impl TestHelper {
    #[must_use]
    pub fn new() -> Self {
        let (sender, channel) = calloop::channel::channel();
        Self {
            sender: Some(sender),
            channel,
        }
    }

    pub fn get_sender(&mut self) -> Box<dyn Fn(TestHelperCommand) + Send + Sync> {
        let s = self.sender.take().unwrap();
        Box::new(move |c| s.send(c).unwrap())
    }

    pub fn run(self, wait_for_im_update: Arc<(Mutex<bool>, Condvar)>) -> anyhow::Result<()> {
        let conn = Connection::connect_to_env()?;

        let (globals, event_queue) = registry_queue_init(&conn)?;
        let qh = event_queue.handle();

        let mut event_loop = EventLoop::<TestHelperState>::try_new()?;

        let seat_state = SeatState::new(&globals, &qh);
        let input_method_manager = InputMethodManager::bind(&globals, &qh)?;
        let seat: WlSeat = seat_state.seats().next().ok_or_else(|| anyhow!("No seat"))?;

        let input_method = input_method_manager.get_input_method(&qh, &seat);
        let virtual_keyboard_manager: zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1 = globals.bind(&qh, 1..=1, ())?;
        let virtual_keyboard = virtual_keyboard_manager.create_virtual_keyboard(&seat, &qh, ());

        let virtual_pointer_manager: zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1 = globals.bind(&qh, 1..=1, ())?;
        let virtual_pointer = virtual_pointer_manager.create_virtual_pointer(Some(&seat), &qh, ());

        let mut state = TestHelperState {
            start_instant: Instant::now(),
            conn: conn.clone(),
            loop_signal: event_loop.get_signal(),
            registry_state: RegistryState::new(&globals),
            seat_state,
            input_method,
            im_state: RefCell::default(),
            base_time: Instant::now(),
            virtual_keyboard,
            xkb_state: None,
            virtual_pointer,
            wait_for_im_update,
        };

        state.update_keymap("en")?;

        event_loop
            .handle()
            .insert_source(self.channel, move |event, (), state| {
                if let calloop::channel::Event::Msg(command) = event {
                    state.do_command(command);
                }
            })
            .map_err(|e| anyhow!(e.to_string()))?;

        WaylandSource::new(conn, event_queue)
            .insert(event_loop.handle())
            .map_err(|e| anyhow!(e.to_string()))?;

        event_loop.run(None, &mut state, |_| {})?;
        Ok(())
    }
}

impl Default for TestHelper {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PreeditStringData {
    pub text: String,
    pub cursor: CursorPosition,
}

pub struct DeleteSurroundingTextData {
    pub before_length: u32,
    pub after_length: u32,
}

pub struct InputCommandData {
    pub commit_string: Option<String>,
    pub preedit_string: Option<PreeditStringData>,
    pub delete_surrounding_text: Option<DeleteSurroundingTextData>,
}

pub struct RawKeyCommandData {
    pub keycode: u32,
    pub down: bool,
}

pub struct MouseMoveData {
    pub x: u32,
    pub y: u32,
    pub x_extent: u32,
    pub y_extent: u32,
}

pub struct MouseButtonData {
    pub button: u32,
    pub down: bool,
}

pub struct MouseScrollData {
    pub axis_source: u8,
    pub vertical_scroll_120: i32,
    pub horizontal_scroll_120: i32,
}

pub type SendableBox<T> = Box<dyn FnOnce(T) + Send>;

pub enum TestHelperCommand {
    Exit(SendableBox<bool>),
    Input(InputCommandData, SendableBox<bool>),
    Uppercase(SendableBox<bool>),
    GetInputState(SendableBox<String>),
    SetKeyboardLayout(String, SendableBox<bool>),
    RawKey(RawKeyCommandData, SendableBox<bool>),
    MouseMove(MouseMoveData, SendableBox<bool>),
    MouseButton(MouseButtonData, SendableBox<bool>),
    MouseScroll(MouseScrollData, SendableBox<bool>),
}

impl SeatHandler for TestHelperState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {}

    fn new_capability(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat, _capability: Capability) {}

    fn remove_capability(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat, _capability: Capability) {}
    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {}
}

delegate_seat!(TestHelperState);
impl ProvidesRegistryState for TestHelperState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![SeatState,];
}

delegate_registry!(TestHelperState);
delegate_input_method!(TestHelperState);

delegate_noop!(TestHelperState: ignore zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1);
delegate_noop!(TestHelperState: ignore zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1);

delegate_noop!(TestHelperState: ignore zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1);
delegate_noop!(TestHelperState: ignore zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1);

fn is_alive<P: Proxy>(proxy: &P) -> bool {
    proxy.is_alive()
}

struct TestHelperState {
    start_instant: Instant,
    conn: Connection,
    loop_signal: calloop::LoopSignal,
    registry_state: RegistryState,
    seat_state: SeatState,
    input_method: InputMethod,
    im_state: RefCell<Option<InputMethodEventState>>,
    base_time: Instant,
    virtual_keyboard: zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
    xkb_state: Option<xkb::State>,
    virtual_pointer: zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
    wait_for_im_update: Arc<(Mutex<bool>, Condvar)>,
}

fn update_xkb_key(
    xkb_state: &mut xkb::State,
    keycode: xkb::Keycode,
    direction: xkb::KeyDirection,
) -> Option<(xkb::ModMask, xkb::ModMask, xkb::ModMask, xkb::LayoutMask)> {
    let depressed_mods_old = xkb_state.serialize_mods(xkb::STATE_MODS_DEPRESSED);
    let latched_mods_old = xkb_state.serialize_mods(xkb::STATE_MODS_LATCHED);
    let locked_mods_old = xkb_state.serialize_mods(xkb::STATE_MODS_LOCKED);
    let effective_layout_old = xkb_state.serialize_layout(xkb::STATE_LAYOUT_EFFECTIVE);

    xkb_state.update_key(keycode, direction);

    let depressed_mods_new = xkb_state.serialize_mods(xkb::STATE_MODS_DEPRESSED);
    let latched_mods_new = xkb_state.serialize_mods(xkb::STATE_MODS_LATCHED);
    let locked_mods_new = xkb_state.serialize_mods(xkb::STATE_MODS_LOCKED);
    let effective_layout_new = xkb_state.serialize_layout(xkb::STATE_LAYOUT_EFFECTIVE);

    if depressed_mods_old != depressed_mods_new
        || latched_mods_old != latched_mods_new
        || locked_mods_old != locked_mods_new
        || effective_layout_old != effective_layout_new
    {
        Some((depressed_mods_new, latched_mods_new, locked_mods_new, effective_layout_new))
    } else {
        None
    }
}

fn get_char_with_index_before_cursor(text: &str, cursor_byte_pos: u32) -> Option<(u32, char)> {
    let mut prev_char_with_pos: Option<(u32, char)> = None;
    let cursor_byte_pos = usize::try_from(cursor_byte_pos).unwrap();
    for (byte_pos, c) in text.char_indices() {
        if byte_pos == cursor_byte_pos {
            return prev_char_with_pos;
        }
        prev_char_with_pos = Some((u32::try_from(byte_pos).unwrap(), c));
    }
    None
}

fn validate_byte_positions(text: &str, start_byte_pos: u32, end_byte_pos: u32) -> bool {
    let start_byte_pos = usize::try_from(start_byte_pos).unwrap();
    let end_byte_pos = usize::try_from(end_byte_pos).unwrap();
    let mut found_start = false;
    for (byte_pos, c) in text.char_indices() {
        debug!("byte_pos={byte_pos}, char={c}");
        if byte_pos == start_byte_pos {
            found_start = true;
        }
        if byte_pos == end_byte_pos {
            return found_start;
        }
    }
    false
}

impl TestHelperState {
    fn get_time(&self) -> u32 {
        let duration = self.base_time.elapsed();
        let time = duration.as_millis();
        time.try_into().unwrap_or(u32::MAX)
    }

    #[allow(clippy::string_lit_as_bytes)]
    fn update_keymap(&mut self, layout_name: &str) -> anyhow::Result<()> {
        debug!("Changing the keyboard layout to {layout_name}");
        let (layout_content, group) = match layout_name {
            "az" => (include_str!("k_az"), 0),
            "de" => (include_str!("k_de"), 2),
            "en" => (include_str!("k_en"), 0),
            "fr" => (include_str!("k_fr"), 0),
            "lv" => (include_str!("k_lv_erg"), 2),
            "tr" => (include_str!("k_tr_f"), 2),
            "rs" => (include_str!("k_rs"), 0),
            _ => bail!("Unknown layout name {layout_name}"),
        };

        let vk = &self.virtual_keyboard;
        if !is_alive(vk) {
            bail!("VK not alive");
        }

        let xkb_context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let xkb_keymap = xkb::Keymap::new_from_string(
            &xkb_context,
            layout_content.to_owned(),
            xkb::KEYMAP_FORMAT_TEXT_V1,
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
        .unwrap();

        let state = xkb::State::new(&xkb_keymap);
        self.xkb_state = Some(state);

        let encoded_layout_content = layout_content.as_bytes();

        let mut tmpfile = tempfile::tempfile()?;
        tmpfile.write_all(encoded_layout_content)?;

        let size = u32::try_from(encoded_layout_content.len())?;

        debug!("vk.keymap({layout_name})");
        vk.keymap(1, tmpfile.as_fd(), size);

        debug!("vk.modifiers(0, 0, 0, {group})");
        vk.modifiers(0, 0, 0, group);

        self.conn.roundtrip()?;

        Ok(())
    }

    fn raw_key(&mut self, data: &RawKeyCommandData) -> anyhow::Result<()> {
        let vk = &self.virtual_keyboard;
        if !is_alive(vk) {
            bail!("VK not alive");
        }

        let xkb_direction = if data.down {
            debug!("raw_key press {:?}", data.keycode);
            xkb::KeyDirection::Down
        } else {
            debug!("raw_key release {:?}", data.keycode);
            xkb::KeyDirection::Up
        };

        if let Some((mods_depressed, mods_latched, mods_locked, group)) =
            update_xkb_key(self.xkb_state.as_mut().unwrap(), xkb::Keycode::new(data.keycode), xkb_direction)
        {
            vk.modifiers(mods_depressed, mods_latched, mods_locked, group);
        } else {
            let time = self.get_time();
            let wayland_direction = u32::from(data.down);
            vk.key(time, data.keycode - 8, wayland_direction);
        }

        self.conn.flush()?;

        Ok(())
    }

    fn input(&self, data: InputCommandData) -> anyhow::Result<()> {
        {
            let im_state_borrow = self.im_state.borrow();
            let Some(im_state) = im_state_borrow.as_ref() else {
                bail!("Input method not activated")
            };
            if !matches!(im_state.active, Active::Active { .. }) {
                bail!("Input method not active: ({:?})", im_state.active);
            }
        }
        let input_method = &self.input_method;
        if let Some(v) = data.commit_string {
            input_method.commit_string(v);
        }
        if let Some(v) = data.preedit_string {
            input_method.set_preedit_string(v.text, v.cursor);
        }
        if let Some(v) = data.delete_surrounding_text {
            input_method.delete_surrounding_text(v.before_length, v.after_length);
        }
        input_method.commit();
        Ok(())
    }

    fn uppercase(&self) -> anyhow::Result<()> {
        let im_state_borrow = self.im_state.borrow();
        let Some(im_state) = im_state_borrow.as_ref() else {
            bail!("Input method not activated")
        };
        if !matches!(
            im_state.active,
            Active::Active {
                surrounding_text: true,
                ..
            }
        ) {
            bail!("Input method not active or didn't set surrounding text: ({:?})", im_state.active);
        }
        let input_method = &self.input_method;
        let text = &im_state.surrounding.text;
        let cursor_byte_pos = im_state.surrounding.cursor;
        let anchor_byte_pos = im_state.surrounding.anchor;

        if cursor_byte_pos == anchor_byte_pos {
            let Some((range_start, c)) = get_char_with_index_before_cursor(text.as_str(), cursor_byte_pos) else {
                bail!("Invalid cursor position: {cursor_byte_pos}");
            };

            let uppercased = c.to_uppercase().collect::<String>();
            input_method.delete_surrounding_text(cursor_byte_pos - range_start, 0);
            input_method.commit_string(uppercased);
        } else {
            let selection_start_byte = cursor_byte_pos.min(anchor_byte_pos);
            let selection_end_byte = cursor_byte_pos.max(anchor_byte_pos);
            if !validate_byte_positions(text, selection_start_byte, selection_end_byte) {
                bail!("Invalid cursor positions: {cursor_byte_pos} or {anchor_byte_pos}");
            }
            let substring = &text[selection_start_byte as usize..selection_end_byte as usize];
            let uppercased = substring.to_uppercase();

            let chars_byte_len = selection_end_byte - selection_start_byte;
            if cursor_byte_pos > anchor_byte_pos {
                input_method.delete_surrounding_text(chars_byte_len, 0);
            } else {
                input_method.delete_surrounding_text(0, chars_byte_len);
            }
            input_method.commit_string(uppercased);
        }
        input_method.commit();
        Ok(())
    }

    fn get_timestamp(&self) -> u32 {
        let timestamp = self.start_instant.elapsed().as_millis();
        u32::try_from(timestamp).unwrap() + 1
    }

    fn send_virtual_pointer_axis_discrete(&self, timestamp: u32, axis: wl_pointer::Axis, scroll_120: i32) {
        // On touchpad, scrolling once produces
        // "wl_pointer vertical=AxisScroll { absolute: -10.0, discrete: 0, value120: -120, ...}"
        let value = f64::from(scroll_120) / 12.;
        self.virtual_pointer.axis(timestamp, axis, value);
        self.virtual_pointer.axis_discrete(timestamp, axis, value, scroll_120 / 120);
    }

    fn mouse_scroll(&self, data: &MouseScrollData) -> anyhow::Result<()> {
        let timestamp = self.get_timestamp();
        let source = match data.axis_source {
            0 => wl_pointer::AxisSource::Wheel,
            1 => wl_pointer::AxisSource::Finger,
            2 => wl_pointer::AxisSource::Continuous,
            3 => wl_pointer::AxisSource::WheelTilt,
            _ => {
                bail!("Invalid axis_source: {}", data.axis_source);
            }
        };
        self.virtual_pointer.axis_source(source);
        if data.horizontal_scroll_120 != 0 {
            self.send_virtual_pointer_axis_discrete(timestamp, wl_pointer::Axis::HorizontalScroll, data.horizontal_scroll_120);
        }
        if data.vertical_scroll_120 != 0 {
            self.send_virtual_pointer_axis_discrete(timestamp, wl_pointer::Axis::VerticalScroll, data.vertical_scroll_120);
        }
        self.virtual_pointer.frame();
        Ok(())
    }

    fn get_input_state_string(&self) -> String {
        let borrow = self.im_state.borrow();
        let Some(im_state) = borrow.as_ref() else { return String::new() };
        let content_hints = im_state
            .content_hint
            .iter()
            .map(|hint| {
                if hint == ContentHint::Completion {
                    "Completion"
                } else if hint == ContentHint::Spellcheck {
                    "Spellcheck"
                } else if hint == ContentHint::AutoCapitalization {
                    "AutoCapitalization"
                } else if hint == ContentHint::Lowercase {
                    "Lowercase"
                } else if hint == ContentHint::Uppercase {
                    "Uppercase"
                } else if hint == ContentHint::Titlecase {
                    "Titlecase"
                } else if hint == ContentHint::HiddenText {
                    "HiddenText"
                } else if hint == ContentHint::SensitiveData {
                    "SensitiveData"
                } else if hint == ContentHint::Latin {
                    "Latin"
                } else if hint == ContentHint::Multiline {
                    "Multiline"
                } else {
                    "Unknown"
                }
            })
            .collect::<Vec<&'static str>>()
            .join(", ");
        let content_purpose = im_state.content_purpose;
        format!("content_purpose: {content_purpose:?}, content_hints: [{content_hints}]")
    }

    fn do_command(&mut self, command: TestHelperCommand) {
        match command {
            TestHelperCommand::Exit(f) => {
                self.loop_signal.stop();
                f(true);
            }
            TestHelperCommand::Input(data, f) => {
                let success = if let Err(e) = self.input(data) {
                    error!("{e}");
                    false
                } else {
                    true
                };
                f(success);
            }
            TestHelperCommand::GetInputState(f) => {
                f(self.get_input_state_string());
            }
            TestHelperCommand::SetKeyboardLayout(layout_name, f) => {
                let success = if let Err(e) = self.update_keymap(&layout_name) {
                    error!("{e}");
                    false
                } else {
                    true
                };
                f(success);
            }
            TestHelperCommand::RawKey(data, f) => {
                let success = if let Err(e) = self.raw_key(&data) {
                    error!("{e}");
                    false
                } else {
                    true
                };
                f(success);
            }
            TestHelperCommand::Uppercase(f) => {
                let success = if let Err(e) = self.uppercase() {
                    error!("{e}");
                    false
                } else {
                    true
                };
                f(success);
            }
            TestHelperCommand::MouseMove(data, f) => {
                let success = if data.x > data.x_extent || data.y > data.y_extent {
                    false
                } else {
                    self.virtual_pointer
                        .motion_absolute(self.get_timestamp(), data.x, data.y, data.x_extent, data.y_extent);
                    self.virtual_pointer.frame();
                    true
                };
                f(success);
            }
            TestHelperCommand::MouseButton(data, f) => {
                let state: wl_pointer::ButtonState = if data.down {
                    wl_pointer::ButtonState::Pressed
                } else {
                    wl_pointer::ButtonState::Released
                };
                self.virtual_pointer.button(self.get_timestamp(), data.button, state);
                self.virtual_pointer.frame();
                f(true);
            }
            TestHelperCommand::MouseScroll(data, f) => {
                let success = if let Err(e) = self.mouse_scroll(&data) {
                    error!("{e}");
                    false
                } else {
                    true
                };
                f(success);
            }
        }
    }
}

impl InputMethodHandler for TestHelperState {
    fn handle_done(
        &self,
        _connection: &Connection,
        _qh: &QueueHandle<Self>,
        _input_method: &ZwpInputMethodV2,
        state: &InputMethodEventState,
    ) {
        *self.im_state.borrow_mut() = Some(state.clone());
        debug!("IM state changed to {state:?}");
        let (wait_for_im_update, wait_for_im_update_condvar) = &*self.wait_for_im_update;
        *wait_for_im_update.lock().unwrap() = false;
        wait_for_im_update_condvar.notify_one();
    }

    fn handle_unavailable(&self, _connection: &Connection, _qh: &QueueHandle<Self>, _input_method: &ZwpInputMethodV2) {
        *self.im_state.borrow_mut() = None;
        debug!("IM state changed to None");
        let (wait_for_im_update, wait_for_im_update_condvar) = &*self.wait_for_im_update;
        *wait_for_im_update.lock().unwrap() = false;
        wait_for_im_update_condvar.notify_one();
    }
}
