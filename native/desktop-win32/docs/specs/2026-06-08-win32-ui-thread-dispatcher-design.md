# desktop-win32: UI-thread dispatcher — message-only window + Rust channel — design

Status: shipped (2026-06-09)
Owners: desktop-win32 native bindings

Scope: back the toolkit's cross-thread UI dispatch (`application_dispatcher_invoke`) with a Win32 message-only window plus an `std::sync::mpsc` channel, in a new module [src/win32/dispatcher.rs](../../src/win32/dispatcher.rs), instead of the WinRT `DispatcherQueue`. FFI signatures and the Kotlin layer are unchanged.

## TL;DR

- `application_dispatcher_invoke` is the toolkit's "run this closure on the UI thread" primitive. It moves off the WinRT `DispatcherQueue`, which can interfere with modal COM dialogs (file pickers) run on the same thread.
- The new primitive: a message-only window (`HWND_MESSAGE`) on the UI thread, an `std::sync::mpsc` channel of `extern "C" fn()` callbacks, and an `Arc<AtomicBool>` wake flag.
- `dispatch(cb)` sends the callback on the channel, then posts one wake message — but only if none is already pending (`wake_posted`). A burst of N calls collapses to a single posted message.
- The window's wndproc handles a private wake message (registered via `RegisterWindowMessage`): it clears `wake_posted` before draining (so a racing send re-posts and is never lost), then runs every queued callback via `rx.try_iter()`.
- A posted **window** message is delivered by Win32's inner modal message loops (file dialogs, popup menus, drag-drop, move/size); a thread message (`PostThreadMessageW`) is not — hence a message-only *window*.
- Teardown swaps the handle out of an `AtomicPtr` once, so `shutdown` and the `Drop` backstop destroy the window exactly once.
- The `DispatcherQueueController` stays: WUC's `Compositor` still needs a `DispatcherQueue` on the thread. Only the public dispatch path moves off it. No new crate dependency.

## 1. Motivation

`application_dispatcher_invoke` is the single cross-thread "run this on the UI thread" entry point. Kotlin's `Application.invokeOnDispatcher` pushes a closure onto a `ConcurrentLinkedQueue` and calls the FFI with one fixed `pollCallbacks` trampoline ([Application.kt](../../../kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/win32/Application.kt)); the Rust side runs that trampoline on the UI thread.

A WinRT `DispatcherQueue` active on a thread can interfere with modal COM dialogs run on the same thread — a file dialog can stop accepting keyboard input. This toolkit runs `IFileOpenDialog::Show` / `IFileSaveDialog::Show` ([file_dialog.rs](../../src/win32/file_dialog.rs)) on the UI thread, so it carries that exposure. Backing dispatch with a plain message-only window + channel removes it. (Zed's gpui made the same move — see §6.)

The `DispatcherQueue` itself stays, only because the WUC `Compositor` cannot be created on a thread without one.

## 2. Goals / Non-goals

**Goals**

- Back `application_dispatcher_invoke` with a message-only window + an `std::sync::mpsc` channel.
- Deliver queued callbacks during Win32 inner modal loops (file dialogs, popup menus, drag-drop, move/size).
- Coalesce a burst of enqueues into one posted message.
- Keep the FFI (`application_dispatcher_invoke`, `application_is_dispatcher_thread`) and the Kotlin layer unchanged.
- No new crate dependency.

**Non-goals**

- Removing the `DispatcherQueue` / `DispatcherQueueController` (WUC needs it).
- Changing the `CommitNeeded` → `DispatcherQueue` drain in [compositor_driver.rs](../../src/win32/compositor_driver.rs) or the WUC shutdown sequencing.
- Changing the `GetMessageW`/`DispatchMessageW` pump in [event_loop.rs](../../src/win32/event_loop.rs) — it already dispatches this window's messages.

## 3. Design — `dispatcher.rs`

### 3.1 Channel

The payload is `extern "C" fn()` — a bare function pointer: `Copy`, `Send`, and `Sync` (it points at code and owns no data). `std::sync::mpsc` fits, with no new dependency:

- `Sender<T>` is `Send + Sync` for `T: Send`, so it lives directly in the shared `Dispatcher` struct and can be cloned to producer threads.
- `Receiver<T>` is `!Sync`, so the type system keeps the consumer end on the UI thread.
- `Receiver::try_iter` is the non-blocking drain — "run everything queued right now."

### 3.2 Struct

```rust
/// cbindgen:ignore
const WNDCLASS_NAME: PCWSTR = w!("KotlinDesktopToolkitWin32Dispatcher");
/// cbindgen:ignore
const DISPATCHER_PTR_PROP_NAME: PCWSTR = w!("KDT_DISPATCHER_PTR");
/// cbindgen:ignore
const WAKE_MESSAGE_NAME: PCWSTR = w!("KotlinDesktopToolkitWin32DispatcherMessage");

type DispatchCallback = extern "C" fn();

/// Producer-facing handle, held by `Application` and shared to any thread.
pub struct Dispatcher {
    hwnd: AtomicPtr<core::ffi::c_void>, // message-only window; swapped to null at teardown
    tx: Sender<DispatchCallback>,
    wake_posted: Arc<AtomicBool>,
}

/// Consumer half, pinned behind the window prop and touched only by the wndproc.
/// `rx` is `!Sync`, so this stays on the UI thread.
struct DispatcherState {
    rx: Receiver<DispatchCallback>,
    wake_posted: Arc<AtomicBool>,
}
```

The wake message id is registered once with `RegisterWindowMessage`:

```rust
fn wake_message() -> u32 {
    static WAKE_MESSAGE: OnceLock<u32> = OnceLock::new();
    // SAFETY: RegisterWindowMessageW has no preconditions; the same string returns the same id.
    *WAKE_MESSAGE.get_or_init(|| unsafe { RegisterWindowMessageW(WAKE_MESSAGE_NAME) })
}
```

### 3.3 dispatch

```rust
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
            // SAFETY: PostMessageW is thread-safe; a concurrently-destroyed handle just Errs.
            unsafe { PostMessageW(Some(HWND(hwnd)), wake_message(), WPARAM(0), LPARAM(0)) }.is_ok()
        })
    }
}
```

A burst of enqueues collapses to one posted message: `wake_posted` gates the post, and Kotlin's `pollCallbacks` drains its `ConcurrentLinkedQueue` to empty on the UI thread regardless of how many wakes fired. If the post fails, `wake_posted` is reset so the next `dispatch` re-posts, and `dispatch` returns `false`.

### 3.4 wndproc + drain

```rust
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
```

Clearing `wake_posted` before the `try_iter` drain is what makes a racing `send` safe: a producer that sends during the drain either was already collected, or sees the cleared flag and posts a fresh wake. (This relies on `mpsc` making a completed `send` visible to a `try_iter` started afterward — do not reorder to clear-after-drain.) Collecting into an owned `Vec` ends the `&DispatcherState` borrow before any callback runs, and bounds the batch so a callback that re-enqueues rides the next wake instead of extending this drain.

### 3.5 Window lifecycle

```rust
impl Dispatcher {
    pub fn new() -> anyhow::Result<Self> {
        static WNDCLASS_INIT: OnceLock<u16> = OnceLock::new();

        // RegisterWindowMessage returns 0 only on failure; refuse rather than fall back to id 0.
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
        let state = Box::new(DispatcherState { rx, wake_posted: Arc::clone(&wake_posted) });

        // SAFETY: HWND_MESSAGE makes this a message-only window on the UI thread.
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0), WNDCLASS_NAME, w!("KDT Dispatcher"), WINDOW_STYLE(0),
                0, 0, 0, 0, Some(HWND_MESSAGE), None, Some(crate::get_dll_instance()), None,
            )?
        };
        let raw_state = Box::into_raw(state);
        // SAFETY: hands the boxed state to the window prop; it lives until WM_NCDESTROY reclaims it.
        if let Err(e) = unsafe { SetPropW(hwnd, DISPATCHER_PTR_PROP_NAME, Some(HANDLE(raw_state.cast()))) } {
            // SAFETY: SetPropW failed, so `raw_state` has no other owner; reclaim it and destroy the window.
            drop(unsafe { Box::from_raw(raw_state) });
            let _ = unsafe { DestroyWindow(hwnd) };
            return Err(e.into());
        }
        Ok(Self { hwnd: AtomicPtr::new(hwnd.0), tx, wake_posted })
    }

    pub fn shutdown(&self) {
        // WM_NCDESTROY drops DispatcherState + rx, so later dispatch() sends return Err.
        if let Some(hwnd) = take_hwnd(&self.hwnd) {
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
    fn drop(&mut self) {
        if let Some(hwnd) = take_hwnd(&self.hwnd) {
            let _ = unsafe { DestroyWindow(hwnd) };
        }
    }
}
```

- Created on the UI thread (the thread that pumps its messages), as a message-only `HWND_MESSAGE` window.
- `new` registers the wake message and returns an error if `RegisterWindowMessage` fails (returns 0), rather than fall back to the colliding id 0.
- `DispatcherState` (owning `rx`) is attached via `SetPropW` and dropped in `WM_NCDESTROY`, closing the consumer end so any later `send` fails cleanly.
- `shutdown` and the `Drop` backstop both take the handle via `take_hwnd` (an `AtomicPtr` swap-to-null), so `DestroyWindow` runs exactly once. `DestroyWindow` must run on the UI thread; `shutdown` is called from `Application::shutdown`, which is on it.

### 3.6 Send + Sync

`HWND` is `!Send`, but producers on other threads need the handle for `PostMessageW`, and teardown needs to take it once. An `AtomicPtr<core::ffi::c_void>` is `Send + Sync` outright (no wrapper, no `unsafe impl`), so `Dispatcher` is `Send + Sync`:

```rust
static_assertions::assert_impl_all!(Dispatcher: Send, Sync);
```

`Application` owns the dispatcher directly and is itself UI-thread-only (`!Send`/`!Sync` via its `Rc`/WinRT fields), so the dispatcher's `Drop` — hence `DestroyWindow` — runs on the UI thread. `Sync` is the load-bearing half of the assert: the off-thread `dispatch` path (§4.1) reaches the dispatcher through a shared `&Dispatcher`.

## 4. Integration

### 4.1 `application.rs`

`Application` gains a `dispatcher: Dispatcher` field; `invoke_on_dispatcher` forwards to it. `Dispatcher::new` runs during `Application::new` on the UI thread, before the message pump starts (a `dispatch` issued earlier just waits in the thread queue). At shutdown the dispatcher is destroyed before `ShutdownQueueAsync`, so no wake outlives the consumer state.

```rust
pub struct Application {
    compositor_driver: Arc<CompositorDriver>,
    dispatcher_queue_controller: DispatcherQueueController,
    dispatcher: Dispatcher,
    event_loop: Rc<EventLoop>,
    ui_thread_id: u32,
}

impl Application {
    pub fn new(event_handler: EventHandler) -> anyhow::Result<Self> {
        let dispatcher_queue_controller = create_dispatcher_queue()?;
        let dispatcher = Dispatcher::new()?;
        // ... event loop, compositor controller / driver ...
        Ok(Self { /* compositor_driver, */ dispatcher_queue_controller, dispatcher, /* event_loop, ui_thread_id */ })
    }

    pub fn invoke_on_dispatcher(&self, callback: extern "C" fn()) -> anyhow::Result<bool> {
        Ok(self.dispatcher.dispatch(callback))
    }

    pub fn shutdown(&self) -> anyhow::Result<()> {
        self.compositor_driver.shutdown();
        super::composition::release_composition_context();
        self.dispatcher.shutdown();
        let _ = self.dispatcher_queue_controller.ShutdownQueueAsync()?;
        Ok(())
    }
}
```

`application_dispatcher_invoke` runs on whatever thread Kotlin calls from and forms `&Application` from the opaque pointer; the dispatch path touches only the dispatcher's `Send + Sync` interior, never `Application`'s `!Sync` fields.

### 4.2 `application_api.rs` / Kotlin

The FFI function `application_dispatcher_invoke` still calls `app.invoke_on_dispatcher(callback)`; its signature is unchanged, so the generated header is stable. Kotlin keeps enqueueing the single fixed `pollCallbacks` trampoline — the "always the same callback" pattern is what makes wake coalescing safe.

### 4.3 `mod.rs`

Add `pub mod dispatcher;`, matching the file's uniform `pub mod`. The module has no cbindgen surface (no `#[no_mangle] extern "C"` items), so `pub` does not widen the C header.

### 4.4 What stays for WUC

The `DispatcherQueueController` (`create_dispatcher_queue` and its `ShutdownCompleted` → `PostQuitMessage` handler), `CompositorController::new()`, the `CommitNeeded` drain in [compositor_driver.rs](../../src/win32/compositor_driver.rs), and `ShutdownQueueAsync` all stay — the `Compositor` still needs the queue. The `event_loop.rs` pump already dispatches this window's messages, since `DispatchMessageW` routes by the message's `hwnd`.

## 5. Test plan

The crate has no harness that spawns a UI thread, registers a window class, or pumps messages, so unit tests cover only what is reachable without a window:

- **Wake coalescing** — drive `try_wake` with a counting post closure: a burst posts at most once per drain.
- **Post-failure reset** — drive `try_wake` with a failing post: `wake_posted` resets and the queued item drains on the next wake.
- **Take-once teardown** — `take_hwnd` returns the handle on the first call, `None` after (sentinel pointer, no real window).

Manual / sample-app verification (the first is the load-bearing case):

- A dispatched callback runs while each modal loop is active — `TrackPopupMenu`, `IFileOpenDialog`/`IFileSaveDialog::Show` ([file_dialog.rs](../../src/win32/file_dialog.rs)), `DoDragDrop` ([drag_drop.rs](../../src/win32/drag_drop.rs)), and the modal move/size loop. This is the reason for a message-only *window*: those loops dispatch window messages but drop thread messages.
- The live `WM_NCDESTROY` teardown, and end-to-end Kotlin `invokeOnDispatcher` → `pollCallbacks` on the UI thread.

Verification bar: `./gradlew lint` (per [CLAUDE.md](../../../CLAUDE.md) / the pre-push hook) plus the tests above.

## 6. References

KDT files: [application.rs](../../src/win32/application.rs), [application_api.rs](../../src/win32/application_api.rs), [compositor_driver.rs](../../src/win32/compositor_driver.rs), [event_loop.rs](../../src/win32/event_loop.rs), [window.rs](../../src/win32/window.rs) (`RegisterClassExW` `OnceLock` + `GetPropW`/`SetPropW` + `WM_NCDESTROY` idiom), [file_dialog.rs](../../src/win32/file_dialog.rs), [Application.kt](../../../kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/win32/Application.kt).

Prior art: Zed `gpui` removed the WinRT `DispatcherQueue` from its Windows backend — [PR #17946](https://github.com/zed-industries/zed/pull/17946).

Win32:

- Message-only windows / `HWND_MESSAGE`: <https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features#message-only-windows>
- `CreateWindowExW`: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-createwindowexw>
- `PostMessageW`: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-postmessagew>
- Posted-message quota (`USERPostMessageLimit`, ~10 000): <https://learn.microsoft.com/en-us/windows/win32/winmsg/about-messages-and-message-queues>
- `RegisterWindowMessageW`: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerwindowmessagew>

Channel: `std::sync::mpsc` — <https://doc.rust-lang.org/std/sync/mpsc/index.html>
