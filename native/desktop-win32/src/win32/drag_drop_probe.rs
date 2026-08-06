//! Run-time probes for the Windows drag-and-drop wedge tracked on this branch.
//!
//! Everything here is inert unless the matching environment variable is set, so it is safe to leave
//! compiled in for the duration of the investigation. See
//! `docs/investigations/2026-08-06-dodragdrop-pointer-input-starvation.md` for what each probe is
//! meant to decide and what has and has not been proven so far.
//!
//! | Variable                                 | Effect                                                                     |
//! |------------------------------------------|----------------------------------------------------------------------------|
//! | `KDT_WIN32_DND_PROBE=1`                  | logs input state around `DoDragDrop` and every message the window receives while a drag is in flight |
//! | `KDT_WIN32_DND_CONVERT_POINTER_TO_MOUSE=1` | calls `ConvertPrimaryPointerToMouseDrag` immediately before `DoDragDrop`  |
//! | `KDT_WIN32_NO_MOUSE_IN_POINTER=1`        | skips the process-wide `EnableMouseInPointer(true)` in `EventLoop::new`     |

use std::sync::{
    OnceLock,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use windows::Win32::{
    System::{
        LibraryLoader::{GetModuleHandleW, GetProcAddress},
        Threading::GetCurrentThreadId,
    },
    UI::{
        Input::{
            KeyboardAndMouse::{GetCapture, GetKeyState, VK_LBUTTON},
            Pointer::IsMouseInPointerEnabled,
        },
        WindowsAndMessaging::{GetQueueStatus, QS_ALLINPUT},
    },
};
use windows_core::{BOOL, HRESULT, PCSTR, s, w};

/// `WM_MOUSEFIRST`..`WM_MOUSELAST`: the legacy mouse band, which is what `DoDragDrop` is documented to
/// consume.
const WM_MOUSE_BAND: std::ops::RangeInclusive<u32> = 0x0200..=0x020E;
/// The `WM_POINTER*` band. Not a documented constant pair; taken from the individual message values in
/// `WinUser.h`, so treat the range ends as our own bookkeeping rather than as an SDK guarantee.
const WM_POINTER_BAND: std::ops::RangeInclusive<u32> = 0x0240..=0x024F;

/// Set while `DoDragDrop` is on the stack. Read by [`note_window_message`] so that the message log is
/// limited to the interval that matters instead of covering the whole session.
static DRAG_IN_FLIGHT: AtomicBool = AtomicBool::new(false);
static MOUSE_BAND_SEEN_IN_DRAG: AtomicU64 = AtomicU64::new(0);
static POINTER_BAND_SEEN_IN_DRAG: AtomicU64 = AtomicU64::new(0);

fn env_flag(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| value == "1")
}

fn probe_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| env_flag("KDT_WIN32_DND_PROBE"))
}

/// Whether the caller asked for the process-wide `EnableMouseInPointer(true)` to be skipped.
///
/// `EnableMouseInPointer` is documented as callable only once per process lifetime, so this decision
/// cannot be revisited later in the run — hence an environment variable read at start-up rather than a
/// runtime toggle.
///
/// On its own this makes the window input-blind, because the wndproc only handles the `WM_POINTER*`
/// band. It is an isolation probe, not a fix; see the investigation doc.
pub fn mouse_in_pointer_disabled_by_env() -> bool {
    static DISABLED: OnceLock<bool> = OnceLock::new();
    *DISABLED.get_or_init(|| env_flag("KDT_WIN32_NO_MOUSE_IN_POINTER"))
}

/// Logs the input state that `DoDragDrop` is documented to depend on.
///
/// That is: whether the process is in mouse-in-pointer mode, whether the left button is down, who
/// holds the capture, and what the thread's input queue reports.
///
/// `GetQueueStatus` is documented to return the messages currently in the queue in the high word and
/// the messages added since the last call in the low word; both are logged raw so no interpretation is
/// baked in here.
pub fn log_input_state(stage: &str) {
    if !probe_enabled() {
        return;
    }
    let mouse_in_pointer = unsafe { IsMouseInPointerEnabled() }.as_bool();
    let lbutton = unsafe { GetKeyState(i32::from(VK_LBUTTON.0)) };
    let capture = unsafe { GetCapture() };
    let queue = unsafe { GetQueueStatus(QS_ALLINPUT) };
    log::info!(
        "DND_PROBE {stage}: thread={} mouseInPointerEnabled={mouse_in_pointer} lButtonState=0x{lbutton:04x} capture={:?} queueStatus=0x{queue:08x}",
        unsafe { GetCurrentThreadId() },
        capture.0
    );
}

/// Marks the start of the `DoDragDrop` call and resets the per-drag message counters.
pub fn drag_entered() {
    MOUSE_BAND_SEEN_IN_DRAG.store(0, Ordering::Relaxed);
    POINTER_BAND_SEEN_IN_DRAG.store(0, Ordering::Relaxed);
    DRAG_IN_FLIGHT.store(true, Ordering::Relaxed);
    log_input_state("before DoDragDrop");
}

/// Marks the end of the `DoDragDrop` call and reports how much input the window saw while it ran.
///
/// This is the decisive counter for the pointer-versus-mouse question: `DoDragDrop` is documented to
/// consume the legacy mouse band and to exit on `WM_LBUTTONUP`, so a drag that ends with
/// `mouseBandDuringDrag=0` never received the input the API says it waits for.
pub fn drag_returned(result: HRESULT, effect: u32) {
    DRAG_IN_FLIGHT.store(false, Ordering::Relaxed);
    if !probe_enabled() {
        return;
    }
    log::info!(
        "DND_PROBE after DoDragDrop: hresult=0x{:08x} effect={effect} mouseBandDuringDrag={} pointerBandDuringDrag={}",
        result.0.cast_unsigned(),
        MOUSE_BAND_SEEN_IN_DRAG.load(Ordering::Relaxed),
        POINTER_BAND_SEEN_IN_DRAG.load(Ordering::Relaxed)
    );
    log_input_state("after DoDragDrop");
}

/// Counts and logs the input messages the window receives while `DoDragDrop` is on the stack.
///
/// `DoDragDrop` runs its own modal loop, so during a healthy drag the messages it retrieves are still
/// dispatched to this wndproc. That makes this the cheapest way to see, locally, which band actually
/// arrives during a wedge.
pub fn note_window_message(msg: u32) {
    if !DRAG_IN_FLIGHT.load(Ordering::Relaxed) {
        return;
    }
    let mouse = WM_MOUSE_BAND.contains(&msg);
    let pointer = WM_POINTER_BAND.contains(&msg);
    if mouse {
        MOUSE_BAND_SEEN_IN_DRAG.fetch_add(1, Ordering::Relaxed);
    }
    if pointer {
        POINTER_BAND_SEEN_IN_DRAG.fetch_add(1, Ordering::Relaxed);
    }
    if probe_enabled() && (mouse || pointer) {
        let band = if mouse { "mouse" } else { "pointer" };
        log::info!("DND_PROBE in-drag message: msg=0x{msg:04x} band={band}");
    }
}

type ConvertPrimaryPointerToMouseDragFn = unsafe extern "system" fn() -> BOOL;

/// Resolves `ConvertPrimaryPointerToMouseDrag` at run time.
///
/// The documentation lists this function with no header and no import library, exported from
/// `User32.dll` at ordinal 2811, so it cannot be linked against and is not in the `windows` crate. Both
/// resolution routes are tried and the one that worked is logged, because which of them is available is
/// exactly the kind of thing the docs do not promise.
fn convert_fn() -> Option<(ConvertPrimaryPointerToMouseDragFn, &'static str)> {
    static RESOLVED: OnceLock<Option<(ConvertPrimaryPointerToMouseDragFn, &'static str)>> = OnceLock::new();
    *RESOLVED.get_or_init(|| {
        let user32 = unsafe { GetModuleHandleW(w!("user32.dll")) }
            .inspect_err(|err| log::warn!("DND_PROBE could not get a handle to user32.dll: {err}"))
            .ok()?;
        if let Some(addr) = unsafe { GetProcAddress(user32, s!("ConvertPrimaryPointerToMouseDrag")) } {
            // SAFETY: the export is documented as `BOOL ConvertPrimaryPointerToMouseDrag()`, which is
            // what `ConvertPrimaryPointerToMouseDragFn` describes.
            return Some((
                unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, ConvertPrimaryPointerToMouseDragFn>(addr) },
                "name",
            ));
        }
        // Ordinal lookup uses the MAKEINTRESOURCE convention: the ordinal goes in the low word of the
        // name pointer.
        let ordinal = PCSTR(2811_usize as *const u8);
        if let Some(addr) = unsafe { GetProcAddress(user32, ordinal) } {
            // SAFETY: as above.
            return Some((
                unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, ConvertPrimaryPointerToMouseDragFn>(addr) },
                "ordinal 2811",
            ));
        }
        log::warn!("DND_PROBE ConvertPrimaryPointerToMouseDrag is not exported by this user32.dll (expected on pre-Windows 11 builds)");
        None
    })
}

/// Promotes the in-flight primary pointer contact to the legacy mouse stream, if asked to.
///
/// The documented contract: call it after `WM_POINTERDOWN` and before `WM_POINTERUP` for that pointer
/// id, with exactly one primary contact active, immediately before `DoDragDrop`. The documentation
/// states the reason it exists — apps that handle pointer messages do not receive mouse messages, which
/// "can cause problems when calling APIs that expect mouse input, such as `DoDragDrop`".
///
/// Whether it repairs *our* wedge is unproven; that is the experiment.
pub fn convert_primary_pointer_to_mouse_drag() {
    if !env_flag("KDT_WIN32_DND_CONVERT_POINTER_TO_MOUSE") {
        return;
    }
    let Some((convert, route)) = convert_fn() else {
        return;
    };
    let promoted = unsafe { convert() };
    log::info!(
        "DND_PROBE ConvertPrimaryPointerToMouseDrag (resolved by {route}) returned {}",
        promoted.as_bool()
    );
}
