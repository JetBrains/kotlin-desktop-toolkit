use crate::wayland_virtual_devices_lib::{
    MouseButtonData, MouseMoveData, MouseScrollData, RawKeyCommandData, TestHelper, TestHelperCommand,
};
use log::{debug, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::{borrow::Cow, collections::HashMap, str::FromStr};
use tiny_http::{Request, Response, StatusCode};
use url::Url;

fn console_appender() -> log4rs::append::console::ConsoleAppender {
    log4rs::append::console::ConsoleAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "[{d(%Y%m%d %H:%M:%S%.3f)} {h({l:5})} {M}:{L}] {m}{n}",
        )))
        .target(log4rs::append::console::Target::Stderr)
        .build()
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stderr = console_appender();
    let config = log4rs::Config::builder()
        .appender(log4rs::config::Appender::builder().build("stderr", Box::new(stderr)))
        .build(log4rs::config::Root::builder().appender("stderr").build(log::LevelFilter::Debug))?;
    log4rs::init_config(config)?;

    let (commands_sender, command_receiver) = channel();
    let test_helper = TestHelper::new()?;

    let server = Arc::new(tiny_http::Server::http("0.0.0.0:8000").unwrap());
    let should_stop = Arc::new(AtomicBool::new(false));

    {
        let server_weak = Arc::downgrade(&server);
        let should_stop = should_stop.clone();
        let commands_sender = commands_sender.clone();
        ctrlc::set_handler(move || {
            debug!("Received Ctrl+C");
            if let Some(server) = server_weak.upgrade() {
                should_stop.store(true, Ordering::SeqCst);
                _ = commands_sender.send(None);
                server.unblock();
            }
        })?;
    }

    std::thread::scope(|s| {
        s.spawn(|| {
            if let Err(e) = server_thread(&server, &should_stop, &commands_sender) {
                warn!("Error in server thread: {e}");
            }
        });
        test_helper.run(&command_receiver);
    });

    Ok(())
}

type UrlParams<'a> = HashMap<Cow<'a, str>, Cow<'a, str>>;

fn get_opt<T: FromStr>(params: &mut UrlParams, name: &str) -> Option<T>
where
    <T as FromStr>::Err: std::fmt::Debug,
{
    params
        .remove(name)
        .map(|s| s.parse::<T>().unwrap_or_else(|_| panic!("Error parsing {name}: {s}")))
}

fn get<T: FromStr>(params: &mut UrlParams, name: &str) -> T
where
    <T as FromStr>::Err: std::fmt::Debug,
{
    get_opt(params, name).unwrap_or_else(|| panic!("Missing field {name}"))
}

fn check_params(params: UrlParams, request: Request) -> Option<Request> {
    if params.is_empty() {
        Some(request)
    } else {
        let unknown_keys = params.into_keys().collect::<Vec<_>>().join(", ");
        let msg = format!("Bad request: unknown query params: {unknown_keys}\n");
        request
            .respond(Response::from_string(msg).with_status_code(StatusCode(400)))
            .unwrap();
        None
    }
}

#[allow(clippy::significant_drop_tightening, clippy::too_many_lines)]
fn server_thread(
    server: &tiny_http::Server,
    should_stop: &AtomicBool,
    sender: &std::sync::mpsc::Sender<Option<TestHelperCommand>>,
) -> Result<(), Box<dyn std::error::Error>> {
    while !should_stop.load(Ordering::SeqCst) {
        let request = server.recv()?;
        debug!("{}", request.url());
        let url = Url::try_from("http://127.0.0.1")?.join(request.url())?;
        let path = url.path();
        let mut params = url.query_pairs().collect::<HashMap<_, _>>();
        if path == "/exit" {
            sender.send(None)?;
            request.respond(Response::empty(StatusCode(200)))?;
            return Ok(());
        }
        let command = match path {
            "/raw_key" => {
                let keycode = get(&mut params, "keycode");
                let direction = get::<u8>(&mut params, "direction");
                check_params(params, request).map(|request| {
                    TestHelperCommand::RawKey(
                        RawKeyCommandData {
                            keycode,
                            down: direction == 1,
                        },
                        Box::new(move |success| {
                            if success {
                                request.respond(Response::empty(StatusCode(200)))
                            } else {
                                request.respond(Response::from_string("Virtual keyboard not initialized").with_status_code(StatusCode(501)))
                            }
                            .unwrap();
                        }),
                    )
                })
            }
            "/mousemove" => {
                let x = get(&mut params, "x");
                let y = get(&mut params, "y");
                check_params(params, request).map(|request| {
                    TestHelperCommand::MouseMove(
                        MouseMoveData { x, y },
                        Box::new(move |success| {
                            if success {
                                request.respond(Response::empty(StatusCode(200)))
                            } else {
                                request.respond(Response::from_string("Error uppercasing").with_status_code(StatusCode(501)))
                            }
                            .unwrap();
                        }),
                    )
                })
            }
            "/mousebutton" => {
                let button = get(&mut params, "button");
                let direction = get::<u8>(&mut params, "direction");
                check_params(params, request).map(|request| {
                    TestHelperCommand::MouseButton(
                        MouseButtonData {
                            button,
                            down: direction == 1,
                        },
                        Box::new(move |success| {
                            if success {
                                request.respond(Response::empty(StatusCode(200)))
                            } else {
                                request.respond(Response::from_string("Error uppercasing").with_status_code(StatusCode(501)))
                            }
                            .unwrap();
                        }),
                    )
                })
            }
            "/mousescroll" => {
                let axis_source = get(&mut params, "axis_source");
                let vertical_scroll_120 = get(&mut params, "vertical_scroll_120");
                let horizontal_scroll_120 = get(&mut params, "horizontal_scroll_120");
                check_params(params, request).map(|request| {
                    TestHelperCommand::MouseScroll(
                        MouseScrollData {
                            axis_source,
                            vertical_scroll_120,
                            horizontal_scroll_120,
                        },
                        Box::new(move |success| {
                            if success {
                                request.respond(Response::empty(StatusCode(200)))
                            } else {
                                request.respond(Response::from_string("Error uppercasing").with_status_code(StatusCode(501)))
                            }
                            .unwrap();
                        }),
                    )
                })
            }
            _ => {
                warn!("Unknown command URL: {path}");
                request.respond(Response::empty(StatusCode(404)))?;
                continue;
            }
        };
        if let Some(command) = command {
            sender.send(Some(command))?;
        }
    }
    Ok(())
}
