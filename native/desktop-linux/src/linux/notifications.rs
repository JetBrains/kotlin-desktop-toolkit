#![allow(clippy::too_many_arguments)]

use anyhow::Context as _;
use futures_lite::StreamExt;
use log::debug;
use std::collections::HashMap;
use zbus::{proxy, zvariant::Value};

#[proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
pub trait Notifications {
    /// Send a notification to the notification server.
    ///
    /// # Arguments
    ///
    /// * `app_name` - The optional name of the application sending the notification.
    ///    Can be blank.
    ///
    /// * `replaces_id` - The optional notification ID that this notification replaces.
    ///   The server must atomically (i.e., with no flicker or other visual cues)
    ///   replace the given notification with this one. This allows clients to
    ///   effectively modify the notification while it's active. A value of 0
    ///   means that this notification won't replace any existing
    ///   notifications.
    ///
    /// * `app_icon` - The optional program icon of the calling application. See Icons and Images.
    ///   Can be an empty string, indicating no icon.
    ///
    /// * `summary` - The summary text briefly describing the notification.
    ///
    /// * `body` - The optional detailed body text. Can be empty.
    ///
    /// * `actions` - Actions are sent over as a list of pairs.
    ///   Each even element in the list (starting at index 0) represents the identifier for the action.
    ///   Each odd element in the list is the localized string that will be displayed to the user.
    ///
    /// * `hints` - Optional hints that can be passed to the server from the client program.
    ///   Although clients and servers should never assume each other supports any specific hints,
    ///   they can be used to pass along information, such as the process PID or window ID,
    ///   that the server may be able to make use of.
    ///   See Hints. Can be empty.
    ///
    /// * `expire_timeout` - The timeout time in milliseconds since the display of the
    ///   notification at which the notification should automatically close.
    ///   If -1, the notification's expiration time is dependent on the
    ///   notification server's settings, and may vary for the type of
    ///   notification. If 0, never expire.
    ///
    /// If `replaces_id` is 0, the return value is a UINT32 that represents the notification.
    /// It is unique, and will not be reused unless a MAXINT number of notifications have
    /// been generated. An acceptable implementation may just use an incrementing counter for
    /// the ID. The returned ID is always greater than zero. Servers must make sure not to
    /// return zero as an ID.
    ///
    /// If `replaces_id` is not 0, the returned value is the same value as `replaces_id`.
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<&str>,
        hints: HashMap<&str, Value<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;

    /// Causes a notification to be forcefully closed and removed from the user's view.
    /// It can be used, for example, in the event that what the notification pertains to
    /// is no longer relevant, or to cancel a notification with no expiration time.
    ///
    /// The `NotificationClosed` signal is emitted by this method.
    ///
    /// If the notification no longer exists, an empty D-BUS Error message is sent back.
    fn close_notification(&self, id: u32) -> zbus::Result<()>;

    /// A completed notification is one that has timed out, or has been dismissed by the user.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the notification that was closed.
    /// * `reason` - The reason the notification was closed.
    ///              1 - The notification expired.
    ///              2 - The notification was dismissed by the user.
    ///              3 - The notification was closed by a call to `CloseNotification`.
    ///              4 - Undefined/reserved reasons.
    ///
    /// The ID specified in the signal is invalidated before the signal is sent and is
    /// no longer valid. Clients should remove any references to the ID.
    #[zbus(signal)]
    fn notification_closed(&self, id: u32, reason: u32) -> zbus::Result<()>;

    /// This signal is emitted when one of the following occurs:
    ///
    /// - The user performs some global "invoking" action upon a notification. For instance,
    ///   clicking somewhere on the notification itself.
    /// - The user invokes a specific action as specified in the original Notify request.
    ///   For instance, clicking on an action button.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the notification emitting the `ActionInvoked` signal.
    /// * `action_key` - The key of the action invoked. These match the keys sent over
    ///                  in the list of actions.
    #[zbus(signal)]
    fn action_invoked(&self, id: u32, action_key: String) -> zbus::Result<()>;

    /// This signal can be emitted before the `ActionInvoked` signal. It carries
    /// an activation token that can be used to activate a toplevel.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the notification emitting the `ActivationToken` signal.
    /// * `activation_token` - The activation token.
    #[zbus(signal)]
    fn activation_token(&self, id: u32, activation_token: String) -> zbus::Result<()>;
}

pub async fn show_notification_async(
    conn: &zbus::Connection,
    summary: &str,
    body: &str,
    sound_file_path: Option<String>,
) -> anyhow::Result<u32> {
    let proxy = NotificationsProxy::new(conn).await?;

    let app_name = ""; // auto
    let app_icon = ""; // auto
    let replaces_id = 0; // no replacement
    let expire_timeout = -1; // auto
    let actions = vec!["default", ""];
    let mut hints = HashMap::new();
    if let Some(sound_file) = sound_file_path {
        hints.insert("sound-file", sound_file.into());
    }

    let notification_id = proxy
        .notify(app_name, replaces_id, app_icon, summary, body, actions, hints, expire_timeout)
        .await?;
    Ok(notification_id)
}

pub async fn close_notification_async(conn: &zbus::Connection, notification_id: u32) -> anyhow::Result<()> {
    let proxy = NotificationsProxy::new(conn).await?;
    proxy.close_notification(notification_id).await?;
    Ok(())
}

pub struct NotificationData {
    pub id: u32,
    pub activation_token: Option<String>,
}

impl NotificationData {
    const fn new(id: u32) -> Self {
        Self {
            id,
            activation_token: None,
        }
    }
}

pub async fn notifications_receiver(
    conn: zbus::Connection,
    sender: impl Fn(NotificationData) -> anyhow::Result<()> + Send + Sync,
) -> anyhow::Result<()> {
    let proxy = NotificationsProxy::new(&conn).await?;
    let mut stream = proxy.inner().receive_all_signals().await?;

    let mut last_notification_data: Option<NotificationData> = None;
    while let Some(msg) = stream.next().await {
        match msg.header().member().map(zbus::names::MemberName::as_str) {
            // Optional, called before "ActionInvoked"
            Some("ActivationToken") => {
                let signal = ActivationToken::from_message(msg).context("Convert to ActivationToken")?;
                let args = signal.args()?;
                let activation_token = args.activation_token;
                debug!("Notification activation token: {activation_token}");

                last_notification_data = Some(NotificationData {
                    id: args.id,
                    activation_token: Some(activation_token),
                });
            }
            // Optional
            Some("ActionInvoked") => {
                let signal = ActionInvoked::from_message(msg).context("Convert to ActionInvoked")?;
                let args = signal.args()?;
                let id = args.id;
                let action_key = args.action_key;
                debug!("Notification action invoked: id={id}, action_key={action_key}");
                let data = last_notification_data.take().unwrap_or_else(|| NotificationData::new(args.id));
                assert_eq!(data.id, args.id);
                sender(data)?;
            }
            Some("NotificationClosed") => {
                let signal = NotificationClosed::from_message(msg).context("Convert to NotificationClosed")?;
                let args = signal.args()?;
                let id = args.id;
                let reason = args.reason;
                debug!("Notification closed: id={id}, reason={reason}");
                let data = last_notification_data.take().unwrap_or_else(|| NotificationData::new(args.id));
                assert_eq!(data.id, args.id);
                sender(data)?;
            }
            _ => {
                debug!("Received unknown signal: {msg:?}");
            }
        }
    }

    Ok(())
}
