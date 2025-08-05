use std::rc::Rc;

use windows::core::Result as WinResult;

use super::{event_loop::EventLoop, events::EventHandler};

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

    #[must_use]
    pub fn event_loop(&self) -> Rc<EventLoop> {
        self.event_loop.clone()
    }
}
