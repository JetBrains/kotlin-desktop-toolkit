use crate::macos::keyboard::KeyCode;
use anyhow::Context;
use desktop_common::logger::ffi_boundary;
use objc2_core_foundation::{kCFRunLoopDefaultMode, CFMachPort, CFRetained, CFRunLoop, CFRunLoopSource};
use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventMask, CGEventSource, CGEventSourceStateID, CGEventTapCallBack, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventTapProxy, CGEventType,
};
use std::cell::LazyCell;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
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

const EVENT_MARKER: i64 = 0x4B44_545F_524F_424F; // "KDT_ROBO" in hex

static EVENT_COUNTER: AtomicI64 = AtomicI64::new(1);

/// Background thread that runs the event tap
struct EventTapThread {
    #[allow(dead_code)]
    handle: JoinHandle<()>,
    events_data_rcv: std::sync::mpsc::Receiver<i64>,
}

impl EventTapThread {
    fn new() -> Self {
        let (mark_is_ready, check_is_ready) = std::sync::mpsc::sync_channel::<anyhow::Result<()>>(1);
        let (events_data_snd, events_data_rcv) = std::sync::mpsc::sync_channel::<i64>(1);

        let handle = thread::spawn(move || {
            // Safety: the arc is alive until the event loop is running
            let events_data_snd = Arc::new(events_data_snd);
            let _subscription = Self::prepare_tap_subscription(Arc::as_ptr(&events_data_snd)).unwrap(); // todo handle error
            mark_is_ready.send(Ok(())).unwrap();
            CFRunLoop::run();
        });

        check_is_ready.recv().expect("Can't fail here").unwrap();

        Self { handle, events_data_rcv }
    }

    fn prepare_tap_subscription(events_data_snd_ptr: *const SyncSender<i64>) -> anyhow::Result<(CFRetained<CFMachPort>, CFRetained<CFRunLoopSource>, CFRetained<CFRunLoop>)> {
        let callback: CGEventTapCallBack = Some(event_tap_callback);
        let tap = unsafe {
            CGEvent::tap_create(
                CGEventTapLocation::HIDEventTap,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::ListenOnly,
                create_keyboard_event_mask(),
                callback,
                events_data_snd_ptr.cast_mut().cast::<c_void>(),
            )
        }.context("Failed to create event tap. Check accessibility permissions.")?;

        let run_loop_source = CFMachPort::new_run_loop_source(None, Some(&tap), 0).context("Failed to create run loop source")?;
        let run_loop = CFRunLoop::current().context("Failed to get current run loop")?;
        run_loop.add_source(Some(&run_loop_source), unsafe { kCFRunLoopDefaultMode });
        Ok((tap, run_loop_source, run_loop))
    }

    fn wait_for_event(&self, event_id: i64) {
        self.events_data_rcv.iter().find(|it| {
            *it == event_id
        });
    }
}

unsafe extern "C-unwind" fn event_tap_callback(
    _proxy: CGEventTapProxy,
    _event_type: CGEventType,
    event: NonNull<CGEvent>,
    user_info: *mut c_void,
) -> *mut CGEvent {
    let event_ref = unsafe { event.as_ref() };
    let user_data = CGEvent::integer_value_field(Some(event_ref), CGEventField::EventSourceUserData);
    let events_data_snd_ptr = user_info.cast::<SyncSender<i64>>();
    unsafe { events_data_snd_ptr.as_ref() }.unwrap().send(user_data).unwrap();
    event.as_ptr()
}

const fn create_keyboard_event_mask() -> CGEventMask {
    (1 << CGEventType::KeyDown.0) | (1 << CGEventType::KeyUp.0) | (1 << CGEventType::FlagsChanged.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn emulate_keyboard_event(keycode: KeyCode, key_down: bool) {
    ffi_boundary("emulate_key_press", || {
        EVENT_TAP_THREAD.with(|tap_thread| {
            let _handle = &tap_thread.handle; // this line is important because we need to initialize the lazy cell

            let keycode = keycode.0;

            let source = get_event_source().unwrap();

            let event_id = EVENT_MARKER ^ EVENT_COUNTER.fetch_add(1, Ordering::Relaxed);

            CGEventSource::set_user_data(Some(&source), event_id);
            let key_event = CGEvent::new_keyboard_event(Some(&source), keycode, key_down).context("Failed to create key event").unwrap();

            CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&key_event));
            tap_thread.wait_for_event(event_id);
        });

        Ok(())
    });
}
