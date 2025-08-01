use log::{debug, error};
use windows::{
    Foundation::TypedEventHandler,
    System::DispatcherQueueController,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::WinRT::{CreateDispatcherQueueController, DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT, DispatcherQueueOptions},
        UI::WindowsAndMessaging::{DefWindowProcW, DispatchMessageW, GetMessageW, MSG, PostQuitMessage, TranslateMessage, WM_CLOSE, WM_PAINT},
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
