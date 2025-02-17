package org.jetbrains.desktop.macos

import org.jetbrains.desktop.LogicalPixels
import org.jetbrains.desktop.LogicalPoint
import org.jetbrains.desktop.LogicalSize
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import org.jetbrains.desktop.macos.generated.WindowBackground as NativeWindowBackground
import org.jetbrains.desktop.macos.generated.WindowParams as NativeWindowParams

typealias WindowId = Long

class Window internal constructor(ptr: MemorySegment) : Managed(ptr, desktop_macos_h::window_drop) {
    data class WindowParams(
        val origin: LogicalPoint = LogicalPoint(0.0, 0.0),
        val size: LogicalSize = LogicalSize(640.0, 480.0),
        val title: String = "Window",
        val isResizable: Boolean = true,
        val isClosable: Boolean = true,
        val isMiniaturizable: Boolean = true,
        val isFullScreenAllowed: Boolean = true,
        val useCustomTitlebar: Boolean = false,
        val titlebarHeight: LogicalPixels = 28.0,
    ) {
        internal fun toNative(arena: Arena): MemorySegment {
            val nativeWindowParams = NativeWindowParams.allocate(arena)
            NativeWindowParams.origin(nativeWindowParams, origin.toNative(arena))
            NativeWindowParams.size(nativeWindowParams, size.toNative(arena))
            NativeWindowParams.title(nativeWindowParams, arena.allocateUtf8String(title))

            NativeWindowParams.is_resizable(nativeWindowParams, isResizable)
            NativeWindowParams.is_closable(nativeWindowParams, isClosable)
            NativeWindowParams.is_miniaturizable(nativeWindowParams, isMiniaturizable)
            NativeWindowParams.is_full_screen_allowed(nativeWindowParams, isFullScreenAllowed)
            NativeWindowParams.use_custom_titlebar(nativeWindowParams, useCustomTitlebar)
            NativeWindowParams.titlebar_height(nativeWindowParams, titlebarHeight)
            return nativeWindowParams
        }
    }

    companion object {
        fun create(params: WindowParams): Window {
            return Arena.ofConfined().use { arena ->
                Window(ffiDownCall { desktop_macos_h.window_create(params.toNative(arena)) })
            }
        }

        fun create(
            origin: LogicalPoint = LogicalPoint(0.0, 0.0),
            size: LogicalSize = LogicalSize(640.0, 480.0),
            title: String = "Window",
            isResizable: Boolean = true,
            isClosable: Boolean = true,
            isMiniaturizable: Boolean = true,
            isFullScreenAllowed: Boolean = true,
            useCustomTitlebar: Boolean = false,
        ): Window {
            return create(
                WindowParams(
                    origin,
                    size,
                    title,
                    isResizable,
                    isClosable,
                    isMiniaturizable,
                    isFullScreenAllowed,
                    useCustomTitlebar,
                ),
            )
        }
    }

    fun windowId(): WindowId {
        return ffiDownCall { desktop_macos_h.window_get_window_id(pointer) }
    }

    fun screenId(): ScreenId {
        return ffiDownCall {
            desktop_macos_h.window_get_screen_id(pointer)
        }
    }

    fun scaleFactor(): Double {
        return ffiDownCall { desktop_macos_h.window_scale_factor(pointer) }
    }

    var title: String
        get() {
            val title = ffiDownCall { desktop_macos_h.window_get_title(pointer) }
            return try {
                title.getUtf8String(0)
            } finally {
                ffiDownCall { desktop_macos_h.string_drop(title) }
            }
        }
        set(value) {
            Arena.ofConfined().use { arena ->
                val title = arena.allocateUtf8String(value)
                ffiDownCall { desktop_macos_h.window_set_title(pointer, title) }
            }
        }

    val origin: LogicalPoint
        get() {
            return Arena.ofConfined().use { arena ->
                LogicalPoint.fromNative(ffiDownCall { desktop_macos_h.window_get_origin(arena, pointer) })
            }
        }

    val size: LogicalSize
        get() {
            return Arena.ofConfined().use { arena ->
                LogicalSize.fromNative(ffiDownCall { desktop_macos_h.window_get_size(arena, pointer) })
            }
        }

    val contentOrigin: LogicalPoint
        get() {
            return Arena.ofConfined().use { arena ->
                LogicalPoint.fromNative(ffiDownCall { desktop_macos_h.window_get_content_origin(arena, pointer) })
            }
        }

    val contentSize: LogicalSize
        get() {
            return Arena.ofConfined().use { arena ->
                LogicalSize.fromNative(ffiDownCall { desktop_macos_h.window_get_content_size(arena, pointer) })
            }
        }

    var maxSize: LogicalSize
        get() {
            return Arena.ofConfined().use { arena ->
                LogicalSize.fromNative(ffiDownCall { desktop_macos_h.window_get_max_size(arena, pointer) })
            }
        }
        set(value) {
            Arena.ofConfined().use { arena ->
                ffiDownCall {
                    desktop_macos_h.window_set_max_size(pointer, value.toNative(arena))
                }
            }
        }

    var minSize: LogicalSize
        get() {
            return Arena.ofConfined().use { arena ->
                LogicalSize.fromNative(ffiDownCall { desktop_macos_h.window_get_min_size(arena, pointer) })
            }
        }
        set(value) {
            Arena.ofConfined().use { arena ->
                ffiDownCall { desktop_macos_h.window_set_min_size(pointer, value.toNative(arena)) }
            }
        }

    val isFullScreen: Boolean
        get() {
            return ffiDownCall { desktop_macos_h.window_is_full_screen(pointer) }
        }

    fun toggleFullScreen() {
        ffiDownCall {
            desktop_macos_h.window_toggle_full_screen(pointer)
        }
    }

    val isKey: Boolean
        get() {
            return ffiDownCall { desktop_macos_h.window_is_key(pointer) }
        }

    val isMain: Boolean
        get() {
            return ffiDownCall { desktop_macos_h.window_is_main(pointer) }
        }

    fun setRect(origin: LogicalPoint, size: LogicalSize, animateTransition: Boolean = true) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_macos_h.window_set_rect(
                    pointer,
                    origin.toNative(arena),
                    size.toNative(arena),
                    animateTransition,
                )
            }
        }
    }

    fun setContentRect(origin: LogicalPoint, size: LogicalSize, animateTransition: Boolean = true) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_macos_h.window_set_content_rect(
                    pointer,
                    origin.toNative(arena),
                    size.toNative(arena),
                    animateTransition,
                )
            }
        }
    }

    fun startDrag() {
        ffiDownCall {
            desktop_macos_h.window_start_drag(pointer)
        }
    }

    fun invalidateShadow() {
        ffiDownCall {
            desktop_macos_h.window_invalidate_shadow(pointer)
        }
    }

    fun attachView(layer: MetalView) {
        ffiDownCall {
            desktop_macos_h.window_attach_layer(pointer, layer.pointer)
        }
    }

    fun setBackground(background: WindowBackground) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_macos_h.window_set_background(pointer, background.toNative(arena))
            }
        }
    }
}

sealed class WindowBackground {
    data object Transparent : WindowBackground()
    data class SolidColor(val color: Color) : WindowBackground()

    data class VisualEffect(val effect: WindowVisualEffect) : WindowBackground()

    internal fun toNative(arena: Arena): MemorySegment {
        val result = NativeWindowBackground.allocate(arena)
        when (this) {
            is Transparent -> {
                NativeWindowBackground.tag(result, desktop_macos_h.WindowBackground_Transparent())
            }
            is SolidColor -> {
                NativeWindowBackground.tag(result, desktop_macos_h.WindowBackground_SolidColor())
                NativeWindowBackground.solid_color(result, color.toNative(arena))
            }
            is VisualEffect -> {
                NativeWindowBackground.tag(result, desktop_macos_h.WindowBackground_VisualEffect())
                NativeWindowBackground.visual_effect(result, effect.toNative())
            }
        }
        return result
    }
}

enum class WindowVisualEffect {
    TitlebarEffect,
    SelectionEffect,
    MenuEffect,
    PopoverEffect,
    SidebarEffect,
    HeaderViewEffect,
    SheetEffect,
    WindowBackgroundEffect,
    HUDWindowEffect,
    FullScreenUIEffect,
    ToolTipEffect,
    ContentBackgroundEffect,
    UnderWindowBackgroundEffect,
    UnderPageBackgroundEffect,
    ;

    internal fun toNative(): Int {
        return when (this) {
            TitlebarEffect -> desktop_macos_h.WindowVisualEffect_TitlebarEffect()
            SelectionEffect -> desktop_macos_h.WindowVisualEffect_SelectionEffect()
            MenuEffect -> desktop_macos_h.WindowVisualEffect_MenuEffect()
            PopoverEffect -> desktop_macos_h.WindowVisualEffect_PopoverEffect()
            SidebarEffect -> desktop_macos_h.WindowVisualEffect_SidebarEffect()
            HeaderViewEffect -> desktop_macos_h.WindowVisualEffect_HeaderViewEffect()
            SheetEffect -> desktop_macos_h.WindowVisualEffect_SheetEffect()
            WindowBackgroundEffect -> desktop_macos_h.WindowVisualEffect_WindowBackgroundEffect()
            HUDWindowEffect -> desktop_macos_h.WindowVisualEffect_HUDWindowEffect()
            FullScreenUIEffect -> desktop_macos_h.WindowVisualEffect_FullScreenUIEffect()
            ToolTipEffect -> desktop_macos_h.WindowVisualEffect_ToolTipEffect()
            ContentBackgroundEffect -> desktop_macos_h.WindowVisualEffect_ContentBackgroundEffect()
            UnderWindowBackgroundEffect -> desktop_macos_h.WindowVisualEffect_UnderWindowBackgroundEffect()
            UnderPageBackgroundEffect -> desktop_macos_h.WindowVisualEffect_UnderPageBackgroundEffect()
        }
    }
}
