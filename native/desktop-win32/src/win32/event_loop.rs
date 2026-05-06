use std::{cell::RefCell, collections::HashMap};

use desktop_common::ffi_utils::RustAllocatedStrPtr;

use anyhow::Context;
use windows::Win32::{
    Foundation::{LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::{
        Dwm::DwmDefWindowProc,
        Gdi::{BeginPaint, EndPaint, GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow, PAINTSTRUCT},
    },
    UI::{
        HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow, GetSystemMetricsForDpi, SetThreadDpiAwarenessContext},
        Input::Pointer::EnableMouseInPointer,
        Shell::{ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_GETAUTOHIDEBAREX, ABM_GETSTATE, ABS_AUTOHIDE, APPBARDATA, SHAppBarMessage},
        WindowsAndMessaging::{
            AdjustWindowRectEx, DefWindowProcW, DispatchMessageW, GWL_EXSTYLE, GWL_STYLE, GetClientRect, GetMessagePos, GetMessageTime,
            GetMessageW, GetWindowLongPtrW, GetWindowRect, HTCAPTION, HTCLIENT, HTTOP, MINMAXINFO, MSG, NCCALCSIZE_PARAMS,
            SM_CXPADDEDBORDER, SM_CYSIZE, SM_CYSIZEFRAME, SPI_SETHIGHCONTRAST, SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
            SetWindowPos, USER_DEFAULT_SCREEN_DPI, WA_INACTIVE, WINDOW_EX_STYLE, WINDOW_STYLE, WINDOWPOS, WM_ACTIVATE, WM_APP, WM_CHAR,
            WM_CLOSE, WM_CREATE, WM_DEADCHAR, WM_DPICHANGED, WM_ERASEBKGND, WM_GETMINMAXINFO, WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS,
            WM_NCCALCSIZE, WM_NCHITTEST, WM_NCMOUSELEAVE, WM_NCPOINTERDOWN, WM_NCPOINTERUP, WM_NCPOINTERUPDATE, WM_PAINT,
            WM_POINTERCAPTURECHANGED, WM_POINTERDOWN, WM_POINTERHWHEEL, WM_POINTERLEAVE, WM_POINTERUP, WM_POINTERUPDATE, WM_POINTERWHEEL,
            WM_SETCURSOR, WM_SETFOCUS, WM_SETTEXT, WM_SETTINGCHANGE, WM_SYSCHAR, WM_SYSCOLORCHANGE, WM_SYSDEADCHAR, WM_SYSKEYDOWN,
            WM_SYSKEYUP, WM_WINDOWPOSCHANGED,
        },
    },
};

use super::{
    appearance::{Appearance, HighContrast},
    caption_buttons::{CaptionButtonAction, PointerDeviceKind, caption_kind_at_screen, hittest_for_caption_button_kind},
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

pub(crate) const WM_APP_CAPTION_BUTTONS_RENDERING_DEVICE_REPLACED: u32 = WM_APP + 0x31;

thread_local! {
    static KEYEVENT_MESSAGES: RefCell<HashMap<u64, MSG>> = RefCell::new(HashMap::new());
    static LAST_KEYEVENT_MESSAGE_ID: RefCell<u64> = const { RefCell::new(0) };
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

            WM_POINTERUPDATE | WM_NCPOINTERUPDATE => on_pointerupdate(self, window, msg, wparam),

            WM_POINTERDOWN | WM_NCPOINTERDOWN => on_pointerdown(self, window, msg, wparam),

            WM_POINTERUP | WM_NCPOINTERUP => on_pointerup(self, window, msg, wparam),

            WM_POINTERWHEEL | WM_POINTERHWHEEL => on_pointerwheel(self, window, msg, wparam),

            WM_POINTERLEAVE => on_pointerleave(self, window, wparam),

            WM_POINTERCAPTURECHANGED => on_pointercapturechanged(window, wparam),

            WM_APP_CAPTION_BUTTONS_RENDERING_DEVICE_REPLACED => on_caption_buttons_rdr(window),

            WM_ACTIVATE => on_activate(self, window, wparam),

            WM_NCCALCSIZE => on_nccalcsize(self, window, wparam, lparam),

            WM_NCHITTEST => on_nchittest(self, window, wparam, lparam),

            // we still have to handle this message because we manually hit-test the non-client area
            // see https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/nf-dwmapi-dwmdefwindowproc
            WM_NCMOUSELEAVE => on_ncmouseleave(window, wparam, lparam),

            WM_SETCURSOR => on_setcursor(window, lparam),

            WM_SETTEXT => on_settext(self, window, wparam, lparam),

            WM_SETTINGCHANGE => on_settingchange(self, window, wparam, lparam),

            WM_SYSCOLORCHANGE => on_syscolorchange(self, window),

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
    let scale = f32::from(dpi) / (USER_DEFAULT_SCREEN_DPI as f32);
    let rect = unsafe { *(lparam.0 as *const RECT) };
    let event = WindowScaleChangedEvent {
        origin: PhysicalPoint::new(rect.left, rect.top),
        size: PhysicalSize::new(rect.right - rect.left, rect.bottom - rect.top),
        scale,
    };
    let result = event_loop.handle_event(window, event);
    if let Some(strip) = window.caption_buttons.borrow_mut().as_mut() {
        let _ = strip
            .on_dpi_change(window.get_scale())
            .inspect_err(|err| log::warn!("strip on_dpi_change failed: {err}"));
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
        let event = WindowResizeEvent {
            size: PhysicalSize::new(windowpos.cx, windowpos.cy),
            scale: window.get_scale(),
        };
        event_loop.handle_event(window, event);
    }
    if let Some(strip) = window.caption_buttons.borrow_mut().as_mut() {
        let now_maximized = window.is_maximized();
        let max_chrome_y = window.max_chrome_y();
        if window.has_custom_title_bar() {
            let _ = window
                .set_content_top_offset(max_chrome_y)
                .inspect_err(|err| log::warn!("set_content_top_offset failed: {err}"));
        }
        let _ = strip
            .on_max_state_change(now_maximized)
            .inspect_err(|err| log::warn!("strip on_max_state_change failed: {err}"));
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
                    let hc = HighContrast::get_current()
                        .inspect_err(|err| log::warn!("strip appearance notify: failed to read high-contrast state: {err}"))
                        .unwrap_or(HighContrast::Off);
                    notify_strip_appearance(window, new_appearance, hc);
                }
                Err(err) => log::error!("failed to get current system appearance: {err}"),
            }
        }
    }
    // High-contrast change: WM_SETTINGCHANGE wParam == SPI_SETHIGHCONTRAST.
    if wparam.0 == SPI_SETHIGHCONTRAST.0 as usize {
        match HighContrast::get_current() {
            Ok(new_high_contrast) => {
                let event = SystemHighContrastChangeEvent { new_high_contrast };
                event_loop.handle_event(window, event);
                let appearance = Appearance::get_current()
                    .inspect_err(|err| log::warn!("strip appearance notify: failed to read appearance: {err}"))
                    .unwrap_or(Appearance::Light);
                notify_strip_appearance(window, appearance, new_high_contrast);
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
    let appearance = Appearance::get_current()
        .inspect_err(|err| log::warn!("strip appearance notify: failed to read appearance: {err}"))
        .unwrap_or(Appearance::Light);
    notify_strip_appearance(window, appearance, new_high_contrast);
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
    if let Some(strip) = window.caption_buttons.borrow_mut().as_mut() {
        let _ = strip
            .on_activate(is_active)
            .inspect_err(|err| log::warn!("strip on_activate failed: {err}"));
    }
    let event = WindowActivatedEvent { is_active, is_minimized };
    event_loop.handle_event(window, event)
}

fn notify_strip_appearance(window: &Window, appearance: Appearance, high_contrast: HighContrast) {
    if let Some(strip) = window.caption_buttons.borrow_mut().as_mut() {
        let _ = strip
            .on_appearance_change(appearance, high_contrast)
            .inspect_err(|err| log::warn!("strip on_appearance_change failed: {err}"));
    }
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
        AdjustWindowRectEx(
            &raw mut rc,
            WINDOW_STYLE(GetWindowLongPtrW(hwnd, GWL_STYLE).try_into().unwrap()),
            false,
            WINDOW_EX_STYLE(GetWindowLongPtrW(hwnd, GWL_EXSTYLE).try_into().unwrap()),
        )
    }
    .inspect_err(|err| log::error!("failed to adjust window size: {err}"))
    .ok()?;
    calcsize_params.rgrc[0].left -= rc.left;
    calcsize_params.rgrc[0].right -= rc.right;
    calcsize_params.rgrc[0].bottom -= rc.bottom;
    // For non-system title bars (`Custom` / `None`), leave the top inset at 0 so
    // the title-bar area is part of the client area. Even when `WS_CAPTION` is
    // present, returning 0 for `WM_NCCALCSIZE` removes the standard frame/caption.
    if !window.has_non_system_title_bar() {
        calcsize_params.rgrc[0].top -= rc.top;
    }

    // The off-monitor overhang only exists when `WS_THICKFRAME` is set
    // (spec §3.6); `WindowStyle::to_system` clears it when `!is_resizable`.
    // `max_chrome_y` stays 0 for non-resizable / system-titlebar / non-maximized
    // windows so the strip's `set_strip_position` does not shift buttons
    // down into the title-bar zone.
    let max_chrome_y = window.max_chrome_y();
    if max_chrome_y != 0 {
        // The non-system-titlebar handler leaves the top inset at 0 so the
        // title-bar area stays in the client rect; add the tested maximized
        // top overhang back here. Manual verification showed that including
        // SM_CXPADDEDBORDER over-insets this backend.
        calcsize_params.rgrc[0].top += max_chrome_y;

        // GH#1438 / GH#5209: 2-px claw-back so the cursor can still reveal an
        // auto-hide taskbar for non-system title bars.
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
    let _ = window.resize_backdrop_tint(size);
    // Custom only: the strip's commit publishes the queued offset. None /
    // System have no strip, so queueing here would leak the mutation.
    if window.has_custom_title_bar() {
        let _ = window
            .set_content_top_offset(max_chrome_y)
            .inspect_err(|err| log::warn!("set_content_top_offset failed: {err}"));
    }
    if let Some(strip) = window.caption_buttons.borrow_mut().as_mut() {
        let _ = strip
            .on_resize(size, max_chrome_y)
            .inspect_err(|err| log::warn!("strip on_resize failed: {err}"));
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

fn on_nchittest(event_loop: &EventLoop, window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if !window.has_non_system_title_bar() {
        return None;
    }
    let hwnd = window.hwnd();
    let original_ht = {
        let mut dwm_result = LRESULT(0);
        if unsafe { DwmDefWindowProc(hwnd, WM_NCHITTEST, wparam, lparam, &raw mut dwm_result) }.as_bool() {
            dwm_result
        } else {
            unsafe { DefWindowProcW(hwnd, WM_NCHITTEST, wparam, lparam) }
        }
    };
    let mouse_x = GET_X_LPARAM!(lparam.0);
    let mouse_y = GET_Y_LPARAM!(lparam.0);

    // Consult the caption-button strip first; the screen→client conversion
    // and hit-test math live in `caption_buttons::caption_kind_at_screen`.
    if let Some(kind) = caption_kind_at_screen(window, PhysicalPoint::new(mouse_x, mouse_y)) {
        let is_enabled = window.caption_buttons.borrow().as_ref().is_some_and(|s| s.is_enabled(kind));
        return Some(LRESULT(hittest_for_caption_button_kind(kind, is_enabled) as _));
    }

    if original_ht != LRESULT(HTCLIENT as _) {
        return Some(original_ht);
    }

    let event = NCHitTestEvent { mouse_x, mouse_y };
    let handled = event_loop.handle_event(window, event);
    if handled.is_some() {
        return Some(LRESULT(HTCLIENT as _));
    }
    let mut window_rect = RECT::default();
    let _ = unsafe { GetWindowRect(hwnd, &raw mut window_rect) };
    let current_dpi = unsafe { GetDpiForWindow(hwnd) };
    let resize_handle_height =
        unsafe { GetSystemMetricsForDpi(SM_CXPADDEDBORDER, current_dpi) + GetSystemMetricsForDpi(SM_CYSIZEFRAME, current_dpi) };
    let title_bar_height = resize_handle_height + unsafe { GetSystemMetricsForDpi(SM_CYSIZE, current_dpi) };
    // Spec §3.2: only the resize-border match is gated on `is_resizable`; the
    // title-bar drag region applies to non-resizable custom-titlebar windows
    // too so they remain draggable by their title bar.
    //
    // `WindowTitleBarKind::None` must not expose a synthetic drag band.
    let allow_titlebar_drag = window.has_custom_title_bar();
    let is_on_resize_border = window.is_resizable() && mouse_y < (window_rect.top + resize_handle_height) as _;
    let is_within_title_bar = mouse_y < (window_rect.top + title_bar_height) as _;
    let hit_test_result = if is_on_resize_border {
        HTTOP
    } else if allow_titlebar_drag && is_within_title_bar {
        HTCAPTION
    } else {
        HTCLIENT
    };
    Some(LRESULT(hit_test_result as _))
}

fn on_ncmouseleave(window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if let Some(strip) = window.caption_buttons.borrow_mut().as_mut() {
        let _ = strip
            .on_nc_mouse_leave()
            .inspect_err(|err| log::warn!("strip on_nc_mouse_leave failed: {err}"));
    }
    window.nc_leave_tracking_fired();
    let mut dwm_result = LRESULT(0);
    if unsafe { DwmDefWindowProc(window.hwnd(), WM_NCMOUSELEAVE, wparam, lparam, &raw mut dwm_result) }.as_bool() {
        Some(dwm_result)
    } else {
        None
    }
}

fn on_keyevent(event_loop: &EventLoop, window: &Window, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    let virtual_key = VirtualKey::from(wparam);
    let timestamp = unsafe { GetMessageTime() }.cast_unsigned();
    let pos = unsafe { GetMessagePos() };
    let original_msg_id = LAST_KEYEVENT_MESSAGE_ID.with_borrow_mut(|msg_id| {
        *msg_id = msg_id.wrapping_add(1);
        *msg_id
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

const fn device_kind_for(pointer_info: &PointerInfo) -> PointerDeviceKind {
    match pointer_info {
        PointerInfo::Touch(_) => PointerDeviceKind::Touch,
        PointerInfo::Pen(_) => PointerDeviceKind::Pen,
        PointerInfo::Common(_) => PointerDeviceKind::Mouse,
    }
}

fn on_pointerupdate(event_loop: &EventLoop, window: &Window, msg: u32, wparam: WPARAM) -> Option<LRESULT> {
    let is_non_client = matches!(msg, WM_NCPOINTERUPDATE);
    let pointer_info = PointerInfo::try_from_message(wparam).ok()?;

    if msg == WM_NCPOINTERUPDATE && window.has_custom_title_bar() {
        let kind = caption_kind_at_screen(window, pointer_info.get_physical_location());
        if let Some(strip) = window.caption_buttons.borrow_mut().as_mut() {
            let device = device_kind_for(&pointer_info);
            let _ = window.ensure_nc_leave_tracking();
            let _ = strip.on_pointer_update(kind, pointer_info.pointer_id(), device);
        }
        if kind.is_some() {
            // First-entry parity (spec §3.5): mirror `on_pointerupdate`'s
            // else-branch so Kotlin gets a `PointerEntered` even when the
            // pointer's first appearance is over a caption button.
            if !window.is_pointer_in_window() {
                window.set_is_pointer_in_window(true);
                event_loop.handle_event(
                    window,
                    Event::PointerEntered(PointerEnteredEvent {
                        location_in_window: pointer_info.get_location_in_window(),
                        location_on_screen: pointer_info.get_physical_location(),
                        state: pointer_info.get_pointer_state(),
                        timestamp: pointer_info.get_timestamp(),
                    }),
                );
            }
            // Strip events are private — suppress the Kotlin dispatch.
            return Some(LRESULT(0));
        }
    }

    let event = if window.is_pointer_in_window() {
        let button_change = pointer_info.get_pointer_button_change();
        match button_change.kind() {
            PointerButtonChangeKind::Pressed => {
                let click_location = pointer_info.get_physical_location();
                let click_count = window.with_mut_pointer_click_counter(|c| c.register_click(button_change.button(), click_location));
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
            PointerButtonChangeKind::Released => Event::PointerUp(PointerUpEvent {
                button: button_change.button(),
                location_in_window: pointer_info.get_location_in_window(),
                location_on_screen: pointer_info.get_physical_location(),
                non_client_area: is_non_client,
                state: pointer_info.get_pointer_state(),
                timestamp: pointer_info.get_timestamp(),
            }),
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
    event_loop.handle_event(window, event)
}

fn on_pointerdown(event_loop: &EventLoop, window: &Window, msg: u32, wparam: WPARAM) -> Option<LRESULT> {
    let pointer_info = PointerInfo::try_from_message(wparam).ok()?;

    if msg == WM_NCPOINTERDOWN && window.has_custom_title_bar() {
        let kind = caption_kind_at_screen(window, pointer_info.get_physical_location());
        if let Some(kind) = kind {
            let button_change = pointer_info.get_pointer_button_change();
            let is_primary = button_change.kind() == PointerButtonChangeKind::Pressed && button_change.button() == PointerButton::Left;
            if let Some(strip) = window.caption_buttons.borrow_mut().as_mut() {
                if is_primary {
                    let device = device_kind_for(&pointer_info);
                    let _ = strip.on_pointer_down(kind, pointer_info.pointer_id(), device);
                } else {
                    strip.track_swallowed_press(kind, pointer_info.pointer_id(), button_change.button());
                }
            }
            // Any press over the strip is chrome — consume even if no strip exists.
            return Some(LRESULT(0));
        }
    }

    let pointer_button = match pointer_info.get_pointer_button_change() {
        change if change.kind() == PointerButtonChangeKind::Pressed => change.button(),
        pointer_button_change => {
            log::error!("Unexpected pointer button change on pointer down: {pointer_button_change:?}");
            return None;
        }
    };
    let click_location = pointer_info.get_physical_location();
    let click_count = window.with_mut_pointer_click_counter(|c| c.register_click(pointer_button, click_location));
    let event = PointerDownEvent {
        button: pointer_button,
        click_count,
        location_in_window: pointer_info.get_location_in_window(),
        location_on_screen: pointer_info.get_physical_location(),
        non_client_area: matches!(msg, WM_NCPOINTERDOWN),
        state: pointer_info.get_pointer_state(),
        timestamp: pointer_info.get_timestamp(),
    };
    event_loop.handle_event(window, Event::PointerDown(event))
}

fn on_pointerup(event_loop: &EventLoop, window: &Window, msg: u32, wparam: WPARAM) -> Option<LRESULT> {
    let is_non_client = matches!(msg, WM_NCPOINTERUP);
    let pointer_info = PointerInfo::try_from_message(wparam).ok()?;

    // Claim either WM_POINTERUP or WM_NCPOINTERUP when the strip has an active
    // press: a press that started on a caption button (NC) but released after
    // the cursor crossed into the client area arrives as WM_POINTERUP, and
    // gating on WM_NCPOINTERUP alone would leave `press_session` stuck until
    // WM_POINTERCAPTURECHANGED fires on deactivation.
    if window.has_custom_title_bar() {
        let button_change = pointer_info.get_pointer_button_change();
        let is_primary = button_change.kind() == PointerButtonChangeKind::Released && button_change.button() == PointerButton::Left;
        let strip_owns_press = window
            .caption_buttons
            .borrow()
            .as_ref()
            .is_some_and(|s| s.has_active_press_for(pointer_info.pointer_id()));
        if is_primary && strip_owns_press {
            let kind_under_pointer = caption_kind_at_screen(window, pointer_info.get_physical_location());
            let action = window.caption_buttons.borrow_mut().as_mut().and_then(|strip| {
                strip
                    .on_pointer_up(kind_under_pointer, pointer_info.pointer_id())
                    .inspect_err(|err| log::warn!("strip on_pointer_up failed: {err}"))
                    .ok()
                    .flatten()
            });
            if let Some(action) = action {
                match action {
                    CaptionButtonAction::Close => {
                        let _ = window.request_close();
                    }
                    CaptionButtonAction::Minimize => window.minimize(),
                    CaptionButtonAction::Maximize => window.maximize(),
                    CaptionButtonAction::Restore => window.restore(),
                }
            }
            return Some(LRESULT(0));
        }
        // Drain tracked Suppressed sessions; press-elsewhere cycles fall
        // through symmetrically with their PointerDown.
        if !is_primary && button_change.kind() == PointerButtonChangeKind::Released {
            let consumed = window
                .caption_buttons
                .borrow_mut()
                .as_mut()
                .is_some_and(|s| s.consume_swallowed_release(pointer_info.pointer_id(), button_change.button()));
            if consumed {
                return Some(LRESULT(0));
            }
        }
    }

    let pointer_button = match pointer_info.get_pointer_button_change() {
        change if change.kind() == PointerButtonChangeKind::Released => change.button(),
        pointer_button_change => {
            log::error!("Unexpected pointer button change on pointer up: {pointer_button_change:?}");
            return None;
        }
    };
    let event = PointerUpEvent {
        button: pointer_button,
        location_in_window: pointer_info.get_location_in_window(),
        location_on_screen: pointer_info.get_physical_location(),
        non_client_area: is_non_client,
        state: pointer_info.get_pointer_state(),
        timestamp: pointer_info.get_timestamp(),
    };
    let result = event_loop.handle_event(window, Event::PointerUp(event));
    // Strip-claimed primary releases return early above; other NC releases pass
    // through so DefWindowProc handles them.
    if is_non_client { None } else { result }
}

fn on_pointercapturechanged(window: &Window, wparam: WPARAM) -> Option<LRESULT> {
    if !window.has_custom_title_bar() {
        return None;
    }
    if let Some(strip) = window.caption_buttons.borrow_mut().as_mut() {
        let pointer_id = u32::from(LOWORD!(wparam.0));
        // Cleanup gate matches every owned session — `has_active_press_for`
        // would leak non-primary `Suppressed` sessions on capture loss.
        if strip.has_press_for(pointer_id) {
            let _ = strip.on_pointer_cancel(pointer_id);
            // Cancellation only — must not fire `CaptionButtonAction`.
            return Some(LRESULT(0));
        }
    }
    None
}

#[allow(clippy::unnecessary_wraps)]
fn on_caption_buttons_rdr(window: &Window) -> Option<LRESULT> {
    if let Some(strip) = window.caption_buttons.borrow_mut().as_mut() {
        let _ = strip
            .on_rendering_device_replaced()
            .inspect_err(|err| log::warn!("strip on_rendering_device_replaced failed: {err}"));
    }
    Some(LRESULT(0))
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
