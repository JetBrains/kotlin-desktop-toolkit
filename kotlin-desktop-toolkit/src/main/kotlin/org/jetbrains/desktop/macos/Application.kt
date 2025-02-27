package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeApplicationCallbacks
import org.jetbrains.desktop.macos.generated.NativeApplicationConfig
import org.jetbrains.desktop.macos.generated.NativeEventHandler
import org.jetbrains.desktop.macos.generated.NativeTextContextHandler
import org.jetbrains.desktop.macos.generated.NativeTextOperationHandler
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public enum class EventHandlerResult {
    Continue,
    Stop,
}

public typealias EventHandler = (Event) -> EventHandlerResult
public typealias TextOperationHandler = (TextOperation) -> Boolean

public object Application {
    public data class ApplicationConfig(
        val disableDictationMenuItem: Boolean = false,
        val disableCharacterPaletteMenuItem: Boolean = false,
    ) {
        internal fun toNative(arena: Arena): MemorySegment {
            val config = NativeApplicationConfig.allocate(arena)
            NativeApplicationConfig.disable_dictation_menu_item(config, disableDictationMenuItem)
            NativeApplicationConfig.disable_character_palette_menu_item(config, disableCharacterPaletteMenuItem)
            return config
        }
    }

    private var eventHandler: EventHandler? = null
    private var textOperationHandler: TextOperationHandler? = null
    private var textContextHandler: TextContextHandler? = null
    public lateinit var screens: AllScreens

    public fun init(applicationConfig: ApplicationConfig = ApplicationConfig()) {
        ffiDownCall {
            Arena.ofConfined().use { arena ->
                desktop_macos_h.application_init(applicationConfig.toNative(arena), applicationCallbacks())
            }
        }
    }

    public fun runEventLoop(eventHandler: EventHandler) {
        ffiDownCall {
            this.eventHandler = eventHandler
            desktop_macos_h.application_run_event_loop()
        }
    }

    public fun setTextOperationHandler(textOperationHandler: TextOperationHandler) {
        this.textOperationHandler = textOperationHandler
    }

    public fun setTextContextHandler(textContextHandler: TextContextHandler) {
        this.textContextHandler = textContextHandler
    }

    public fun stopEventLoop() {
        ffiDownCall {
            desktop_macos_h.application_stop_event_loop()
        }
    }

    public fun requestTermination() {
        ffiDownCall {
            desktop_macos_h.application_request_termination()
        }
    }

    // called from native
    private fun onShouldTerminate(): Boolean {
        Logger.info { "onShouldTerminate" }
        return ffiUpCall(default = false) {
            // todo send event to request user interaction?
            true
        }
    }

    // called from native
    private fun onWillTerminate() {
        Logger.info { "onWillTerminate" }
        // This method will never be executed because
        // the application halt is performed immediately after that
        // which means that JVM shutdown hooks might be interupted
        ffiUpCall {
        }
    }

    private fun runEventHandler(event: Event): EventHandlerResult {
        return eventHandler?.let { eventHandler ->
            eventHandler(event)
        } ?: run {
            Logger.warn { "eventHandler is null; event: $event was ignored!" }
            EventHandlerResult.Continue
        }
    }

    // called from native
    private fun onEvent(nativeEvent: MemorySegment): Boolean {
        return ffiUpCall(default = false) {
            val event = Event.fromNative(nativeEvent)
            when (event) {
                is Event.ApplicationDidFinishLaunching -> {
                    screens = Screen.allScreens()
                }
                is Event.DisplayConfigurationChange -> {
                    screens = Screen.allScreens()
                }
                else -> {}
            }
            val result = runEventHandler(event)
            when (result) {
                EventHandlerResult.Continue -> false
                EventHandlerResult.Stop -> true
            }
        }
    }

    private fun onTextOperation(nativeOperation: MemorySegment): Boolean {
        val operation = TextOperation.fromNative(nativeOperation)
        return ffiUpCall(default = false) {
            textOperationHandler?.invoke(operation)
        } ?: run {
            Logger.warn { "textOperationHandler is null; event: $operation was ignored!" }
            false
        }
    }

    private fun onTextContextGetSelectedRange(nativeArgs: MemorySegment): MemorySegment {
        val operation = GetSelectedRangeArgs.fromNative(nativeArgs)
        val result = ffiUpCall(default = null) {
            textContextHandler?.getSelectedRange(operation)
        } ?: run {
            Logger.warn { "textContextHandler is null; event: $operation was ignored!" }
            GetSelectedRangeResult(range = TextRange(location = 0, length = 0))
        }
        return result.toNative(Arena.global())
    }

    private fun onTextContextFirstRectForCharacterRange(nativeArgs: MemorySegment): MemorySegment {
        val operation = FirstRectForCharacterRangeArgs.fromNative(nativeArgs)
        val result = ffiUpCall(default = null) {
            textContextHandler?.firstRectForCharacterRange(operation)
        } ?: run {
            Logger.warn { "textContextHandler is null; event: $operation was ignored!" }
            FirstRectForCharacterRangeResult(x = 0.0, y = 0.0, w = 0.0, h = 0.0)
        }
        return result.toNative(Arena.global())
    }

    private fun textContextCallbacks(): MemorySegment {
        val arena = Arena.global()
        val textContextHandler = NativeTextContextHandler.allocate(arena)
        NativeTextContextHandler.get_selected_range(
            textContextHandler,
            NativeTextContextHandler.get_selected_range.allocate(::onTextContextGetSelectedRange, arena),
        )
        NativeTextContextHandler.first_rect_for_character_range(
            textContextHandler,
            NativeTextContextHandler.first_rect_for_character_range.allocate(::onTextContextFirstRectForCharacterRange, arena),
        )
        return textContextHandler
    }

    private fun applicationCallbacks(): MemorySegment {
        val arena = Arena.global()
        val callbacks = NativeApplicationCallbacks.allocate(arena)
        NativeApplicationCallbacks.on_should_terminate(
            callbacks,
            NativeApplicationCallbacks.on_should_terminate.allocate(::onShouldTerminate, arena),
        )
        NativeApplicationCallbacks.on_will_terminate(
            callbacks,
            NativeApplicationCallbacks.on_will_terminate.allocate(::onWillTerminate, arena),
        )
        NativeApplicationCallbacks.event_handler(callbacks, NativeEventHandler.allocate(::onEvent, arena))
        NativeApplicationCallbacks.text_operation_handler(callbacks, NativeTextOperationHandler.allocate(::onTextOperation, arena))
        NativeApplicationCallbacks.text_context_handler(callbacks, textContextCallbacks())
        return callbacks
    }
}
