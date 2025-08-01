use log::{debug, error};
use windows::{
    Foundation::TypedEventHandler,
    System::DispatcherQueueController,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::{BeginPaint, EndPaint},
        System::WinRT::{CreateDispatcherQueueController, DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT, DispatcherQueueOptions},
        UI::WindowsAndMessaging::{
            DefWindowProcW, DispatchMessageW, GetClientRect, GetMessageW, MSG, PostQuitMessage, TranslateMessage, WM_CLOSE, WM_PAINT,
        },
    },
    core::Result as WinResult,
};

use super::{
    events::{Event, EventHandler, WindowDrawEvent},
    geometry::{PhysicalPixels, PhysicalSize},
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
                let event = Event::WindowDraw(WindowDrawEvent {
                    physical_size: PhysicalSize {
                        width: PhysicalPixels(rect.right - rect.left),
                        height: PhysicalPixels(rect.bottom - rect.top),
                    },
                    scale: 1.0, // TODO
                });
                let handled = (self.event_handler)(hwnd.into(), &event);
                let _ = unsafe { EndPaint(hwnd, &paint) };
                if handled { LRESULT(0) } else { LRESULT(1) }
            }
            WM_CLOSE => {
                if let Err(_err) = self.shutdown() {
                    error!("failed to request the shutdown of the dispatcher queue");
                    unsafe { PostQuitMessage(0) };
                }
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }
}
