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
        ResizeLeftCursor,
        ResizeRightCursor,
        ResizeLeftRightCursor,
        ResizeUpCursor,
        ResizeDownCursor,
        ResizeUpDownCursor,
        DisappearingItemCursor,
        IBeamCursorForVerticalLayout,
        OperationNotAllowedCursor,
        DragLinkCursor,
        DragCopyCursor,
        ContextualMenuCursor,
        ZoomInCursor,
        ZoomOutCursor,
        ColumnResizeCursor,
        RowResizeCursor,
        ;

        internal fun toNative(): Int {
            return when (this) {
                ArrowCursor -> desktop_macos_h.NativeCursorIcon_ArrowCursor()
                IBeamCursor -> desktop_macos_h.NativeCursorIcon_IBeamCursor()
                CrosshairCursor -> desktop_macos_h.NativeCursorIcon_CrosshairCursor()
                ClosedHandCursor -> desktop_macos_h.NativeCursorIcon_ClosedHandCursor()
                OpenHandCursor -> desktop_macos_h.NativeCursorIcon_OpenHandCursor()
                PointingHandCursor -> desktop_macos_h.NativeCursorIcon_PointingHandCursor()
                ResizeLeftCursor -> desktop_macos_h.NativeCursorIcon_ResizeLeftCursor()
                ResizeRightCursor -> desktop_macos_h.NativeCursorIcon_ResizeRightCursor()
                ResizeLeftRightCursor -> desktop_macos_h.NativeCursorIcon_ResizeLeftRightCursor()
                ResizeUpCursor -> desktop_macos_h.NativeCursorIcon_ResizeUpCursor()
                ResizeDownCursor -> desktop_macos_h.NativeCursorIcon_ResizeDownCursor()
                ResizeUpDownCursor -> desktop_macos_h.NativeCursorIcon_ResizeUpDownCursor()
                DisappearingItemCursor -> desktop_macos_h.NativeCursorIcon_DisappearingItemCursor()
                IBeamCursorForVerticalLayout -> desktop_macos_h.NativeCursorIcon_IBeamCursorForVerticalLayout()
                OperationNotAllowedCursor -> desktop_macos_h.NativeCursorIcon_OperationNotAllowedCursor()
                DragLinkCursor -> desktop_macos_h.NativeCursorIcon_DragLinkCursor()
                DragCopyCursor -> desktop_macos_h.NativeCursorIcon_DragCopyCursor()
                ContextualMenuCursor -> desktop_macos_h.NativeCursorIcon_ContextualMenuCursor()
                ZoomInCursor -> desktop_macos_h.NativeCursorIcon_ZoomInCursor()
                ZoomOutCursor -> desktop_macos_h.NativeCursorIcon_ZoomOutCursor()
                ColumnResizeCursor -> desktop_macos_h.NativeCursorIcon_ColumnResizeCursor()
                RowResizeCursor -> desktop_macos_h.NativeCursorIcon_RowResizeCursor()
            }
        }

        internal companion object {
            internal fun fromNative(value: Int): Cursor.Icon {
                return when (value) {
                    desktop_macos_h.NativeCursorIcon_Unknown() ->
                        throw Error("Cursor have unknown type, probably it was set outside of KDT")
                    desktop_macos_h.NativeCursorIcon_ArrowCursor() -> ArrowCursor
                    desktop_macos_h.NativeCursorIcon_IBeamCursor() -> IBeamCursor
                    desktop_macos_h.NativeCursorIcon_CrosshairCursor() -> CrosshairCursor
                    desktop_macos_h.NativeCursorIcon_ClosedHandCursor() -> ClosedHandCursor
                    desktop_macos_h.NativeCursorIcon_OpenHandCursor() -> OpenHandCursor
                    desktop_macos_h.NativeCursorIcon_PointingHandCursor() -> PointingHandCursor
                    desktop_macos_h.NativeCursorIcon_ResizeLeftCursor() -> ResizeLeftCursor
                    desktop_macos_h.NativeCursorIcon_ResizeRightCursor() -> ResizeRightCursor
                    desktop_macos_h.NativeCursorIcon_ResizeLeftRightCursor() -> ResizeLeftRightCursor
                    desktop_macos_h.NativeCursorIcon_ResizeUpCursor() -> ResizeUpCursor
                    desktop_macos_h.NativeCursorIcon_ResizeDownCursor() -> ResizeDownCursor
                    desktop_macos_h.NativeCursorIcon_ResizeUpDownCursor() -> ResizeUpDownCursor
                    desktop_macos_h.NativeCursorIcon_DisappearingItemCursor() -> DisappearingItemCursor
                    desktop_macos_h.NativeCursorIcon_IBeamCursorForVerticalLayout() -> IBeamCursorForVerticalLayout
                    desktop_macos_h.NativeCursorIcon_OperationNotAllowedCursor() -> OperationNotAllowedCursor
                    desktop_macos_h.NativeCursorIcon_DragLinkCursor() -> DragLinkCursor
                    desktop_macos_h.NativeCursorIcon_DragCopyCursor() -> DragCopyCursor
                    desktop_macos_h.NativeCursorIcon_ContextualMenuCursor() -> ContextualMenuCursor
                    desktop_macos_h.NativeCursorIcon_ZoomInCursor() -> ZoomInCursor
                    desktop_macos_h.NativeCursorIcon_ZoomOutCursor() -> ZoomOutCursor
                    desktop_macos_h.NativeCursorIcon_ColumnResizeCursor() -> ColumnResizeCursor
                    desktop_macos_h.NativeCursorIcon_RowResizeCursor() -> RowResizeCursor
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

    public var icon: Icon
        get() {
            return ffiDownCall { Icon.fromNative(desktop_macos_h.cursor_get_icon()) }
        }
        set(value) {
            ffiDownCall { desktop_macos_h.cursor_set_icon(value.toNative()) }
        }
}
