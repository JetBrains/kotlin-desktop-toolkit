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
    utils::WLParamUtil,
    window::{WM_REQUEST_UPDATE, Window},
    window_api::WindowId,
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
    fn handle_event(&self, window_id: WindowId, event: &Event) -> Option<LRESULT> {
        (self.event_handler)(window_id, &event).then_some(LRESULT(0))
    }

    pub fn window_proc(&self, window: &Window, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let hwnd = window.hwnd();

        let handled = match msg {
            WM_PAINT => {
                let mut paint = Default::default();
                unsafe { BeginPaint(hwnd, &mut paint) };
                let mut rect = Default::default();
                if let Err(err) = unsafe { GetClientRect(hwnd, &mut rect) } {
                    error!("Failed to get client rect: {err:?}");
                    return LRESULT(1);
                }
                let event = WindowDrawEvent {
                    size: PhysicalSize::new(rect.right - rect.left, rect.bottom - rect.top),
                    scale: window.get_scale(),
                };
                let handled = self.handle_event(hwnd.into(), &event.into());
                let _ = unsafe { EndPaint(hwnd, &paint) };
                handled
            }

            WM_REQUEST_UPDATE => unsafe { InvalidateRect(Some(hwnd), None, false).as_bool() }.then_some(LRESULT(0)),

            WM_DPICHANGED => {
                let new_dpi = wparam.HIWORD();
                assert_eq!(
                    new_dpi,
                    wparam.LOWORD(),
                    "The DPI values of the X-axis and the Y-axis should be identical for Windows apps."
                );
                let new_scale = (new_dpi as f32) / (USER_DEFAULT_SCREEN_DPI as f32);
                let new_rect = unsafe { *(lparam.0 as *const RECT) };
                let event = WindowScaleChangedEvent {
                    new_origin: PhysicalPoint::new(new_rect.left, new_rect.top),
                    new_size: PhysicalSize::new(new_rect.right - new_rect.left, new_rect.bottom - new_rect.top),
                    new_scale,
                };
                self.handle_event(hwnd.into(), &event.into())
            }

            WM_SIZE => {
                let width = lparam.LOWORD();
                let height = lparam.HIWORD();
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
                self.handle_event(hwnd.into(), &event.into())
            }

            WM_GETMINMAXINFO => 'wm_getminmaxinfo: {
                if let Some(min_max_info) = unsafe { (lparam.0 as *mut MINMAXINFO).as_mut() } {
                    if let Some(min_size) = window.get_min_size() {
                        let scale = window.get_scale();
                        min_max_info.ptMinTrackSize.x = f32::round(min_size.width.0 * scale + 0.5_f32) as i32;
                        min_max_info.ptMinTrackSize.y = f32::round(min_size.height.0 * scale + 0.5_f32) as i32;
                        break 'wm_getminmaxinfo Some(LRESULT(0));
                    }
                }
                None
            }

            WM_ACTIVATE => {
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

            WM_NCCALCSIZE => 'wm_nccalcsize: {
                if window.has_custom_title_bar() && wparam.0 == windows::Win32::Foundation::TRUE.0 as usize {
                    if let Some(calcsize_params) = unsafe { (lparam.0 as *mut NCCALCSIZE_PARAMS).as_mut() } {
                        let top = calcsize_params.rgrc[0].top;
                        unsafe { DefWindowProcW(hwnd, WM_NCCALCSIZE, wparam, lparam) };
                        // the top inset should be 0 otherwise Windows will draw full native title bar
                        calcsize_params.rgrc[0].top = top;
                        break 'wm_nccalcsize Some(LRESULT(0));
                    }
                }
                None
            }

            WM_NCHITTEST => 'wm_nchittest: {
                if !window.has_custom_title_bar() || !window.is_resizable() {
                    break 'wm_nchittest None;
                }
                let original_ht = {
                    let mut dwm_result = LRESULT(0);
                    unsafe { DwmDefWindowProc(hwnd, WM_NCHITTEST, wparam, lparam, &mut dwm_result) }
                        .as_bool()
                        .then(|| dwm_result)
                        .unwrap_or_else(|| unsafe { DefWindowProcW(hwnd, WM_NCHITTEST, wparam, lparam) })
                };
                if original_ht != LRESULT(HTCLIENT as _) {
                    return original_ht;
                }
                let mouse_y = lparam.HIWORD();
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

            WM_NCMOUSELEAVE => {
                let mut dwm_result = LRESULT(0);
                unsafe { DwmDefWindowProc(hwnd, WM_NCMOUSELEAVE, wparam, lparam, &mut dwm_result) }
                    .as_bool()
                    .then(|| dwm_result)
            }

            WM_CLOSE => self.handle_event(hwnd.into(), &Event::WindowCloseRequest),

            _ => None,
        };

        match handled {
            Some(result) => result,
            None => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }
}
