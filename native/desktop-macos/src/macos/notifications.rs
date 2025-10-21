use super::notifications_api::{
    AuthorizationRequestId, AuthorizationStatus, NotificationAction, NotificationActionCallback, NotificationActionOptionsFlags,
    NotificationCallbacks, NotificationCategory, NotificationDeliveryCallback, NotificationRequest, NotificationSoundType, StatusRequestId,
};
use crate::macos::bundle_proxy::LSBundleProxy;
use crate::macos::string::{copy_to_c_string, copy_to_ns_string};
use anyhow::{Context, ensure};
use block2::RcBlock;
use desktop_common::ffi_utils::{BorrowedArray, BorrowedStrPtr, RustAllocatedStrPtr};
use desktop_common::logger::catch_panic;
use dispatch2::DispatchQueue;
use objc2::__framework_prelude::{Bool, Retained};
use objc2::{DeclaredClass, MainThreadMarker, MainThreadOnly, define_class};
use objc2_foundation::{NSArray, NSError, NSObject, NSObjectProtocol, NSSet, NSString};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNAuthorizationStatus, UNMutableNotificationContent, UNNotificationAction, UNNotificationActionOptions,
    UNNotificationCategory, UNNotificationCategoryOptions, UNNotificationRequest, UNNotificationResponse, UNNotificationSettings,
    UNNotificationSound, UNUserNotificationCenter, UNUserNotificationCenterDelegate,
};
use std::cell::RefCell;
use std::ptr::NonNull;

thread_local! {
    static NOTIFICATION_CENTER_STATE: RefCell<Option<NotificationCenterState>> = const { RefCell::new(None) };
}

pub struct NotificationCenterState {
    center: Retained<UNUserNotificationCenter>,
    _delegate: Retained<NotificationCenterDelegate>,
    callbacks: NotificationCallbacks,
}

impl NotificationCenterState {
    pub fn init(mtm: MainThreadMarker, callbacks: NotificationCallbacks) -> anyhow::Result<bool> {
        ensure!(
            NOTIFICATION_CENTER_STATE.with_borrow(std::option::Option::is_none),
            "Can't initialize a second time"
        );
        match get_notification_center(mtm) {
            Some(center) => {
                let delegate = NotificationCenterDelegate::new(mtm, callbacks.on_action);
                unsafe {
                    center.setDelegate(Some(objc2::runtime::ProtocolObject::from_ref(&*delegate)));
                }
                NOTIFICATION_CENTER_STATE.set(Some(Self {
                    center,
                    _delegate: delegate,
                    callbacks,
                }));
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub fn deinit(_mtm: MainThreadMarker) -> anyhow::Result<()> {
        let state = NOTIFICATION_CENTER_STATE.replace(None).context("Already deinitialized")?;
        unsafe { state.center.setDelegate(None) };
        unsafe {
            state.center.setNotificationCategories(&NSSet::new());
        }
        Ok(())
    }

    fn with_state<T>(f: impl FnOnce(&Self) -> anyhow::Result<T>) -> anyhow::Result<T> {
        NOTIFICATION_CENTER_STATE.with_borrow(|state| {
            state.as_ref().context("Notification center is not initialized")?;
            f(state.as_ref().unwrap())
        })
    }

    pub fn request_authorization(_mtm: MainThreadMarker, request_id: AuthorizationRequestId) -> anyhow::Result<()> {
        Self::with_state(|state| {
            let center = &state.center;
            let callback = state.callbacks.on_authorization_request;
            let options = UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound | UNAuthorizationOptions::Badge;
            let handler = RcBlock::new(move |granted: Bool, _error: *mut NSError| {
                let granted = granted.as_bool();
                DispatchQueue::main().exec_async(move || {
                    callback(request_id, granted);
                });
            });
            unsafe {
                center.requestAuthorizationWithOptions_completionHandler(options, &handler);
            }
            Ok(())
        })
    }

    pub fn get_authorization_status(_mtm: MainThreadMarker, request_id: StatusRequestId) -> anyhow::Result<()> {
        Self::with_state(|state| {
            let center = &state.center;
            let callback = state.callbacks.on_authorization_status_request;
            let handler = RcBlock::new(move |settings: NonNull<UNNotificationSettings>| {
                let settings = unsafe { settings.as_ref() };
                let status: AuthorizationStatus = unsafe { settings.authorizationStatus().into() };
                DispatchQueue::main().exec_async(move || {
                    catch_panic(|| {
                        callback(request_id, status);
                        Ok(())
                    });
                });
            });

            unsafe {
                center.getNotificationSettingsWithCompletionHandler(&handler);
            }
            Ok(())
        })
    }

    pub fn register_categories(categories: &BorrowedArray<'_, NotificationCategory<'_>>) -> anyhow::Result<()> {
        Self::with_state(|state| {
            let categories: anyhow::Result<Vec<Retained<UNNotificationCategory>>> =
                categories.as_slice()?.iter().map(NotificationCategory::unpack_category).collect();
            let mut categories = categories?;
            let default_category = unsafe {
                UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
                    &NSString::from_str("com.jetbrains.kdt.DefaultCategory"),
                    &NSArray::new(),
                    &NSArray::new(), // Empty intent identifiers
                    UNNotificationCategoryOptions::CustomDismissAction,
                )
            };
            categories.push(default_category);
            unsafe {
                state.center.setNotificationCategories(&NSSet::from_retained_slice(&categories));
            }
            Ok(())
        })
    }

    pub fn show_notification(_mtm: MainThreadMarker, request: &NotificationRequest) -> anyhow::Result<()> {
        Self::with_state(|state| {
            let content = unsafe { UNMutableNotificationContent::new() };
            let title = request.unpack_title()?;
            let body = request.unpack_body()?;
            let sound = request.unpack_sound()?;
            let action_identifier = request.unpack_action_identifier()?;
            let category_identifier = request.unpack_category_identifier()?;
            unsafe {
                content.setTitle(&title);
                content.setBody(&body);
                content.setSound(sound.as_ref().map(std::convert::AsRef::as_ref));
                content.setCategoryIdentifier(&category_identifier);
            }
            let request = unsafe {
                UNNotificationRequest::requestWithIdentifier_content_trigger(
                    &action_identifier,
                    &content,
                    None, // nil trigger means deliver immediately
                )
            };
            let delivery_handler = Self::create_notification_delivery_handler(action_identifier, state.callbacks.on_delivery);
            unsafe {
                state
                    .center
                    .addNotificationRequest_withCompletionHandler(&request, Some(&delivery_handler));
            }
            Ok(())
        })
    }

    fn create_notification_delivery_handler(
        notification_identifier: Retained<NSString>,
        callback: NotificationDeliveryCallback,
    ) -> RcBlock<dyn Fn(*mut NSError)> {
        RcBlock::new(move |error: *mut NSError| {
            catch_panic(|| {
                let error_msg = if error.is_null() {
                    RustAllocatedStrPtr::null().to_auto_drop()
                } else {
                    let error_ref = unsafe { &*error };
                    copy_to_c_string(&error_ref.localizedDescription())
                        .expect("Error converting to c string")
                        .to_auto_drop()
                };
                let identifier = copy_to_c_string(&notification_identifier)
                    .expect("Error converting to c string")
                    .to_auto_drop();
                DispatchQueue::main().exec_async(move || {
                    catch_panic(|| {
                        callback(identifier, error_msg);
                        Ok(())
                    });
                });
                Ok(())
            });
        })
    }

    pub fn remove_notification(identifier: &BorrowedStrPtr) -> anyhow::Result<()> {
        Self::with_state(|state| {
            let notification_id = copy_to_ns_string(identifier)?;
            unsafe {
                let notifications_array = &*NSArray::from_slice(&[&*notification_id]);
                state.center.removePendingNotificationRequestsWithIdentifiers(notifications_array);
                state.center.removeDeliveredNotificationsWithIdentifiers(notifications_array);
            }
            Ok(())
        })
    }
}

#[must_use]
pub fn get_notification_center(_mtm: MainThreadMarker) -> Option<Retained<UNUserNotificationCenter>> {
    // UNUserNotificationCenter requires macOS 10.14+
    // Also requires a valid app bundle with CFBundleIdentifier
    use objc2::exception;
    let result = exception::catch(|| {
        if LSBundleProxy::bundleProxyForCurrentProcess().is_some() {
            Some(unsafe { UNUserNotificationCenter::currentNotificationCenter() })
        } else {
            None
        }
    });
    result.ok().flatten()
}

#[derive(Debug)]
pub(super) struct NotificationDelegateIvars {
    pub(super) on_action: NotificationActionCallback,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "NotificationCenterDelegate"]
    #[ivars = NotificationDelegateIvars]
    #[derive(Debug)]
    pub(super) struct NotificationCenterDelegate;

    unsafe impl NSObjectProtocol for NotificationCenterDelegate {}

    unsafe impl UNUserNotificationCenterDelegate for NotificationCenterDelegate {
        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn did_receive_notification_response(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            completion_handler: &block2::Block<dyn Fn()>,
        ) {
            catch_panic(|| {
                let ns_action_id = unsafe { &response.actionIdentifier() };
                let ns_notification_id = unsafe { &response.notification().request().identifier() };
                let action_id = copy_to_c_string(ns_action_id).unwrap().to_auto_drop();
                let notification_id = copy_to_c_string(ns_notification_id).unwrap().to_auto_drop();
                let callback = self.ivars().on_action;

                if MainThreadMarker::new().is_some() {
                    callback(action_id, notification_id);
                    completion_handler.call(());
                } else {
                    DispatchQueue::main().exec_sync(move || {
                        catch_panic(|| {
                            callback(action_id, notification_id);
                            Ok(())
                        });
                    });
                    completion_handler.call(());
                }
                Ok(())
            });
        }

        // Let's follow macos default behavior, when the app is in foreground
        // #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        // fn will_present_notification(
        //     &self,
        //     _center: &UNUserNotificationCenter,
        //     _notification: &UNNotification,
        //     completion_handler: &block2::Block<dyn Fn(objc2_user_notifications::UNNotificationPresentationOptions)>,
        // ) {
        //     catch_panic(|| {
        //         // Allow notifications to be presented even when app is in foreground
        //         // Show banner, sound, and badge
        //         let options = objc2_user_notifications::UNNotificationPresentationOptions::Banner
        //             | objc2_user_notifications::UNNotificationPresentationOptions::Sound
        //             | objc2_user_notifications::UNNotificationPresentationOptions::Badge;
        //
        //         completion_handler.call((options,));
        //         Ok(())
        //     });
        // }
    }
);

impl NotificationCenterDelegate {
    fn new(mtm: MainThreadMarker, on_action: NotificationActionCallback) -> Retained<Self> {
        let this = mtm.alloc::<Self>();
        let this = this.set_ivars(NotificationDelegateIvars { on_action });
        let delegate: Retained<Self> = unsafe { objc2::msg_send![super(this), init] };
        delegate
    }
}

impl From<UNAuthorizationStatus> for AuthorizationStatus {
    fn from(status: UNAuthorizationStatus) -> Self {
        match status {
            UNAuthorizationStatus::NotDetermined => Self::NotDetermined,
            UNAuthorizationStatus::Denied => Self::Denied,
            UNAuthorizationStatus::Authorized => Self::Authorized,
            UNAuthorizationStatus::Provisional => Self::Provisional,
            UNAuthorizationStatus::Ephemeral => Self::Ephemeral,
            _ => Self::NotDetermined,
        }
    }
}

impl NotificationRequest<'_> {
    fn unpack_action_identifier(&self) -> anyhow::Result<Retained<NSString>> {
        copy_to_ns_string(&self.identifier)
    }

    fn unpack_title(&self) -> anyhow::Result<Retained<NSString>> {
        copy_to_ns_string(&self.title)
    }

    fn unpack_body(&self) -> anyhow::Result<Retained<NSString>> {
        copy_to_ns_string(&self.body)
    }

    fn unpack_sound(&self) -> anyhow::Result<Option<Retained<UNNotificationSound>>> {
        let sound = match self.sound_type {
            NotificationSoundType::Default => Some(unsafe { UNNotificationSound::defaultSound() }),
            NotificationSoundType::None => None,
            NotificationSoundType::Critical => Some(unsafe { UNNotificationSound::defaultCriticalSound() }),
            NotificationSoundType::Ringtone => Some(unsafe { UNNotificationSound::defaultRingtoneSound() }),
            NotificationSoundType::Named => {
                let sound_name = copy_to_ns_string(&self.sound_name)?;
                Some(unsafe { UNNotificationSound::soundNamed(&sound_name) })
            }
            NotificationSoundType::CriticalNamed => {
                let sound_name = copy_to_ns_string(&self.sound_name)?;
                Some(unsafe { UNNotificationSound::criticalSoundNamed(&sound_name) })
            }
        };
        Ok(sound)
    }

    fn unpack_category_identifier(&self) -> anyhow::Result<Retained<NSString>> {
        copy_to_ns_string(&self.category_identifier)
    }
}

#[must_use]
pub fn convert_action_options(flags: NotificationActionOptionsFlags) -> UNNotificationActionOptions {
    let mut options = UNNotificationActionOptions::empty();

    if flags.foreground {
        options |= UNNotificationActionOptions::Foreground;
    }
    if flags.destructive {
        options |= UNNotificationActionOptions::Destructive;
    }
    if flags.authentication_required {
        options |= UNNotificationActionOptions::AuthenticationRequired;
    }

    options
}

impl NotificationAction<'_> {
    fn unpack_action(&self) -> anyhow::Result<Retained<UNNotificationAction>> {
        let action_id = copy_to_ns_string(&self.identifier)?;
        let action_title = copy_to_ns_string(&self.title)?;
        let action_options = convert_action_options(self.options);

        let un_action = unsafe { UNNotificationAction::actionWithIdentifier_title_options(&action_id, &action_title, action_options) };
        Ok(un_action)
    }
}

impl NotificationCategory<'_> {
    fn unpack_category_id(&self) -> anyhow::Result<Retained<NSString>> {
        copy_to_ns_string(&self.category_id)
    }

    fn unpack_actions(&self) -> anyhow::Result<Retained<NSArray<UNNotificationAction>>> {
        let actions: anyhow::Result<Vec<Retained<UNNotificationAction>>> =
            self.actions.as_slice()?.iter().map(NotificationAction::unpack_action).collect();
        let actions = actions?;
        Ok(NSArray::from_retained_slice(&actions))
    }

    fn unpack_category(&self) -> anyhow::Result<Retained<UNNotificationCategory>> {
        let category_id = self.unpack_category_id()?;
        let actions = self.unpack_actions()?;
        let result = unsafe {
            UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
                &category_id,
                &actions,
                &NSArray::new(), // Empty intent identifiers
                UNNotificationCategoryOptions::CustomDismissAction,
            )
        };
        Ok(result)
    }
}
