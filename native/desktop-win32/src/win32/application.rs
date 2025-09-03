use std::rc::Rc;

use windows::{
    Foundation::TypedEventHandler,
    System::DispatcherQueueController,
    UI::Composition::Compositor,
    Win32::{
        System::WinRT::{CreateDispatcherQueueController, DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT, DispatcherQueueOptions},
        UI::WindowsAndMessaging::PostQuitMessage,
    },
    core::Result as WinResult,
};

use super::{event_loop::EventLoop, events::EventHandler, window::Window, window_api::WindowParams};

pub struct Application {
    dispatcher_queue_controller: DispatcherQueueController,
    event_loop: Rc<EventLoop>,
    compositor: Rc<Compositor>,
}

impl Application {
    pub fn new(event_handler: EventHandler) -> WinResult<Self> {
        let dispatcher_queue_controller = create_dispatcher_queue()?;
        let event_loop = EventLoop::new(event_handler);
        let compositor = Compositor::new()?;
        Ok(Self {
            dispatcher_queue_controller,
            event_loop: Rc::new(event_loop),
            compositor: Rc::new(compositor),
        })
    }

    pub fn run_event_loop(&self) {
        self.event_loop.run();
    }

    pub fn shutdown(&self) -> WinResult<()> {
        self.dispatcher_queue_controller
            .ShutdownQueueAsync()
            .map(|_async| ())
            .inspect_err(|err| log::error!("Failed to shut down the dispatcher queue: {err:?}"))
    }

    pub(crate) fn create_window(&self, params: &WindowParams) -> WinResult<Rc<Window>> {
        Window::new(params, Rc::downgrade(&self.event_loop), Rc::downgrade(&self.compositor))
    }
}

#[allow(clippy::cast_possible_truncation)]
#[inline]
fn create_dispatcher_queue() -> WinResult<DispatcherQueueController> {
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
            log::debug!("Shutting down the dispatcher queue");
            unsafe { PostQuitMessage(0) };
            Ok(())
        }))?;

    Ok(dispatcher_queue_controller)
}
