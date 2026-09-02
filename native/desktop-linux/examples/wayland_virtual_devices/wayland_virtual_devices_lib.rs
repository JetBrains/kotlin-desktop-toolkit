#![cfg(target_os = "linux")]

use log::{debug, error};
use std::ffi::c_int;
use uinput::event::{Kind, Press, Release};

struct RawKey {
    raw: i32,
}

impl Press for RawKey {}
impl Release for RawKey {}

impl uinput::event::Code for RawKey {
    fn code(&self) -> c_int {
        self.raw
    }
}

impl Kind for RawKey {
    fn kind(&self) -> c_int {
        uinput_sys::EV_KEY
    }
}

pub struct TestHelper {
    current_mouse_position: (i32, i32),
    keyboard_device: uinput::Device,
    mouse_device: uinput::Device,
}

impl TestHelper {
    pub fn new() -> anyhow::Result<Self> {
        let keyboard_device = uinput::default()?
            .name("kdt_test_keyboard")?
            .vendor(0x1234)
            .product(0x001)
            .event(uinput::event::Keyboard::All)?
            .create()?;

        let mouse_device = {
            use uinput::event::relative::Position;
            use uinput::event::relative::Wheel;

            let mut builder = uinput::default()?
                .name("kdt_test_mouse")?
                .vendor(0x1234)
                .product(0x002)
                .event(Position::X)?
                .event(Position::Y)?
                .event(Wheel::Horizontal)?
                .event(Wheel::Vertical)?;

            for e in uinput::event::controller::Mouse::iter_variants() {
                builder = builder.event(e)?;
            }

            builder.create()?
        };

        Ok(Self {
            current_mouse_position: (0, 0),
            keyboard_device,
            mouse_device,
        })
    }

    pub fn run(mut self, command_receiver: &std::sync::mpsc::Receiver<Option<TestHelperCommand>>) {
        while let Ok(command) = command_receiver.recv() {
            if let Some(command) = command {
                self.do_command(command);
            } else {
                return;
            }
        }
    }
}

pub struct RawKeyCommandData {
    pub keycode: i32,
    pub down: bool,
}

pub struct MouseMoveData {
    pub x: i32,
    pub y: i32,
}

pub struct MouseButtonData {
    pub button: i32,
    pub down: bool,
}

pub struct MouseScrollData {
    pub axis_source: u8,
    pub vertical_scroll_120: i32,
    pub horizontal_scroll_120: i32,
}

pub type SendableBox<T> = Box<dyn FnOnce(T) + Send>;

pub enum TestHelperCommand {
    RawKey(RawKeyCommandData, SendableBox<bool>),
    MouseMove(MouseMoveData, SendableBox<bool>),
    MouseButton(MouseButtonData, SendableBox<bool>),
    MouseScroll(MouseScrollData, SendableBox<bool>),
}

impl TestHelper {
    fn raw_key(&mut self, data: &RawKeyCommandData) -> anyhow::Result<()> {
        if data.down {
            self.keyboard_device.press(&RawKey { raw: data.keycode })?;
        } else {
            self.keyboard_device.release(&RawKey { raw: data.keycode })?;
        }
        self.keyboard_device.synchronize()?;

        Ok(())
    }

    fn mouse_scroll(&mut self, data: &MouseScrollData) -> anyhow::Result<()> {
        use uinput::event::relative::Relative::Wheel;
        use uinput::event::relative::Wheel::Horizontal;
        use uinput::event::relative::Wheel::Vertical;
        if data.horizontal_scroll_120 != 0 {
            self.mouse_device.send(Wheel(Horizontal), data.horizontal_scroll_120)?;
        }
        if data.vertical_scroll_120 != 0 {
            self.mouse_device.send(Wheel(Vertical), data.vertical_scroll_120)?;
        }

        self.mouse_device.synchronize()?;

        Ok(())
    }

    fn mouse_move(&mut self, data: &MouseMoveData) -> anyhow::Result<()> {
        use uinput::event::relative::Position::{X, Y};
        use uinput::event::relative::Relative::Position;
        if data.x == 0 && data.y == 0 {
            self.current_mouse_position = (0, 0);
            self.mouse_device.position(&Position(X), i32::MIN)?;
            self.mouse_device.position(&Position(Y), i32::MIN)?;
        } else {
            let (current_x, current_y) = self.current_mouse_position;
            let delta_x = data.x - current_x;
            let delta_y = data.y - current_y;
            debug!("Moving mouse with delta {delta_x},{delta_y}");
            self.mouse_device.position(&Position(X), delta_x)?;
            self.mouse_device.position(&Position(Y), delta_y)?;
            self.current_mouse_position = (current_x + delta_x, current_y + delta_y);
        }
        self.mouse_device.synchronize()?;
        Ok(())
    }

    fn mouse_button(&mut self, data: &MouseButtonData) -> anyhow::Result<()> {
        if data.down {
            self.mouse_device.press(&RawKey { raw: data.button })?;
        } else {
            self.mouse_device.release(&RawKey { raw: data.button })?;
        }
        self.mouse_device.synchronize()?;
        Ok(())
    }

    fn do_command(&mut self, command: TestHelperCommand) {
        match command {
            TestHelperCommand::RawKey(data, f) => {
                let success = if let Err(e) = self.raw_key(&data) {
                    error!("{e}");
                    false
                } else {
                    true
                };
                f(success);
            }
            TestHelperCommand::MouseMove(data, f) => {
                let success = if let Err(e) = self.mouse_move(&data) {
                    error!("{e}");
                    false
                } else {
                    true
                };
                f(success);
            }
            TestHelperCommand::MouseButton(data, f) => {
                let success = if let Err(e) = self.mouse_button(&data) {
                    error!("{e}");
                    false
                } else {
                    true
                };
                f(success);
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
