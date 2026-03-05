use gtk4::gio::subclass::ArgumentList;
use gtk4::glib::ExitCode;
use gtk4::subclass::prelude::*;
use gtk4::{gio, glib};
use std::ops::ControlFlow;

// Object holding the state
#[derive(Default)]
pub struct KdtApplicationImpl;

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for KdtApplicationImpl {
    /// cbindgen:ignore
    const NAME: &'static str = "KdtApplication";
    type Type = KdtApplication;
    type ParentType = gtk4::Application;
}

impl ObjectImpl for KdtApplicationImpl {}

impl GtkApplicationImpl for KdtApplicationImpl {}

impl ApplicationImpl for KdtApplicationImpl {
    fn local_command_line(&self, _arguments: &mut ArgumentList) -> ControlFlow<ExitCode> {
        ControlFlow::Continue(())
    }
}

glib::wrapper! {
    pub struct KdtApplication(ObjectSubclass<KdtApplicationImpl>)
        @extends gio::Application, gtk4::Application,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl KdtApplication {
    pub fn new(app_id: &str) -> Self {
        glib::Object::builder()
            .property("application-id", app_id)
            .property("flags", gio::ApplicationFlags::NON_UNIQUE)
            .build()
    }
}
