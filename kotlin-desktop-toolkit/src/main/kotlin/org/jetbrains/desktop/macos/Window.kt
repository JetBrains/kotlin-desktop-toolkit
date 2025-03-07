package org.jetbrains.desktop.macos

import org.jetbrains.desktop.LogicalPixels
import org.jetbrains.desktop.LogicalPoint
import org.jetbrains.desktop.LogicalSize
import org.jetbrains.desktop.macos.generated.NativeWindowBackground
import org.jetbrains.desktop.macos.generated.NativeWindowParams
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public typealias WindowId = Long

public class Window internal constructor(ptr: MemorySegment) : Managed(ptr, desktop_macos_h::window_drop) {
    public data class WindowParams(
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

    public companion object {
        public fun create(params: WindowParams): Window {
            return Arena.ofConfined().use { arena ->
                Window(ffiDownCall { desktop_macos_h.window_create(params.toNative(arena)) })
            }
        }

        public fun create(
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

    public fun windowId(): WindowId {
        return ffiDownCall { desktop_macos_h.window_get_window_id(pointer) }
    }

    public fun screenId(): ScreenId {
        return ffiDownCall {
            desktop_macos_h.window_get_screen_id(pointer)
        }
    }

    public fun scaleFactor(): Double {
        return ffiDownCall { desktop_macos_h.window_scale_factor(pointer) }
    }

    public var title: String
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

    public val origin: LogicalPoint
        get() {
            return Arena.ofConfined().use { arena ->
                LogicalPoint.fromNative(ffiDownCall { desktop_macos_h.window_get_origin(arena, pointer) })
            }
        }

    public val size: LogicalSize
        get() {
            return Arena.ofConfined().use { arena ->
                LogicalSize.fromNative(ffiDownCall { desktop_macos_h.window_get_size(arena, pointer) })
            }
        }

    public val contentOrigin: LogicalPoint
        get() {
            return Arena.ofConfined().use { arena ->
                LogicalPoint.fromNative(ffiDownCall { desktop_macos_h.window_get_content_origin(arena, pointer) })
            }
        }

    public val contentSize: LogicalSize
        get() {
            return Arena.ofConfined().use { arena ->
                LogicalSize.fromNative(ffiDownCall { desktop_macos_h.window_get_content_size(arena, pointer) })
            }
        }

    public var maxSize: LogicalSize
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

    public var minSize: LogicalSize
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

    public val isFullScreen: Boolean
        get() {
            return ffiDownCall { desktop_macos_h.window_is_full_screen(pointer) }
        }

    public fun toggleFullScreen() {
        ffiDownCall {
            desktop_macos_h.window_toggle_full_screen(pointer)
        }
    }

    public val isKey: Boolean
        get() {
            return ffiDownCall { desktop_macos_h.window_is_key(pointer) }
        }

    public val isMain: Boolean
        get() {
            return ffiDownCall { desktop_macos_h.window_is_main(pointer) }
        }

    public fun setRect(origin: LogicalPoint, size: LogicalSize, animateTransition: Boolean = true) {
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

    public fun setContentRect(origin: LogicalPoint, size: LogicalSize, animateTransition: Boolean = true) {
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

    public fun startDrag() {
        ffiDownCall {
            desktop_macos_h.window_start_drag(pointer)
        }
    }

    public fun invalidateShadow() {
        ffiDownCall {
            desktop_macos_h.window_invalidate_shadow(pointer)
        }
    }

    public fun attachView(layer: MetalView) {
        ffiDownCall {
            desktop_macos_h.window_attach_layer(pointer, layer.pointer)
        }
    }

    public fun setBackground(background: WindowBackground) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_macos_h.window_set_background(pointer, background.toNative(arena))
            }
        }
    }
}

public sealed class WindowBackground {
    public data object Transparent : WindowBackground()
    public data class SolidColor(val color: Color) : WindowBackground()

    public data class VisualEffect(val effect: WindowVisualEffect) : WindowBackground()

    internal fun toNative(arena: Arena): MemorySegment {
        val result = NativeWindowBackground.allocate(arena)
        when (this) {
            is Transparent -> {
                NativeWindowBackground.tag(result, desktop_macos_h.NativeWindowBackground_Transparent())
            }
            is SolidColor -> {
                NativeWindowBackground.tag(result, desktop_macos_h.NativeWindowBackground_SolidColor())
                NativeWindowBackground.solid_color(result, color.toNative(arena))
            }
            is VisualEffect -> {
                NativeWindowBackground.tag(result, desktop_macos_h.NativeWindowBackground_VisualEffect())
                NativeWindowBackground.visual_effect(result, effect.toNative())
            }
        }
        return result
    }
}

public enum class WindowVisualEffect {
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
            TitlebarEffect -> desktop_macos_h.NativeWindowVisualEffect_TitlebarEffect()
            SelectionEffect -> desktop_macos_h.NativeWindowVisualEffect_SelectionEffect()
            MenuEffect -> desktop_macos_h.NativeWindowVisualEffect_MenuEffect()
            PopoverEffect -> desktop_macos_h.NativeWindowVisualEffect_PopoverEffect()
            SidebarEffect -> desktop_macos_h.NativeWindowVisualEffect_SidebarEffect()
            HeaderViewEffect -> desktop_macos_h.NativeWindowVisualEffect_HeaderViewEffect()
            SheetEffect -> desktop_macos_h.NativeWindowVisualEffect_SheetEffect()
            WindowBackgroundEffect -> desktop_macos_h.NativeWindowVisualEffect_WindowBackgroundEffect()
            HUDWindowEffect -> desktop_macos_h.NativeWindowVisualEffect_HUDWindowEffect()
            FullScreenUIEffect -> desktop_macos_h.NativeWindowVisualEffect_FullScreenUIEffect()
            ToolTipEffect -> desktop_macos_h.NativeWindowVisualEffect_ToolTipEffect()
            ContentBackgroundEffect -> desktop_macos_h.NativeWindowVisualEffect_ContentBackgroundEffect()
            UnderWindowBackgroundEffect -> desktop_macos_h.NativeWindowVisualEffect_UnderWindowBackgroundEffect()
            UnderPageBackgroundEffect -> desktop_macos_h.NativeWindowVisualEffect_UnderPageBackgroundEffect()
        }
    }
}
