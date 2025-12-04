use std::ffi::CStr;
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
use std::time::{Duration, Instant};
use sdl3_sys::everything::SDL_GetWindowSize;
use sdl3_sys::video::{SDL_DestroyWindow, SDL_GetWindowSizeInPixels, SDL_MaximizeWindow, SDL_MinimizeWindow, SDL_RestoreWindow, SDL_SetWindowFullscreen, SDL_SetWindowMaximumSize, SDL_SetWindowMinimumSize, SDL_SetWindowTitle, SDL_ShowWindowSystemMenu, SDL_Window};
use crate::linux::geometry::{LogicalPixels, PhysicalPixels};

pub enum RenderingData {
    Egl(EglRendering<'static>),
    Software(SoftwareRendering),
}

impl RenderingData {
    pub fn new(window: *mut SDL_Window, egl: Option<&'static EglInstance>) -> anyhow::Result<Self> {
        let rendering_data = if let Some(egl) = egl {
            match EglRendering::new(egl, window) {
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
    pub window: *mut SDL_Window,
    pub current_scale: f32,
    // decoration_mode: DecorationMode,
    pub rendering_data: RenderingData,
    pub last_draw_measure_time: Instant,
    pub draw_call_count: u32,
}

pub fn get_physical_window_size(window: *mut SDL_Window) -> PhysicalSize {
    let mut current_width = 0;
    let mut current_height = 0;
    if !unsafe { SDL_GetWindowSizeInPixels(window, &raw mut current_width, &raw mut current_height) } {
        panic!("Failed to get window size");
    }
    PhysicalSize { width: PhysicalPixels(current_width), height: PhysicalPixels(current_height) }
}

pub fn get_logical_window_size(window: *mut SDL_Window) -> LogicalSize {
    let mut current_width = 0;
    let mut current_height = 0;
    if !unsafe { SDL_GetWindowSize(window, &raw mut current_width, &raw mut current_height) } {
        panic!("Failed to get window size");
    }
    LogicalSize { width: LogicalPixels(current_width.into()), height: LogicalPixels(current_height.into()) }
}

impl Drop for SimpleWindow {
    fn drop(&mut self) {
        unsafe { SDL_DestroyWindow(self.window) }
    }
}

impl SimpleWindow {
    pub fn draw(&mut self, callback: &dyn Fn(WindowDrawEvent) -> bool) {
        let physical_size = self.get_physical_size();

        let do_draw = |software_draw_data: SoftwareDrawData| {
            callback(WindowDrawEvent {
                window_id: self.window_id,
                software_draw_data,
                physical_size,
                scale: self.current_scale.into(),
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

    pub fn scale_changed(&mut self, new_scale: f32) {
        debug!("scale_changed: {new_scale}");
        self.current_scale = new_scale;
        let physical_size = self.get_physical_size();

        self.on_physical_size_changed(physical_size);
    }

    pub fn request_redraw(&self) {
        // self.window.request_redraw();
    }

    pub fn get_physical_size(&self) -> PhysicalSize {
        get_physical_window_size(self.window)
    }

    pub fn get_logical_size(&self) -> LogicalSize {
        get_logical_window_size(self.window)
    }

    pub fn maximize(&self) {
        if !unsafe { SDL_MaximizeWindow(self.window) } {
            warn!("maximize failed");
        }
    }

    pub fn unmaximize(&self) {
        if !unsafe { SDL_RestoreWindow(self.window) } {
            warn!("unmaximize failed");
        }
    }

    pub fn minimize(&self) {
        if !unsafe { SDL_MinimizeWindow(self.window) } {
            warn!("minimize failed");
        }
    }

    pub fn set_max_size(&self, size: LogicalSize) {
        if !unsafe { SDL_SetWindowMaximumSize(self.window, size.width.round(), size.height.round()) } {
            warn!("set_max_size failed");
        }
    }

    pub fn set_min_size(&self, size: LogicalSize) {
        if !unsafe { SDL_SetWindowMinimumSize(self.window, size.width.round(), size.height.round()) } {
            warn!("set_min_size failed");
        }
    }

    pub fn set_title(&self, new_title: &CStr) {
        if !unsafe { SDL_SetWindowTitle(self.window, new_title.as_ptr()) } {
            warn!("set_title failed");
        }
    }

    pub fn set_cursor_icon(&self, pointer_shape: PointerShape) {
        // if let Some(cursor) = pointer_shape.into() {
        //     self.window.set_cursor(cursor);
        //     self.window.set_cursor_visible(true);
        // } else {
        //     self.window.set_cursor_visible(false);
        // }
    }

    pub fn start_move(&self) -> anyhow::Result<()> {
        // self.window.drag_window()?;
        Ok(())
    }

    pub fn start_resize(&self, edge: WindowResizeEdge) -> anyhow::Result<()> {
        // if let Some(direction) = edge.into() {
        //     self.window.drag_resize_window(direction)?;
        // }
        Ok(())
    }

    pub fn show_menu(&self, position: LogicalPoint) {
        if !unsafe { SDL_ShowWindowSystemMenu(self.window, position.x.round(), position.y.round()) } {
            warn!("show_menu failed");
        }
    }

    pub fn set_fullscreen(&self) {
        if !unsafe { SDL_SetWindowFullscreen(self.window, true) } {
            warn!("set_fullscreen failed");
        }
    }

    pub fn unset_fullscreen(&self) {
        if !unsafe { SDL_SetWindowFullscreen(self.window, false) } {
            warn!("unset_fullscreen failed");
        }
    }
}
