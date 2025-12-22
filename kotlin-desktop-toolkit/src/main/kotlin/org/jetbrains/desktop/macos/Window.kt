package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeBorrowedArray_DraggingItem
import org.jetbrains.desktop.macos.generated.NativeDraggingItem
import org.jetbrains.desktop.macos.generated.NativeTitlebarConfiguration
import org.jetbrains.desktop.macos.generated.NativeTitlebarConfiguration_NativeCustom_Body
import org.jetbrains.desktop.macos.generated.NativeWindowBackground
import org.jetbrains.desktop.macos.generated.NativeWindowParams
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public typealias WindowId = Long

public class Window internal constructor(
    ptr: MemorySegment,
    internal val textInputClientHolder: TextInputClientHolder,
) : Managed(ptr, desktop_macos_h::window_drop) {
    public data class WindowParams(
        val origin: LogicalPoint = LogicalPoint(0.0, 0.0),
        val size: LogicalSize = LogicalSize(640.0, 480.0),
        val title: String = "Window",
        val isResizable: Boolean = true,
        val isClosable: Boolean = true,
        val isMiniaturizable: Boolean = true,
        val isFullScreenAllowed: Boolean = true,
        val titlebarConfiguration: TitlebarConfiguration = TitlebarConfiguration.Regular,
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
            NativeWindowParams.titlebar_configuration(nativeWindowParams, titlebarConfiguration.toNative(arena))
            return nativeWindowParams
        }
    }

    public companion object {
        public fun create(params: WindowParams): Window {
            return Arena.ofConfined().use { arena ->
                val textInputClientHolder = TextInputClientHolder()
                Window(
                    ffiDownCall {
                        desktop_macos_h.window_create(params.toNative(arena), textInputClientHolder.toNative())
                    },
                    textInputClientHolder,
                )
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
            titlebarConfiguration: TitlebarConfiguration = TitlebarConfiguration.Regular,
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
                    titlebarConfiguration,
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

    public var isResizable: Boolean
        get() {
            return ffiDownCall { desktop_macos_h.window_get_resizable(pointer) }
        }
        set(value) {
            ffiDownCall { desktop_macos_h.window_set_resizable(pointer, value) }
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

    // see: https://developer.apple.com/documentation/appkit/nswindow/occlusionstate-swift.property
    public val isVisible: Boolean get() = ffiDownCall { desktop_macos_h.window_is_visible(pointer) }

    /*
     * Though it's calls macOS maximize in maximized state it actually shrinks it back,
     * so it's toggle
     */
    public fun toggleMaximize() {
        ffiDownCall {
            desktop_macos_h.window_maximize(pointer)
        }
    }

    public val isMaximized: Boolean get() = ffiDownCall { desktop_macos_h.window_is_maximized(pointer) }

    public fun miniaturize() {
        ffiDownCall {
            desktop_macos_h.window_miniaturize(pointer)
        }
    }

    public fun deminiaturize() {
        ffiDownCall {
            desktop_macos_h.window_deminiaturize(pointer)
        }
    }

    public val isMiniaturized: Boolean get() = ffiDownCall { desktop_macos_h.window_is_miniaturized(pointer) }

    public val isKey: Boolean
        get() {
            return ffiDownCall { desktop_macos_h.window_is_key(pointer) }
        }

    public val isMain: Boolean
        get() {
            return ffiDownCall { desktop_macos_h.window_is_main(pointer) }
        }

    public fun makeKeyAndOrderFront() {
        ffiDownCall {
            desktop_macos_h.window_make_key_and_order_front(pointer)
        }
    }

    public fun orderFront() {
        ffiDownCall {
            desktop_macos_h.window_order_front(pointer)
        }
    }

    public fun orderBack() {
        ffiDownCall {
            desktop_macos_h.window_order_back(pointer)
        }
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

    public fun startDragWindow() {
        ffiDownCall {
            desktop_macos_h.window_start_drag_window(pointer)
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

    public fun setTitlebarConfiguration(configuration: TitlebarConfiguration) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_macos_h.window_set_titlebar_configuration(pointer, configuration.toNative(arena))
            }
        }
    }

    public var overriddenAppearance: Appearance?
        get() {
            return if (ffiDownCall { desktop_macos_h.window_appearance_is_overridden(pointer) }) {
                ffiDownCall {
                    Appearance.fromNative(desktop_macos_h.window_get_appearance(pointer))
                }
            } else {
                null
            }
        }
        set(value) {
            if (value == null) {
                ffiDownCall {
                    desktop_macos_h.window_appearance_set_follow_application(pointer)
                }
            } else {
                ffiDownCall {
                    desktop_macos_h.window_appearance_override(pointer, value.toNative())
                }
            }
        }

    public fun setTextInputClient(textInputClient: TextInputClient) {
        textInputClientHolder.textInputClient = textInputClient
    }

    public val textInputContext: TextInputContext = TextInputContext(this)

    public val textDirection: TextDirection
        get() {
            return ffiDownCall {
                TextDirection.fromNative(desktop_macos_h.window_get_text_direction(pointer))
            }
        }

    public fun registerForDraggedTypes(types: List<String>) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_macos_h.window_register_for_dragged_types(pointer, listOfStringsToNative(arena, types))
            }
        }
    }

    public fun unregisterDraggedTypes() {
        ffiDownCall {
            desktop_macos_h.window_unregister_dragged_types(pointer)
        }
    }

    /**
     * This function should be called from mouse down event handler only.
     * It's possible to remove this restriction in the future, e.g., chrome and electron do it.
     */
    public fun startDragSession(positionInWindow: LogicalPoint, items: List<DraggingItem>) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_macos_h.window_start_drag_session(pointer, positionInWindow.toNative(arena), items.toNative(arena))
            }
        }
    }

    override fun close() {
        super.close()
        textInputClientHolder.close()
    }
}

/**
 * @param rect is relative to window origin. But the cursor should be inside this rect.
 * If it's not, the rect will move with animation to match this condition
 */
public data class DraggingItem(
    val pasteboardItem: Pasteboard.Item,
    val rect: LogicalRect,
    val image: Image,
)

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

public sealed class TitlebarConfiguration {
    public data object Regular : TitlebarConfiguration()
    public data class Custom(val titlebarHeight: LogicalPixels) : TitlebarConfiguration()

    internal fun toNative(arena: Arena): MemorySegment {
        val result = NativeTitlebarConfiguration.allocate(arena)
        when (this) {
            Regular -> {
                NativeTitlebarConfiguration.tag(result, desktop_macos_h.NativeTitlebarConfiguration_Regular())
            }
            is Custom -> {
                NativeTitlebarConfiguration.tag(result, desktop_macos_h.NativeTitlebarConfiguration_Custom())
                val custom = NativeTitlebarConfiguration_NativeCustom_Body.allocate(arena)
                NativeTitlebarConfiguration_NativeCustom_Body.title_bar_height(custom, titlebarHeight)
                NativeTitlebarConfiguration.custom(result, custom)
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

// DraggingItem conversion functions

internal fun DraggingItem.toNative(nativeItem: MemorySegment, arena: Arena) {
    pasteboardItem.toNative(NativeDraggingItem.pasteboard_item(nativeItem), arena)
    rect.toNative(NativeDraggingItem.rect(nativeItem))
    NativeDraggingItem.image(nativeItem, image.toNative(arena))
}

internal fun List<DraggingItem>.toNative(arena: Arena): MemorySegment {
    val itemsCount = this.count().toLong()
    val itemsArray = NativeDraggingItem.allocateArray(itemsCount, arena)
    this.forEachIndexed { i, item ->
        item.toNative(NativeDraggingItem.asSlice(itemsArray, i.toLong()), arena)
    }

    val result = NativeBorrowedArray_DraggingItem.allocate(arena)
    NativeBorrowedArray_DraggingItem.ptr(result, itemsArray)
    NativeBorrowedArray_DraggingItem.len(result, itemsCount)
    return result
}
