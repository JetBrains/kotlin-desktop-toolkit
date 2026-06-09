use std::{rc::Rc, sync::Arc};

use windows::{
    Foundation::TypedEventHandler,
    System::DispatcherQueueController,
    UI::Composition::Core::CompositorController,
    Win32::{
        System::{
            Ole::OleInitialize,
            Threading::GetCurrentThreadId,
            WinRT::{CreateDispatcherQueueController, DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT, DispatcherQueueOptions},
        },
        UI::WindowsAndMessaging::PostQuitMessage,
    },
};

use super::{
    composition,
    compositor_driver::CompositorDriver,
    dispatcher::Dispatcher,
    event_loop::EventLoop,
    events::EventHandler,
    renderer_angle::AngleDevice,
    window::{Window, WindowId},
};

pub struct Application {
    compositor_driver: Arc<CompositorDriver>,
    dispatcher_queue_controller: DispatcherQueueController,
    dispatcher: Dispatcher,
    event_loop: Rc<EventLoop>,
    ui_thread_id: u32,
}

impl Application {
    pub fn init_apartment() -> anyhow::Result<()> {
        unsafe { OleInitialize(None)? };
        Ok(())
    }

    pub fn new(event_handler: EventHandler) -> anyhow::Result<Self> {
        let dispatcher_queue_controller = create_dispatcher_queue()?;
        let dispatcher = Dispatcher::new()?;
        let event_loop = EventLoop::new(event_handler)?;
        let compositor_controller = CompositorController::new()?;
        let compositor_driver = CompositorDriver::new(&compositor_controller, dispatcher_queue_controller.DispatcherQueue()?)?;
        Ok(Self {
            compositor_driver,
            dispatcher_queue_controller,
            dispatcher,
            event_loop: Rc::new(event_loop),
            // SAFETY: GetCurrentThreadId has no preconditions.
            // INVARIANT: Application::new runs on the UI thread; ui_thread_id is the comparand for is_dispatcher_thread.
            ui_thread_id: unsafe { GetCurrentThreadId() },
        })
    }

    pub fn invoke_on_dispatcher(&self, callback: extern "C" fn()) -> anyhow::Result<bool> {
        Ok(self.dispatcher.dispatch(callback))
    }

    #[must_use]
    pub fn is_dispatcher_thread(&self) -> bool {
        // `DispatcherQueue::HasThreadAccess` would be the natural WinRT call here,
        // but it was introduced in Windows 10 build 18362 (1903); this toolkit
        // supports down to 17763 (1809). `GetCurrentThreadId` works on every
        // supported version.
        //
        // SAFETY: GetCurrentThreadId has no preconditions.
        (unsafe { GetCurrentThreadId() }) == self.ui_thread_id
    }

    pub fn run_event_loop(&self) -> anyhow::Result<()> {
        self.event_loop.run()
    }

    pub fn shutdown(&self) -> anyhow::Result<()> {
        self.compositor_driver.shutdown();
        composition::release_composition_context();
        self.dispatcher.shutdown();
        let _ = self.dispatcher_queue_controller.ShutdownQueueAsync()?;
        Ok(())
    }

    pub(crate) fn new_window(&self, window_id: WindowId) -> anyhow::Result<Window> {
        Window::new(window_id, Rc::downgrade(&self.event_loop), self.compositor_driver.compositor()?)
    }

    pub(crate) fn create_angle_device(&self, window: &Window) -> anyhow::Result<AngleDevice> {
        AngleDevice::create_for_window(window, Arc::clone(&self.compositor_driver))
    }
}

#[inline]
fn create_dispatcher_queue() -> anyhow::Result<DispatcherQueueController> {
    let dispatcher_queue_controller = unsafe {
        CreateDispatcherQueueController(DispatcherQueueOptions {
            dwSize: size_of::<DispatcherQueueOptions>().try_into()?,
            threadType: DQTYPE_THREAD_CURRENT,
            apartmentType: DQTAT_COM_NONE,
        })?
    };

    // See https://devblogs.microsoft.com/oldnewthing/20240509-52/?p=109738
    dispatcher_queue_controller
        .DispatcherQueue()?
        .ShutdownCompleted(&TypedEventHandler::new(|_, _| {
            log::trace!("Shutting down the dispatcher queue");
            unsafe { PostQuitMessage(0) };
            Ok(())
        }))?;

    Ok(dispatcher_queue_controller)
}
