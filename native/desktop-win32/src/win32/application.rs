use windows::{
    Foundation::TypedEventHandler,
    System::{DispatcherQueueController, DispatcherQueueHandler},
    UI::Composition::Core::CompositorController,
    Win32::{
        System::{
            Com::{COINIT_APARTMENTTHREADED, CoInitializeEx},
            WinRT::{CreateDispatcherQueueController, DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT, DispatcherQueueOptions},
        },
        UI::WindowsAndMessaging::PostQuitMessage,
    },
};

use super::{
    event_loop::EventLoop,
    events::EventHandler,
    renderer_angle::AngleDevice,
    window::{Window, WindowId},
};

pub struct Application {
    dispatcher_queue_controller: DispatcherQueueController,
    compositor_controller: CompositorController,
}

impl Application {
    pub fn init_apartment() -> anyhow::Result<()> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.ok()?;
        Ok(())
    }

    pub fn new(event_handler: EventHandler) -> anyhow::Result<Self> {
        let dispatcher_queue_controller = create_dispatcher_queue()?;
        EventLoop::init(event_handler)?;
        let compositor_controller = CompositorController::new()?;
        Ok(Self {
            dispatcher_queue_controller,
            compositor_controller,
        })
    }

    pub fn invoke_on_dispatcher_queue(&self, callback: extern "C" fn()) -> anyhow::Result<bool> {
        self.dispatcher_queue_controller
            .DispatcherQueue()?
            .TryEnqueue(&DispatcherQueueHandler::new(move || {
                log::trace!("Application dispatcher invoke");
                callback();
                Ok(())
            }))
            .map_err(Into::into)
    }

    pub fn is_dispatcher_thread(&self) -> anyhow::Result<bool> {
        Ok(self.dispatcher_queue_controller.DispatcherQueue()?.HasThreadAccess()?)
    }

    pub fn run_event_loop(&self) -> anyhow::Result<()> {
        EventLoop::run()
    }

    pub fn shutdown(&self) -> anyhow::Result<()> {
        let _ = self.dispatcher_queue_controller.ShutdownQueueAsync()?;
        Ok(())
    }

    pub(crate) fn new_window(&self, window_id: WindowId) -> anyhow::Result<Window> {
        Window::new(window_id, self.compositor_controller.Compositor()?)
    }

    pub(crate) fn create_angle_device(&self, window: &Window) -> anyhow::Result<AngleDevice> {
        AngleDevice::create_for_window(window, self.compositor_controller.clone())
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
