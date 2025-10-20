package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeBorrowedArray_NotificationAction
import org.jetbrains.desktop.macos.generated.NativeBorrowedArray_NotificationCategory
import org.jetbrains.desktop.macos.generated.NativeNotificationAction
import org.jetbrains.desktop.macos.generated.NativeNotificationActionCallback
import org.jetbrains.desktop.macos.generated.NativeNotificationActionOptionsFlags
import org.jetbrains.desktop.macos.generated.NativeNotificationAuthorizationCallback
import org.jetbrains.desktop.macos.generated.NativeNotificationCallbacks
import org.jetbrains.desktop.macos.generated.NativeNotificationCategory
import org.jetbrains.desktop.macos.generated.NativeNotificationDeliveryCallback
import org.jetbrains.desktop.macos.generated.NativeNotificationRequest
import org.jetbrains.desktop.macos.generated.NativeNotificationStatusCallback
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

/**
 * Authorization status for system notifications.
 */
public enum class AuthorizationStatus {
    /** The user has not yet been asked for notification permissions. */
    NotDetermined,

    /** The user has denied notification permissions. */
    Denied,

    /** The user has authorized notifications. */
    Authorized,

    /** Provisional authorization (notifications delivered quietly). */
    Provisional,

    /** Ephemeral authorization (temporary, for app clips). */
    Ephemeral,
    ;

    internal companion object {
        internal fun fromNative(value: Int): AuthorizationStatus {
            return when (value) {
                desktop_macos_h.NativeAuthorizationStatus_NotDetermined() -> NotDetermined
                desktop_macos_h.NativeAuthorizationStatus_Denied() -> Denied
                desktop_macos_h.NativeAuthorizationStatus_Authorized() -> Authorized
                desktop_macos_h.NativeAuthorizationStatus_Provisional() -> Provisional
                desktop_macos_h.NativeAuthorizationStatus_Ephemeral() -> Ephemeral
                else -> throw Error("Unexpected AuthorizationStatus variant $value")
            }
        }
    }

    internal fun toNative(): Int {
        return when (this) {
            NotDetermined -> desktop_macos_h.NativeAuthorizationStatus_NotDetermined()
            Denied -> desktop_macos_h.NativeAuthorizationStatus_Denied()
            Authorized -> desktop_macos_h.NativeAuthorizationStatus_Authorized()
            Provisional -> desktop_macos_h.NativeAuthorizationStatus_Provisional()
            Ephemeral -> desktop_macos_h.NativeAuthorizationStatus_Ephemeral()
        }
    }
}

/**
 * Sound options for system notifications.
 */
public sealed class NotificationSound {
    /** Play the default system notification sound. */
    public data object Default : NotificationSound()

    /** No sound will be played. */
    public data object None : NotificationSound()

    /** Play a critical alert sound that bypasses Do Not Disturb and mute switch. */
    public data object Critical : NotificationSound()

    /** Play the default ringtone sound. */
    public data object Ringtone : NotificationSound()

    /** Play a named sound file from the app bundle. */
    public data class Named(val soundName: String) : NotificationSound()

    /** Play a named sound file as a critical alert that bypasses Do Not Disturb. */
    public data class CriticalNamed(val soundName: String) : NotificationSound()

    internal fun toNativeType(): Int {
        return when (this) {
            is Default -> desktop_macos_h.NativeNotificationSoundType_Default()
            is None -> desktop_macos_h.NativeNotificationSoundType_None()
            is Critical -> desktop_macos_h.NativeNotificationSoundType_Critical()
            is Ringtone -> desktop_macos_h.NativeNotificationSoundType_Ringtone()
            is Named -> desktop_macos_h.NativeNotificationSoundType_Named()
            is CriticalNamed -> desktop_macos_h.NativeNotificationSoundType_CriticalNamed()
        }
    }

    internal fun getSoundName(): String {
        return when (this) {
            is Named -> soundName
            is CriticalNamed -> soundName
            else -> ""
        }
    }
}

/**
 * Represents an action button on a notification.
 *
 * @param actionId Unique identifier for this action
 * @param title The text displayed on the action button
 * @param isForeground If true, brings app to foreground when clicked
 * @param isDestructive If true, displays the action in a destructive style (red)
 * @param requiresAuthentication If true, requires device unlock before executing
 */
public data class NotificationAction(
    val actionId: NotificationCenter.ActionId,
    val title: String,
    val isForeground: Boolean = true,
    val isDestructive: Boolean = false,
    val requiresAuthentication: Boolean = false,
)

public data class NotificationCategory(
    val categoryId: NotificationCenter.CategoryId,
    val actions: List<NotificationAction>,
)

/**
 * macOS System Notification Center API.
 *
 * Provides access to the native notification system on macOS 10.14+.
 */
public object NotificationCenter : AutoCloseable {
    @JvmInline
    public value class NotificationId public constructor(public val value: String)

    @JvmInline
    public value class CategoryId public constructor(public val value: String)

    @JvmInline
    public value class ActionId public constructor(public val value: String)

    /**
     * This category is always registered and have no actions in it.
     * We need it because we want to be notified when notifications is dismissed
     * See `notifications.rs` for details
     */
    public val DefaultCategory: CategoryId = CategoryId("com.jetbrains.kdt.DefaultCategory")

    // Triggered when cross closes notification
    public val DismissAction: ActionId = ActionId("com.apple.UNNotificationDismissActionIdentifier")

    // Triggered when the user clicked on notification
    public val DefaultAction: ActionId = ActionId("com.apple.UNNotificationDefaultActionIdentifier")

    init {
        initializeCallbacks()
    }

    /**
     * It might be inaccessible on Mac OS 10.13 and lower.
     * Also, the application should be packaged in an application image with valid Info.plist.
     */
    public var isSupportedByApplication: Boolean = false
        private set

    private fun initializeCallbacks() {
        globalArena = Arena.ofConfined()
        authorizationCallbackStub = NativeNotificationAuthorizationCallback.allocate(::onRequestAuthorizationResult, globalArena)
        statusCallbackStub = NativeNotificationStatusCallback.allocate(::onAuthorizationStatus, globalArena)
        deliveryCallbackStub = NativeNotificationDeliveryCallback.allocate(::onNotificationDeliveryComplete, globalArena)
        actionCallbackStub = NativeNotificationActionCallback.allocate(::onActionResponse, globalArena)

        isSupportedByApplication = ffiDownCall {
            val callbacksSegment = globalArena.allocate(NativeNotificationCallbacks.layout())
            NativeNotificationCallbacks.on_authorization_request(callbacksSegment, authorizationCallbackStub)
            NativeNotificationCallbacks.on_authorization_status_request(callbacksSegment, statusCallbackStub)
            NativeNotificationCallbacks.on_delivery(callbacksSegment, deliveryCallbackStub)
            NativeNotificationCallbacks.on_action(callbacksSegment, actionCallbackStub)

            desktop_macos_h.notifications_init(callbacksSegment)
        }
    }

    /**
     * Requests authorization to display notifications.
     *
     * This will show a system dialog asking the user for permission to send notifications.
     * The request includes alert, sound, and badge options.
     *
     * @param onResult Callback invoked with the result (true if granted, false if denied)
     */
    public fun requestAuthorization(onResult: (Boolean) -> Unit) {
        val requestId = nextId()
        requestAuthorizationCallbacks[requestId] = onResult
        try {
            ffiDownCall {
                desktop_macos_h.notification_request_authorization(requestId)
            }
        } catch (e: Throwable) {
            requestAuthorizationCallbacks.remove(requestId)
            throw e
        }
    }

    /**
     * Gets the current authorization status for notifications.
     *
     * @param onResult Callback invoked with the current authorization status
     */
    public fun getAuthorizationStatus(onResult: (AuthorizationStatus) -> Unit) {
        val requestId = nextId()
        getAuthorizationStatusCallbacks[requestId] = onResult
        try {
            ffiDownCall {
                desktop_macos_h.notification_get_authorization_status(requestId)
            }
        } catch (e: Throwable) {
            getAuthorizationStatusCallbacks.remove(requestId)
            throw e
        }
    }

    public fun registerNotificationCategories(
        categories: List<NotificationCategory>,
        onNotificationAction: (NotificationId, ActionId) -> Unit,
    ) {
        actionResponseCallback = onNotificationAction
        ffiDownCall {
            Arena.ofConfined().use { arena ->
                // Allocate array of NativeNotificationCategory structs
                val categoriesArray = arena.allocateArray(NativeNotificationCategory.layout(), categories.size.toLong())

                categories.forEachIndexed { categoryIndex, category ->
                    val categorySegment = categoriesArray.asSlice(
                        categoryIndex * NativeNotificationCategory.layout().byteSize(),
                        NativeNotificationCategory.layout(),
                    )

                    // Allocate category ID
                    val categoryIdPtr = arena.allocateUtf8String(category.categoryId.value)
                    NativeNotificationCategory.category_id(categorySegment, categoryIdPtr)

                    // Allocate array of NativeNotificationAction structs for this category
                    val actionsArray = arena.allocateArray(NativeNotificationAction.layout(), category.actions.size.toLong())
                    category.actions.forEachIndexed { actionIndex, action ->
                        val actionSegment = actionsArray.asSlice(
                            actionIndex * NativeNotificationAction.layout().byteSize(),
                            NativeNotificationAction.layout(),
                        )

                        val actionIdPtr = arena.allocateUtf8String(action.actionId.value)
                        val actionTitlePtr = arena.allocateUtf8String(action.title)

                        NativeNotificationAction.identifier(actionSegment, actionIdPtr)
                        NativeNotificationAction.title(actionSegment, actionTitlePtr)

                        // Set action options
                        val optionsSegment = NativeNotificationAction.options(actionSegment)
                        NativeNotificationActionOptionsFlags.foreground(optionsSegment, action.isForeground)
                        NativeNotificationActionOptionsFlags.destructive(optionsSegment, action.isDestructive)
                        NativeNotificationActionOptionsFlags.authentication_required(optionsSegment, action.requiresAuthentication)
                    }

                    // Create BorrowedArray for actions
                    val borrowedActionsArray = arena.allocate(NativeBorrowedArray_NotificationAction.layout())
                    NativeBorrowedArray_NotificationAction.ptr(borrowedActionsArray, actionsArray)
                    NativeBorrowedArray_NotificationAction.len(borrowedActionsArray, category.actions.size.toLong())
                    NativeBorrowedArray_NotificationAction.deinit(borrowedActionsArray, MemorySegment.NULL)

                    // Set the actions array on the category
                    NativeNotificationCategory.actions(categorySegment, borrowedActionsArray)
                }

                // Create BorrowedArray for categories
                val borrowedCategoriesArray = arena.allocate(NativeBorrowedArray_NotificationCategory.layout())
                NativeBorrowedArray_NotificationCategory.ptr(borrowedCategoriesArray, categoriesArray)
                NativeBorrowedArray_NotificationCategory.len(borrowedCategoriesArray, categories.size.toLong())
                NativeBorrowedArray_NotificationCategory.deinit(borrowedCategoriesArray, MemorySegment.NULL)

                // Register the categories
                desktop_macos_h.register_notification_categories(borrowedCategoriesArray)
            }
        }
    }

    /**
     * Shows a notification.
     *
     * @param title The notification title
     * @param body The notification body text
     * @param notificationId The unique identifier for this notification
     * @param sound The sound to play with the notification
     * @param categoryId The category identifier for action buttons (use empty string for no actions)
     * @param onComplete Callback invoked when the notification is delivered or fails.
     *                   The callback receives null on success, or an error message string on failure.
     */
    public fun showNotification(
        title: String,
        body: String,
        sound: NotificationSound = NotificationSound.Default,
        notificationId: NotificationId,
        categoryId: CategoryId = DefaultCategory,
        onComplete: (error: String?) -> Unit,
    ) {
        showNotificationCallbacks[notificationId] = onComplete

        try {
            ffiDownCall {
                Arena.ofConfined().use { arena ->
                    val identifierPtr = arena.allocateUtf8String(notificationId.value)
                    val titlePtr = arena.allocateUtf8String(title)
                    val bodyPtr = arena.allocateUtf8String(body)
                    val soundNamePtr = arena.allocateUtf8String(sound.getSoundName())
                    val categoryIdPtr = arena.allocateUtf8String(categoryId.value)

                    // Allocate and populate the NotificationRequest struct
                    val requestSegment = arena.allocate(NativeNotificationRequest.layout())
                    NativeNotificationRequest.identifier(requestSegment, identifierPtr)
                    NativeNotificationRequest.title(requestSegment, titlePtr)
                    NativeNotificationRequest.body(requestSegment, bodyPtr)
                    NativeNotificationRequest.sound_type(requestSegment, sound.toNativeType())
                    NativeNotificationRequest.sound_name(requestSegment, soundNamePtr)
                    NativeNotificationRequest.category_identifier(requestSegment, categoryIdPtr)

                    desktop_macos_h.notification_show(requestSegment)
                }
            }
        } catch (e: Throwable) {
            showNotificationCallbacks.remove(notificationId)
            throw e
        }
    }

    /**
     * Removes a previously delivered notification from the notification center.
     *
     * @param notificationId The unique identifier of the notification to remove
     */
    public fun removeNotification(notificationId: NotificationId) {
        ffiDownCall {
            Arena.ofConfined().use { arena ->
                val identifierPtr = arena.allocateUtf8String(notificationId.value)
                desktop_macos_h.notification_remove(identifierPtr)
            }
        }
    }

    /**
     * Closes the NotificationCenter and releases native callback resources.
     *
     * After calling close(), the NotificationCenter cannot be used anymore.
     * This is typically called during application shutdown.
     */
    override fun close() {
        assert(::globalArena.isInitialized)
        // Clean up native notification resources (delegate and categories)
        ffiDownCall {
            desktop_macos_h.notifications_deinit()
        }
        requestAuthorizationCallbacks.clear()
        getAuthorizationStatusCallbacks.clear()
        showNotificationCallbacks.clear()
        globalArena.close()
    }

    // private
    private lateinit var globalArena: Arena

    private lateinit var authorizationCallbackStub: MemorySegment
    private lateinit var statusCallbackStub: MemorySegment
    private lateinit var deliveryCallbackStub: MemorySegment
    private lateinit var actionCallbackStub: MemorySegment

    private val requestAuthorizationCallbacks = mutableMapOf<Long, (granted: Boolean) -> Unit>()
    private val getAuthorizationStatusCallbacks = mutableMapOf<Long, (AuthorizationStatus) -> Unit>()
    private val showNotificationCallbacks = mutableMapOf<NotificationId, (error: String?) -> Unit>()

    private var actionResponseCallback: (NotificationId, ActionId) -> Unit = { notificationId, actionId -> }

    private var idCounter: Long = 0

    private fun nextId(): Long = idCounter++

    // Called from native when authorization request completes
    private fun onRequestAuthorizationResult(requestId: Long, granted: Boolean) {
        ffiUpCall {
            requestAuthorizationCallbacks.remove(requestId)?.invoke(granted)
                ?: Logger.error { "onRequestAuthorizationResult: no callback registered for request $requestId" }
        }
    }

    // Called from native when authorization status is retrieved
    private fun onAuthorizationStatus(requestId: Long, status: Int) {
        ffiUpCall {
            getAuthorizationStatusCallbacks.remove(requestId)?.invoke(AuthorizationStatus.fromNative(status))
                ?: Logger.error { "onAuthorizationStatus: no callback registered for request $requestId" }
        }
    }

    // Called from native when notification delivery completes
    private fun onNotificationDeliveryComplete(notificationIdentifier: MemorySegment, errorMessage: MemorySegment) {
        ffiUpCall {
            val notificationId = NotificationId(notificationIdentifier.getUtf8String(0))
            val error = if (errorMessage.address() == 0L) {
                null
            } else {
                errorMessage.getUtf8String(0)
            }
            showNotificationCallbacks.remove(notificationId)?.invoke(error)
                ?: Logger.error { "onNotificationDeliveryComplete: no callback registered for notification $notificationId" }
        }
    }

    // Called from native when user clicks an action button
    private fun onActionResponse(actionIdPtr: MemorySegment, notificationIdPtr: MemorySegment) {
        ffiUpCall {
            val actionId = ActionId(actionIdPtr.getUtf8String(0))
            val notificationId = NotificationId(notificationIdPtr.getUtf8String(0))
            actionResponseCallback.invoke(notificationId, actionId)
        }
    }
}
