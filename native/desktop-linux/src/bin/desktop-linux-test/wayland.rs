use desktop_common::{
    ffi_utils::BorrowedStrPtr,
    logger_api::{LogLevel, LoggerConfiguration, logger_init_impl},
};
use desktop_linux::linux::{
    application::{Application, ApplicationCallbacks, WindowParams},
    events::Event,
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
    let mut app = Application::new(ApplicationCallbacks {
        on_should_terminate,
        on_will_terminate,
        on_display_configuration_change,
    })
    .unwrap();
    app.new_window(&WindowParams {
        event_handler: event_handler_1,
        width: 200,
        height: 300,
    });
    app.new_window(&WindowParams {
        event_handler: event_handler_2,
        width: 300,
        height: 200,
    });
    app.run();
}
