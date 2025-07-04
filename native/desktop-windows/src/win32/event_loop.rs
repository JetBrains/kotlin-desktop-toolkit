use log::{debug, error};
use windows::{
    Foundation::TypedEventHandler,
    System::DispatcherQueueController,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{BeginPaint, EndPaint},
        System::WinRT::{CreateDispatcherQueueController, DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT, DispatcherQueueOptions},
        UI::WindowsAndMessaging::{
            DefWindowProcW, DispatchMessageW, GetClientRect, GetMessageW, MSG, PostQuitMessage, TranslateMessage, USER_DEFAULT_SCREEN_DPI,
            WM_CLOSE, WM_DPICHANGED, WM_PAINT,
        },
    },
    core::Result as WinResult,
};

use super::{
    events::{Event, EventHandler, WindowDrawEvent, WindowScaleChangedEvent},
    geometry::{PhysicalPoint, PhysicalSize},
    utils,
    window::Window,
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
                let _ = TranslateMessage(&msg);
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

    pub fn window_proc(&self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_PAINT => {
                let mut paint = Default::default();
                unsafe { BeginPaint(hwnd, &mut paint) };
                let mut rect = Default::default();
                if let Err(err) = unsafe { GetClientRect(hwnd, &mut rect) } {
                    error!("Failed to get client rect: {err:?}");
                    return LRESULT(1);
                }
                let event = WindowDrawEvent {
                    physical_size: PhysicalSize::new(rect.right - rect.left, rect.bottom - rect.top),
                    scale: Window::hwnd_get_scale(hwnd),
                };
                let handled = (self.event_handler)(hwnd.into(), &event.into());
                let _ = unsafe { EndPaint(hwnd, &paint) };
                if handled { LRESULT(0) } else { LRESULT(1) }
            }

            WM_DPICHANGED => {
                let new_dpi = utils::HIWORD(wparam.0);
                assert_eq!(
                    new_dpi,
                    utils::LOWORD(wparam.0),
                    "The DPI values of the X-axis and the Y-axis should be identical for Windows apps."
                );
                let new_scale = (new_dpi as f32) / (USER_DEFAULT_SCREEN_DPI as f32);
                let new_rect = unsafe { *(lparam.0 as *const RECT) };
                let event = WindowScaleChangedEvent {
                    new_origin: PhysicalPoint::new(new_rect.left, new_rect.top),
                    new_size: PhysicalSize::new(new_rect.right - new_rect.left, new_rect.bottom - new_rect.top),
                    new_scale,
                };
                let handled = (self.event_handler)(hwnd.into(), &event.into());
                if handled { LRESULT(0) } else { LRESULT(1) }
            }

            WM_CLOSE => {
                (self.event_handler)(hwnd.into(), &Event::WindowCloseRequest);
                LRESULT(0)
            }

            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }
}
