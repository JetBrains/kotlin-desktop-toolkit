use crate::macos::notifications::NotificationCenterState;
use desktop_common::ffi_utils::{AutoDropStrPtr, BorrowedArray, BorrowedStrPtr};
use desktop_common::logger::ffi_boundary;
use objc2_foundation::MainThreadMarker;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationStatus {
    NotDetermined = 0,
    Denied = 1,
    Authorized = 2,
    Provisional = 3,
    Ephemeral = 4,
}

#[unsafe(no_mangle)]
pub extern "C" fn notification_request_authorization(request_id: AuthorizationRequestId) {
    ffi_boundary("notification_request_authorization", || {
        let mtm = MainThreadMarker::new().unwrap();
        NotificationCenterState::request_authorization(mtm, request_id)?;
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn notification_get_authorization_status(request_id: StatusRequestId) {
    ffi_boundary("notification_get_authorization_status", || {
        let mtm = MainThreadMarker::new().unwrap();
        NotificationCenterState::get_authorization_status(mtm, request_id)?;
        Ok(())
    });
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationSoundType {
    Default = 0,
    None = 1,
    Critical = 2,
    Ringtone = 3,
    Named = 4,
    CriticalNamed = 5,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorizationRequestId(i64);
pub type NotificationAuthorizationCallback = extern "C" fn(request_id: AuthorizationRequestId, granted: bool);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusRequestId(i64);
pub type NotificationStatusCallback = extern "C" fn(request_id: StatusRequestId, status: AuthorizationStatus);

pub type NotificationDeliveryCallback = extern "C" fn(notification_identifier: AutoDropStrPtr, error_message: AutoDropStrPtr);
pub type NotificationActionCallback = extern "C" fn(action_identifier: AutoDropStrPtr, notification_identifier: AutoDropStrPtr);

#[repr(C)]
#[derive(Debug)]
pub struct NotificationCallbacks {
    pub on_authorization_request: NotificationAuthorizationCallback,
    pub on_authorization_status_request: NotificationStatusCallback,
    pub on_delivery: NotificationDeliveryCallback,
    pub on_action: NotificationActionCallback,
}

// FFI-safe struct for notification configuration
#[repr(C)]
#[derive(Debug)]
pub struct NotificationRequest<'a> {
    pub identifier: BorrowedStrPtr<'a>,
    pub title: BorrowedStrPtr<'a>,
    pub body: BorrowedStrPtr<'a>,
    pub sound_type: NotificationSoundType,
    pub sound_name: BorrowedStrPtr<'a>,
    pub category_identifier: BorrowedStrPtr<'a>,
}

#[unsafe(no_mangle)]
pub extern "C" fn notification_show(request: &NotificationRequest) {
    ffi_boundary("notification_show", || {
        let mtm = MainThreadMarker::new().unwrap();
        NotificationCenterState::show_notification(mtm, request)?;
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn notifications_init(callbacks: NotificationCallbacks) -> bool {
    ffi_boundary("notifications_init", || {
        let mtm = MainThreadMarker::new().unwrap();
        NotificationCenterState::init(mtm, callbacks)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn notifications_deinit() {
    ffi_boundary("notifications_deinit", || {
        let mtm = MainThreadMarker::new().unwrap();
        NotificationCenterState::deinit(mtm)?;
        Ok(())
    });
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotificationActionOptionsFlags {
    pub foreground: bool,
    pub destructive: bool,
    pub authentication_required: bool,
}

#[repr(C)]
#[derive(Debug)]
pub struct NotificationAction<'a> {
    pub identifier: BorrowedStrPtr<'a>,
    pub title: BorrowedStrPtr<'a>,
    pub options: NotificationActionOptionsFlags,
}

#[repr(C)]
#[derive(Debug)]
pub struct NotificationCategory<'a> {
    pub category_id: BorrowedStrPtr<'a>,
    pub actions: BorrowedArray<'a, NotificationAction<'a>>,
}

#[unsafe(no_mangle)]
pub extern "C" fn register_notification_categories(categories: BorrowedArray<'_, NotificationCategory<'_>>) {
    ffi_boundary("register_notification_categories", || {
        NotificationCenterState::register_categories(&categories)?;
        Ok(())
    });
}
