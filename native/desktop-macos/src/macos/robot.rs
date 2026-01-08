use crate::macos::keyboard::KeyCode;
use anyhow::Context;
use objc2_core_foundation::{CFMachPort, CFRetained, CFRunLoop, CFRunLoopSource, kCFRunLoopDefaultMode};
use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventMask, CGEventSource, CGEventSourceStateID, CGEventTapCallBack, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventTapProxy, CGEventType,
};
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::mpsc::SyncSender;
use std::thread;
use std::thread::JoinHandle;

#[allow(clippy::struct_field_names)]
pub struct Robot {
    event_tap_thread: EventTapThread,
    event_source: CFRetained<CGEventSource>,
    event_counter: i64,
}

impl Robot {
    const EVENT_MARKER: i64 = 0x4B44_545F_524F_424F; // "KDT_ROBO" in hex

    pub(crate) fn new() -> anyhow::Result<Self> {
        let robot = Self {
            event_tap_thread: EventTapThread::new()?,
            event_source: CGEventSource::new(CGEventSourceStateID::HIDSystemState).context("Can't create even source")?,
            event_counter: 0,
        };
        Ok(robot)
    }

    const fn next_event_id(&mut self) -> i64 {
        self.event_counter += 1;
        Self::EVENT_MARKER ^ self.event_counter
    }

    pub(crate) fn emulate_keyboard_event(&mut self, keycode: KeyCode, key_down: bool) -> anyhow::Result<()> {
        let keycode = keycode.0;
        let event_id = self.next_event_id();
        CGEventSource::set_user_data(Some(&self.event_source), event_id);
        let key_event = CGEvent::new_keyboard_event(Some(&self.event_source), keycode, key_down).context("Failed to create key event")?;
        CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&key_event));
        self.event_tap_thread.wait_for_event(event_id);
        Ok(())
    }

    pub(crate) fn shutdown(&mut self) -> anyhow::Result<()> {
        self.event_tap_thread.join()
    }
}

struct EventTapThread {
    #[allow(dead_code)]
    handle: Option<JoinHandle<()>>,
    events_data_rcv: std::sync::mpsc::Receiver<i64>,
    run_loop_wrapper: RunLoopWrapper,
}

struct RunLoopWrapper(CFRetained<CFRunLoop>);
impl RunLoopWrapper {
    fn stop(&self) {
        self.0.stop();
    }
}

// SAFETY: Still under discussion, see:
// https://github.com/madsmtm/objc2/issues/696
#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for RunLoopWrapper {}

struct TapSubscription {
    tap: CFRetained<CFMachPort>,
    run_loop_source: CFRetained<CFRunLoopSource>,
    run_loop: CFRetained<CFRunLoop>,
}

impl EventTapThread {
    unsafe extern "C-unwind" fn event_tap_callback(
        _proxy: CGEventTapProxy,
        _event_type: CGEventType,
        event: NonNull<CGEvent>,
        user_info: *mut c_void,
    ) -> *mut CGEvent {
        let event_ref = unsafe { event.as_ref() };
        let user_data = CGEvent::integer_value_field(Some(event_ref), CGEventField::EventSourceUserData);
        let events_data_snd_ptr = user_info.cast::<SyncSender<i64>>();
        let event_data_snd = unsafe { events_data_snd_ptr.as_ref() }.unwrap_or_else(|| panic!("user_info: {user_info:?}"));
        event_data_snd
            .send(user_data)
            .unwrap_or_else(|_| log::error!("Failed to send event data"));
        event.as_ptr()
    }

    const fn create_keyboard_event_mask() -> CGEventMask {
        (1 << CGEventType::KeyDown.0) | (1 << CGEventType::KeyUp.0) | (1 << CGEventType::FlagsChanged.0)
    }

    fn new() -> anyhow::Result<Self> {
        let (mark_is_ready, check_is_ready) = std::sync::mpsc::sync_channel::<anyhow::Result<RunLoopWrapper>>(1);
        let (events_data_snd, events_data_rcv) = std::sync::mpsc::sync_channel::<i64>(1);
        let handle = thread::spawn(move || {
            // Safety: the Arc is alive until the event loop is running
            let events_data_snd = Arc::new(events_data_snd);
            match Self::create_tap_subscription(Arc::as_ptr(&events_data_snd)) {
                Ok(subscription) => {
                    mark_is_ready
                        .send(Ok(RunLoopWrapper(subscription.run_loop.clone())))
                        .expect("Can't fail here");
                    CFRunLoop::run();
                    Self::remove_tap_subscription(subscription);
                }
                Err(err) => {
                    mark_is_ready.send(Err(err)).expect("Can't fail here");
                }
            }
        });

        let run_loop_wrapper = check_is_ready.recv().expect("Can't fail here")?;

        Ok(Self {
            handle: Some(handle),
            events_data_rcv,
            run_loop_wrapper,
        })
    }

    fn create_tap_subscription(events_data_snd_ptr: *const SyncSender<i64>) -> anyhow::Result<TapSubscription> {
        let callback: CGEventTapCallBack = Some(Self::event_tap_callback);
        let tap = unsafe {
            CGEvent::tap_create(
                CGEventTapLocation::HIDEventTap,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::ListenOnly,
                Self::create_keyboard_event_mask(),
                callback,
                events_data_snd_ptr.cast_mut().cast::<c_void>(),
            )
        }
        .context("Failed to create event tap. Check accessibility permissions.")?;

        let run_loop_source = CFMachPort::new_run_loop_source(None, Some(&tap), 0).context("Failed to create run loop source")?;
        let run_loop = CFRunLoop::current().context("Failed to get current run loop")?;
        run_loop.add_source(Some(&run_loop_source), unsafe { kCFRunLoopDefaultMode });

        Ok(TapSubscription {
            tap,
            run_loop_source,
            run_loop,
        })
    }

    fn remove_tap_subscription(subscription: TapSubscription) {
        subscription
            .run_loop
            .remove_source(Some(&subscription.run_loop_source), unsafe { kCFRunLoopDefaultMode });
        subscription.tap.invalidate();
        drop(subscription);
    }

    fn wait_for_event(&self, event_id: i64) {
        self.events_data_rcv.iter().find(|it| *it == event_id);
    }

    fn join(&mut self) -> anyhow::Result<()> {
        self.run_loop_wrapper.stop();
        self.handle.take().context("Already joined")?.join().expect("Failed to join");
        Ok(())
    }
}
