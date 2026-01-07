use crate::macos::keyboard::KeyCode;
use anyhow::Context;
use desktop_common::logger::ffi_boundary;
use objc2_core_foundation::{kCFRunLoopDefaultMode, CFMachPort, CFRetained, CFRunLoop, CFRunLoopRunResult};
use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventMask, CGEventSource, CGEventSourceStateID, CGEventTapCallBack,
    CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventTapProxy, CGEventType,
};
use std::cell::LazyCell;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicI64, Ordering};

thread_local! {
    static EVENT_SOURCE: LazyCell<Option<CFRetained<CGEventSource>>> = LazyCell::new(|| {
        CGEventSource::new(CGEventSourceStateID::HIDSystemState)
    });
}

fn get_event_source() -> anyhow::Result<CFRetained<CGEventSource>> {
    EVENT_SOURCE.with(|source| source.as_ref().context("Failed to create CGEventSource").cloned())
}

/// Magic marker value to identify our emulated events
const EVENT_MARKER: i64 = 0x4B44_545F_524F_424F; // "KDT_ROBO" in hex

/// Counter to generate unique event IDs
static EVENT_COUNTER: AtomicI64 = AtomicI64::new(1);

/// Event tap callback that checks for our marked event
unsafe extern "C-unwind" fn event_tap_callback(
    _proxy: CGEventTapProxy,
    _event_type: CGEventType,
    event: NonNull<CGEvent>,
    user_info: *mut c_void,
) -> *mut CGEvent {
    let event_ref = unsafe { event.as_ref() };
    let user_data = CGEvent::integer_value_field(Some(event_ref), CGEventField::EventSourceUserData);

    // Check if this is our marked event
    let wait_flag = unsafe { &*user_info.cast::<AtomicI64>() };
    let expected_id = wait_flag.load(Ordering::Acquire);
    if user_data == expected_id {
        // Signal that we received our event by setting the flag to 0
        wait_flag.store(0, Ordering::Release);
    }

    // Return the event unchanged (we're just listening)
    event.as_ptr()
}

const fn create_keyboard_event_mask() -> CGEventMask {
    (1 << CGEventType::KeyDown.0) | (1 << CGEventType::KeyUp.0) | (1 << CGEventType::FlagsChanged.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn emulate_keyboard_event(keycode: KeyCode, key_down: bool) {
    ffi_boundary("emulate_key_press", || {
        let keycode = keycode.0;

        let source = get_event_source()?;

        // Generate unique event ID
        let event_id = EVENT_MARKER ^ EVENT_COUNTER.fetch_add(1, Ordering::Relaxed);

        // Set user data on the event source so the event will carry it
        CGEventSource::set_user_data(Some(&source), event_id);

        // Create key event
        let key_event = CGEvent::new_keyboard_event(Some(&source), keycode, key_down)
            .context("Failed to create key event")?;

        // Storage for the expected event ID - will be set to 0 when event is received
        // Using AtomicI64 to ensure proper synchronization with the callback
        let wait_flag = AtomicI64::new(event_id);

        // Create event tap to listen for keyboard events
        let callback: CGEventTapCallBack = Some(event_tap_callback);
        let tap = unsafe {
            CGEvent::tap_create(
                CGEventTapLocation::HIDEventTap,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::ListenOnly,
                create_keyboard_event_mask(),
                callback,
                std::ptr::addr_of!(wait_flag).cast_mut().cast::<c_void>(),
            )
        }
        .context("Failed to create event tap. Check accessibility permissions.")?;

        // Create run loop source from the tap
        let run_loop_source = CFMachPort::new_run_loop_source(None, Some(&tap), 0)
            .context("Failed to create run loop source")?;

        // Add source to current run loop
        let run_loop = CFRunLoop::current().context("Failed to get current run loop")?;
        let mode = unsafe { kCFRunLoopDefaultMode };
        run_loop.add_source(Some(&run_loop_source), mode);

        // Post the event
        CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&key_event));

        // Pump the run loop until we receive our event
        while wait_flag.load(Ordering::Acquire) != 0 {
            let result = CFRunLoop::run_in_mode(mode, 0.0, true);
            if result == CFRunLoopRunResult::Finished {
                break;
            }
        }

        // Cleanup
        run_loop.remove_source(Some(&run_loop_source), mode);

        Ok(())
    });
}
