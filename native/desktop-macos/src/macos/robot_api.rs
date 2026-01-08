use crate::macos::keyboard::KeyCode;
use crate::macos::robot::Robot;
use anyhow::{bail, Context};
use desktop_common::logger::ffi_boundary;
use objc2::MainThreadMarker;
use std::cell::RefCell;

thread_local! {
    static ROBOT: RefCell<Option<Robot>> = RefCell::new(None);
}

#[unsafe(no_mangle)]
pub extern "C" fn robot_initialize() {
    ffi_boundary("robot_initialize", || {
        let _mtm = MainThreadMarker::new().context("Robot can be initialized only from the main thread")?;
        ROBOT.with_borrow_mut(|robot| {
            if robot.is_some() {
                bail!("Robot is already initialized");
            }
            robot.replace(Robot::new()?);
            Ok(())
        })
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn robot_deinitialize() {
    ffi_boundary("robot_deinitialize", || {
        let _mtm = MainThreadMarker::new().context("Robot can be initialized only from the main thread")?;
        ROBOT.with_borrow_mut(|robot| {
            match robot.take() {
                None => {
                    bail!("Robot is not initialized");
                }
                Some(mut robot) => {
                    robot.shutdown()?;
                }
            }
            Ok(())
        })
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn emulate_keyboard_event(keycode: KeyCode, key_down: bool) {
    ffi_boundary("emulate_key_press", || {
        let _mtm = MainThreadMarker::new().context("Robot can be initialized only from the main thread")?;
        ROBOT.with_borrow_mut(|robot| {
            let robot = robot.as_mut().context("Robot is not initialized")?;
            robot.emulate_keyboard_event(keycode, key_down)?;
            Ok(())
        })
    });
}
