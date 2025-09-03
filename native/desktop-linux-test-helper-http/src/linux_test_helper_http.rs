use std::{borrow::Cow, collections::HashMap, str::FromStr, time::Duration};

use desktop_linux_test_helper::{
    CursorPosition, DeleteSurroundingTextData, InputCommandData, PreeditStringData, RawKeyCommandData, TestHelper, TestHelperCommand,
};
use log::warn;
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

    let mut test_helper = TestHelper::new();
    let commands_sender = test_helper.get_sender();

    let server_thread = std::thread::spawn(move || {
        server_thread(&commands_sender).unwrap();
    });

    test_helper.run(Box::new(|| {}))?;
    server_thread.join().unwrap();
    Ok(())
}

type UrlParams<'a> = HashMap<Cow<'a, str>, Cow<'a, str>>;

fn get_opt<T: FromStr>(params: &mut UrlParams, name: &str) -> Option<T>
where
    <T as FromStr>::Err: std::fmt::Debug,
{
    params
        .remove(name)
        .map(|s| s.parse::<T>().unwrap_or_else(|_| panic!("Error parsing {name}")))
}

fn get<T: FromStr>(params: &mut UrlParams, name: &str) -> T
where
    <T as FromStr>::Err: std::fmt::Debug,
{
    get_opt(params, name).unwrap_or_else(|| panic!("Missing field {name}"))
}

fn get_opt_str(params: &mut UrlParams, name: &str) -> Option<String> {
    params.remove(name).map(Cow::into_owned)
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

fn create_input_command(mut params: UrlParams, request: Request) -> Option<TestHelperCommand> {
    let commit_string: Option<String> = get_opt_str(&mut params, "commit_string");
    let preedit_string: Option<PreeditStringData> = get_opt_str(&mut params, "preedit_string_text").map(|text| {
        let cursor_begin = get(&mut params, "preedit_string_cursor_begin");
        let cursor_end = get(&mut params, "preedit_string_cursor_end");
        PreeditStringData {
            text,
            cursor: CursorPosition::Visible {
                start: cursor_begin,
                end: cursor_end,
            },
        }
    });
    let delete_surrounding_text = get_opt(&mut params, "delete_surrounding_text_before_length").map(|before_length| {
        let after_length = get(&mut params, "delete_surrounding_text_after_length");
        DeleteSurroundingTextData {
            before_length,
            after_length,
        }
    });
    check_params(params, request).map(|request| {
        TestHelperCommand::Input(
            InputCommandData {
                commit_string,
                preedit_string,
                delete_surrounding_text,
            },
            Box::new(|success| {
                if success {
                    request.respond(Response::empty(StatusCode(200)))
                } else {
                    request.respond(Response::from_string("Input method not initialized").with_status_code(StatusCode(501)))
                }
                .unwrap();
            }),
        )
    })
}

fn server_thread(sender: &dyn Fn(TestHelperCommand)) -> Result<(), Box<dyn std::error::Error>> {
    let server = tiny_http::Server::http("0.0.0.0:8000").unwrap();
    let mut should_stop = false;

    while !should_stop {
        if let Some(request) = server.recv_timeout(Duration::from_millis(16))? {
            let url = Url::try_from("http://127.0.0.1")?.join(request.url())?;
            let path = url.path();
            let mut params = url.query_pairs().collect::<HashMap<_, _>>();
            let command = match path {
                "/input" => create_input_command(params, request),
                "/get_input_state" => check_params(params, request).map(|request| {
                    TestHelperCommand::GetInputState(Box::new(move |v| {
                        if let Some(v) = v {
                            let msg = format!("{{\"text\": \"{}\", \"anchor\": {}, \"cursor\": {}}}\n", v.text, v.anchor, v.cursor);
                            request.respond(tiny_http::Response::from_string(msg)).unwrap();
                        } else {
                            request.respond(tiny_http::Response::from_string("{}\n")).unwrap();
                        }
                    }))
                }),
                "/set_keyboard_layout" => {
                    let layout_name = get(&mut params, "name");
                    check_params(params, request).map(|request| {
                        TestHelperCommand::SetKeyboardLayout(
                            layout_name,
                            Box::new(move |success| {
                                if success {
                                    request.respond(Response::empty(StatusCode(200)))
                                } else {
                                    request
                                        .respond(Response::from_string("Error setting keyboard layout").with_status_code(StatusCode(501)))
                                }
                                .unwrap();
                            }),
                        )
                    })
                }
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
                                    request.respond(
                                        Response::from_string("Virtual keyboard not initialized").with_status_code(StatusCode(501)),
                                    )
                                }
                                .unwrap();
                            }),
                        )
                    })
                }
                "/exit" => {
                    should_stop = true;
                    check_params(params, request).map(|request| {
                        TestHelperCommand::Exit(Box::new(|_| {
                            request.respond(Response::empty(StatusCode(200))).unwrap();
                        }))
                    })
                }
                _ => {
                    warn!("Unknown command URL: {path}");
                    request.respond(Response::empty(StatusCode(404)))?;
                    continue;
                }
            };
            if let Some(command) = command {
                sender(command);
            }
        }
    }
    Ok(())
}
