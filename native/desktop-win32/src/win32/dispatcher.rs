//! Cross-thread "run on the UI thread" dispatch.
//!
//! A message-only window whose wndproc drains an mpsc channel of `extern "C" fn()`
//! callbacks: producers `dispatch` from any thread, and the UI thread runs them when it
//! pumps the posted wake message.

use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, AtomicPtr, Ordering},
    mpsc::{Receiver, Sender, channel},
};

use windows::Win32::{
    Foundation::{HANDLE, HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, GetPropW, HWND_MESSAGE, PostMessageW, RegisterClassExW, RegisterWindowMessageW,
        RemovePropW, SetPropW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_NCDESTROY, WNDCLASSEXW,
    },
};
use windows_core::{PCWSTR, w};

/// cbindgen:ignore
const WNDCLASS_NAME: PCWSTR = w!("KotlinDesktopToolkitWin32Dispatcher");
/// cbindgen:ignore
const DISPATCHER_PTR_PROP_NAME: PCWSTR = w!("KDT_DISPATCHER_PTR");
/// cbindgen:ignore
const WAKE_MESSAGE_NAME: PCWSTR = w!("KotlinDesktopToolkitWin32DispatcherMessage");

type DispatchCallback = extern "C" fn();

/// Wake message id, registered once with `RegisterWindowMessage`.
fn wake_message() -> u32 {
    static WAKE_MESSAGE: OnceLock<u32> = OnceLock::new();
    // SAFETY: RegisterWindowMessageW has no preconditions and returns the same id for the
    // same string throughout the process.
    *WAKE_MESSAGE.get_or_init(|| unsafe { RegisterWindowMessageW(WAKE_MESSAGE_NAME) })
}

/// Created on, and owned by, the UI thread.
pub struct Dispatcher {
    // AtomicPtr (Send + Sync): producers read it cross-thread; shutdown/Drop swap it to
    // null to destroy the window exactly once.
    hwnd: AtomicPtr<core::ffi::c_void>,
    tx: Sender<DispatchCallback>,
    wake_posted: Arc<AtomicBool>,
}

/// Owns the consumer end; reached from the wndproc via the window's prop.
/// `rx` is `!Sync`, so this stays on the UI thread.
struct DispatcherState {
    rx: Receiver<DispatchCallback>,
    wake_posted: Arc<AtomicBool>,
}

/// Only the producer that flips `wake_posted` false->true posts a wake; on post failure
/// the flag is reset so a later dispatch re-posts. Returns whether the dispatch is live.
fn try_wake(wake_posted: &AtomicBool, post: impl Fn() -> bool) -> bool {
    if !wake_posted.swap(true, Ordering::AcqRel) && !post() {
        wake_posted.store(false, Ordering::Release);
        return false;
    }
    true
}

impl Dispatcher {
    pub fn dispatch(&self, cb: extern "C" fn()) -> bool {
        if self.tx.send(cb).is_err() {
            return false; // Receiver dropped: dispatcher is shutting down.
        }
        // Null = shutdown already took the handle. Acquire pairs with take_hwnd's swap.
        let hwnd = self.hwnd.load(Ordering::Acquire);
        if hwnd.is_null() {
            return false;
        }
        try_wake(&self.wake_posted, || {
            // SAFETY: `hwnd` is the message-only window; PostMessageW is thread-safe, and a
            // concurrently-destroyed handle just Errs.
            unsafe { PostMessageW(Some(HWND(hwnd)), wake_message(), WPARAM(0), LPARAM(0)) }.is_ok()
        })
    }
}

// Clear the flag (Release) BEFORE draining so a producer racing the drain re-posts and
// nothing is lost; collect into an owned Vec so the &DispatcherState borrow ends before
// any cb() runs and re-enqueues ride the next wake (bounded drain).
fn take_batch(state: &DispatcherState) -> Vec<DispatchCallback> {
    state.wake_posted.store(false, Ordering::Release);
    state.rx.try_iter().collect()
}

extern "system" fn dispatcher_wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCDESTROY {
        if let Ok(raw) = unsafe { RemovePropW(hwnd, DISPATCHER_PTR_PROP_NAME) } {
            // SAFETY: the prop holds the Box<DispatcherState> from `new`, yielded once.
            drop(unsafe { Box::from_raw(raw.0.cast::<DispatcherState>()) });
        }
        return LRESULT(0);
    }
    if msg == wake_message() {
        // SAFETY: the prop, when present, is the live Box<DispatcherState> from `new`.
        let batch = match unsafe { GetPropW(hwnd, DISPATCHER_PTR_PROP_NAME).0.cast::<DispatcherState>().as_ref() } {
            Some(state) => take_batch(state),
            None => return LRESULT(0),
        };
        for cb in batch {
            cb();
        }
        return LRESULT(0);
    }
    // SAFETY: default window-procedure forwarding.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

impl Dispatcher {
    pub fn new() -> anyhow::Result<Self> {
        static WNDCLASS_INIT: OnceLock<u16> = OnceLock::new();

        // RegisterWindowMessage returns 0 only on failure; refuse to run rather than fall
        // back to id 0 (WM_NULL). This first call also performs the one-time registration.
        anyhow::ensure!(wake_message() != 0, windows_core::Error::from_thread());

        let wndclass_size = size_of::<WNDCLASSEXW>().try_into()?;
        let _ = WNDCLASS_INIT.get_or_init(|| {
            let wndclass = WNDCLASSEXW {
                cbSize: wndclass_size,
                hInstance: crate::get_dll_instance(),
                lpszClassName: WNDCLASS_NAME,
                lpfnWndProc: Some(dispatcher_wndproc),
                ..Default::default()
            };
            // SAFETY: `wndclass` is a fully-initialized WNDCLASSEXW valid for this call.
            unsafe { RegisterClassExW(&raw const wndclass) }
        });

        let (tx, rx) = channel::<DispatchCallback>();
        let wake_posted = Arc::new(AtomicBool::new(false));
        let state = Box::new(DispatcherState {
            rx,
            wake_posted: Arc::clone(&wake_posted),
        });

        // SAFETY: all arguments are valid; the class is registered above and HWND_MESSAGE
        // makes this a message-only window on the UI thread.
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                WNDCLASS_NAME,
                w!("KDT Dispatcher"),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(crate::get_dll_instance()),
                None,
            )?
        };
        let raw_state = Box::into_raw(state);
        // SAFETY: hands the boxed state to the window prop; it lives until WM_NCDESTROY
        // reclaims it.
        if let Err(e) = unsafe { SetPropW(hwnd, DISPATCHER_PTR_PROP_NAME, Some(HANDLE(raw_state.cast()))) } {
            // SAFETY: SetPropW failed, so `raw_state` has no other owner; reclaim it and
            // destroy the window so neither leaks.
            drop(unsafe { Box::from_raw(raw_state) });
            let _ = unsafe { DestroyWindow(hwnd) };
            return Err(e.into());
        }

        Ok(Self {
            hwnd: AtomicPtr::new(hwnd.0),
            tx,
            wake_posted,
        })
    }

    pub fn shutdown(&self) {
        // Take the handle so DestroyWindow runs exactly once (across shutdown + Drop).
        // WM_NCDESTROY drops DispatcherState + rx, so later dispatch() sends return Err.
        if let Some(hwnd) = take_hwnd(&self.hwnd) {
            // SAFETY: `hwnd` is the live window from `new`, taken once; DestroyWindow runs
            // on the owning UI thread.
            let _ = unsafe { DestroyWindow(hwnd) };
        }
    }
}

/// Take the handle atomically (swap to null): only the first caller gets it, so
/// `DestroyWindow` runs exactly once across `shutdown` and the `Drop` backstop.
fn take_hwnd(hwnd: &AtomicPtr<core::ffi::c_void>) -> Option<HWND> {
    let raw = hwnd.swap(core::ptr::null_mut(), Ordering::AcqRel);
    (!raw.is_null()).then_some(HWND(raw))
}

impl Drop for Dispatcher {
    // Backstop when shutdown() never ran (early-error path). If it did, take_hwnd returns
    // None and this is a no-op.
    fn drop(&mut self) {
        if let Some(hwnd) = take_hwnd(&self.hwnd) {
            // SAFETY: `hwnd` is the live window from `new`, taken once; Drop runs on the
            // owning UI thread.
            let _ = unsafe { DestroyWindow(hwnd) };
        }
    }
}

static_assertions::assert_impl_all!(Dispatcher: Send, Sync);

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering},
        mpsc::channel,
    };

    use super::{DispatchCallback, take_hwnd, try_wake};

    // Modal-loop callback delivery and end-to-end Kotlin -> pollCallbacks are verified
    // manually in the sample app: the crate has no UI-thread/window/pump test harness.

    #[test]
    fn try_wake_coalesces_bursts_into_single_post_between_drains() {
        const N: usize = 100;

        let wake_posted = AtomicBool::new(false);
        let post_count = AtomicUsize::new(0);
        let post = || {
            post_count.fetch_add(1, Ordering::Relaxed);
            true
        };

        // A burst with no intervening drain posts at most once.
        for _ in 0..N {
            assert!(try_wake(&wake_posted, post));
        }
        assert_eq!(post_count.load(Ordering::Relaxed), 1);

        // A drain clears the flag; the next burst posts once more.
        wake_posted.store(false, Ordering::Release);
        for _ in 0..N {
            assert!(try_wake(&wake_posted, post));
        }
        assert_eq!(post_count.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn try_wake_resets_flag_on_post_failure_and_item_drains_next_wake() {
        extern "C" fn noop() {}

        let (tx, rx) = channel::<DispatchCallback>();
        let wake_posted = AtomicBool::new(false);

        tx.send(noop).expect("send into live channel");

        // Post fails: try_wake returns false and resets the flag; the item stays queued.
        assert!(!try_wake(&wake_posted, || false));
        assert!(!wake_posted.load(Ordering::Acquire));

        // Next wake succeeds; the still-queued item is drainable.
        assert!(try_wake(&wake_posted, || true));
        assert!(wake_posted.load(Ordering::Acquire));
        assert_eq!(rx.try_iter().count(), 1);
    }

    #[test]
    fn take_hwnd_yields_handle_once_then_none() {
        // shutdown() and the Drop backstop both call take_hwnd; only the first gets the
        // handle, so DestroyWindow runs exactly once. The sentinel is never dereferenced.
        let sentinel = core::ptr::without_provenance_mut::<core::ffi::c_void>(0x1234);
        let hwnd = AtomicPtr::new(sentinel);
        assert_eq!(take_hwnd(&hwnd).map(|h| h.0), Some(sentinel));
        assert!(take_hwnd(&hwnd).is_none());
        assert!(take_hwnd(&hwnd).is_none());
    }
}
