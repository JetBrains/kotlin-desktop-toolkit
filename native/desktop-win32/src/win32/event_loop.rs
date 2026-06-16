use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};

use desktop_common::ffi_utils::RustAllocatedStrPtr;

use super::{
    appearance::{Appearance, HighContrast},
    events::{
        CharacterReceivedEvent, Event, EventHandler, KeyEvent, NCCalcSizeEvent, NCHitTestEvent, PointerDownEvent, PointerEnteredEvent,
        PointerExitedEvent, PointerUpEvent, PointerUpdatedEvent, ScrollWheelEvent, SystemAppearanceChangeEvent,
        SystemHighContrastChangeEvent, Timestamp, WindowActivatedEvent, WindowDrawEvent, WindowMoveEvent, WindowResizeEvent,
        WindowScaleChangedEvent, WindowTitleChangedEvent,
    },
    geometry::{PhysicalPoint, PhysicalSize},
    keyboard::{PhysicalKeyStatus, VirtualKey},
    pointer::{PointerButton, PointerButtonChangeKind, PointerClickCounter, PointerInfo},
    strings::copy_from_wide_string,
    utils::{GET_WHEEL_DELTA_WPARAM, GET_X_LPARAM, GET_Y_LPARAM, HIWORD, LOWORD},
    window::Window,
};
use anyhow::Context;
use windows::Win32::UI::HiDpi::AdjustWindowRectExForDpi;
use windows::Win32::{
    Foundation::{LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::{
        Dwm::DwmDefWindowProc,
        Gdi::{BeginPaint, EndPaint, GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow, PAINTSTRUCT},
    },
    UI::{
        HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext},
        Input::{
            KeyboardAndMouse::{GetCapture, ReleaseCapture, SetCapture},
            Pointer::EnableMouseInPointer,
        },
        Shell::{ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_GETAUTOHIDEBAREX, ABM_GETSTATE, ABS_AUTOHIDE, APPBARDATA, SHAppBarMessage},
        WindowsAndMessaging::{
            DefWindowProcW, DispatchMessageW, GWL_EXSTYLE, GWL_STYLE, GetClientRect, GetMessagePos, GetMessageTime, GetMessageW,
            GetWindowLongPtrW, GetWindowRect, HMENU, HTCAPTION, HTCLIENT, HTCLOSE, HTMAXBUTTON, HTMINBUTTON, HTTOP, MINMAXINFO, MSG,
            NCCALCSIZE_PARAMS, SC_KEYMENU, SPI_SETHIGHCONTRAST, SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
            USER_DEFAULT_SCREEN_DPI, WA_INACTIVE, WINDOW_EX_STYLE, WINDOW_STYLE, WINDOWPOS, WM_ACTIVATE, WM_CANCELMODE, WM_CAPTURECHANGED,
            WM_CHAR, WM_CLOSE, WM_CREATE, WM_DEADCHAR, WM_DPICHANGED, WM_ERASEBKGND, WM_GETMINMAXINFO, WM_INITMENUPOPUP, WM_KEYDOWN,
            WM_KEYUP, WM_KILLFOCUS, WM_NCCALCSIZE, WM_NCHITTEST, WM_NCLBUTTONDOWN, WM_NCMOUSELEAVE, WM_NCPOINTERDOWN, WM_NCPOINTERUP,
            WM_NCPOINTERUPDATE, WM_NCRBUTTONUP, WM_PAINT, WM_POINTERCAPTURECHANGED, WM_POINTERDOWN, WM_POINTERHWHEEL, WM_POINTERLEAVE,
            WM_POINTERUP, WM_POINTERUPDATE, WM_POINTERWHEEL, WM_SETCURSOR, WM_SETFOCUS, WM_SETTEXT, WM_SETTINGCHANGE, WM_SYSCHAR,
            WM_SYSCOLORCHANGE, WM_SYSCOMMAND, WM_SYSDEADCHAR, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_WINDOWPOSCHANGED,
        },
    },
};

thread_local! {
    static KEYEVENT_MESSAGES: RefCell<HashMap<u64, MSG>> = RefCell::new(HashMap::new());
    static LAST_KEYEVENT_MESSAGE_ID: Cell<u64> = const { Cell::new(0) };
    static NCHITTEST_RESULTS: RefCell<HashMap<u64, Option<i32>>> = RefCell::new(HashMap::new());
    static LAST_NCHITTEST_MESSAGE_ID: Cell<u64> = const { Cell::new(0) };
}

pub struct EventLoop {
    event_handler: EventHandler,
}

impl EventLoop {
    pub fn new(event_handler: EventHandler) -> windows_core::Result<Self> {
        unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        unsafe { EnableMouseInPointer(true)? };
        Ok(Self { event_handler })
    }

    #[allow(clippy::unused_self)]
    pub fn run(&self) -> anyhow::Result<()> {
        log::trace!("Event loop is starting");
        let mut msg = MSG::default();
        loop {
            match unsafe { GetMessageW(&raw mut msg, None, 0, 0).0 } {
                -1 => anyhow::bail!("Event loop has exited with an error: {}", windows_core::Error::from_thread()),
                0 => break,
                _ => unsafe { DispatchMessageW(&raw const msg) },
            };
        }
        log::trace!("Event loop has finished");
        Ok(())
    }

    pub fn with_keyevent_message<F, R>(msg_id: u64, f: F) -> anyhow::Result<R>
    where
        F: FnOnce(&MSG) -> anyhow::Result<R>,
    {
        KEYEVENT_MESSAGES.with_borrow(|map| map.get(&msg_id).context("unknown message id").and_then(f))
    }

    /// Records the explicit hit-test result an `NCHitTest` handler requested. The slot is
    /// registered by `on_nchittest` for the duration of the handler call, so an unknown
    /// `msg_id` means the call happened outside the active hit-test.
    pub fn set_nchittest_result(msg_id: u64, result: i32) -> anyhow::Result<()> {
        NCHITTEST_RESULTS.with_borrow_mut(|map| {
            let slot = map.get_mut(&msg_id).context("unknown nc hit-test message id")?;
            *slot = Some(result);
            Ok(())
        })
    }

    #[inline]
    fn handle_event<T: Into<Event>>(&self, window: &Window, event: T) -> Option<LRESULT> {
        (self.event_handler)(window.id(), &event.into()).then_some(LRESULT(0))
    }

    pub(crate) fn window_proc(&self, window: &Window, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let handled = match msg {
            WM_CREATE => on_create(window),

            WM_PAINT => on_paint(self, window),

            WM_ERASEBKGND => Some(LRESULT(1)),

            WM_DPICHANGED => on_dpichanged(self, window, wparam, lparam),

            WM_WINDOWPOSCHANGED => on_windowposchanged(self, window, lparam),

            WM_GETMINMAXINFO => on_getminmaxinfo(window, lparam),

            WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP => on_keyevent(self, window, msg, wparam, lparam),

            WM_SETFOCUS => self.handle_event(window, Event::WindowKeyboardEnter),

            WM_KILLFOCUS => self.handle_event(window, Event::WindowKeyboardLeave),

            WM_CHAR | WM_DEADCHAR | WM_SYSCHAR | WM_SYSDEADCHAR => on_char(self, window, msg, wparam, lparam),

            // Pointer events start
            WM_POINTERUPDATE | WM_NCPOINTERUPDATE => on_pointerupdate(self, window, msg, wparam),

            WM_POINTERDOWN | WM_NCPOINTERDOWN => on_pointerdown(self, window, msg, wparam),

            WM_POINTERUP | WM_NCPOINTERUP => on_pointerup(self, window, msg, wparam),

            WM_POINTERWHEEL | WM_POINTERHWHEEL => on_pointerwheel(self, window, msg, wparam),

            WM_POINTERLEAVE => on_pointerleave(self, window, wparam),

            WM_POINTERCAPTURECHANGED => on_pointercapturechanged(window, wparam),
            // Pointer event end
            WM_CANCELMODE => on_cancelmode(window),

            WM_CAPTURECHANGED => on_capturechanged(window),

            WM_ACTIVATE => on_activate(self, window, wparam),

            WM_NCCALCSIZE => on_nccalcsize(self, window, wparam, lparam),

            WM_NCHITTEST => on_nchittest(self, window, wparam, lparam),

            // we still have to handle this message because we manually hit-test the non-client area
            // see https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/nf-dwmapi-dwmdefwindowproc
            WM_NCMOUSELEAVE => on_ncmouseleave(self, window, wparam, lparam),

            // NC
            WM_NCLBUTTONDOWN => on_nclbuttondown(window, wparam, lparam),

            WM_NCRBUTTONUP => on_ncrbuttonup(window, wparam, lparam),
            // NC
            WM_SETCURSOR => on_setcursor(window, lparam),

            WM_SETTEXT => on_settext(self, window, wparam, lparam),

            WM_SETTINGCHANGE => on_settingchange(self, window, wparam, lparam),

            WM_SYSCOLORCHANGE => on_syscolorchange(self, window),

            WM_SYSCOMMAND => on_syscommand(window, wparam, lparam),

            WM_INITMENUPOPUP => on_initmenupopup(window, wparam),

            WM_CLOSE => self.handle_event(window, Event::WindowCloseRequest),

            _ => None,
        };

        match handled {
            Some(result) => result,
            None => unsafe { DefWindowProcW(window.hwnd(), msg, wparam, lparam) },
        }
    }
}

#[allow(clippy::unnecessary_wraps)]
fn on_create(window: &Window) -> Option<LRESULT> {
    let _ = unsafe {
        SetWindowPos(
            window.hwnd(),
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
        )
    };
    Some(LRESULT(0))
}

fn on_paint(event_loop: &EventLoop, window: &Window) -> Option<LRESULT> {
    let hwnd = window.hwnd();
    let mut paint = PAINTSTRUCT::default();
    unsafe { BeginPaint(hwnd, &raw mut paint) };
    let mut rect = RECT::default();
    if let Err(err) = unsafe { GetClientRect(hwnd, &raw mut rect) } {
        log::error!("Failed to get client rect: {err:?}");
        return Some(LRESULT(1));
    }
    let event = WindowDrawEvent {
        size: PhysicalSize::new(rect.right - rect.left, rect.bottom - rect.top),
        scale: window.get_scale(),
    };
    let handled = event_loop.handle_event(window, event);
    let _ = unsafe { EndPaint(hwnd, &raw const paint) };
    handled
}

#[allow(clippy::cast_precision_loss)]
fn on_dpichanged(event_loop: &EventLoop, window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    let dpi = HIWORD!(wparam.0);
    assert_eq!(
        dpi,
        LOWORD!(wparam.0),
        "The DPI values of the X-axis and the Y-axis should be identical for Windows apps."
    );
    // Refresh DPI cache from wparam before any downstream handler reads it.
    window.set_dpi_metrics(u32::from(dpi));
    let scale = f32::from(dpi) / (USER_DEFAULT_SCREEN_DPI as f32);
    let rect = unsafe { *(lparam.0 as *const RECT) };
    let event = WindowScaleChangedEvent {
        origin: PhysicalPoint::new(rect.left, rect.top),
        size: PhysicalSize::new(rect.right - rect.left, rect.bottom - rect.top),
        scale,
    };
    let result = event_loop.handle_event(window, event);
    // `max_chrome_y` is DPI-scaled; update content offset and strip on the same tick.
    let has_custom_tb = window.has_custom_title_bar();
    if has_custom_tb {
        let _ = window
            .set_content_top_offset(window.max_chrome_y())
            .inspect_err(|err| log::warn!("set_content_top_offset on DPI change failed: {err}"));
    }
    result
}

fn on_windowposchanged(event_loop: &EventLoop, window: &Window, lparam: LPARAM) -> Option<LRESULT> {
    let windowpos = unsafe { (lparam.0 as *mut WINDOWPOS).as_ref() }?;
    if windowpos.flags.0 & SWP_NOMOVE.0 == 0 {
        let event = WindowMoveEvent {
            origin: PhysicalPoint::new(windowpos.x, windowpos.y),
            scale: window.get_scale(),
        };
        event_loop.handle_event(window, event);
    }
    if windowpos.flags.0 & SWP_NOSIZE.0 == 0 {
        // `windowpos.cx/cy` is the outer rect; use GetClientRect for the
        // client-area size that ANGLE / Kotlin drawing actually consume.
        let mut client_rect = RECT::default();
        if unsafe { GetClientRect(window.hwnd(), &raw mut client_rect) }.is_ok() {
            let event = WindowResizeEvent {
                size: PhysicalSize::new(client_rect.right - client_rect.left, client_rect.bottom - client_rect.top),
                scale: window.get_scale(),
            };
            event_loop.handle_event(window, event);
        } else {
            log::warn!("GetClientRect during WM_WINDOWPOSCHANGED failed; skipping WindowResizeEvent for this tick");
        }
    }
    Some(LRESULT(0))
}

fn on_getminmaxinfo(window: &Window, lparam: LPARAM) -> Option<LRESULT> {
    let min_max_info = unsafe { (lparam.0 as *mut MINMAXINFO).as_mut() }?;
    let min_size = window.get_min_size()?;
    let scale = window.get_scale();
    let physical_size = min_size.to_physical(scale);
    min_max_info.ptMinTrackSize.x = physical_size.width.0;
    min_max_info.ptMinTrackSize.y = physical_size.height.0;
    Some(LRESULT(0))
}

fn on_setcursor(window: &Window, lparam: LPARAM) -> Option<LRESULT> {
    let hit_test = LOWORD!(lparam.0);
    if u32::from(hit_test) == HTCLIENT {
        window
            .refresh_cursor()
            .inspect_err(|err| log::error!("failed to set cursor: {err}"))
            .ok()
            .map(|()| LRESULT(windows::Win32::Foundation::TRUE.0 as isize))
    } else {
        None
    }
}

#[allow(clippy::unnecessary_wraps)]
fn on_settext(event_loop: &EventLoop, window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    let result = unsafe { DefWindowProcW(window.hwnd(), WM_SETTEXT, wparam, lparam) };
    if result.0 == windows::Win32::Foundation::TRUE.0 as isize {
        let pwstr = windows_core::PWSTR(lparam.0 as *mut u16);
        let title = match copy_from_wide_string(unsafe { pwstr.as_wide() }) {
            Ok(text) => RustAllocatedStrPtr::from_c_string(text).to_auto_drop(),
            Err(err) => {
                log::error!("Failed to get a C-string from the slice {}: {:?}", unsafe { pwstr.display() }, err);
                return Some(result);
            }
        };
        let event = WindowTitleChangedEvent { title };
        event_loop.handle_event(window, event);
    }
    Some(result)
}

fn on_settingchange(event_loop: &EventLoop, window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    // Light/dark theme change: WM_SETTINGCHANGE wParam=0, lParam="ImmersiveColorSet".
    // borrowed from https://github.com/microsoft/terminal/blob/73948072120beb51d355b3c74b5f774a6526b277/src/cascadia/WindowsTerminal/IslandWindow.cpp#L748
    if wparam.0 == 0 && lparam.0 != 0 {
        let param = unsafe { windows_core::PWSTR(lparam.0 as *mut u16).to_hstring() };
        if &param == windows_core::h!("ImmersiveColorSet") {
            match Appearance::get_current() {
                Ok(new_appearance) => {
                    let event = SystemAppearanceChangeEvent { new_appearance };
                    event_loop.handle_event(window, event);
                }
                Err(err) => log::error!("failed to get current system appearance: {err}"),
            }
        }
    }
    if wparam.0 == SPI_SETHIGHCONTRAST.0 as usize {
        match HighContrast::get_current() {
            Ok(new_high_contrast) => {
                let event = SystemHighContrastChangeEvent { new_high_contrast };
                event_loop.handle_event(window, event);
            }
            Err(err) => log::error!("failed to get high-contrast state: {err}"),
        }
    }
    None
}

fn on_syscolorchange(event_loop: &EventLoop, window: &Window) -> Option<LRESULT> {
    // Per the High contrast parameter doc, WM_SYSCOLORCHANGE is the prescribed
    // signal to re-determine the high-contrast state. Both this and the
    // WM_SETTINGCHANGE/SPI_SETHIGHCONTRAST signal in on_settingchange may fire
    // on a single toggle; the toolkit fires the event idempotently and apps
    // treat it as a current-state snapshot.
    let new_high_contrast = HighContrast::get_current()
        .inspect_err(|err| log::error!("failed to get high-contrast state on WM_SYSCOLORCHANGE: {err}"))
        .ok()?;
    let event = SystemHighContrastChangeEvent { new_high_contrast };
    event_loop.handle_event(window, event);
    None
}

fn on_activate(event_loop: &EventLoop, window: &Window, wparam: WPARAM) -> Option<LRESULT> {
    let is_active = u32::from(LOWORD!(wparam.0)) != WA_INACTIVE;
    let is_minimized = HIWORD!(wparam.0) != 0;
    window.with_mut_pointer_click_counter(PointerClickCounter::reset);
    if is_active && !is_minimized {
        let _ = window
            .extend_content_into_titlebar()
            .inspect_err(|err| log::error!("failed to extend content into the title bar: {err}"));
        let _ = window
            .apply_system_backdrop()
            .inspect_err(|err| log::error!("failed to apply the requested system backdrop: {err}"));
    }
    let event = WindowActivatedEvent { is_active, is_minimized };
    event_loop.handle_event(window, event)
}

fn on_nccalcsize(event_loop: &EventLoop, window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    // Documentation (https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-nccalcsize):
    //   "If wParam is FALSE, the application does not need to indicate the valid part of the client area."
    if wparam.0 != windows::Win32::Foundation::TRUE.0 as usize {
        return None;
    }
    let calcsize_params = unsafe { (lparam.0 as *mut NCCALCSIZE_PARAMS).as_mut() }?;
    let hwnd = window.hwnd();
    let mut rc = RECT::default();
    // https://devblogs.microsoft.com/oldnewthing/20131017-00/?p=2903
    unsafe {
        AdjustWindowRectExForDpi(
            &raw mut rc,
            WINDOW_STYLE(GetWindowLongPtrW(hwnd, GWL_STYLE).try_into().unwrap()),
            false,
            WINDOW_EX_STYLE(GetWindowLongPtrW(hwnd, GWL_EXSTYLE).try_into().unwrap()),
            window.dpi_metrics().dpi,
        )
    }
    .inspect_err(|err| log::error!("failed to adjust window size: {err}"))
    .ok()?;
    calcsize_params.rgrc[0].left -= rc.left;
    calcsize_params.rgrc[0].right -= rc.right;
    calcsize_params.rgrc[0].bottom -= rc.bottom;
    // For a custom title bar, leave the top inset at 0 so the title-bar area is
    // part of the client area. Even when `WS_CAPTION` is present, returning 0 for
    // `WM_NCCALCSIZE` removes the standard frame/caption.
    if !window.has_custom_title_bar() {
        calcsize_params.rgrc[0].top -= rc.top;
    }

    // The off-monitor overhang only exists when `WS_THICKFRAME` is set;
    // `WindowStyle::to_system` clears it when `!is_resizable`.
    // `max_chrome_y` stays 0 for non-resizable / system-titlebar / non-maximized
    // windows so the strip's `set_strip_position` does not shift buttons
    // down into the title-bar zone.
    let max_chrome_y = window.max_chrome_y();
    if max_chrome_y != 0 {
        // Add the maximized off-monitor overhang back so the client top sits
        // at the visible monitor edge. SM_CYSIZEFRAME only — see spec §3.6
        // for the SM_CXPADDEDBORDER divergence from Windows Terminal.
        calcsize_params.rgrc[0].top += max_chrome_y;

        // GH#1438 / GH#5209: 2-px claw-back so the cursor can still reveal an
        // auto-hide taskbar for a custom title bar.
        let _ = apply_autohide_taskbar_inset(hwnd, &mut calcsize_params.rgrc[0])
            .inspect_err(|err| log::warn!("autohide taskbar inset failed: {err}"));
    }

    let origin = PhysicalPoint::new(calcsize_params.rgrc[0].left, calcsize_params.rgrc[0].top);
    let size = PhysicalSize::new(
        calcsize_params.rgrc[0].right - calcsize_params.rgrc[0].left,
        calcsize_params.rgrc[0].bottom - calcsize_params.rgrc[0].top,
    );
    let scale = window.get_scale();
    let event = NCCalcSizeEvent { origin, size, scale };
    event_loop.handle_event(window, event);
    if window.has_custom_title_bar() {
        let _ = window
            .set_content_top_offset(max_chrome_y)
            .inspect_err(|err| log::warn!("set_content_top_offset failed: {err}"));
    }
    Some(LRESULT(0))
}

/// Per Windows Terminal `_OnNcCalcSize` GH#1438 / GH#5209: when an auto-hide
/// taskbar lives on an edge of the window's monitor, reduce the maximized
/// client rect by 2 px on that edge so the cursor can reach the screen edge
/// to trigger the taskbar reveal.
#[allow(clippy::cast_possible_truncation)]
fn apply_autohide_taskbar_inset(hwnd: windows::Win32::Foundation::HWND, rect: &mut RECT) -> anyhow::Result<()> {
    const AUTOHIDE_TASKBAR_SIZE: i32 = 2;

    let mut autohide = APPBARDATA {
        cbSize: size_of::<APPBARDATA>() as u32,
        ..Default::default()
    };
    let state = unsafe { SHAppBarMessage(ABM_GETSTATE, &raw mut autohide) } as u32;
    if state & ABS_AUTOHIDE == 0 {
        return Ok(());
    }

    let hmon = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if hmon.is_invalid() {
        anyhow::bail!("MonitorFromWindow returned invalid HMONITOR for HWND {hwnd:?}");
    }
    let mut mon_info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(hmon, &raw mut mon_info) }.as_bool() {
        anyhow::bail!("GetMonitorInfoW failed for HMONITOR {hmon:?}");
    }

    let has_autohide = |edge: u32| -> bool {
        let mut data = APPBARDATA {
            cbSize: size_of::<APPBARDATA>() as u32,
            uEdge: edge,
            rc: mon_info.rcMonitor,
            ..Default::default()
        };
        let h = unsafe { SHAppBarMessage(ABM_GETAUTOHIDEBAREX, &raw mut data) };
        h != 0
    };
    if has_autohide(ABE_TOP) {
        rect.top += AUTOHIDE_TASKBAR_SIZE;
    }
    if has_autohide(ABE_BOTTOM) {
        rect.bottom -= AUTOHIDE_TASKBAR_SIZE;
    }
    if has_autohide(ABE_LEFT) {
        rect.left += AUTOHIDE_TASKBAR_SIZE;
    }
    if has_autohide(ABE_RIGHT) {
        rect.right -= AUTOHIDE_TASKBAR_SIZE;
    }
    Ok(())
}

/// Dispatch an [`NCHitTestEvent`] to userspace and return the hit-test result it
/// decided, if any. Userspace can answer either explicitly (by recording a hit-test
/// code keyed on `original_msg_id`) or implicitly (by merely consuming the event, in
/// which case the area is treated as `HTCLIENT`). Returns `None` when userspace leaves
/// the hit test to the default handling.
fn query_userspace_hit_test(event_loop: &EventLoop, window: &Window, mouse_x: i32, mouse_y: i32) -> Option<i32> {
    let original_msg_id = LAST_NCHITTEST_MESSAGE_ID.with(|c| {
        c.update(|v| v.wrapping_add(1));
        c.get()
    });
    NCHITTEST_RESULTS.with_borrow_mut(|map| map.insert(original_msg_id, None));
    let event = NCHitTestEvent {
        mouse_x,
        mouse_y,
        original_msg_id,
    };
    if let Some(_result) = event_loop.handle_event(window, event) {
        // The stored code is `i32` on purpose: `HTTRANSPARENT` is `-1` and must
        // reach `LRESULT` as `-1`, not as a large positive `u32`.
        NCHITTEST_RESULTS.with_borrow_mut(|map| map.remove(&original_msg_id)).flatten()
    } else {
        None
    }
}

#[allow(clippy::unnecessary_wraps)]
fn on_nchittest(event_loop: &EventLoop, window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    let hwnd = window.hwnd();
    let original_ht = {
        let mut dwm_result = LRESULT(0);
        if unsafe { DwmDefWindowProc(hwnd, WM_NCHITTEST, wparam, lparam, &raw mut dwm_result) }.as_bool() {
            dwm_result
        } else {
            unsafe { DefWindowProcW(hwnd, WM_NCHITTEST, wparam, lparam) }
        }
    };

    if original_ht != LRESULT(HTCLIENT as _) {
        return Some(original_ht);
    }

    // TODO [pavel.sergeev] Check what coordinate system is used here
    let mouse_x = GET_X_LPARAM!(lparam.0);
    let mouse_y = GET_Y_LPARAM!(lparam.0);

    let mut window_rect = RECT::default();
    let _ = unsafe { GetWindowRect(hwnd, &raw mut window_rect) };
    let m = window.dpi_metrics();
    let resize_handle_height = m.padded_border + m.size_frame;
    let is_on_resize_border = window.is_resizable() && mouse_y < (window_rect.top + resize_handle_height) as _;
    if is_on_resize_border {
        return Some(LRESULT(HTTOP as _));
    }

    if let Some(ht) = query_userspace_hit_test(event_loop, window, mouse_x, mouse_y) {
        return Some(LRESULT(ht as _));
    }

    Some(LRESULT(HTCLIENT as _))
}

fn on_ncmouseleave(event_loop: &EventLoop, window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    event_loop.handle_event(window, Event::NCPointerLeave);
    let mut dwm_result = LRESULT(0);
    if unsafe { DwmDefWindowProc(window.hwnd(), WM_NCMOUSELEAVE, wparam, lparam, &raw mut dwm_result) }.as_bool() {
        Some(dwm_result)
    } else {
        None
    }
}

/// Bypass `DefWindowProc`'s NC modal click loop, which wedges under Win11
/// Snap Layouts + `EnableMouseInPointer` until the window deactivates. The
/// consumed-press path (`on_pointerdown` → `SetCapture`, release drained in
/// `on_pointerup` as the capture-redirected client variant) handles the common
/// case; this arm is the fallback for presses userspace left unhandled.
fn on_nclbuttondown(window: &Window, wparam: WPARAM, _lparam: LPARAM) -> Option<LRESULT> {
    if !window.has_custom_title_bar() {
        return None;
    }

    // Mouse capture for *consumed* presses is taken in `on_pointerdown` (the pointer-input
    // path). This legacy `WM_NCLBUTTONDOWN` arm only runs when userspace left the press
    // unhandled, where there is no userspace press to deliver a release to — we just stop
    // `DefWindowProc`'s NC button loop from wedging.

    // If we are here it means that user space code haven't handled the `PointerDownEvent`
    // We want to prevent blocking of the event loop in `DefWindowProc` so we handle it here for caption buttons
    #[allow(clippy::cast_possible_truncation)]
    let ht = wparam.0 as u32;
    if matches!(ht, HTMINBUTTON | HTMAXBUTTON | HTCLOSE) {
        Some(LRESULT(0))
    } else {
        None
    }
}

/// Title-bar right-click → system menu.
fn on_ncrbuttonup(window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if !window.has_custom_title_bar() {
        return None;
    }
    #[allow(clippy::cast_possible_truncation)]
    let ht = wparam.0 as u32;
    if ht != HTCAPTION {
        return None;
    }
    let pt = PhysicalPoint::new(GET_X_LPARAM!(lparam.0), GET_Y_LPARAM!(lparam.0));
    match window.show_system_menu(pt) {
        Ok(()) => Some(LRESULT(0)),
        Err(err) => {
            log::warn!("show_system_menu (WM_NCRBUTTONUP) failed: {err}");
            None
        }
    }
}

/// Alt+Space → system menu, via `SC_KEYMENU` synthesised by `DefWindowProc`.
/// Other `SC_*` codes fall through.
fn on_syscommand(window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if !window.has_custom_title_bar() {
        return None;
    }
    #[allow(clippy::cast_possible_truncation)]
    let cmd = (wparam.0 & 0xFFF0) as u32;
    if cmd != SC_KEYMENU || lparam.0 != ' ' as isize {
        return None;
    }
    let anchor = alt_space_anchor(window);
    match window.show_system_menu(anchor) {
        Ok(()) => Some(LRESULT(0)),
        Err(err) => {
            log::warn!("show_system_menu (Alt+Space) failed: {err}");
            None
        }
    }
}

/// Apply system-menu enable state during `TrackPopupMenu`'s popup-init phase.
fn on_initmenupopup(window: &Window, wparam: WPARAM) -> Option<LRESULT> {
    let h_menu = HMENU(wparam.0 as *mut _);
    if h_menu.is_invalid() || h_menu != window.system_menu() {
        return None;
    }
    window.sync_system_menu();
    Some(LRESULT(0))
}

/// Alt+Space anchor: top-left of the visible window frame.
fn alt_space_anchor(window: &Window) -> PhysicalPoint {
    let rect = window.get_physical_rect().unwrap_or_else(|err| {
        log::warn!("get_physical_rect failed, falling back to GetWindowRect: {err}");
        let mut r = RECT::default();
        let _ = unsafe { GetWindowRect(window.hwnd(), &raw mut r) };
        r
    });
    PhysicalPoint::new(rect.left, rect.top)
}

fn on_keyevent(event_loop: &EventLoop, window: &Window, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    let virtual_key = VirtualKey::from(wparam);
    let timestamp = unsafe { GetMessageTime() }.cast_unsigned();
    let pos = unsafe { GetMessagePos() };
    let original_msg_id = LAST_KEYEVENT_MESSAGE_ID.with(|c| {
        c.update(|v| v.wrapping_add(1));
        c.get()
    });
    KEYEVENT_MESSAGES.with_borrow_mut(|map| {
        map.insert(
            original_msg_id,
            MSG {
                hwnd: window.hwnd(),
                message: msg,
                wParam: wparam,
                lParam: lparam,
                time: timestamp,
                pt: POINT {
                    x: GET_X_LPARAM!(pos),
                    y: GET_Y_LPARAM!(pos),
                },
            },
        )
    });
    let key_event = KeyEvent {
        is_system_key: matches!(msg, WM_SYSKEYDOWN | WM_SYSKEYUP),
        key_status: PhysicalKeyStatus::from(lparam),
        virtual_key,
        timestamp: Timestamp::from_millis(timestamp.into()),
        original_msg_id,
    };
    let event = match msg {
        WM_KEYDOWN | WM_SYSKEYDOWN => Event::KeyDown(key_event),
        WM_KEYUP | WM_SYSKEYUP => Event::KeyUp(key_event),
        _ => unreachable!("unknown key event"),
    };
    let result = event_loop.handle_event(window, event);
    KEYEVENT_MESSAGES.with_borrow_mut(|map| map.remove(&original_msg_id));
    result
}

fn on_char(event_loop: &EventLoop, window: &Window, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    let character = LOWORD!(wparam.0);
    let event = CharacterReceivedEvent {
        character,
        key_status: PhysicalKeyStatus::from(lparam),
        is_dead_char: matches!(msg, WM_DEADCHAR | WM_SYSDEADCHAR),
        is_system_key: matches!(msg, WM_SYSCHAR | WM_SYSDEADCHAR),
    };
    event_loop.handle_event(window, event)
}

/// Take an explicit mouse capture when userspace *consumes* a non-client press, so the rest of
/// the gesture — moves and the release — is redirected back to this window as the client
/// `WM_POINTER*` variants even when the pointer is dragged outside its bounds.
///
/// Non-client pointer input is not implicitly captured the way client input is, and we
/// deliberately bypass `DefWindowProc`'s NC modal loop (which would otherwise hold the capture)
/// to avoid wedging under Snap Layouts + `EnableMouseInPointer`. Without this, the matching
/// `WM_NCPOINTERUP` is hit-tested to whatever window is under the cursor on release, so we never
/// see `PointerUp` and e.g. a custom caption button sticks.
///
/// No-op for client presses (already implicitly captured by the OS) and for unconsumed presses
/// (where `DefWindowProc` — e.g. the window-move loop — manages capture itself). Released in
/// [`release_self_captured_pointer`].
fn capture_consumed_nc_press(window: &Window, non_client_area: bool, consumed: bool) {
    if non_client_area && consumed {
        unsafe {
            SetCapture(window.hwnd());
        }
        window.set_self_captured_pointer(true);
    }
}

/// Release the capture taken by [`capture_consumed_nc_press`], but only if we still own it.
/// `DefWindowProc`'s window-move/resize modal loop also `SetCapture`s to this hwnd; releasing
/// *that* would cancel the drag and snap the window back, so we guard on our own ownership flag.
fn release_self_captured_pointer(window: &Window) {
    if window.take_self_captured_pointer() && unsafe { GetCapture() } == window.hwnd() {
        let _ = unsafe { ReleaseCapture() };
    }
}

// Called when additional pressed some pointer button, or mouse was moved
fn on_pointerupdate(event_loop: &EventLoop, window: &Window, msg: u32, wparam: WPARAM) -> Option<LRESULT> {
    let is_non_client = matches!(msg, WM_NCPOINTERUPDATE);
    let pointer_info = PointerInfo::try_from_message(wparam).ok()?;

    // A button transition while the pointer is already in contact (e.g. pressing a second,
    // chorded button) arrives here as `WM_*POINTERUPDATE`, not `WM_*POINTERDOWN`. Track when
    // this update is such a press so we can take the same capture as `on_pointerdown` does.
    let mut is_press = false;
    let event = if window.is_pointer_in_window() {
        let button_change = pointer_info.get_pointer_button_change();
        match button_change.kind() {
            PointerButtonChangeKind::Pressed => {
                is_press = true;
                let click_location = pointer_info.get_physical_location();
                let click_count = window.with_mut_pointer_click_counter(|c| c.register_click(button_change.button(), click_location));
                window.mark_pointer_button_pressed(button_change.button());
                Event::PointerDown(PointerDownEvent {
                    button: button_change.button(),
                    click_count,
                    location_in_window: pointer_info.get_location_in_window(),
                    location_on_screen: pointer_info.get_physical_location(),
                    non_client_area: is_non_client,
                    state: pointer_info.get_pointer_state(),
                    timestamp: pointer_info.get_timestamp(),
                })
            }
            PointerButtonChangeKind::Released => {
                window.mark_pointer_button_released(button_change.button());
                Event::PointerUp(PointerUpEvent {
                    button: button_change.button(),
                    location_in_window: pointer_info.get_location_in_window(),
                    location_on_screen: pointer_info.get_physical_location(),
                    non_client_area: is_non_client,
                    state: pointer_info.get_pointer_state(),
                    timestamp: pointer_info.get_timestamp(),
                })
            }
            PointerButtonChangeKind::Other => Event::PointerUpdated(PointerUpdatedEvent {
                location_in_window: pointer_info.get_location_in_window(),
                location_on_screen: pointer_info.get_physical_location(),
                non_client_area: is_non_client,
                state: pointer_info.get_pointer_state(),
                timestamp: pointer_info.get_timestamp(),
            }),
        }
    } else {
        // see https://devblogs.microsoft.com/oldnewthing/20031013-00/?p=42193
        window.set_is_pointer_in_window(true);
        Event::PointerEntered(PointerEnteredEvent {
            location_in_window: pointer_info.get_location_in_window(),
            location_on_screen: pointer_info.get_physical_location(),
            state: pointer_info.get_pointer_state(),
            timestamp: pointer_info.get_timestamp(),
        })
    };
    let result = event_loop.handle_event(window, event);
    if is_press {
        // Capture a consumed chorded press here just as `on_pointerdown` does for the first
        // button. We don't release on an intermediate button-up (the `Released` arm): contact —
        // and our capture — ends at the last button up, delivered as `WM_POINTERUP`.
        capture_consumed_nc_press(window, is_non_client, result.is_some());
    }
    // Recover from any `WM_POINTERUP` Windows dropped: deliver a synthetic `PointerUp` for every
    // button we still believe is down but the OS no longer reports as pressed in this message.
    flush_missed_pointer_ups(event_loop, window, &pointer_info, is_non_client);
    result
}

/// Reconcile the buttons we've tracked as pressed against the OS-reported pressed set for the
/// current pointer message, synthesising a `PointerUp` for each button whose release we never saw.
///
/// Windows can drop a `WM_POINTERUP` — most visibly when a press is released over a custom caption
/// button — which would otherwise leave the button stuck "pressed" in userspace until the next real
/// down/up for it. The synthesised event reuses the current message's location/state/timestamp; the
/// missed release happened in the recent past so this is the closest data we have.
fn flush_missed_pointer_ups(event_loop: &EventLoop, window: &Window, pointer_info: &PointerInfo, is_non_client: bool) {
    let state = pointer_info.get_pointer_state();
    let missed = window.pressed_pointer_buttons() & !state.pressed_buttons_mask();
    if missed == 0 {
        return;
    }
    for button in PointerButton::ALL.into_iter().filter(|button| missed & *button as u32 != 0) {
        window.mark_pointer_button_released(button);
        let event = PointerUpEvent {
            button,
            location_in_window: pointer_info.get_location_in_window(),
            location_on_screen: pointer_info.get_physical_location(),
            non_client_area: is_non_client,
            state,
            timestamp: pointer_info.get_timestamp(),
        };
        event_loop.handle_event(window, Event::PointerUp(event));
    }
}

fn on_pointerdown(event_loop: &EventLoop, window: &Window, msg: u32, wparam: WPARAM) -> Option<LRESULT> {
    let pointer_info = PointerInfo::try_from_message(wparam).ok()?;

    let pointer_button = match pointer_info.get_pointer_button_change() {
        change if change.kind() == PointerButtonChangeKind::Pressed => change.button(),
        pointer_button_change => {
            log::error!("Unexpected pointer button change on pointer down: {pointer_button_change:?}");
            return None;
        }
    };
    let click_location = pointer_info.get_physical_location();
    let click_count = window.with_mut_pointer_click_counter(|c| c.register_click(pointer_button, click_location));
    window.mark_pointer_button_pressed(pointer_button);
    let non_client_area = matches!(msg, WM_NCPOINTERDOWN);
    let event = PointerDownEvent {
        button: pointer_button,
        click_count,
        location_in_window: pointer_info.get_location_in_window(),
        location_on_screen: pointer_info.get_physical_location(),
        non_client_area,
        state: pointer_info.get_pointer_state(),
        timestamp: pointer_info.get_timestamp(),
    };
    let result = event_loop.handle_event(window, Event::PointerDown(event));
    capture_consumed_nc_press(window, non_client_area, result.is_some());
    result
}

fn on_pointerup(event_loop: &EventLoop, window: &Window, msg: u32, wparam: WPARAM) -> Option<LRESULT> {
    let is_non_client = matches!(msg, WM_NCPOINTERUP);
    let pointer_info = PointerInfo::try_from_message(wparam).ok()?;

    let pointer_button = match pointer_info.get_pointer_button_change() {
        change if change.kind() == PointerButtonChangeKind::Released => change.button(),
        pointer_button_change => {
            log::error!("Unexpected pointer button change on pointer up: {pointer_button_change:?}");
            return None;
        }
    };
    window.mark_pointer_button_released(pointer_button);
    let event = PointerUpEvent {
        button: pointer_button,
        location_in_window: pointer_info.get_location_in_window(),
        location_on_screen: pointer_info.get_physical_location(),
        non_client_area: is_non_client,
        state: pointer_info.get_pointer_state(),
        timestamp: pointer_info.get_timestamp(),
    };
    // `WM_POINTERUP` is the last-button-up / contact-end message, so this is where any capture we
    // took for the gesture is dropped (the client variant the capture redirected it into).
    release_self_captured_pointer(window);
    event_loop.handle_event(window, Event::PointerUp(event))
}

fn on_pointercapturechanged(_window: &Window, _wparam: WPARAM) -> Option<LRESULT> {
    log::trace!("on_pointercapturechanged");
    None
}

fn on_capturechanged(window: &Window) -> Option<LRESULT> {
    log::trace!("on_capturechanged");
    // We've lost the mouse capture (released by us, or stolen). Clear our ownership flag so
    // `on_pointerup` won't later mistake someone else's capture for ours.
    window.set_self_captured_pointer(false);
    None
}

fn on_cancelmode(_window: &Window) -> Option<LRESULT> {
    // Usually is triggered when dialog is shown
    // We should notify user space to drop tooltips and hovered states
    log::trace!("on_cancelmode");

    // Returning `None` lets `DefWindowProc` complete the standard cancel
    // (releases mouse capture, cancels menu / scrollbar tracking).
    None
}

fn on_pointerwheel(event_loop: &EventLoop, window: &Window, msg: u32, wparam: WPARAM) -> Option<LRESULT> {
    let pointer_info = PointerInfo::try_from_message(wparam).ok()?;
    let event_args = ScrollWheelEvent {
        scrolling_delta: GET_WHEEL_DELTA_WPARAM!(wparam),
        location_in_window: pointer_info.get_location_in_window(),
        location_on_screen: pointer_info.get_physical_location(),
        state: pointer_info.get_pointer_state(),
        timestamp: pointer_info.get_timestamp(),
    };
    let event = match msg {
        WM_POINTERWHEEL => Event::ScrollWheelY(event_args),
        WM_POINTERHWHEEL => Event::ScrollWheelX(event_args),
        _ => unreachable!("Expected WM_POINTERWHEEL or WM_POINTERHWHEEL"),
    };
    event_loop.handle_event(window, event)
}

fn on_pointerleave(event_loop: &EventLoop, window: &Window, wparam: WPARAM) -> Option<LRESULT> {
    let pointer_info = PointerInfo::try_from_message(wparam).ok()?;
    window.set_is_pointer_in_window(false);
    let event = PointerExitedEvent {
        location_in_window: pointer_info.get_location_in_window(),
        location_on_screen: pointer_info.get_physical_location(),
        state: pointer_info.get_pointer_state(),
        timestamp: pointer_info.get_timestamp(),
    };
    event_loop.handle_event(window, event)
}
