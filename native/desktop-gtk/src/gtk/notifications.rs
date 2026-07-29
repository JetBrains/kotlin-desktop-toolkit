#![allow(clippy::too_many_arguments)]

use anyhow::anyhow;
use gdk4::prelude::{CancellableExt, DBusProxyExt};
use gdk4::subclass::prelude::{ObjectSubclassExt, ObjectSubclassIsExt};
use gdk4::{gio, glib};
use gtk4::gdk as gdk4;
use log::{debug, warn};
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::fmt::Formatter;
use std::rc::Rc;

pub struct NotificationData {
    pub id: u32,
    pub action: Option<String>,
    pub activation_token: Option<String>,
}

impl NotificationData {
    const fn new(id: u32) -> Self {
        Self {
            id,
            action: None,
            activation_token: None,
        }
    }
}

pub struct NewNotificationData {
    pub summary: String,
    pub body: String,
    pub sound_file_path: Option<String>,
    pub result_reporter: Box<dyn FnOnce(anyhow::Result<u32>) + 'static>,
}

impl std::fmt::Debug for NewNotificationData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NewNotificationData {{ summary: {}, body: {}, sound_file_path: {:?} }}",
            self.summary, self.body, self.sound_file_path
        )
    }
}

#[derive(Debug)]
enum NotificationAction {
    Show(NewNotificationData),
    Close(u32),
}

struct PendingNotificationService {
    actions: Vec<NotificationAction>,
    should_try_connecting: bool,
}

impl PendingNotificationService {
    fn add_action(&mut self, action: NotificationAction) {
        self.actions.push(action);
    }

    fn report_failures(&mut self, error: &anyhow::Error) {
        for action in std::mem::take(&mut self.actions) {
            match action {
                NotificationAction::Show(data) => {
                    (data.result_reporter)(Err(anyhow!("{error}")));
                }
                NotificationAction::Close(_) => {}
            }
        }
    }
}

pub struct NotificationsState {
    last_sent_notification: Cell<u32>,
    current_notification_data: Cell<Option<NotificationData>>,
    on_notification_interaction: Rc<dyn Fn(NotificationData)>,
}

#[derive(Default)]
pub struct DBusNotificationsImpl {
    pub state: OnceCell<NotificationsState>,
}

#[glib::object_subclass]
impl gdk4::subclass::prelude::ObjectSubclass for DBusNotificationsImpl {
    /// cbindgen:ignore
    const NAME: &'static str = "Notifications";
    type Type = DBusNotifications;
    type ParentType = gio::DBusProxy;
}

impl glib::subclass::prelude::ObjectImpl for DBusNotificationsImpl {}
impl gio::subclass::prelude::InitableImpl for DBusNotificationsImpl {}
impl gio::subclass::prelude::AsyncInitableImpl for DBusNotificationsImpl {}
impl gio::subclass::prelude::DBusInterfaceImpl for DBusNotificationsImpl {}

impl gio::subclass::prelude::DBusProxyImpl for DBusNotificationsImpl {
    fn g_signal(&self, _sender_name: Option<&glib::GStr>, signal_name: &glib::GStr, parameters: &glib::Variant) {
        if let Err(e) = self.obj().on_signal(signal_name, parameters) {
            warn!("Error handling notification signal {signal_name}: {e}");
        }
    }
}

glib::wrapper! {
    pub struct DBusNotifications(ObjectSubclass<DBusNotificationsImpl>)
    @extends gio::DBusProxy,
    @implements gio::AsyncInitable, gio::Initable, gio::DBusInterface;
}

impl DBusNotifications {
    pub fn new_async(
        cancellable: gio::Cancellable,
        on_notification_interaction: Rc<dyn Fn(NotificationData)>,
        callback: impl Fn(anyhow::Result<Self>) + 'static,
    ) {
        let state = NotificationsState {
            last_sent_notification: Cell::new(0),
            current_notification_data: Cell::new(None),
            on_notification_interaction,
        };
        gio::bus_get(
            gio::BusType::Session,
            Some(&cancellable.clone()),
            move |connection_res| match connection_res {
                Ok(connection) => {
                    gio::AsyncInitable::builder::<Self>()
                        .property("g-connection", connection)
                        .property("g-flags", gio::DBusProxyFlags::NONE)
                        .property("g-name", "org.freedesktop.Notifications")
                        .property("g-object-path", "/org/freedesktop/Notifications")
                        .property("g-interface-name", "org.freedesktop.Notifications")
                        .build(glib::Priority::DEFAULT, Some(&cancellable), move |res| match res {
                            Ok(obj) => {
                                let imp = obj.imp();
                                imp.state.get_or_init(|| state);
                                callback(Ok(obj));
                            }
                            Err(e) => {
                                callback(Err(anyhow!(e)));
                            }
                        });
                }
                Err(e) => callback(Err(anyhow!(e))),
            },
        );
    }

    fn get_state(&self) -> &NotificationsState {
        self.imp().state.get().unwrap()
    }

    fn on_signal(&self, signal_name: &str, parameters: &glib::Variant) -> anyhow::Result<()> {
        let state = self.get_state();
        match signal_name {
            // Optional, called before "ActionInvoked"
            "ActivationToken" => {
                let (id, activation_token): (u32, String) = parameters.try_get()?;
                debug!("Notification activation token: {activation_token}");

                state.current_notification_data.set(Some(NotificationData {
                    id,
                    action: None,
                    activation_token: Some(activation_token),
                }));
            }
            // Optional
            "ActionInvoked" => {
                let (id, action_key): (u32, String) = parameters.try_get()?;
                debug!("Notification action invoked: id={id}, action_key={action_key}");
                let mut data = state.current_notification_data.take().unwrap_or_else(|| NotificationData::new(id));
                assert_eq!(data.id, id);
                data.action = Some(action_key);
                state.last_sent_notification.set(id);
                (state.on_notification_interaction)(data);
            }
            "NotificationClosed" => {
                let (id, reason): (u32, u32) = parameters.try_get()?;
                debug!("Notification closed: id={id}, reason={reason}");
                if state.last_sent_notification.replace(id) != id {
                    let data = state.current_notification_data.take().unwrap_or_else(|| NotificationData::new(id));
                    assert_eq!(data.id, id);
                    (state.on_notification_interaction)(data);
                }
            }
            _ => {
                debug!("Received unknown signal: {signal_name:?}");
            }
        }

        Ok(())
    }

    fn handle_action(&self, action: NotificationAction, cancellable: &gio::Cancellable) {
        match action {
            NotificationAction::Show(data) => {
                self.show_notification(
                    &data.summary,
                    &data.body,
                    data.sound_file_path.as_deref(),
                    cancellable,
                    move |res| {
                        (data.result_reporter)(res);
                    },
                );
            }
            NotificationAction::Close(notification_id) => {
                self.close_notification(notification_id, cancellable);
            }
        }
    }

    fn show_notification(
        &self,
        summary: &str,
        body: &str,
        sound_file_path: Option<&str>,
        cancellable: &gio::Cancellable,
        callback: impl FnOnce(anyhow::Result<u32>) + 'static,
    ) {
        let app_name = ""; // auto
        let app_icon = ""; // auto
        let replaces_id = 0u32; // no replacement
        let expire_timeout = -1; // auto
        let actions = vec!["default", ""];
        let mut hints = HashMap::new();
        if let Some(sound_file) = sound_file_path {
            hints.insert("sound-file", glib::Variant::from(sound_file));
        }

        self.call(
            "Notify",
            Some(&glib::Variant::from((
                app_name,
                replaces_id,
                app_icon,
                summary,
                body,
                actions,
                hints,
                expire_timeout,
            ))),
            gio::DBusCallFlags::NONE,
            -1,
            Some(cancellable),
            move |call_result| {
                let res = match call_result {
                    Ok(variant) => match variant.try_get() {
                        Ok((notification_id,)) => Ok(notification_id),
                        Err(e) => Err(e.into()),
                    },
                    Err(e) => Err(e.into()),
                };
                callback(res);
            },
        );
    }

    fn close_notification(&self, notification_id: u32, cancellable: &gio::Cancellable) {
        self.call(
            "CloseNotification",
            Some(&glib::Variant::from((notification_id,))),
            gio::DBusCallFlags::NONE,
            -1,
            Some(cancellable),
            move |res| {
                if let Err(e) = res {
                    warn!("Error closing notification: {e}");
                }
            },
        );
    }
}

enum NotificationsServiceState {
    Pending(PendingNotificationService),
    Ready(DBusNotifications),
}

impl Drop for Notifications {
    fn drop(&mut self) {
        debug!("NotificationsState::drop");
        self.cancellable.cancel();
    }
}

pub struct Notifications {
    state: Rc<RefCell<NotificationsServiceState>>,
    cancellable: gio::Cancellable,
    on_notification_interaction: Rc<dyn Fn(NotificationData)>,
}

impl Notifications {
    pub fn new(on_notification_interaction: impl Fn(NotificationData) + Clone + 'static) -> Self {
        let state = Rc::new(RefCell::new(NotificationsServiceState::Pending(PendingNotificationService {
            actions: Vec::new(),
            should_try_connecting: true,
        })));

        let cancellable = gio::Cancellable::new();

        Self {
            state,
            cancellable,
            on_notification_interaction: Rc::new(on_notification_interaction),
        }
    }

    pub fn show_notification(
        &self,
        summary: &str,
        body: &str,
        sound_file_path: Option<&str>,
        callback: impl FnOnce(anyhow::Result<u32>) + 'static,
    ) {
        match &mut *self.state.borrow_mut() {
            NotificationsServiceState::Pending(pending) => {
                pending.add_action(NotificationAction::Show(NewNotificationData {
                    summary: summary.to_owned(),
                    body: body.to_owned(),
                    sound_file_path: sound_file_path.map(ToOwned::to_owned),
                    result_reporter: Box::new(callback),
                }));
                if pending.should_try_connecting {
                    pending.should_try_connecting = false;
                    let state = Rc::downgrade(&self.state);
                    let cancellable = self.cancellable.clone();
                    DBusNotifications::new_async(cancellable.clone(), self.on_notification_interaction.clone(), move |res| {
                        if let Some(state) = state.upgrade() {
                            let mut borrow = state.borrow_mut();
                            match &mut *borrow {
                                NotificationsServiceState::Pending(pending) => match res {
                                    Ok(ready) => {
                                        for action in std::mem::take(&mut pending.actions) {
                                            ready.handle_action(action, &cancellable);
                                        }

                                        *borrow = NotificationsServiceState::Ready(ready);
                                    }
                                    Err(e) => {
                                        pending.should_try_connecting = true;
                                        pending.report_failures(&e);
                                        warn!("Error initializing notification: {e}");
                                    }
                                },
                                NotificationsServiceState::Ready(_) => {}
                            }
                        } else {
                            warn!("Initialization result after destruction");
                        }
                    });
                }
            }
            NotificationsServiceState::Ready(v) => {
                v.show_notification(summary, body, sound_file_path, &self.cancellable, callback);
            }
        }
    }

    pub fn close_notification(&self, notification_id: u32) {
        match &mut *self.state.borrow_mut() {
            NotificationsServiceState::Pending(v) => {
                v.add_action(NotificationAction::Close(notification_id));
            }
            NotificationsServiceState::Ready(v) => {
                v.close_notification(notification_id, &self.cancellable);
            }
        }
    }
}
