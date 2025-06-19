use desktop_common::ffi_utils::RustAllocatedStrPtr;
use log::{debug, error};
use windows::{
    Foundation::TypedEventHandler,
    System::DispatcherQueueController,
    Win32::{
        Foundation::{LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::{
            Dwm::DwmDefWindowProc,
            Gdi::{BeginPaint, EndPaint, InvalidateRect, PAINTSTRUCT},
        },
        System::WinRT::{CreateDispatcherQueueController, DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT, DispatcherQueueOptions},
        UI::{
            HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi},
            WindowsAndMessaging::{
                DefWindowProcW, DispatchMessageW, GetClientRect, GetMessagePos, GetMessageTime, GetMessageW, GetWindowRect, HTCAPTION,
                HTCLIENT, HTTOP, MINMAXINFO, MSG, NCCALCSIZE_PARAMS, PostQuitMessage, SIZE_MAXIMIZED, SIZE_MINIMIZED, SIZE_RESTORED,
                SM_CXPADDEDBORDER, SM_CYSIZE, SM_CYSIZEFRAME, SWP_FRAMECHANGED, SetWindowPos, TranslateMessage, USER_DEFAULT_SCREEN_DPI,
                WM_ACTIVATE, WM_CHAR, WM_CLOSE, WM_DEADCHAR, WM_DPICHANGED, WM_GETMINMAXINFO, WM_KEYDOWN, WM_KEYUP, WM_NCCALCSIZE,
                WM_NCHITTEST, WM_NCMOUSELEAVE, WM_PAINT, WM_SIZE, WM_SYSCHAR, WM_SYSDEADCHAR, WM_SYSKEYDOWN, WM_SYSKEYUP,
            },
        },
    },
    core::Result as WinResult,
};

use super::{
    events::{
        CharacterReceivedEvent, Event, EventHandler, KeyEvent, NCHitTestEvent, Timestamp, WindowDrawEvent, WindowResizeEvent,
        WindowResizeKind, WindowScaleChangedEvent,
    },
    geometry::{PhysicalPoint, PhysicalSize},
    keyboard::{PhysicalKeyStatus, VirtualKey},
    strings::copy_from_wide_string,
    utils,
    window::{WM_REQUEST_UPDATE, Window},
};

pub struct EventLoop {
    dispatcher_queue_controller: DispatcherQueueController,
    event_handler: EventHandler,
}

impl EventLoop {
    #[allow(clippy::cast_possible_truncation)]
    pub fn new(event_handler: EventHandler) -> WinResult<Self> {
        let dispatcher_queue_controller = unsafe {
            CreateDispatcherQueueController(DispatcherQueueOptions {
                dwSize: size_of::<DispatcherQueueOptions>() as _,
                threadType: DQTYPE_THREAD_CURRENT,
                apartmentType: DQTAT_COM_NONE,
            })?
        };

        // See https://devblogs.microsoft.com/oldnewthing/20240509-52/?p=109738
        dispatcher_queue_controller
            .DispatcherQueue()?
            .ShutdownCompleted(&TypedEventHandler::new(|_, _| {
                debug!("Shutting down the dispatcher queue");
                unsafe { PostQuitMessage(0) };
                Ok(())
            }))?;

        Ok(Self {
            dispatcher_queue_controller,
            event_handler,
        })
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

    pub fn shutdown(&self) -> WinResult<()> {
        self.dispatcher_queue_controller
            .ShutdownQueueAsync()
            .map(|_async| ())
            .inspect_err(|err| error!("Failed to shut down the dispatcher queue: {err:?}"))
    }

    #[allow(clippy::needless_pass_by_value)]
    #[inline]
    fn handle_event(&self, window: &Window, event: Event) -> Option<LRESULT> {
        (self.event_handler)(window.id(), &event).then_some(LRESULT(0))
    }

    pub(crate) fn window_proc(&self, window: &Window, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let hwnd = window.hwnd();

        let handled = match msg {
            WM_REQUEST_UPDATE => {
                let _ = unsafe { InvalidateRect(Some(hwnd), None, false) };
                Some(LRESULT(0))
            }

            WM_PAINT => on_paint(self, window),

            WM_DPICHANGED => on_dpichanged(self, window, wparam, lparam),

            WM_SIZE => on_size(self, window, wparam, lparam),

            WM_GETMINMAXINFO => on_getminmaxinfo(window, lparam),

            WM_KEYDOWN | WM_SYSKEYDOWN => on_keydown(self, window, msg, wparam, lparam),

            WM_KEYUP | WM_SYSKEYUP => on_keyup(self, window, msg, wparam, lparam),

            WM_CHAR | WM_DEADCHAR | WM_SYSCHAR | WM_SYSDEADCHAR => on_char(self, window, msg, wparam, lparam),

            WM_ACTIVATE => on_activate(window),

            WM_NCCALCSIZE => on_nccalcsize(window, wparam, lparam),

            WM_NCHITTEST => on_nchittest(self, window, wparam, lparam),

            WM_NCMOUSELEAVE => on_ncmouseleave(window, wparam, lparam),

            WM_CLOSE => self.handle_event(window, Event::WindowCloseRequest),

            _ => None,
        };

        match handled {
            Some(result) => result,
            None => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }
}

fn on_paint(event_loop: &EventLoop, window: &Window) -> Option<LRESULT> {
    let hwnd = window.hwnd();
    let mut paint = PAINTSTRUCT::default();
    unsafe { BeginPaint(hwnd, &raw mut paint) };
    let mut rect = RECT::default();
    if let Err(err) = unsafe { GetClientRect(hwnd, &raw mut rect) } {
        error!("Failed to get client rect: {err:?}");
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
    let new_dpi = utils::HIWORD!(wparam.0);
    assert_eq!(
        new_dpi,
        utils::LOWORD!(wparam.0),
        "The DPI values of the X-axis and the Y-axis should be identical for Windows apps."
    );
    let new_scale = (new_dpi as f32) / (USER_DEFAULT_SCREEN_DPI as f32);
    let new_rect = unsafe { *(lparam.0 as *const RECT) };
    let event = WindowScaleChangedEvent {
        new_origin: PhysicalPoint::new(new_rect.left, new_rect.top),
        new_size: PhysicalSize::new(new_rect.right - new_rect.left, new_rect.bottom - new_rect.top),
        new_scale,
    };
    event_loop.handle_event(window, event.into())
}

#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_sign_loss)]
fn on_size(event_loop: &EventLoop, window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    let width = utils::LOWORD!(lparam.0);
    let height = utils::HIWORD!(lparam.0);
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
    if let Some(min_max_info) = unsafe { (lparam.0 as *mut MINMAXINFO).as_mut() } {
        if let Some(min_size) = window.get_min_size() {
            let scale = window.get_scale();
            let physical_size = min_size.to_physical(scale);
            min_max_info.ptMinTrackSize.x = physical_size.width.0;
            min_max_info.ptMinTrackSize.y = physical_size.height.0;
            return Some(LRESULT(0));
        }
    }
    None
}

#[allow(clippy::unnecessary_wraps)]
fn on_activate(window: &Window) -> Option<LRESULT> {
    let hwnd = window.hwnd();
    let _ = window.extend_content_into_titlebar().and_then(|()| window.apply_system_backdrop());
    let mut rect = RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd, &raw mut rect).and_then(|()| {
            SetWindowPos(
                hwnd,
                None,
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_FRAMECHANGED,
            )
        });
    }
    Some(LRESULT(0))
}

fn on_nccalcsize(window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if window.has_custom_title_bar() && wparam.0 == windows::Win32::Foundation::TRUE.0 as usize {
        if let Some(calcsize_params) = unsafe { (lparam.0 as *mut NCCALCSIZE_PARAMS).as_mut() } {
            let top = calcsize_params.rgrc[0].top;
            unsafe { DefWindowProcW(window.hwnd(), WM_NCCALCSIZE, wparam, lparam) };
            // the top inset should be 0 otherwise Windows will draw full native title bar
            calcsize_params.rgrc[0].top = top;
            return Some(LRESULT(0));
        }
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
    let mouse_x = utils::GET_X_LPARAM!(lparam.0);
    let mouse_y = utils::GET_Y_LPARAM!(lparam.0);
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
    let vk_code = utils::LOWORD!(wparam.0);
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
                x: utils::GET_X_LPARAM!(pos),
                y: utils::GET_Y_LPARAM!(pos),
            },
        };
        let _ = unsafe { TranslateMessage(&raw const msg) };
    }
    result
}

#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_sign_loss)]
fn on_keyup(event_loop: &EventLoop, window: &Window, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    let vk_code = utils::LOWORD!(wparam.0);
    let event = KeyEvent {
        key_code: VirtualKey(vk_code),
        key_status: PhysicalKeyStatus::parse(lparam),
        is_system_key: matches!(msg, WM_SYSKEYUP),
        timestamp: Timestamp(unsafe { GetMessageTime() } as _),
    };
    event_loop.handle_event(window, Event::KeyUp(event))
}

#[allow(clippy::cast_possible_truncation)]
fn on_char(event_loop: &EventLoop, window: &Window, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    let char = wparam.0 as u16;
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
