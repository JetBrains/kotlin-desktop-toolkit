use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::mpsc::SyncSender;
use std::thread;
use std::thread::JoinHandle;
use anyhow::Context;
use objc2_core_foundation::{kCFRunLoopDefaultMode, CFMachPort, CFRetained, CFRunLoop, CFRunLoopSource};
use objc2_core_graphics::{CGEvent, CGEventField, CGEventMask, CGEventSource, CGEventSourceStateID, CGEventTapCallBack, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventTapProxy, CGEventType};
use crate::macos::keyboard::KeyCode;

const EVENT_MARKER: i64 = 0x4B44_545F_524F_424F; // "KDT_ROBO" in hex

pub(crate) struct Robot {
    event_tap_thread: EventTapThread,
    event_source: CFRetained<CGEventSource>,
    event_counter: AtomicI64,
}

impl Robot {
    pub(crate) fn new() -> anyhow::Result<Self> {
        let robot = Self {
            event_tap_thread: EventTapThread::new(),
            event_source: CGEventSource::new(CGEventSourceStateID::HIDSystemState).context("Can't create even source")?,
            event_counter: AtomicI64::new(0),
        };
        Ok(robot)
    }

    pub(crate) fn emulate_keyboard_event(&self, keycode: KeyCode, key_down: bool) -> anyhow::Result<()> {
        let keycode = keycode.0;

        let source = &self.event_source;

        let event_id = EVENT_MARKER ^ self.event_counter.fetch_add(1, Ordering::Relaxed);

        CGEventSource::set_user_data(Some(source), event_id);
        let key_event = CGEvent::new_keyboard_event(Some(source), keycode, key_down).context("Failed to create key event")?;

        CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&key_event));
        self.event_tap_thread.wait_for_event(event_id);
        Ok(())
    }
}

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