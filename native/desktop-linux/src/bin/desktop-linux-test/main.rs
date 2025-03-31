use std::ffi::CStr;

use desktop_common::{
    ffi_utils::BorrowedStrPtr,
    logger_api::{LogLevel, LoggerConfiguration, logger_init_impl},
};
use desktop_linux::linux::{
    application::{ApplicationCallbacks, application_init, application_run_event_loop},
    events::{Event, WindowDrawEvent},
    window::WindowParams,
    window_api::window_create,
    xdg_desktop_settings::XdgDesktopSetting,
};

extern "C" fn on_should_terminate() -> bool {
    println!("on_should_terminate");
    true
}

extern "C" fn on_will_terminate() {
    println!("on_will_terminate");
}

extern "C" fn on_display_configuration_change() {
    println!("on_display_configuration_change");
}

fn between(val: f64, min: f64, max: f64) -> bool {
    val > min && val < max
}

type GLenum = u32;
type GLbitfield = u32;
type GLfloat = f32;

const GL_COLOR_BUFFER_BIT: GLenum = 0x0000_4000;

#[link(kind = "dylib", name = "GL")]
unsafe extern "C" {
    unsafe fn glClearColor(red: GLfloat, green: GLfloat, blue: GLfloat, alpha: GLfloat);
    unsafe fn glClear(mask: GLbitfield);
}

#[allow(clippy::many_single_char_names)]
fn draw(data: &WindowDrawEvent) {
    const BYTES_PER_PIXEL: u8 = 4;
    if data.buffer.is_null() {
        unsafe { glClearColor(0.0, 1.0, 0.0, 1.0) };
        unsafe { glClear(GL_COLOR_BUFFER_BIT) };
        return;
    }
    let canvas = unsafe { std::slice::from_raw_parts_mut(data.buffer, usize::try_from(data.height * data.stride).unwrap()) };
    let w = f64::from(data.width);
    let h = f64::from(data.height);
    let scale = data.scale;
    let line_thickness = 5.0 * scale;

    // Order of bytes in `pixel` is [b, g, r, a] (for the Argb8888 format)
    for (pixel, i) in canvas.chunks_exact_mut(BYTES_PER_PIXEL.into()).zip(1u32..) {
        let i = f64::from(i);
        let x = i % w;
        let y = (i / f64::from(data.stride)) * f64::from(BYTES_PER_PIXEL);
        if between(x, line_thickness,  line_thickness * 2.0)  // left border
           || between(y, line_thickness,  line_thickness * 2.0)  // top border
           || between(x, line_thickness.mul_add(-2.0, w), w - line_thickness)  // right border
           || between(y, line_thickness.mul_add(-2.0, h), h - line_thickness)  // bottom border
           || between(x, (i / h) - (line_thickness / 2.0), (i / h) + (line_thickness / 2.0))
        {
            pixel[0] = 0;
            pixel[1] = 0;
        } else {
            pixel[0] = 255;
            pixel[1] = 255;
        }
        pixel[2] = 255;
        pixel[3] = 255;
    }
}

extern "C" fn event_handler_1(event: &Event) -> bool {
    match event {
        Event::WindowDraw(data) => {
            draw(data);
            return true;
        }
        Event::MouseMoved(_) => {}
        _ => {
            dbg!(event);
        }
    }
    true
}

extern "C" fn event_handler_2(event: &Event) -> bool {
    match event {
        Event::WindowDraw(data) => {
            draw(data);
            return true;
        }
        Event::MouseMoved(_) => {}
        _ => {
            dbg!(event);
        }
    }
    true
}

extern "C" fn on_xdg_desktop_settings_change(s: XdgDesktopSetting) {
    dbg!(s);
}

pub fn main() {
    const APP_ID: &CStr = c"org.jetbrains.desktop.linux.native.sample1";
    logger_init_impl(&LoggerConfiguration {
        file_path: BorrowedStrPtr::new(c"/tmp/a"),
        console_level: LogLevel::Debug,
        file_level: LogLevel::Error,
    });
    let app_ptr = application_init(ApplicationCallbacks {
        on_should_terminate,
        on_will_terminate,
        on_display_configuration_change,
        on_xdg_desktop_settings_change,
    });
    window_create(
        app_ptr.clone(),
        event_handler_1,
        WindowParams {
            width: 200,
            height: 300,
            title: BorrowedStrPtr::new(c"Window 1"),
            app_id: BorrowedStrPtr::new(APP_ID),
            force_client_side_decoration: false,
            force_software_rendering: true,
        },
    );
    window_create(
        app_ptr.clone(),
        event_handler_2,
        WindowParams {
            width: 300,
            height: 200,
            title: BorrowedStrPtr::new(c"Window 2"),
            app_id: BorrowedStrPtr::new(APP_ID),
            force_client_side_decoration: true,
            force_software_rendering: false,
        },
    );
    application_run_event_loop(app_ptr);
}
