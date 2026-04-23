#![allow(clippy::too_many_arguments)]

use anyhow::anyhow;
use gtk4::prelude::{CancellableExt, DBusProxyExt, DBusProxyExtManual, ObjectExt};
use gtk4::{gio, glib};
use log::{debug, warn};
use std::collections::HashMap;
use std::fmt::Formatter;
use std::sync::{Arc, Mutex};

pub struct NewNotificationData {
    pub summary: String,
    pub body: String,
    pub sound_file_path: Option<String>,
    pub result_reporter: Box<dyn FnOnce(anyhow::Result<u32>) + 'static + Send + Sync>,
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

fn show_notification(
    proxy: &gio::DBusProxy,
    summary: &str,
    body: &str,
    sound_file_path: Option<String>,
    cancellable: &gio::Cancellable,
    callback: impl FnOnce(anyhow::Result<u32>) + 'static,
) {
    let app_name = ""; // auto
    let app_icon = ""; // auto
    let replaces_id = 0u32; // no replacement
    let expire_timeout = -1; // auto
    let actions = vec!["default", ""];
    let mut hints = HashMap::<&str, glib::Variant>::new();
    if let Some(sound_file) = sound_file_path {
        hints.insert("sound-file", sound_file.into());
    }

    proxy.call(
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

fn close_notification(proxy: &gio::DBusProxy, notification_id: u32, cancellable: &gio::Cancellable) {
    proxy.call(
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

#[derive(Debug)]
pub enum NotificationAction {
    Show(NewNotificationData),
    Close(u32),
    Exit,
}

fn on_signal(
    signal_name: &str,
    parameters: &glib::Variant,
    last_sent_notification: &mut u32,
    current_notification_data: &mut Option<NotificationData>,
    callback: &dyn Fn(NotificationData),
) -> anyhow::Result<()> {
    match signal_name {
        // Optional, called before "ActionInvoked"
        "ActivationToken" => {
            let (id, activation_token): (u32, String) = parameters.try_get()?;
            debug!("Notification activation token: {activation_token}");

            *current_notification_data = Some(NotificationData {
                id,
                action: None,
                activation_token: Some(activation_token),
            });
        }
        // Optional
        "ActionInvoked" => {
            let (id, action_key): (u32, String) = parameters.try_get()?;
            debug!("Notification action invoked: id={id}, action_key={action_key}");
            let mut data = current_notification_data.take().unwrap_or_else(|| NotificationData::new(id));
            assert_eq!(data.id, id);
            data.action = Some(action_key);
            *last_sent_notification = id;
            callback(data);
        }
        "NotificationClosed" => {
            let (id, reason): (u32, u32) = parameters.try_get()?;
            debug!("Notification closed: id={id}, reason={reason}");
            if *last_sent_notification != id {
                let data = current_notification_data.take().unwrap_or_else(|| NotificationData::new(id));
                assert_eq!(data.id, id);
                *last_sent_notification = id;
                callback(data);
            }
        }
        _ => {
            debug!("Received unknown signal: {signal_name:?}");
        }
    }
    debug!("Notifications receiver stopped");

    Ok(())
}

fn handle_action(action: NotificationAction, proxy: Option<&gio::DBusProxy>, cancellable: &gio::Cancellable) {
    match action {
        NotificationAction::Show(data) => {
            if let Some(proxy) = proxy {
                show_notification(proxy, &data.summary, &data.body, data.sound_file_path, cancellable, move |res| {
                    (data.result_reporter)(res);
                });
            } else {
                warn!("Could not connect to DBus Notifications");
                (data.result_reporter)(Err(anyhow!("Could not connect to DBus Notifications")));
            }
        }
        NotificationAction::Close(notification_id) => {
            if let Some(proxy) = proxy {
                close_notification(proxy, notification_id, cancellable);
            }
        }
        NotificationAction::Exit => {
            // Handled elsewhere
        }
    }
}

fn do_notifications_action(
    connection_cache: Arc<Mutex<Option<gio::DBusProxy>>>,
    on_notification_interaction: impl Fn(NotificationData) + Clone + Send + Sync + 'static,
    cancellable: gio::Cancellable,
    action: NotificationAction,
) {
    let connection_opt = connection_cache.lock().expect("").clone();
    if let Some(connection) = connection_opt {
        handle_action(action, Some(&connection), &cancellable);
    } else {
        gio::DBusProxy::for_bus(
            gio::BusType::Session,
            gio::DBusProxyFlags::NONE,
            None,
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            Some(&cancellable.clone()),
            move |res| match res {
                Ok(proxy) => {
                    let callback = on_notification_interaction.clone();
                    let state = Mutex::new((0, None));
                    proxy.connect_g_signal(move |_proxy, _sender_name, signal_name, parameters| {
                        let mut guard = state.lock().unwrap();
                        let (last_sent_notification, current_notification_data) = &mut *guard;
                        on_signal(
                            signal_name,
                            parameters,
                            last_sent_notification,
                            current_notification_data,
                            &callback,
                        )
                        .unwrap();
                        drop(guard);
                    });
                    handle_action(action, Some(&proxy), &cancellable);

                    let connection_cache_clone = connection_cache.clone();
                    proxy.connection().connect_closed(move |_c, _remote_peer_vanished, _error| {
                        debug!("Notifications connection closed");
                        *connection_cache_clone.lock().unwrap() = None;
                    });
                    *connection_cache.lock().expect("") = Some(proxy);
                }
                Err(e) => {
                    warn!("Error getting DBus Notifications proxy: {e}");
                    handle_action(action, None, &cancellable);
                }
            },
        );
    }
}

pub async fn init_notifications_task(
    on_notification_interaction: impl Fn(NotificationData) + Clone + Send + Sync + 'static,
    notification_action_receiver: async_channel::Receiver<NotificationAction>,
) {
    let connection_cache: Arc<Mutex<Option<gio::DBusProxy>>> = Arc::default();
    let mut cancellables = Vec::<glib::WeakRef<gio::Cancellable>>::new();
    while let Ok(action) = notification_action_receiver.recv().await {
        if matches!(action, NotificationAction::Exit) {
            for cancellable in cancellables {
                if let Some(cancellable) = cancellable.upgrade() {
                    cancellable.cancel();
                }
            }
            break;
        }
        let cancellable = gio::Cancellable::new();
        cancellables.retain(|c| c.upgrade().is_some());
        cancellables.push(cancellable.downgrade());
        do_notifications_action(connection_cache.clone(), on_notification_interaction.clone(), cancellable, action);
    }
}
