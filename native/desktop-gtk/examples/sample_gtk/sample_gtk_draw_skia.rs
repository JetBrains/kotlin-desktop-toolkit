use crate::sample_gtk_impl::{Drawable, WindowState};
use desktop_common::ffi_utils::BorrowedStrPtr;
use desktop_gtk::gtk::application_api::application_get_egl_proc_func;
use desktop_gtk::gtk::events::OpenGlDrawData;
use desktop_gtk::gtk::geometry::{LogicalPixels, PhysicalSize, Scale};
use skia_safe::gpu::ganesh::gl::{backend_render_targets, direct_contexts};
use skia_safe::gpu::gl::{Format, FramebufferInfo, Interface};
use skia_safe::gpu::{DirectContext, SurfaceOrigin, surfaces};
use skia_safe::{ColorSpace, ColorType, Paint, Rect, colors};

pub struct SkiaOpenglState {
    direct_context: DirectContext,
    fb: u32,
}

impl SkiaOpenglState {
    pub fn new(draw_data: &OpenGlDrawData) -> Self {
        // let opengl_interface = skia_safe::gpu::ganesh::gl::make_egl_interface::interfaces::make_egl().expect("interfaces::make_egl");
        let egl_func = application_get_egl_proc_func();
        let opengl_interface = Interface::new_load_with_cstr(|name| (egl_func.f)(egl_func.ctx.clone(), BorrowedStrPtr::new(name)))
            .expect("Interface::new_load_with failed");
        let direct_context = direct_contexts::make_gl(opengl_interface, None).expect("direct_contexts::make_gl failed");
        Self {
            direct_context,
            fb: draw_data.framebuffer,
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn scaled(v: LogicalPixels, scale: Scale) -> f32 {
    v.to_raw_physical(scale) as f32
}

impl Drawable for SkiaOpenglState {
    fn draw(&mut self, physical_size: PhysicalSize, window_state: &WindowState) {
        let direct_context = &mut self.direct_context;
        let scale = window_state.scale;
        let animation_progress = window_state.animation_progress;
        let mut framebuffer_info = FramebufferInfo::from_fboid(self.fb);
        framebuffer_info.format = Format::RGBA8.into();
        let backend_render_target = backend_render_targets::make_gl(
            (physical_size.width.raw_physical(), physical_size.height.raw_physical()),
            1,
            0,
            framebuffer_info,
        );
        let mut surface = surfaces::wrap_backend_render_target(
            direct_context,
            &backend_render_target,
            SurfaceOrigin::TopLeft,
            ColorType::RGBA8888,
            ColorSpace::new_srgb(),
            None,
        )
        .expect("Failed to create surface");

        let canvas = surface.canvas();
        canvas.clear(colors::BLUE);

        {
            let paint = Paint::new(colors::RED, None);
            canvas.draw_rect(
                Rect::from_xywh(
                    0.,
                    0.,
                    scaled(LogicalPixels::new(100.), scale),
                    scaled(LogicalPixels::new(50.), scale),
                ),
                &paint,
            );
        }

        {
            let paint = Paint::new(colors::GREEN, None);
            canvas.draw_circle(
                skia_safe::Point::new(
                    scaled(LogicalPixels::new(100. + animation_progress), scale),
                    scaled(LogicalPixels::new(100.), scale),
                ),
                scaled(LogicalPixels::new(50.), scale),
                &paint,
            );
        }

        {
            let mut paint = Paint::new(colors::WHITE, None);
            paint.set_stroke_width(scaled(LogicalPixels::new(2.), scale));
            canvas.draw_line(
                skia_safe::Point::new(0., 0.),
                #[allow(clippy::cast_precision_loss)]
                skia_safe::Point::new(
                    physical_size.width.raw_physical() as f32,
                    physical_size.height.raw_physical() as f32,
                ),
                &paint,
            );
        }
        direct_context.flush_and_submit();
    }
}
