use crate::macos::keyboard::KeyCode;
use anyhow::Context;
use desktop_common::logger::ffi_boundary;
use objc2_core_foundation::CFRetained;
use objc2_core_graphics::{CGEvent, CGEventSource, CGEventSourceStateID, CGEventTapLocation};
use std::cell::LazyCell;

thread_local! {
    static EVENT_SOURCE: LazyCell<Option<CFRetained<CGEventSource>>> = LazyCell::new(|| {
        CGEventSource::new(CGEventSourceStateID::HIDSystemState)
    });
}

fn get_event_source() -> anyhow::Result<CFRetained<CGEventSource>> {
    EVENT_SOURCE.with(|source| source.as_ref().context("Failed to create CGEventSource").cloned())
}

#[unsafe(no_mangle)]
pub extern "C" fn emulate_keyboard_event(keycode: KeyCode, key_down: bool) {
    ffi_boundary("emulate_key_press", || {
        let keycode = keycode.0;

        let source = get_event_source()?;
        // CGEventSource::set_keyboard_type(Some(&source), todo!());

        // Create and post key event
        let key_event = CGEvent::new_keyboard_event(Some(&source), keycode, key_down).context("Failed to create key event")?;
        CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&key_event));
        Ok(())
    });
}
