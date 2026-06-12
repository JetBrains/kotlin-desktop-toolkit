use desktop_gtk::gtk::data_transfer_api::DataSource;
use gdk4::prelude::OutputStreamExtManual;
use gtk4::prelude::{
    ApplicationExt, ApplicationExtManual, DisplayExt, DragExt, EventControllerExt, FileExt, GtkWindowExt, InputStreamExtManual, NativeExt,
    WidgetExt,
};
use gtk4::{gdk as gdk4, gio, glib};
use std::cell::RefCell;
use std::future::Future;
use std::io::Write;
use std::path::PathBuf;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::LazyLock;

fn log(msg: &str) {
    let mut stdout = std::io::stdout();
    stdout.write_all(msg.as_bytes()).expect("eprint write_all msg");
    stdout.write_all(b"\n").expect("eprint write_all newline");
    stdout.flush().expect("eprint flush");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationMode {
    Normal,
    Slow,
    ExitImmediately,
    ExitAfterStartWriting,
}

struct AllArgs {
    data_source: DataSource,
    drag_actions: gdk4::DragAction,
    operation_mode: OperationMode,
    file_path_per_mime_type: Vec<(String, PathBuf)>,
}

static ALL_ARGS: LazyLock<AllArgs> = LazyLock::new(|| {
    let usage = "Usage: test_app_data_source <--clipboard <clipboard|primary> | --drag <<copy|move>,...>> <normal|slow> --data <mime_type> <file_path> ...";
    let mut args = std::env::args();
    args.next().expect("arg 0"); // skip the program name

    let mut drag_actions = gdk4::DragAction::empty();
    let data_source = match args.next().expect("data source arg").as_str() {
        "--clipboard" => match args.next().expect("clipboard type arg").as_str() {
            "clipboard" => DataSource::Clipboard,
            "primary" => DataSource::PrimarySelection,
            _ => {
                log(usage);
                std::process::exit(1);
            }
        },
        "--drag" => {
            let drag_actions_str = args.next().expect("drag actions arg");
            for action_str in drag_actions_str.split(',') {
                match action_str {
                    "none" => assert_eq!(action_str, drag_actions_str),
                    "copy" => drag_actions |= gdk4::DragAction::COPY,
                    "move" => drag_actions |= gdk4::DragAction::MOVE,
                    "link" => drag_actions |= gdk4::DragAction::LINK,
                    "ask" => drag_actions |= gdk4::DragAction::ASK,
                    _ => {
                        log(usage);
                        std::process::exit(1);
                    }
                }
            }
            DataSource::DragAndDrop
        }
        _ => {
            log(usage);
            std::process::exit(1);
        }
    };

    let operation_mode = match args.next().expect("operation mode arg").as_str() {
        "normal" => OperationMode::Normal,
        "slow" => OperationMode::Slow,
        "exit-immediately" => OperationMode::ExitImmediately,
        "exit-after-start-writing" => OperationMode::ExitAfterStartWriting,
        _ => {
            log(usage);
            std::process::exit(1);
        }
    };

    let mut file_path_per_mime_type = Vec::new();
    while let Some(arg) = args.next() {
        assert_eq!("--data", arg.as_str());
        let mime_type = args.next().expect("mime type arg");
        let file_path = args.next().unwrap_or_else(|| panic!("file path arg for {mime_type}"));
        file_path_per_mime_type.push((mime_type, PathBuf::from(file_path)));
    }

    AllArgs {
        data_source,
        drag_actions,
        operation_mode,
        file_path_per_mime_type,
    }
});

fn get_file_path_for_mime(mime_type: &str) -> Option<&'static PathBuf> {
    ALL_ARGS
        .file_path_per_mime_type
        .iter()
        .find_map(|e| if e.0 == mime_type { Some(&e.1) } else { None })
}

pub struct TestContentProviderImpl {
    pub formats: gdk4::ContentFormats,
    pub operation_mode: OperationMode,
}

impl TestContentProvider {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

#[glib::object_subclass]
impl gdk4::subclass::prelude::ObjectSubclass for TestContentProviderImpl {
    /// cbindgen:ignore
    const NAME: &'static str = "TestContentProvider";
    type Type = TestContentProvider;
    type ParentType = gdk4::ContentProvider;

    fn new() -> Self {
        let all_args = &*ALL_ARGS;
        let mime_types = all_args.file_path_per_mime_type.iter().map(|e| e.0.as_str()).collect::<Box<_>>();
        let formats = gdk4::ContentFormats::new(&mime_types);
        Self {
            formats,
            operation_mode: all_args.operation_mode,
        }
    }
}

impl gdk4::subclass::prelude::ObjectImpl for TestContentProviderImpl {}
impl gdk4::subclass::content_provider::ContentProviderImpl for TestContentProviderImpl {
    fn formats(&self) -> gdk4::ContentFormats {
        self.formats.clone()
    }

    fn write_mime_type_future(
        &self,
        mime_type: &str,
        stream: &gio::OutputStream,
        io_priority: glib::Priority,
    ) -> Pin<Box<dyn Future<Output = Result<(), glib::Error>> + 'static>> {
        let operation_mode = self.operation_mode;
        if operation_mode == OperationMode::ExitImmediately {
            std::process::exit(0);
        }
        if let Some(file_path) = get_file_path_for_mime(mime_type) {
            let stream = stream.clone();
            Box::pin(async move {
                let f = gio::File::for_path(file_path);
                let read_stream = f.read_future(io_priority).await?;
                loop {
                    let (mut chunk, size) = read_stream.read_future(vec![0; 10], io_priority).await.map_err(|(_, e)| e)?;
                    if size == 0 {
                        break;
                    }
                    chunk.truncate(size);
                    stream.write_future(chunk, io_priority).await.map_err(|(_, e)| e)?;
                    if operation_mode == OperationMode::ExitAfterStartWriting {
                        glib::usleep(100 * 1000); // 100ms
                        std::process::exit(0);
                    }
                    if operation_mode == OperationMode::Slow {
                        log("sleeping");
                        glib::usleep(100 * 1000); // 100ms
                    }
                }
                Ok(())
            })
        } else {
            let e = glib::Error::new(gio::IOErrorEnum::NotSupported, &format!("Mime type {mime_type} not supported"));
            Box::pin(async { Err(e) })
        }
    }
}

glib::wrapper! {
    pub struct TestContentProvider(ObjectSubclass<TestContentProviderImpl>)
    @extends gdk4::ContentProvider;
}

fn build_ui(application: &gtk4::Application) {
    let dnd_in_progress = Rc::new(RefCell::new(false));
    let data_source = ALL_ARGS.data_source;
    let window = gtk4::ApplicationWindow::new(application);

    window.set_title(Some("Test Data Source"));
    {
        let dnd_in_progress = dnd_in_progress.clone();
        window.connect_is_active_notify(move |window| {
            if window.is_active() && !*dnd_in_progress.borrow() {
                log("ready");
            }
        });
    }

    match data_source {
        DataSource::DragAndDrop => {
            let click_gesture = gtk4::GestureClick::new();
            {
                let window = window.clone();
                let dnd_in_progress = dnd_in_progress.clone();
                click_gesture.connect_pressed(move |click_gesture, _n_press, x, y| {
                    let surface = window.surface().expect("window surface");
                    let device = click_gesture.current_event_device().expect("current event device");
                    let content = TestContentProvider::new();
                    let actions = ALL_ARGS.drag_actions;
                    let drag = gdk4::Drag::begin(&surface, &device, &content, actions, x, y).expect("drag begin");
                    *dnd_in_progress.borrow_mut() = true;
                    {
                        let dnd_in_progress = dnd_in_progress.clone();
                        drag.connect_dnd_finished(move |_drag| {
                            *dnd_in_progress.borrow_mut() = false;
                            log("dnd-finished");
                        });
                    }
                });
            }
            window.add_controller(click_gesture);

            let motion_controller = gtk4::EventControllerMotion::new();
            motion_controller.connect_leave(move |_motion_controller| {
                if *dnd_in_progress.borrow() {
                    log("TestAppDragSource drag begin");
                }
            });
            window.add_controller(motion_controller);
        }
        DataSource::Clipboard | DataSource::PrimarySelection => {
            let event_controller_key = gtk4::EventControllerKey::new();

            // This is done for Wayland compatibility: even though it's enough to get the keyboard focus to set the clipboard,
            // GTK uses only key (and pointer) down events:
            // https://github.com/GNOME/gtk/blob/5301a91f1c74764facb4d60f40ab8621dd7af198/gdk/wayland/gdkseat-wayland.c#L4602
            event_controller_key.connect_key_pressed(move |_event_controller_key, _keyval, _keycode, _state| {
                let display = gdk4::Display::default().expect("default display");
                let clipboard = match data_source {
                    DataSource::Clipboard => display.clipboard(),
                    DataSource::PrimarySelection => display.primary_clipboard(),
                    DataSource::DragAndDrop => panic!("unexpected DataSource::DragAndDrop"),
                };
                let content_provider = TestContentProvider::new();
                clipboard.set_content(Some(&content_provider)).expect("clipboard set content");
                log("set clipboard content");
                glib::Propagation::Stop
            });
            window.add_controller(event_controller_key);
        }
    }

    window.present();
}

pub fn main_impl() {
    let application = gtk4::Application::builder()
        .application_id("org.jetbrains.desktop.linux.tests.TestAppDataSource")
        .build();
    glib::set_application_name("Data Source Test App");
    application.connect_activate(build_ui);
    let ret = application.run_with_args::<glib::GString>(&[]);
    assert_eq!(ret, glib::ExitCode::SUCCESS, "{ret:?}");
}
