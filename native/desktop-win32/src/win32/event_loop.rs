use desktop_common::ffi_utils::RustAllocatedStrPtr;

use windows::Win32::{
    Foundation::{LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::{
        Dwm::DwmDefWindowProc,
        Gdi::{BeginPaint, EndPaint, PAINTSTRUCT},
    },
    UI::{
        HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi},
        Input::Pointer::EnableMouseInPointer,
        WindowsAndMessaging::{
            DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessagePos, GetMessageTime, GetMessageW, GetWindowRect,
            HTCAPTION, HTCLIENT, HTTOP, MINMAXINFO, MSG, NCCALCSIZE_PARAMS, SIZE_MAXIMIZED, SIZE_MINIMIZED, SIZE_RESTORED,
            SM_CXPADDEDBORDER, SM_CYSIZE, SM_CYSIZEFRAME, SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
            TranslateMessage, USER_DEFAULT_SCREEN_DPI, WM_ACTIVATE, WM_CHAR, WM_CLOSE, WM_CREATE, WM_DEADCHAR, WM_DPICHANGED,
            WM_GETMINMAXINFO, WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS, WM_NCCALCSIZE, WM_NCHITTEST, WM_NCMOUSELEAVE, WM_PAINT, WM_POINTERDOWN,
            WM_POINTERHWHEEL, WM_POINTERLEAVE, WM_POINTERUP, WM_POINTERUPDATE, WM_POINTERWHEEL, WM_SETFOCUS, WM_SIZE, WM_SYSCHAR,
            WM_SYSDEADCHAR, WM_SYSKEYDOWN, WM_SYSKEYUP,
        },
    },
};

use super::{
    events::{
        CharacterReceivedEvent, Event, EventHandler, KeyEvent, NCCalcSizeEvent, NCHitTestEvent, PointerButtonEvent, PointerEnteredEvent,
        PointerExitedEvent, PointerUpdatedEvent, ScrollWheelEvent, Timestamp, WindowDrawEvent, WindowResizeEvent, WindowResizeKind,
        WindowScaleChangedEvent,
    },
    geometry::{PhysicalPoint, PhysicalSize},
    keyboard::{PhysicalKeyStatus, VirtualKey},
    pointer::{PointerButtons, PointerInfo},
    strings::copy_from_wide_string,
    utils::{GET_WHEEL_DELTA_WPARAM, GET_X_LPARAM, GET_Y_LPARAM, HIWORD, LOWORD},
    window::Window,
};

pub struct EventLoop {
    event_handler: EventHandler,
}

impl EventLoop {
    pub fn new(event_handler: EventHandler) -> windows::core::Result<Self> {
        unsafe { EnableMouseInPointer(true)? };
        Ok(Self { event_handler })
    }

    #[allow(clippy::unused_self)]
    pub fn run(&self) {
        let mut msg = MSG::default();
        unsafe {
            while GetMessageW(&raw mut msg, None, 0, 0).as_bool() {
                DispatchMessageW(&raw const msg);
            }
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    #[inline]
    fn handle_event(&self, window: &Window, event: Event) -> Option<LRESULT> {
        (self.event_handler)(window.id(), &event).then_some(LRESULT(0))
    }

    pub(crate) fn window_proc(&self, window: &Window, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let handled = match msg {
            WM_CREATE => on_create(window),

            WM_PAINT => on_paint(self, window),

            WM_DPICHANGED => on_dpichanged(self, window, wparam, lparam),

            WM_SIZE => on_size(self, window, wparam, lparam),

            WM_GETMINMAXINFO => on_getminmaxinfo(window, lparam),

            WM_KEYDOWN | WM_SYSKEYDOWN => on_keydown(self, window, msg, wparam, lparam),

            WM_KEYUP | WM_SYSKEYUP => on_keyup(self, window, msg, wparam, lparam),

            WM_SETFOCUS => self.handle_event(window, Event::WindowKeyboardEnter),

            WM_KILLFOCUS => self.handle_event(window, Event::WindowKeyboardLeave),

            WM_CHAR | WM_DEADCHAR | WM_SYSCHAR | WM_SYSDEADCHAR => on_char(self, window, msg, wparam, lparam),

            WM_POINTERUPDATE => on_pointerupdate(self, window, wparam),

            WM_POINTERDOWN => on_pointerdown(self, window, wparam),

            WM_POINTERUP => on_pointerup(self, window, wparam),

            WM_POINTERWHEEL | WM_POINTERHWHEEL => on_pointerwheel(self, window, msg, wparam),

            WM_POINTERLEAVE => on_pointerleave(self, window, wparam),

            WM_ACTIVATE => on_activate(window),

            WM_NCCALCSIZE => on_nccalcsize(self, window, wparam, lparam),

            WM_NCHITTEST => on_nchittest(self, window, wparam, lparam),

            // we still have to handle this message because we manually hit-test the non-client area
            // see https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/nf-dwmapi-dwmdefwindowproc
            WM_NCMOUSELEAVE => on_ncmouseleave(window, wparam, lparam),

            WM_CLOSE => on_close(self, window),

            _ => None,
        };

        match handled {
            Some(result) => result,
            None => unsafe { DefWindowProcW(window.hwnd(), msg, wparam, lparam) },
        }
    }
}

#[allow(clippy::cast_sign_loss)]
#[inline]
fn get_message_timestamp() -> Timestamp {
    Timestamp(unsafe { GetMessageTime() } as u64 * 1000)
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

fn on_close(event_loop: &EventLoop, window: &Window) -> Option<LRESULT> {
    let result = event_loop.handle_event(window, Event::WindowCloseRequest);
    let _ = unsafe { DestroyWindow(window.hwnd()) };
    result
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
    let handled = event_loop.handle_event(window, event.into());
    let _ = unsafe { EndPaint(hwnd, &raw const paint) };
    handled
}

#[allow(clippy::cast_lossless)]
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_precision_loss)]
fn on_dpichanged(event_loop: &EventLoop, window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    let dpi = HIWORD!(wparam.0);
    assert_eq!(
        dpi,
        LOWORD!(wparam.0),
        "The DPI values of the X-axis and the Y-axis should be identical for Windows apps."
    );
    let scale = (dpi as f32) / (USER_DEFAULT_SCREEN_DPI as f32);
    let rect = unsafe { *(lparam.0 as *const RECT) };
    let event = WindowScaleChangedEvent {
        origin: PhysicalPoint::new(rect.left, rect.top),
        size: PhysicalSize::new(rect.right - rect.left, rect.bottom - rect.top),
        scale,
    };
    event_loop.handle_event(window, event.into())
}

#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_sign_loss)]
fn on_size(event_loop: &EventLoop, window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    let width = LOWORD!(lparam.0);
    let height = HIWORD!(lparam.0);
    let kind = match wparam.0 as u32 {
        SIZE_MAXIMIZED => WindowResizeKind::Maximized,
        SIZE_MINIMIZED => WindowResizeKind::Minimized,
        SIZE_RESTORED => WindowResizeKind::Restored,
        kind => WindowResizeKind::Other(kind),
    };
    let event = WindowResizeEvent {
        size: PhysicalSize::new(width.into(), height.into()),
        scale: window.get_scale(),
        kind,
    };
    event_loop.handle_event(window, event.into())
}

fn on_getminmaxinfo(window: &Window, lparam: LPARAM) -> Option<LRESULT> {
    if let Some(min_max_info) = unsafe { (lparam.0 as *mut MINMAXINFO).as_mut() }
        && let Some(min_size) = window.get_min_size()
    {
        let scale = window.get_scale();
        let physical_size = min_size.to_physical(scale);
        min_max_info.ptMinTrackSize.x = physical_size.width.0;
        min_max_info.ptMinTrackSize.y = physical_size.height.0;
        return Some(LRESULT(0));
    }
    None
}

#[allow(clippy::unnecessary_wraps)]
fn on_activate(window: &Window) -> Option<LRESULT> {
    let _ = window
        .extend_content_into_titlebar()
        .inspect_err(|err| log::error!("failed to extend content into the title bar: {err}"));
    let _ = window
        .apply_system_backdrop()
        .inspect_err(|err| log::error!("failed to apply the requested system backdrop: {err}"));
    Some(LRESULT(0))
}

fn on_nccalcsize(event_loop: &EventLoop, window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if wparam.0 == windows::Win32::Foundation::TRUE.0 as usize
        && let Some(calcsize_params) = unsafe { (lparam.0 as *mut NCCALCSIZE_PARAMS).as_mut() }
    {
        let top = calcsize_params.rgrc[0].top;
        let result = unsafe { DefWindowProcW(window.hwnd(), WM_NCCALCSIZE, wparam, lparam) };
        if window.has_custom_title_bar() && result.0 == 0 {
            // the top inset should be 0 otherwise Windows will draw full native title bar
            calcsize_params.rgrc[0].top = top;
        }
        let origin = PhysicalPoint::new(calcsize_params.rgrc[0].left, calcsize_params.rgrc[0].top);
        let size = PhysicalSize::new(
            calcsize_params.rgrc[0].right - calcsize_params.rgrc[0].left,
            calcsize_params.rgrc[0].bottom - calcsize_params.rgrc[0].top,
        );
        let scale = window.get_scale();
        let event = NCCalcSizeEvent { origin, size, scale };
        event_loop.handle_event(window, event.into());
        return Some(result);
    }
    None
}

#[allow(clippy::cast_lossless)]
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_sign_loss)]
fn on_nchittest(event_loop: &EventLoop, window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if !window.has_custom_title_bar() || !window.is_resizable() {
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
    if original_ht != LRESULT(HTCLIENT as _) {
        return Some(original_ht);
    }
    let mouse_x = GET_X_LPARAM!(lparam.0);
    let mouse_y = GET_Y_LPARAM!(lparam.0);
    let event = NCHitTestEvent { mouse_x, mouse_y };
    let handled = event_loop.handle_event(window, event.into());
    if handled.is_some() {
        return Some(LRESULT(HTCLIENT as _));
    }
    let mut window_rect = RECT::default();
    let _ = unsafe { GetWindowRect(hwnd, &raw mut window_rect) };
    let current_dpi = unsafe { GetDpiForWindow(hwnd) };
    let resize_handle_height = unsafe {
        let current_dpi = GetDpiForWindow(hwnd);
        GetSystemMetricsForDpi(SM_CXPADDEDBORDER, current_dpi) + GetSystemMetricsForDpi(SM_CYSIZEFRAME, current_dpi)
    };
    let title_bar_height = resize_handle_height + unsafe { GetSystemMetricsForDpi(SM_CYSIZE, current_dpi) };
    let is_on_resize_border = mouse_y < (window_rect.top + resize_handle_height) as _;
    let is_within_title_bar = mouse_y < (window_rect.top + title_bar_height) as _;
    let hit_test_result = if is_on_resize_border {
        HTTOP
    } else if is_within_title_bar {
        HTCAPTION
    } else {
        HTCLIENT
    };
    Some(LRESULT(hit_test_result as _))
}

fn on_ncmouseleave(window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    let mut dwm_result = LRESULT(0);
    if unsafe { DwmDefWindowProc(window.hwnd(), WM_NCMOUSELEAVE, wparam, lparam, &raw mut dwm_result) }.as_bool() {
        Some(dwm_result)
    } else {
        None
    }
}

#[allow(clippy::cast_lossless)]
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_sign_loss)]
fn on_keydown(event_loop: &EventLoop, window: &Window, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    let vk_code = LOWORD!(wparam.0);
    let timestamp = unsafe { GetMessageTime() };
    let event = KeyEvent {
        key_code: VirtualKey(vk_code),
        key_status: PhysicalKeyStatus::parse(lparam),
        is_system_key: matches!(msg, WM_SYSKEYDOWN),
        timestamp: Timestamp(timestamp as _),
    };
    let result = event_loop.handle_event(window, Event::KeyDown(event));
    if result.is_none() {
        let pos = unsafe { GetMessagePos() };
        let msg = MSG {
            hwnd: window.hwnd(),
            message: msg,
            wParam: wparam,
            lParam: lparam,
            time: timestamp as _,
            pt: POINT {
                x: GET_X_LPARAM!(pos),
                y: GET_Y_LPARAM!(pos),
            },
        };
        let _ = unsafe { TranslateMessage(&raw const msg) };
    }
    result
}

#[allow(clippy::cast_possible_truncation)]
fn on_keyup(event_loop: &EventLoop, window: &Window, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    let vk_code = LOWORD!(wparam.0);
    let event = KeyEvent {
        key_code: VirtualKey(vk_code),
        key_status: PhysicalKeyStatus::parse(lparam),
        is_system_key: matches!(msg, WM_SYSKEYUP),
        timestamp: get_message_timestamp(),
    };
    event_loop.handle_event(window, Event::KeyUp(event))
}

#[allow(clippy::cast_possible_truncation)]
fn on_char(event_loop: &EventLoop, window: &Window, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    let char = LOWORD!(wparam.0);
    let characters = match copy_from_wide_string(&[char]) {
        Ok(chars) => chars,
        Err(err) => {
            log::error!("Failed to get a C-string from the char {char}: {err:?}");
            return Some(LRESULT(1));
        }
    };
    let event = CharacterReceivedEvent {
        key_code: char,
        characters: RustAllocatedStrPtr::from_c_string(characters),
        key_status: PhysicalKeyStatus::parse(lparam),
        is_dead_char: matches!(msg, WM_DEADCHAR | WM_SYSDEADCHAR),
        is_system_key: matches!(msg, WM_SYSCHAR | WM_SYSDEADCHAR),
    };
    event_loop.handle_event(window, event.into())
}

#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::double_parens)]
fn on_pointerupdate(event_loop: &EventLoop, window: &Window, wparam: WPARAM) -> Option<LRESULT> {
    let pointer_info = PointerInfo::try_from_message(wparam).ok()?;
    let event: Event = if window.is_pointer_in_client() {
        PointerUpdatedEvent {
            location_in_window: pointer_info.get_location_in_window(),
            state: pointer_info.get_pointer_state(),
            timestamp: pointer_info.get_timestamp(),
        }
        .into()
    } else {
        // see https://devblogs.microsoft.com/oldnewthing/20031013-00/?p=42193
        window.set_is_pointer_in_client(true);
        PointerEnteredEvent {
            location_in_window: pointer_info.get_location_in_window(),
            state: pointer_info.get_pointer_state(),
            timestamp: pointer_info.get_timestamp(),
        }
        .into()
    };
    event_loop.handle_event(window, event)
}

fn on_pointerdown(event_loop: &EventLoop, window: &Window, wparam: WPARAM) -> Option<LRESULT> {
    let event = PointerInfo::try_from_message(wparam)
        .map(|pointer_info| PointerButtonEvent {
            button: PointerButtons::from_message_flags(wparam),
            location_in_window: pointer_info.get_location_in_window(),
            state: pointer_info.get_pointer_state(),
            timestamp: pointer_info.get_timestamp(),
        })
        .inspect_err(|err| log::error!("failed to create a PointerButtonEvent from WPARAM: {err}"))
        .ok()?;
    event_loop.handle_event(window, Event::PointerDown(event))
}

fn on_pointerup(event_loop: &EventLoop, window: &Window, wparam: WPARAM) -> Option<LRESULT> {
    let event = PointerInfo::try_from_message(wparam)
        .map(|pointer_info| PointerButtonEvent {
            button: PointerButtons::from_message_flags(wparam),
            location_in_window: pointer_info.get_location_in_window(),
            state: pointer_info.get_pointer_state(),
            timestamp: pointer_info.get_timestamp(),
        })
        .inspect_err(|err| log::error!("failed to create a PointerButtonEvent from WPARAM: {err}"))
        .ok()?;
    event_loop.handle_event(window, Event::PointerUp(event))
}

#[allow(clippy::cast_lossless)]
#[allow(clippy::cast_possible_truncation)]
fn on_pointerwheel(event_loop: &EventLoop, window: &Window, msg: u32, wparam: WPARAM) -> Option<LRESULT> {
    let pointer_info = PointerInfo::try_from_message(wparam).ok()?;
    let scrolling_delta = GET_WHEEL_DELTA_WPARAM!(wparam);
    let location_in_window = pointer_info.get_location_in_window();
    let event_args = ScrollWheelEvent {
        scrolling_delta,
        location_in_window,
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
    window.set_is_pointer_in_client(false);
    let event = PointerExitedEvent {
        timestamp: pointer_info.get_timestamp(),
    };
    event_loop.handle_event(window, event.into())
}
