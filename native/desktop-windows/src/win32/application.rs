use std::rc::Rc;

use windows::core::Result as WinResult;

use super::{event_loop::EventLoop, events::EventHandler, window::Window, window_api::WindowParams};

pub struct Application {
    event_loop: Rc<EventLoop>,
}

impl Application {
    pub fn new(event_handler: EventHandler) -> WinResult<Self> {
        let event_loop = EventLoop::new(event_handler)?;
        Ok(Self {
            event_loop: Rc::new(event_loop),
        })
    }

    pub fn run_event_loop(&self) {
        self.event_loop.run();
    }

    pub fn shutdown(&self) -> WinResult<()> {
        self.event_loop.shutdown()
    }

    pub(crate) fn create_window(&self, params: &WindowParams) -> WinResult<Rc<Window>> {
        Window::new(params, Rc::downgrade(&self.event_loop))
    }
}
