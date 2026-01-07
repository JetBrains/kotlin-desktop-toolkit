use crate::macos::keyboard::KeyCode;
use anyhow::Context;
use desktop_common::logger::ffi_boundary;
use objc2_core_foundation::{CFMachPort, CFRetained, CFRunLoop, CFRunLoopRunResult, kCFRunLoopDefaultMode};
use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventMask, CGEventSource, CGEventSourceStateID, CGEventTapCallBack, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventTapProxy, CGEventType,
};
use std::cell::LazyCell;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::thread::{self, JoinHandle};

thread_local! {
    static EVENT_SOURCE: LazyCell<Option<CFRetained<CGEventSource>>> = LazyCell::new(|| {
        CGEventSource::new(CGEventSourceStateID::HIDSystemState)
    });

    static EVENT_TAP_THREAD: LazyCell<EventTapThread> = LazyCell::new(EventTapThread::new);
}

fn get_event_source() -> anyhow::Result<CFRetained<CGEventSource>> {
    EVENT_SOURCE.with(|source| source.as_ref().context("Failed to create CGEventSource").cloned())
}

/// Magic marker value to identify our emulated events
const EVENT_MARKER: i64 = 0x4B44_545F_524F_424F; // "KDT_ROBO" in hex

/// Counter to generate unique event IDs
static EVENT_COUNTER: AtomicI64 = AtomicI64::new(1);

/// Shared state for communication with the event tap thread
struct SharedState {
    /// The event ID we're waiting for (0 means no pending event, set back to 0 when received)
    wait_flag: AtomicI64,
    /// Signal that the tap is ready and listening (or failed to initialize)
    tap_ready: AtomicBool,
    /// Whether the tap was successfully created
    tap_ok: AtomicBool,
    /// Signal that the run loop is actively running
    run_loop_active: AtomicBool,
}

/// Background thread that runs the event tap
struct EventTapThread {
    state: Arc<SharedState>,
    #[allow(dead_code)]
    handle: JoinHandle<()>,
}

impl EventTapThread {
    fn new() -> Self {
        let state = Arc::new(SharedState {
            wait_flag: AtomicI64::new(0),
            tap_ready: AtomicBool::new(false),
            tap_ok: AtomicBool::new(false),
            run_loop_active: AtomicBool::new(false),
        });
        let state_for_thread = Arc::clone(&state);

        let handle = thread::spawn(move || {
            if let Err(e) = Self::run_tap_loop(&state_for_thread) {
                log::error!("Event tap thread error: {e}");
                // Signal that we're "ready" (but failed) so the main thread doesn't hang
                state_for_thread.tap_ready.store(true, Ordering::Release);
            }
        });

        // Wait for the tap to be ready (or failed) with a timeout
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !state.tap_ready.load(Ordering::Acquire) {
            if std::time::Instant::now() > deadline {
                log::error!("Event tap thread initialization timed out");
                break;
            }
            thread::sleep(std::time::Duration::from_millis(1));
        }

        // Wait for the run loop to actually start processing
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !state.run_loop_active.load(Ordering::Acquire) {
            if std::time::Instant::now() > deadline {
                log::error!("Timeout waiting for run loop to become active");
                break;
            }
            thread::sleep(std::time::Duration::from_millis(1));
        }

        Self { state, handle }
    }

    fn run_tap_loop(state: &Arc<SharedState>) -> anyhow::Result<()> {
        // Create event tap to listen for keyboard events
        let callback: CGEventTapCallBack = Some(event_tap_callback);
        let tap = unsafe {
            CGEvent::tap_create(
                CGEventTapLocation::HIDEventTap,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::ListenOnly,
                create_keyboard_event_mask(),
                callback,
                Arc::as_ptr(state).cast_mut().cast::<c_void>(),
            )
        }
        .context("Failed to create event tap. Check accessibility permissions.")?;

        // Create run loop source from the tap
        let run_loop_source = CFMachPort::new_run_loop_source(None, Some(&tap), 0).context("Failed to create run loop source")?;

        // Add source to this thread's run loop
        let run_loop = CFRunLoop::current().context("Failed to get current run loop")?;
        let mode = unsafe { kCFRunLoopDefaultMode };
        run_loop.add_source(Some(&run_loop_source), mode);

        // Do one iteration first to ensure the tap is fully registered
        CFRunLoop::run_in_mode(mode, 0.0, false);

        // Signal that the tap is ready and working
        state.tap_ok.store(true, Ordering::Release);
        state.run_loop_active.store(true, Ordering::Release);
        state.tap_ready.store(true, Ordering::Release);

        // Run the event loop forever
        loop {
            let result = CFRunLoop::run_in_mode(mode, 0.1, true);
            if result == CFRunLoopRunResult::Finished {
                break;
            }
        }

        Ok(())
    }

    fn prepare_for_event(&self, event_id: i64) {
        // Set the event ID we're waiting for BEFORE posting the event
        self.state.wait_flag.store(event_id, Ordering::Release);
    }

    fn wait_for_event(&self) {
        // If tap failed to initialize, just return immediately (non-blocking mode)
        if !self.state.tap_ok.load(Ordering::Acquire) {
            return;
        }

        // Spin until the callback sets wait_flag to 0
        while self.state.wait_flag.load(Ordering::Acquire) != 0 {
            std::hint::spin_loop();
        }
    }
}

/// Event tap callback that checks for our marked event
unsafe extern "C-unwind" fn event_tap_callback(
    _proxy: CGEventTapProxy,
    _event_type: CGEventType,
    event: NonNull<CGEvent>,
    user_info: *mut c_void,
) -> *mut CGEvent {
    let event_ref = unsafe { event.as_ref() };
    let user_data = CGEvent::integer_value_field(Some(event_ref), CGEventField::EventSourceUserData);

    let state = unsafe { &*user_info.cast::<SharedState>() };
    let expected_id = state.wait_flag.load(Ordering::Acquire);

    // Check if this is our marked event (and we're actually waiting for something)
    if expected_id != 0 && user_data == expected_id {
        // Signal that we received our event
        state.wait_flag.store(0, Ordering::Release);
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
        let key_event = CGEvent::new_keyboard_event(Some(&source), keycode, key_down).context("Failed to create key event")?;

        // Get the background tap thread (initialized on first use)
        EVENT_TAP_THREAD.with(|tap_thread| {
            // Set the wait flag BEFORE posting so the callback knows what to look for
            tap_thread.prepare_for_event(event_id);

            // Post the event
            CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&key_event));

            // Wait for the event to be received by the tap
            tap_thread.wait_for_event();
        });

        Ok(())
    });
}
