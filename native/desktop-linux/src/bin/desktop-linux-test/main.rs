use desktop_common::{
    ffi_utils::BorrowedStrPtr,
    logger_api::{LogLevel, LoggerConfiguration, logger_init_impl},
};
use desktop_linux::linux::{
    application::{Application, ApplicationCallbacks, WindowParams, application_init, application_run_event_loop},
    events::Event,
    window_api::window_create,
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

extern "C" fn event_handler_1(event: &Event) -> bool {
    dbg!(event);
    true
}

extern "C" fn event_handler_2(event: &Event) -> bool {
    dbg!(event);
    true
}

pub fn main() {
    logger_init_impl(&LoggerConfiguration {
        file_path: BorrowedStrPtr::new(c"/tmp/a"),
        console_level: LogLevel::Debug,
        file_level: LogLevel::Error,
    });
    let app_ptr = application_init(ApplicationCallbacks {
        on_should_terminate,
        on_will_terminate,
        on_display_configuration_change,
    });
    window_create(app_ptr.clone(), event_handler_1, WindowParams { width: 200, height: 300 });
    window_create(app_ptr.clone(), event_handler_2, WindowParams { width: 300, height: 200 });
    application_run_event_loop(app_ptr);
}

pub fn main2() {
    logger_init_impl(&LoggerConfiguration {
        file_path: BorrowedStrPtr::new(c"/tmp/a"),
        console_level: LogLevel::Debug,
        file_level: LogLevel::Error,
    });
    let mut app = Application::new(ApplicationCallbacks {
        on_should_terminate,
        on_will_terminate,
        on_display_configuration_change,
    })
    .unwrap();
    app.new_window(event_handler_1, &WindowParams { width: 200, height: 300 });
    app.new_window(event_handler_2, &WindowParams { width: 300, height: 200 });
    app.run();
}
