package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.desktop_macos_h

public object Cursor {
    public enum class Icon {
        ArrowCursor,
        IBeamCursor,
        CrosshairCursor,
        ClosedHandCursor,
        OpenHandCursor,
        PointingHandCursor,
        ColumnResizeLeftCursor,
        ColumnResizeRightCursor,
        ColumnResizeLeftRightCursor,
        RowResizeUpCursor,
        RowResizeDownCursor,
        RowResizeUpDownCursor,
        FrameResizeUpLeftDownRightCursor,
        FrameResizeUpRightDownLeftCursor,
        DisappearingItemCursor,
        IBeamCursorForVerticalLayout,
        OperationNotAllowedCursor,
        DragLinkCursor,
        DragCopyCursor,
        ContextualMenuCursor,
        ZoomInCursor,
        ZoomOutCursor,
        ;

        internal fun toNative(): Int {
            return when (this) {
                ArrowCursor -> desktop_macos_h.NativeCursorIcon_ArrowCursor()
                IBeamCursor -> desktop_macos_h.NativeCursorIcon_IBeamCursor()
                CrosshairCursor -> desktop_macos_h.NativeCursorIcon_CrosshairCursor()
                ClosedHandCursor -> desktop_macos_h.NativeCursorIcon_ClosedHandCursor()
                OpenHandCursor -> desktop_macos_h.NativeCursorIcon_OpenHandCursor()
                PointingHandCursor -> desktop_macos_h.NativeCursorIcon_PointingHandCursor()
                ColumnResizeLeftCursor -> desktop_macos_h.NativeCursorIcon_ColumnResizeLeftCursor()
                ColumnResizeRightCursor -> desktop_macos_h.NativeCursorIcon_ColumnResizeRightCursor()
                ColumnResizeLeftRightCursor -> desktop_macos_h.NativeCursorIcon_ColumnResizeLeftRightCursor()
                RowResizeUpCursor -> desktop_macos_h.NativeCursorIcon_RowResizeUpCursor()
                RowResizeDownCursor -> desktop_macos_h.NativeCursorIcon_RowResizeDownCursor()
                RowResizeUpDownCursor -> desktop_macos_h.NativeCursorIcon_RowResizeUpDownCursor()
                FrameResizeUpLeftDownRightCursor -> desktop_macos_h.NativeCursorIcon_FrameResizeUpLeftDownRight()
                FrameResizeUpRightDownLeftCursor -> desktop_macos_h.NativeCursorIcon_FrameResizeUpRightDownLeft()
                DisappearingItemCursor -> desktop_macos_h.NativeCursorIcon_DisappearingItemCursor()
                IBeamCursorForVerticalLayout -> desktop_macos_h.NativeCursorIcon_IBeamCursorForVerticalLayout()
                OperationNotAllowedCursor -> desktop_macos_h.NativeCursorIcon_OperationNotAllowedCursor()
                DragLinkCursor -> desktop_macos_h.NativeCursorIcon_DragLinkCursor()
                DragCopyCursor -> desktop_macos_h.NativeCursorIcon_DragCopyCursor()
                ContextualMenuCursor -> desktop_macos_h.NativeCursorIcon_ContextualMenuCursor()
                ZoomInCursor -> desktop_macos_h.NativeCursorIcon_ZoomInCursor()
                ZoomOutCursor -> desktop_macos_h.NativeCursorIcon_ZoomOutCursor()
            }
        }

        internal companion object {
            internal fun fromNative(value: Int): Icon {
                return when (value) {
                    desktop_macos_h.NativeCursorIcon_Unknown() ->
                        throw Error("Cursor have unknown type, probably it was set outside of KDT")
                    desktop_macos_h.NativeCursorIcon_ArrowCursor() -> ArrowCursor
                    desktop_macos_h.NativeCursorIcon_IBeamCursor() -> IBeamCursor
                    desktop_macos_h.NativeCursorIcon_CrosshairCursor() -> CrosshairCursor
                    desktop_macos_h.NativeCursorIcon_ClosedHandCursor() -> ClosedHandCursor
                    desktop_macos_h.NativeCursorIcon_OpenHandCursor() -> OpenHandCursor
                    desktop_macos_h.NativeCursorIcon_PointingHandCursor() -> PointingHandCursor
                    desktop_macos_h.NativeCursorIcon_ColumnResizeLeftCursor() -> ColumnResizeLeftCursor
                    desktop_macos_h.NativeCursorIcon_ColumnResizeRightCursor() -> ColumnResizeRightCursor
                    desktop_macos_h.NativeCursorIcon_ColumnResizeLeftRightCursor() -> ColumnResizeLeftRightCursor
                    desktop_macos_h.NativeCursorIcon_RowResizeUpCursor() -> RowResizeUpCursor
                    desktop_macos_h.NativeCursorIcon_RowResizeDownCursor() -> RowResizeDownCursor
                    desktop_macos_h.NativeCursorIcon_RowResizeUpDownCursor() -> RowResizeUpDownCursor
                    desktop_macos_h.NativeCursorIcon_FrameResizeUpLeftDownRight() -> FrameResizeUpLeftDownRightCursor
                    desktop_macos_h.NativeCursorIcon_FrameResizeUpRightDownLeft() -> FrameResizeUpRightDownLeftCursor
                    desktop_macos_h.NativeCursorIcon_DisappearingItemCursor() -> DisappearingItemCursor
                    desktop_macos_h.NativeCursorIcon_IBeamCursorForVerticalLayout() -> IBeamCursorForVerticalLayout
                    desktop_macos_h.NativeCursorIcon_OperationNotAllowedCursor() -> OperationNotAllowedCursor
                    desktop_macos_h.NativeCursorIcon_DragLinkCursor() -> DragLinkCursor
                    desktop_macos_h.NativeCursorIcon_DragCopyCursor() -> DragCopyCursor
                    desktop_macos_h.NativeCursorIcon_ContextualMenuCursor() -> ContextualMenuCursor
                    desktop_macos_h.NativeCursorIcon_ZoomInCursor() -> ZoomInCursor
                    desktop_macos_h.NativeCursorIcon_ZoomOutCursor() -> ZoomOutCursor
                    else -> throw Error("Unexpected cursor icon id: $value")
                }
            }
        }
    }

    private var hideCount = 0

    public fun pushHide() {
        ffiDownCall { desktop_macos_h.cursor_push_hide() }
        hideCount += 1
    }

    public fun popHide() {
        ffiDownCall { desktop_macos_h.cursor_pop_hide() }
        hideCount -= 1
    }

    public var hidden: Boolean
        get() {
            return hideCount > 0
        }
        set(value) {
            while (hideCount > 0) {
                popHide()
            }
            if (value) {
                pushHide()
            }
        }

    /**
     * You can change mouse cursor with this property.
     * Though it will have an effect only for the time when cursor is in the same window.
     * Basically cursor might be changed by OS at any moment when it leaves or enters any window.
     */
    public var icon: Icon
        get() {
            return ffiDownCall { Icon.fromNative(desktop_macos_h.cursor_get_icon()) }
        }
        set(value) {
            ffiDownCall { desktop_macos_h.cursor_set_icon(value.toNative()) }
        }
}
