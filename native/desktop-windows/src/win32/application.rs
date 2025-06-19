use windows::{
    Foundation::TypedEventHandler,
    System::DispatcherQueueController,
    Win32::{
        System::WinRT::{CreateDispatcherQueueController, DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT, DispatcherQueueOptions},
        UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG, PostQuitMessage},
    },
    core::Result as WinResult,
};

pub struct Application {
    dispatcher_queue_controller: DispatcherQueueController,
}

impl Application {
    pub fn new() -> WinResult<Self> {
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
                unsafe { PostQuitMessage(0) };
                Ok(())
            }))?;

        Ok(Self {
            dispatcher_queue_controller,
        })
    }

    pub fn run(&self) {
        let mut msg = MSG::default();
        unsafe {
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                // let _ = windows::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    pub fn dispatcher_queue_controller(&self) -> &DispatcherQueueController {
        &self.dispatcher_queue_controller
    }
}
