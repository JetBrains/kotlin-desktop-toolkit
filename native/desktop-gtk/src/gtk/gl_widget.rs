use crate::gtk::application_state::{GL_INSTANCE, get_egl};
use gtk4::prelude::{NativeExt, SurfaceExt};

use crate::gtk::application_api::gl_get_proc_address_impl;
use crate::gtk::events::OpenGlDrawData;
use crate::gtk::geometry::{LogicalSize, PhysicalSize};
use anyhow::bail;
use gtk4::gdk as gdk4;
use gtk4::glib;
use gtk4::prelude::{GLContextExt, SnapshotExt, WidgetExt};
use gtk4::subclass::prelude::{ObjectSubclassExt, ObjectSubclassIsExt};
use gtk4::subclass::widget::WidgetImplExt;
use log::debug;
use std::cell::RefCell;
use std::ffi::{c_int, c_uint};
use std::mem::ManuallyDrop;

/// cbindgen:ignore
const GL_TEXTURE_2D: u32 = 0x0DE1;

/// cbindgen:ignore
const GL_FRAMEBUFFER: u32 = 0x8D40;

const unsafe fn cast_f<T, S>(t: T) -> S {
    unsafe { std::mem::transmute_copy::<ManuallyDrop<T>, S>(&ManuallyDrop::new(t)) }
}

fn get_egl_f<T>(name: &str) -> anyhow::Result<T> {
    let f_raw = if let Some(egl) = get_egl() {
        if let Some(f) = egl.get_proc_address(name) {
            f as *mut std::os::raw::c_void
        } else {
            std::ptr::null_mut()
        }
    } else {
        let gl = GL_INSTANCE
            .get()
            .expect("GL not initialized")
            .as_ref()
            .expect("GL library not found");
        gl_get_proc_address_impl(gl, name)
    };
    if f_raw.is_null() {
        bail!(format!("{name:?}"))
    }
    let f = unsafe { cast_f(f_raw) };
    Ok(f)
}

#[allow(non_snake_case)]
pub struct GlFuncs {
    pub GetIntegerv: extern "C" fn(pname: c_uint, data: *mut c_int),
    pub BindTexture: extern "C" fn(target: c_uint, texture: c_uint),
    pub TexImage2D: extern "C" fn(
        target: c_uint,
        level: c_int,
        internalformat: c_int,
        width: c_int,
        height: c_int,
        border: c_int,
        format: c_uint,
        type_: c_uint,
        data: *const std::ffi::c_void,
    ),
    pub BindFramebuffer: extern "C" fn(target: c_uint, framebuffer: c_uint),
    pub FramebufferTexture2D: extern "C" fn(target: c_uint, attachment: c_uint, textarget: c_uint, texture: c_uint, level: c_int),
    pub CheckFramebufferStatus: extern "C" fn(target: c_uint) -> c_uint,
    pub DeleteTextures: extern "C" fn(n: c_int, textures: *const c_uint),
    pub DeleteFramebuffers: extern "C" fn(n: c_int, framebuffers: *const c_uint),
}

impl GlFuncs {
    fn new() -> Self {
        Self {
            GetIntegerv: get_egl_f("glGetIntegerv").unwrap(),
            BindTexture: get_egl_f("glBindTexture").unwrap(),
            TexImage2D: get_egl_f("glTexImage2D").unwrap(),
            BindFramebuffer: get_egl_f("glBindFramebuffer").unwrap(),
            FramebufferTexture2D: get_egl_f("glFramebufferTexture2D").unwrap(),
            CheckFramebufferStatus: get_egl_f("glCheckFramebufferStatus").unwrap(),
            DeleteTextures: get_egl_f("glDeleteTextures").unwrap(),
            DeleteFramebuffers: get_egl_f("glDeleteFramebuffers").unwrap(),
        }
    }

    #[allow(clippy::cast_sign_loss)]
    fn get_active_texture_id(&self) -> c_uint {
        const GL_TEXTURE_BINDING_2D: c_uint = 0x8069;
        let mut active_texture = 0;
        (self.GetIntegerv)(GL_TEXTURE_BINDING_2D, &raw mut active_texture);
        active_texture as c_uint
    }

    fn resize_texture(&self, texture_id: c_uint, physical_size: PhysicalSize) {
        const GL_UNSIGNED_BYTE: u32 = 0x1401;
        const GL_RGBA8: c_int = 0x8058;
        const GL_RGBA: c_uint = 0x1908;

        let active_texture = self.get_active_texture_id();

        (self.BindTexture)(GL_TEXTURE_2D, texture_id);

        (self.TexImage2D)(
            GL_TEXTURE_2D,
            0,
            GL_RGBA8,
            physical_size.width.0,
            physical_size.height.0,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            std::ptr::null(),
        );

        (self.BindTexture)(GL_TEXTURE_2D, active_texture);
    }
}

pub struct GlRenderData {
    pub context: gdk4::GLContext,
    pub framebuffer_id: c_uint,
    pub texture_id: c_uint,
    pub texture_size: PhysicalSize,
    // pub frame_clock: Option<gdk4::FrameClock>,
    pub gl: GlFuncs,
}

impl Drop for GlRenderData {
    fn drop(&mut self) {
        let gl = &self.gl;

        self.context.make_current();

        (gl.BindFramebuffer)(GL_FRAMEBUFFER, 0);
        (gl.DeleteFramebuffers)(1, &raw const self.framebuffer_id);

        (gl.DeleteTextures)(1, &raw const self.texture_id);

        gdk4::GLContext::clear_current();
    }
}
type DrawFn = Box<dyn Fn(OpenGlDrawData, PhysicalSize)>;

pub struct GlWidgetImpl {
    pub data: RefCell<Option<GlRenderData>>,
    pub do_draw: RefCell<DrawFn>,
    pub on_resize: RefCell<Box<dyn Fn(LogicalSize)>>,
}

impl Default for GlWidgetImpl {
    fn default() -> Self {
        Self {
            data: RefCell::new(None),
            do_draw: RefCell::new(Box::new(|_data, _size| {})),
            on_resize: RefCell::new(Box::new(|_size| {})),
        }
    }
}

#[glib::object_subclass]
impl gdk4::subclass::prelude::ObjectSubclass for GlWidgetImpl {
    /// cbindgen:ignore
    const NAME: &'static str = "GlWidget";
    type Type = GlWidget;
    type ParentType = gtk4::Widget;

    fn new() -> Self {
        Self::default()
    }
}

impl gdk4::subclass::prelude::ObjectImpl for GlWidgetImpl {}
impl gtk4::subclass::widget::WidgetImpl for GlWidgetImpl {
    fn realize(&self) {
        self.parent_realize();
        let obj = self.obj();

        let native = obj.native().expect("Cannot get native widget");
        let surface = native.surface().expect("Cannot get a surface of a native widget");
        let context = surface.create_gl_context().expect("Cannot create a GTK OpenGL context");

        let gl_gen_textures: extern "C" fn(c_int, *mut c_uint) = get_egl_f("glGenTextures").unwrap();
        let gl_gen_framebuffers: extern "C" fn(c_int, *mut c_uint) = get_egl_f("glGenFramebuffers").unwrap();

        let mut texture_id = 0;
        gl_gen_textures(1, &raw mut texture_id);
        dbg!(texture_id);

        let mut framebuffer_id = 0;
        gl_gen_framebuffers(1, &raw mut framebuffer_id);
        dbg!(framebuffer_id);

        let gl = GlFuncs::new();

        let scale = f64::from(surface.scale_factor());
        let logical_size = LogicalSize {
            width: obj.width(),
            height: obj.height(),
        };
        let physical_size = logical_size.to_physical(scale);
        gl.resize_texture(texture_id, physical_size);

        let data = GlRenderData {
            context,
            framebuffer_id,
            texture_id,
            texture_size: physical_size,
            gl,
        };
        *self.data.borrow_mut() = Some(data);
    }

    fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
        self.parent_size_allocate(width, height, baseline);
        dbg!(width, height);
        let on_resize = self.on_resize.borrow();
        on_resize(LogicalSize { width, height });
    }

    fn snapshot(&self, snapshot: &gtk4::Snapshot) {
        const GL_FRAMEBUFFER_COMPLETE: c_uint = 0x8CD5;
        const GL_COLOR_ATTACHMENT0: c_uint = 0x8CE0;

        let mut data = self.data.borrow_mut();
        let data = data.as_mut().unwrap();
        let gl = &data.gl;

        let context = &data.context;

        let obj = self.obj();

        let scale = f64::from(obj.scale_factor());
        let logical_size = LogicalSize {
            width: obj.width(),
            height: obj.height(),
        };
        let physical_size = logical_size.to_physical(scale);

        context.make_current();

        if data.texture_size != physical_size {
            data.texture_size = physical_size;
            gl.resize_texture(data.texture_id, physical_size);
        }

        (gl.BindFramebuffer)(GL_FRAMEBUFFER, data.framebuffer_id);
        (gl.FramebufferTexture2D)(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, data.texture_id, 0);

        let framebuffer_status = (gl.CheckFramebufferStatus)(GL_FRAMEBUFFER);
        assert_eq!(framebuffer_status, GL_FRAMEBUFFER_COMPLETE);

        {
            let do_draw = self.do_draw.borrow();
            do_draw(
                OpenGlDrawData {
                    framebuffer: data.framebuffer_id,
                    is_es: context.uses_es(),
                },
                physical_size,
            );
        }

        let gl_texture = unsafe { gdk4::GLTexture::new(context, data.texture_id, physical_size.width.0, physical_size.height.0) };
        snapshot.append_texture(&gl_texture, &logical_size.into());
    }

    fn unrealize(&self) {
        debug!("GlWidget::unrealize");
        *self.data.borrow_mut() = None;
        *self.do_draw.borrow_mut() = Box::new(|_data, _size| {});
        *self.on_resize.borrow_mut() = Box::new(|_size| {});
        self.parent_unrealize();
    }
}

glib::wrapper! {
    pub struct GlWidget(ObjectSubclass<GlWidgetImpl>)
    @extends gtk4::Widget,
    @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl GlWidget {
    pub fn new(do_draw: DrawFn, on_resize: Box<dyn Fn(LogicalSize)>, min_size: Option<LogicalSize>) -> Self {
        let obj = glib::Object::new::<Self>();
        let imp = obj.imp();
        *imp.do_draw.borrow_mut() = do_draw;
        *imp.on_resize.borrow_mut() = on_resize;
        if let Some(min_size) = min_size {
            obj.set_size_request(min_size.width, min_size.height);
        }

        obj
    }
}
