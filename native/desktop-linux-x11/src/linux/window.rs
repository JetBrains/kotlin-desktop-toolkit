use crate::linux::{
    application_state::EglInstance,
    events::{SoftwareDrawData, WindowDrawEvent, WindowId},
    geometry::{LogicalPoint, LogicalSize, PhysicalSize},
    pointer_shapes_api::PointerShape,
    rendering_egl::EglRendering,
    rendering_software::SoftwareRendering,
    window_resize_edge_api::WindowResizeEdge,
};
use log::{debug, warn};
use std::rc::Rc;
use std::time::{Duration, Instant};
use winit_core::monitor::Fullscreen;
use winit_core::window::Window as WinitWindow;

pub enum RenderingData {
    Egl(EglRendering<'static>),
    Software(SoftwareRendering),
}

impl RenderingData {
    pub fn new(window: Rc<Box<dyn WinitWindow>>, egl: Option<&'static EglInstance>) -> anyhow::Result<Self> {
        let rendering_data = if let Some(egl) = egl {
            match EglRendering::new(egl, window.clone()) {
                Ok(egl_rendering_data) => Self::Egl(egl_rendering_data),
                Err(e) => {
                    warn!("Failed to create EGL rendering, falling back to software rendering. Error: {e:?}");
                    Self::Software(SoftwareRendering::new(window)?)
                }
            }
        } else {
            Self::Software(SoftwareRendering::new(window)?)
        };
        Ok(rendering_data)
    }

    pub fn draw<F>(&mut self, size: PhysicalSize, do_draw: F)
    where
        F: FnOnce(SoftwareDrawData) -> bool,
    {
        match self {
            Self::Egl(r) => r.draw(do_draw),
            Self::Software(r) => r.draw(size, do_draw),
        }
    }
}

pub struct SimpleWindow {
    pub window_id: WindowId,
    // pub app_id: String,
    pub window: Rc<Box<dyn WinitWindow>>,
    pub current_scale: f64,
    // decoration_mode: DecorationMode,
    pub rendering_data: RenderingData,
    pub last_draw_measure_time: Instant,
    pub draw_call_count: u32,
}

impl SimpleWindow {
    pub fn draw(&mut self, callback: &dyn Fn(WindowDrawEvent) -> bool) {
        let physical_size = self.get_physical_size();

        let do_draw = |software_draw_data: SoftwareDrawData| {
            callback(WindowDrawEvent {
                window_id: self.window_id,
                software_draw_data,
                physical_size,
                scale: self.current_scale,
            })
        };

        let now = Instant::now();
        self.draw_call_count += 1;
        if now - self.last_draw_measure_time > Duration::from_millis(1000) {
            debug!("{:?}: {} FPS", self.window_id, self.draw_call_count);
            self.last_draw_measure_time = now;
            self.draw_call_count = 0;
        }
        self.rendering_data.draw(physical_size, do_draw);
    }

    fn on_physical_size_changed(&mut self, physical_size: PhysicalSize) {
        match &mut self.rendering_data {
            RenderingData::Egl(egl_data) => {
                egl_data.resize(physical_size);
            }
            RenderingData::Software(data) => {
                data.resize(physical_size);
            }
        }
    }

    pub fn on_resize(&mut self, physical_size: PhysicalSize) {
        self.on_physical_size_changed(physical_size);
    }

    pub fn scale_changed(&mut self, new_scale: f64) {
        debug!("scale_changed: {new_scale}");
        self.current_scale = new_scale;
        let physical_size = self.get_physical_size();

        self.on_physical_size_changed(physical_size);
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn get_physical_size(&self) -> PhysicalSize {
        self.window.surface_size().into()
    }

    pub fn get_logical_size(&self) -> LogicalSize {
        self.get_physical_size().to_logical(self.current_scale)
    }

    pub fn maximize(&self) {
        self.window.set_maximized(true);
    }

    pub fn unmaximize(&self) {
        self.window.set_maximized(false);
    }

    pub fn minimize(&self) {
        self.window.set_minimized(true);
    }

    pub fn set_max_size(&self, size: LogicalSize) {
        self.window.set_max_surface_size(Some(size.as_winit_size()));
    }

    pub fn set_min_size(&self, size: LogicalSize) {
        self.window.set_min_surface_size(Some(size.as_winit_size()));
    }

    pub fn set_title(&self, new_title: &str) {
        self.window.set_title(new_title);
    }

    pub fn set_cursor_icon(&self, pointer_shape: PointerShape) {
        if let Some(cursor) = pointer_shape.into() {
            self.window.set_cursor(cursor);
            self.window.set_cursor_visible(true);
        } else {
            self.window.set_cursor_visible(false);
        }
    }

    pub fn start_move(&self) -> anyhow::Result<()> {
        self.window.drag_window()?;
        Ok(())
    }

    pub fn start_resize(&self, edge: WindowResizeEdge) -> anyhow::Result<()> {
        if let Some(direction) = edge.into() {
            self.window.drag_resize_window(direction)?;
        }
        Ok(())
    }

    pub fn show_menu(&self, position: LogicalPoint) {
        self.window.show_window_menu(position.as_winit_position());
    }

    pub fn set_fullscreen(&self) {
        self.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
    }

    pub fn unset_fullscreen(&self) {
        self.window.set_fullscreen(None);
    }
}
