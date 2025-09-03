#![cfg(target_os = "linux")]

use std::{
    io::Write,
    os::fd::AsFd,
    sync::Mutex,
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail};
use log::{debug, error};
use smithay_client_toolkit::{
    delegate_input_method, delegate_registry, delegate_seat,
    reexports::{
        calloop::{self, EventLoop},
        calloop_wayland_source::WaylandSource,
        client::{Connection, Proxy, QueueHandle, delegate_noop, globals::registry_queue_init, protocol::wl_seat::WlSeat},
        protocols_misc::zwp_virtual_keyboard_v1::client::{zwp_virtual_keyboard_manager_v1, zwp_virtual_keyboard_v1},
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        input_method::{InputMethod, InputMethodEventState, InputMethodHandler, InputMethodManager, SurroundingText, ZwpInputMethodV2},
    },
};
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

    pub fn run(self, callback: Box<dyn FnOnce()>) -> anyhow::Result<()> {
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

        let mut state = TestHelperState {
            conn: conn.clone(),
            loop_signal: event_loop.get_signal(),
            registry_state: RegistryState::new(&globals),
            seat_state,
            input_method: Mutex::new(Some(input_method)),
            surrounding_text: Mutex::new(None),
            base_time: Instant::now(),
            virtual_keyboard,
            xkb_state: None,
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

        event_loop.dispatch(Duration::from_millis(100), &mut state)?;
        callback();
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

pub type SendableBox<T> = Box<dyn FnOnce(T) + Send>;
pub type SendableBoxRef<T> = Box<dyn FnOnce(&T) + Send>;

pub enum TestHelperCommand {
    Exit(SendableBox<bool>),
    Input(InputCommandData, SendableBox<bool>),
    GetInputState(SendableBoxRef<Option<SurroundingText>>),
    SetKeyboardLayout(String, SendableBox<bool>),
    RawKey(RawKeyCommandData, SendableBox<bool>),
}

impl SeatHandler for TestHelperState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {}

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {}

    fn new_capability(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat, _capability: Capability) {}
    fn remove_capability(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat, _capability: Capability) {}
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

fn is_alive<P: Proxy>(proxy: &P) -> bool {
    proxy.is_alive()
}

struct TestHelperState {
    conn: Connection,
    loop_signal: calloop::LoopSignal,
    registry_state: RegistryState,
    seat_state: SeatState,
    input_method: Mutex<Option<InputMethod>>,
    surrounding_text: Mutex<Option<SurroundingText>>,
    base_time: Instant,
    virtual_keyboard: zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
    xkb_state: Option<xkb::State>,
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
            update_xkb_key(self.xkb_state.as_mut().unwrap(), xkb::Keycode::new(data.keycode + 8), xkb_direction)
        {
            vk.modifiers(mods_depressed, mods_latched, mods_locked, group);
        } else {
            let time = self.get_time();
            let wayland_direction = u32::from(data.down);
            vk.key(time, data.keycode, wayland_direction);
        }

        self.conn.flush()?;

        Ok(())
    }

    fn do_command(&mut self, command: TestHelperCommand) {
        match command {
            TestHelperCommand::Exit(f) => {
                self.loop_signal.stop();
                f(true);
            }
            TestHelperCommand::Input(data, f) => {
                let success = if let Some(input_method) = &*self.input_method.lock().unwrap() {
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
                    true
                } else {
                    false
                };
                f(success);
            }
            TestHelperCommand::GetInputState(f) => {
                f(&self.surrounding_text.lock().unwrap());
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
        *self.surrounding_text.lock().unwrap() = Some(state.surrounding.clone());
    }

    fn handle_unavailable(&self, _connection: &Connection, _qh: &QueueHandle<Self>, _input_method: &ZwpInputMethodV2) {
        *self.surrounding_text.lock().unwrap() = None;
        *self.input_method.lock().unwrap() = None;
    }
}
