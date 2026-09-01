use crate::sample_linux::{DRAG_AND_DROP_LEFT_OF, Drawable, WindowState};
use desktop_linux::linux::application_api::{AppPtr, application_get_egl_proc_func};
use desktop_linux::linux::geometry::{LogicalPixelsInt, PhysicalSize, Scale};
use skia_safe::gpu::ganesh::gl::{backend_render_targets, direct_contexts};
use skia_safe::gpu::gl::{Format, FramebufferInfo, Interface};
use skia_safe::gpu::{DirectContext, SurfaceOrigin, surfaces};
use skia_safe::{Color, Color4f, ColorSpace, ColorType, Paint, Rect, colors};

pub struct SkiaOpenglState {
    direct_context: DirectContext,
    fb: u32,
}

impl SkiaOpenglState {
    pub fn new(app_ptr: AppPtr) -> Self {
        let egl_func = application_get_egl_proc_func(app_ptr);
        let opengl_interface =
            Interface::new_load_with_cstr(|name| (egl_func.f)(egl_func.ctx, name.as_ptr())).expect("Interface::new_load_with failed");
        let direct_context = direct_contexts::make_gl(opengl_interface, None).expect("direct_contexts::make_gl failed");
        Self { direct_context, fb: 0 }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn scaled(v: LogicalPixelsInt, scale: Scale) -> f32 {
    v.to_raw_physical(scale) as f32
}

impl Drawable for SkiaOpenglState {
    #[allow(clippy::too_many_lines)]
    fn draw(&mut self, physical_size: PhysicalSize, window_state: &WindowState) {
        let scale = window_state.scale;

        let direct_context = &mut self.direct_context;
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
            SurfaceOrigin::BottomLeft,
            ColorType::RGBA8888,
            ColorSpace::new_srgb(),
            None,
        )
        .expect("Failed to create surface");

        let canvas = surface.canvas();
        canvas.clear(Color::from_argb(128, 128, 128, 128));

        #[allow(clippy::cast_precision_loss)]
        let w = physical_size.width.raw_physical() as f32;
        #[allow(clippy::cast_precision_loss)]
        let h = physical_size.height.raw_physical() as f32;
        let padding_left = scaled(window_state.frame.padding.left, scale);
        let padding_right = scaled(window_state.frame.padding.right, scale);
        let padding_top = scaled(window_state.frame.padding.top, scale);
        let padding_bottom = scaled(window_state.frame.padding.bottom, scale);

        {
            let background_color = if window_state.active {
                colors::WHITE
            } else {
                Color4f::from(Color::from_rgb(128, 128, 128))
            };

            let paint = Paint::new(background_color, None);

            canvas.draw_rect(Rect::new(padding_left, padding_top, w - padding_right, h - padding_bottom), &paint);
        }

        let line_thickness = scaled(LogicalPixelsInt::new(2), scale);
        let drag_and_drop_left_of = scaled(DRAG_AND_DROP_LEFT_OF, scale) + padding_left;
        let drag_source_indicator_height = scaled(LogicalPixelsInt::new(100), scale) + padding_top;

        {
            let mut paint = Paint::new(colors::BLACK, None);
            paint.set_stroke_width(line_thickness * 2.);

            canvas.draw_line(
                skia_safe::Point::new(drag_and_drop_left_of, padding_top),
                skia_safe::Point::new(drag_and_drop_left_of, h - padding_bottom),
                &paint,
            );
        }

        if window_state.drag_and_drop_target {
            let paint = Paint::new(colors::BLUE, None);

            canvas.draw_rect(
                Rect::new(padding_left, padding_top, drag_and_drop_left_of, h - padding_bottom),
                &paint,
            );
        }

        if window_state.drag_and_drop_source {
            let mut paint = Paint::new(colors::BLUE, None);
            paint.set_stroke_width(line_thickness * 2.);

            canvas.draw_line(
                skia_safe::Point::new(padding_left + line_thickness, drag_source_indicator_height),
                skia_safe::Point::new(drag_and_drop_left_of, drag_source_indicator_height),
                &paint,
            );
        }

        {
            let mut paint = Paint::new(colors::RED, None);
            paint.set_stroke_width(line_thickness * 2.);

            canvas.draw_line(
                skia_safe::Point::new(padding_left + line_thickness, padding_top),
                skia_safe::Point::new(padding_left + line_thickness, h - padding_bottom),
                &paint,
            );
            canvas.draw_line(
                skia_safe::Point::new(w - padding_right - line_thickness, padding_top),
                skia_safe::Point::new(w - padding_right - line_thickness, h - padding_bottom),
                &paint,
            );
            canvas.draw_line(
                skia_safe::Point::new(padding_left, padding_top + line_thickness),
                skia_safe::Point::new(w - padding_right, padding_top + line_thickness),
                &paint,
            );
            canvas.draw_line(
                skia_safe::Point::new(padding_left, h - padding_bottom - line_thickness),
                skia_safe::Point::new(w - padding_right, h - padding_bottom - line_thickness),
                &paint,
            );

            canvas.draw_line(
                skia_safe::Point::new(padding_left, padding_top),
                skia_safe::Point::new(w - padding_right, h - padding_bottom - line_thickness),
                &paint,
            );
        }
        direct_context.flush_and_submit();
    }
}
