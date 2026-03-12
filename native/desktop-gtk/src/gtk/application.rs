use crate::gtk::application_api::ApplicationCallbacks;
use crate::gtk::application_state::ApplicationState;
use crate::gtk::events::{Event, EventHandler};
use anyhow::{Context, bail};
use desktop_common::logger::catch_panic;
use gtk4::glib;
use gtk4::prelude::ApplicationExtManual;
use log::debug;
use std::cell::{OnceCell, RefCell};
use std::sync::OnceLock;
use std::thread::ThreadId;

#[derive(Default)]
pub struct Application {
    state: RefCell<Option<ApplicationState>>,
}

/// cbindgen:ignore
static EVENT_LOOP_THREAD_ID: OnceLock<ThreadId> = OnceLock::new();

thread_local! {
    static APP_INSTANCE: OnceCell<Application> = const { OnceCell::new() };
}

fn app_not_initialized_error() -> String {
    if let Some(t) = EVENT_LOOP_THREAD_ID.get() {
        let current_thread_id = std::thread::current().id();
        format!("Application initialized on a different thread ({t:?}). Current thread = {current_thread_id:?}")
    } else {
        "Application not initialized".to_owned()
    }
}

pub fn with_app_state_mut<T>(f: impl FnOnce(&mut ApplicationState) -> anyhow::Result<T>) -> anyhow::Result<T> {
    APP_INSTANCE.with(|app_cell| {
        let app = app_cell.get().with_context(app_not_initialized_error)?;
        let mut state_borrow = app.state.borrow_mut();
        let state = state_borrow.as_mut().context("Application not running")?;
        f(state)
    })
}

pub fn with_app_state<T>(f: impl FnOnce(&ApplicationState) -> anyhow::Result<T>) -> anyhow::Result<T> {
    APP_INSTANCE.with(|app_cell| {
        let app = app_cell.get().with_context(app_not_initialized_error)?;
        let state_borrow = app.state.borrow();
        let state = state_borrow.as_ref().context("Application not running")?;
        f(state)
    })
}

#[allow(clippy::needless_pass_by_value)]
pub fn send_event<'a, T: Into<Event<'a>>>(event_handler: EventHandler, event_data: T) -> bool {
    let event: Event = event_data.into();
    match event {
        Event::MouseMoved(_) | Event::WindowFrameTick(_) | Event::WindowDraw(_) | Event::DragIconDraw(_) | Event::DragIconFrameTick => {}
        _ => debug!("Sending event: {event:?}"),
    }
    catch_panic(|| Ok(event_handler(&event))).unwrap_or(false)
}

impl Application {
    pub fn init(app_id: &str) -> anyhow::Result<()> {
        let current_thread_id = std::thread::current().id();
        let event_loop_thread_id = *EVENT_LOOP_THREAD_ID.get_or_init(|| current_thread_id);
        if current_thread_id != event_loop_thread_id {
            bail!("Application already initialized in different thread ({event_loop_thread_id:?}, current thread = {current_thread_id:?})");
        }

        APP_INSTANCE.with(|cell| {
            if cell.set(Self::default()).is_ok() {
                glib::set_prgname(Some(app_id));
                gtk4::init()?;
            }
            Ok(())
        })
    }

    pub fn run_event_loop(callbacks: &ApplicationCallbacks) -> anyhow::Result<()> {
        APP_INSTANCE.with(|app_cell| {
            let app = app_cell.get().with_context(app_not_initialized_error)?;
            app.run(callbacks)
        })
    }

    fn run(&self, callbacks: &ApplicationCallbacks) -> anyhow::Result<()> {
        debug!("application_run_event_loop begin");
        let state = ApplicationState::new(callbacks)?;
        let app = state.gtk_app.clone();
        {
            *self.state.borrow_mut() = Some(state);
            debug!("Initialized application state");
        }

        debug!("Will start application event loop");

        app.run();

        debug!("Application event loop stopped");
        Ok(())
    }

    pub fn stop_event_loop() {
        Self::run_on_event_loop_async(|| {
            APP_INSTANCE.with(|app_cell| {
                let app = app_cell.get().expect("Application not initialized");
                debug!("Deinitializing application state");
                *app.state.borrow_mut() = None;
                debug!("Deinitialized application state");
            });
        });
    }

    pub fn run_on_event_loop_async<F>(func: F) -> glib::SourceId
    where
        F: FnOnce() + Send + 'static,
    {
        glib::source::idle_add_once(func)
    }

    pub fn is_event_loop_thread() -> anyhow::Result<bool> {
        let event_loop_thread = *EVENT_LOOP_THREAD_ID.get().context("Event loop not yet started")?;
        let current_thread_id = std::thread::current().id();
        Ok(event_loop_thread == current_thread_id)
    }
}
