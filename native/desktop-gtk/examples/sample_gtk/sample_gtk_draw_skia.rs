use desktop_common::ffi_utils::BorrowedStrPtr;
use desktop_gtk::gtk::application_api::application_get_egl_proc_func;
use desktop_gtk::gtk::events::OpenGlDrawData;
use desktop_gtk::gtk::geometry::PhysicalSize;
use skia_safe::gpu::ganesh::gl::{backend_render_targets, direct_contexts};
use skia_safe::gpu::gl::{Format, FramebufferInfo, Interface};
use skia_safe::gpu::{DirectContext, SurfaceOrigin, surfaces};
use skia_safe::{ColorSpace, ColorType, Paint, Rect, colors};

pub struct OpenglState {
    direct_context: DirectContext,
    fb: u32,
}

impl OpenglState {
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

pub fn draw(gl_state: &mut OpenglState, physical_size: PhysicalSize, scale: f32, animation_progress: f32) {
    let direct_context = &mut gl_state.direct_context;
    let mut framebuffer_info = FramebufferInfo::from_fboid(gl_state.fb);
    framebuffer_info.format = Format::RGBA8.into();
    let backend_render_target = backend_render_targets::make_gl((physical_size.width.0, physical_size.height.0), 1, 0, framebuffer_info);
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
        canvas.draw_rect(Rect::from_xywh(0., 0., 100. * scale, 50. * scale), &paint);
    }

    {
        let paint = Paint::new(colors::GREEN, None);
        canvas.draw_circle(
            skia_safe::Point::new((100. + animation_progress) * scale, 100.),
            50. * scale,
            &paint,
        );
    }

    {
        let mut paint = Paint::new(colors::WHITE, None);
        paint.set_stroke_width(2. * scale);
        canvas.draw_line(
            skia_safe::Point::new(0., 0.),
            #[allow(clippy::cast_precision_loss)]
            skia_safe::Point::new(physical_size.width.0 as f32, physical_size.height.0 as f32),
            &paint,
        );
    }
    direct_context.flush_and_submit();
}
