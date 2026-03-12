use crate::gtk::geometry::LogicalSize;
use gtk4::glib;
use gtk4::prelude::{LayoutManagerExt, WidgetExt};
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use std::cell::OnceCell;

type OnAllocate = Box<dyn Fn(LogicalSize)>;

#[derive(Default)]
pub struct LayoutManagerWrapperImpl {
    pub layout_manager: OnceCell<gtk4::LayoutManager>,
    pub on_allocate: OnceCell<OnAllocate>,
}

#[glib::object_subclass]
impl gtk4::subclass::prelude::ObjectSubclass for LayoutManagerWrapperImpl {
    /// cbindgen:ignore
    const NAME: &'static str = "LayoutManagerWrapper";
    type Type = LayoutManagerWrapper;
    type ParentType = gtk4::LayoutManager;

    fn new() -> Self {
        Self::default()
    }
}

impl gtk4::subclass::prelude::ObjectImpl for LayoutManagerWrapperImpl {}
impl gtk4::subclass::prelude::WidgetImpl for LayoutManagerWrapperImpl {}
impl gtk4::subclass::prelude::LayoutManagerImpl for LayoutManagerWrapperImpl {
    fn allocate(&self, widget: &gtk4::Widget, width: i32, height: i32, baseline: i32) {
        let Some(layout_manager) = self.layout_manager.get() else { return };
        layout_manager.allocate(widget, width, height, baseline);

        if let Some(on_allocate) = self.on_allocate.get() {
            let alloc_size = LogicalSize { width, height };
            on_allocate(alloc_size);
        }
    }

    fn measure(&self, widget: &gtk4::Widget, orientation: gtk4::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
        if let Some(layout_manager) = self.layout_manager.get() {
            layout_manager.measure(widget, orientation, for_size)
        } else {
            (0, 0, -1, -1)
        }
    }
}

glib::wrapper! {
    pub struct LayoutManagerWrapper(ObjectSubclass<LayoutManagerWrapperImpl>)
    @extends gtk4::Widget, gtk4::LayoutManager,
    @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl LayoutManagerWrapper {
    fn new(layout_manager: gtk4::LayoutManager, on_allocate: OnAllocate) -> Self {
        let obj = glib::Object::new::<Self>();
        let imp = obj.imp();
        imp.layout_manager.set(layout_manager).unwrap();
        imp.on_allocate.set(on_allocate).ok().unwrap();
        obj
    }

    pub fn wrap(widget: &gtk4::Widget, on_allocate: impl Fn(LogicalSize) + 'static) {
        let Some(layout_manager) = widget.layout_manager() else { return };
        widget.set_layout_manager(Some(Self::new(layout_manager, Box::new(on_allocate))));
    }
}
