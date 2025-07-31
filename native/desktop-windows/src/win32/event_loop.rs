use log::{debug, error};
use windows::{
    Foundation::TypedEventHandler,
    System::DispatcherQueueController,
    Win32::{
        Foundation::{LPARAM, LRESULT, RECT, WPARAM},
        Graphics::{
            Dwm::DwmDefWindowProc,
            Gdi::{BeginPaint, EndPaint, InvalidateRect},
        },
        System::WinRT::{CreateDispatcherQueueController, DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT, DispatcherQueueOptions},
        UI::{
            HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi},
            WindowsAndMessaging::{
                DefWindowProcW, DispatchMessageW, GetClientRect, GetMessageW, GetWindowRect, HTCAPTION, HTCLIENT, HTTOP, MINMAXINFO, MSG,
                NCCALCSIZE_PARAMS, PostQuitMessage, SIZE_MAXIMIZED, SIZE_MINIMIZED, SIZE_RESTORED, SM_CXPADDEDBORDER, SM_CYSIZE,
                SM_CYSIZEFRAME, SWP_FRAMECHANGED, SetWindowPos, USER_DEFAULT_SCREEN_DPI, WM_ACTIVATE, WM_CLOSE, WM_DPICHANGED,
                WM_GETMINMAXINFO, WM_NCCALCSIZE, WM_NCHITTEST, WM_NCMOUSELEAVE, WM_PAINT, WM_SIZE,
            },
        },
    },
    core::Result as WinResult,
};

use super::{
    events::{Event, EventHandler, WindowDrawEvent, WindowResizeEvent, WindowResizeKind, WindowScaleChangedEvent},
    geometry::{PhysicalPoint, PhysicalSize},
    utils,
    window::{WM_REQUEST_UPDATE, Window},
};

pub struct EventLoop {
    dispatcher_queue_controller: DispatcherQueueController,
    event_handler: EventHandler,
}

impl EventLoop {
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

    pub fn run(&self) {
        let mut msg = MSG::default();
        unsafe {
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                //let _ = windows::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    pub fn shutdown(&self) -> WinResult<()> {
        self.dispatcher_queue_controller
            .ShutdownQueueAsync()
            .map(|_async| ())
            .inspect_err(|err| error!("Failed to shut down the dispatcher queue: {:?}", err))
    }

    #[inline]
    fn handle_event(&self, window: &Window, event: Event) -> Option<LRESULT> {
        (self.event_handler)(window.id(), &event).then_some(LRESULT(0))
    }

    pub fn window_proc(&self, window: &Window, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
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

            WM_ACTIVATE => on_activate(window),

            WM_NCCALCSIZE => on_nccalcsize(window, wparam, lparam),

            WM_NCHITTEST => on_nchittest(window, wparam, lparam),

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
    let mut paint = Default::default();
    unsafe { BeginPaint(hwnd, &mut paint) };
    let mut rect = Default::default();
    if let Err(err) = unsafe { GetClientRect(hwnd, &mut rect) } {
        error!("Failed to get client rect: {err:?}");
        return Some(LRESULT(1));
    }
    let event = WindowDrawEvent {
        size: PhysicalSize::new(rect.right - rect.left, rect.bottom - rect.top),
        scale: window.get_scale(),
    };
    let handled = event_loop.handle_event(window, event.into());
    let _ = unsafe { EndPaint(hwnd, &paint) };
    handled
}

fn on_dpichanged(event_loop: &EventLoop, window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    let new_dpi = utils::HIWORD!(wparam);
    assert_eq!(
        new_dpi,
        utils::LOWORD!(wparam),
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

fn on_size(event_loop: &EventLoop, window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    let width = utils::LOWORD!(lparam);
    let height = utils::HIWORD!(lparam);
    let kind = match wparam.0 as u32 {
        SIZE_MAXIMIZED => WindowResizeKind::Maximized,
        SIZE_MINIMIZED => WindowResizeKind::Minimized,
        SIZE_RESTORED => WindowResizeKind::Restored,
        kind => WindowResizeKind::Other(kind),
    };
    let event = WindowResizeEvent {
        size: PhysicalSize::new(width as _, height as _),
        scale: window.get_scale(),
        kind,
    };
    event_loop.handle_event(window, event.into())
}

fn on_getminmaxinfo(window: &Window, lparam: LPARAM) -> Option<LRESULT> {
    if let Some(min_max_info) = unsafe { (lparam.0 as *mut MINMAXINFO).as_mut() } {
        if let Some(min_size) = window.get_min_size() {
            let scale = window.get_scale();
            min_max_info.ptMinTrackSize.x = f32::round(min_size.width.0 * scale + 0.5_f32) as i32;
            min_max_info.ptMinTrackSize.y = f32::round(min_size.height.0 * scale + 0.5_f32) as i32;
            return Some(LRESULT(0));
        }
    }
    None
}

fn on_activate(window: &Window) -> Option<LRESULT> {
    let hwnd = window.hwnd();
    let _ = window.extend_content_into_titlebar().and_then(|_| window.apply_system_backdrop());
    let mut rect = RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd, &mut rect).and_then(|_| {
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

fn on_nchittest(window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if !window.has_custom_title_bar() || !window.is_resizable() {
        return None;
    }
    let hwnd = window.hwnd();
    let original_ht = {
        let mut dwm_result = LRESULT(0);
        unsafe { DwmDefWindowProc(hwnd, WM_NCHITTEST, wparam, lparam, &mut dwm_result) }
            .as_bool()
            .then(|| dwm_result)
            .unwrap_or_else(|| unsafe { DefWindowProcW(hwnd, WM_NCHITTEST, wparam, lparam) })
    };
    if original_ht != LRESULT(HTCLIENT as _) {
        return Some(original_ht);
    }
    let mouse_y = utils::GET_Y_LPARAM!(lparam);
    let mut window_rect = RECT::default();
    let _ = unsafe { GetWindowRect(hwnd, &mut window_rect) };
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
    unsafe { DwmDefWindowProc(window.hwnd(), WM_NCMOUSELEAVE, wparam, lparam, &mut dwm_result) }
        .as_bool()
        .then(|| dwm_result)
}
