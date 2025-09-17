package org.jetbrains.desktop.linux

import org.jetbrains.desktop.linux.generated.NativeBorrowedArray_u32
import org.jetbrains.desktop.linux.generated.NativeBorrowedArray_u8
import org.jetbrains.desktop.linux.generated.NativeColor
import org.jetbrains.desktop.linux.generated.NativeCommonFileDialogParams
import org.jetbrains.desktop.linux.generated.NativeDataTransferAvailableEvent
import org.jetbrains.desktop.linux.generated.NativeDataTransferCancelledEvent
import org.jetbrains.desktop.linux.generated.NativeDataTransferContent
import org.jetbrains.desktop.linux.generated.NativeDragAndDropQueryData
import org.jetbrains.desktop.linux.generated.NativeEvent
import org.jetbrains.desktop.linux.generated.NativeFileChooserResponse
import org.jetbrains.desktop.linux.generated.NativeKeyDownEvent
import org.jetbrains.desktop.linux.generated.NativeKeyUpEvent
import org.jetbrains.desktop.linux.generated.NativeLogicalPoint
import org.jetbrains.desktop.linux.generated.NativeLogicalRect
import org.jetbrains.desktop.linux.generated.NativeLogicalSize
import org.jetbrains.desktop.linux.generated.NativeModifiersChangedEvent
import org.jetbrains.desktop.linux.generated.NativeMouseDownEvent
import org.jetbrains.desktop.linux.generated.NativeMouseEnteredEvent
import org.jetbrains.desktop.linux.generated.NativeMouseExitedEvent
import org.jetbrains.desktop.linux.generated.NativeMouseMovedEvent
import org.jetbrains.desktop.linux.generated.NativeMouseUpEvent
import org.jetbrains.desktop.linux.generated.NativeOpenFileDialogParams
import org.jetbrains.desktop.linux.generated.NativePhysicalSize
import org.jetbrains.desktop.linux.generated.NativeSaveFileDialogParams
import org.jetbrains.desktop.linux.generated.NativeScrollWheelEvent
import org.jetbrains.desktop.linux.generated.NativeSoftwareDrawData
import org.jetbrains.desktop.linux.generated.NativeTextInputAvailabilityEvent
import org.jetbrains.desktop.linux.generated.NativeTextInputContext
import org.jetbrains.desktop.linux.generated.NativeTextInputDeleteSurroundingTextData
import org.jetbrains.desktop.linux.generated.NativeTextInputEvent
import org.jetbrains.desktop.linux.generated.NativeTextInputPreeditStringData
import org.jetbrains.desktop.linux.generated.NativeWindowCapabilities
import org.jetbrains.desktop.linux.generated.NativeWindowCloseRequestEvent
import org.jetbrains.desktop.linux.generated.NativeWindowConfigureEvent
import org.jetbrains.desktop.linux.generated.NativeWindowDrawEvent
import org.jetbrains.desktop.linux.generated.NativeWindowKeyboardEnterEvent
import org.jetbrains.desktop.linux.generated.NativeWindowKeyboardLeaveEvent
import org.jetbrains.desktop.linux.generated.NativeWindowScaleChangedEvent
import org.jetbrains.desktop.linux.generated.NativeWindowScreenChangeEvent
import org.jetbrains.desktop.linux.generated.NativeXdgDesktopSetting
import org.jetbrains.desktop.linux.generated.desktop_linux_h
import java.lang.foreign.Arena
import java.lang.foreign.MemoryLayout
import java.lang.foreign.MemorySegment
import kotlin.experimental.and
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.DurationUnit
import kotlin.time.toDuration

internal fun fromOptionalNativeString(s: MemorySegment): String? {
    return if (s == MemorySegment.NULL) null else s.getUtf8String(0)
}

/**
 * Converts UTF-8 offset to UTF-16 offset.
 */
public fun utf8OffsetToUtf16Offset(string: CharSequence, offset: Int): Int {
    if (offset == 0) {
        return 0
    }
    var utf8Offset = offset
    var utf16Offset = 0
    for (codePoint in string.codePoints()) {
        utf8Offset -= when {
            codePoint < 128 -> 1
            codePoint < 2048 -> 2
            codePoint < 65536 -> 3
            else -> 4
        }

        utf16Offset += 1
        // Code points from the supplementary planes are encoded as a surrogate pair in utf-16,
        // meaning we'll have one extra utf-16 code unit for every code point in this range.
        if (codePoint >= 65536) utf16Offset += 1

        if (utf8Offset <= 0) break
    }

    return utf16Offset
}

internal fun LogicalSize.Companion.fromNative(s: MemorySegment) = LogicalSize(
    width = NativeLogicalSize.width(s).toFloat(),
    height = NativeLogicalSize.height(s).toFloat(),
)

internal fun LogicalSize.toNative(arena: Arena): MemorySegment {
    val result = NativeLogicalSize.allocate(arena)
    NativeLogicalSize.width(result, width.toDouble())
    NativeLogicalSize.height(result, height.toDouble())
    return result
}

internal fun LogicalPoint.Companion.fromNative(s: MemorySegment) = LogicalPoint(
    x = NativeLogicalPoint.x(s).toFloat(),
    y = NativeLogicalPoint.y(s).toFloat(),
)

internal fun LogicalPoint.toNative(arena: Arena): MemorySegment {
    val result = NativeLogicalPoint.allocate(arena)
    NativeLogicalPoint.x(result, x.toDouble())
    NativeLogicalPoint.y(result, y.toDouble())
    return result
}

internal fun LogicalRect.toNative(arena: Arena): MemorySegment {
    val result = NativeLogicalRect.allocate(arena)
    NativeLogicalRect.origin(result, point.toNative(arena))
    NativeLogicalRect.size(result, size.toNative(arena))
    return result
}

internal fun PhysicalSize.Companion.fromNative(s: MemorySegment) = PhysicalSize(
    width = NativePhysicalSize.width(s),
    height = NativePhysicalSize.height(s),
)

private fun keyModifiersFromNative(nativeModifiers: Byte): Set<KeyModifiers> {
    val modifiers = buildSet {
        if (nativeModifiers and desktop_linux_h.NativeKeyModifier_Ctrl().toByte() > 0) {
            add(KeyModifiers.Control)
        }
        if (nativeModifiers and desktop_linux_h.NativeKeyModifier_Alt().toByte() > 0) {
            add(KeyModifiers.Alt)
        }
        if (nativeModifiers and desktop_linux_h.NativeKeyModifier_Shift().toByte() > 0) {
            add(KeyModifiers.Shift)
        }
        if (nativeModifiers and desktop_linux_h.NativeKeyModifier_CapsLock().toByte() > 0) {
            add(KeyModifiers.CapsLock)
        }
        if (nativeModifiers and desktop_linux_h.NativeKeyModifier_Logo().toByte() > 0) {
            add(KeyModifiers.Logo)
        }
        if (nativeModifiers and desktop_linux_h.NativeKeyModifier_NumLock().toByte() > 0) {
            add(KeyModifiers.NumLock)
        }
    }

    return modifiers
}

internal fun PointerShape.toNative(): Int {
    return when (this) {
        PointerShape.Default -> desktop_linux_h.NativePointerShape_Default()
        PointerShape.ContextMenu -> desktop_linux_h.NativePointerShape_ContextMenu()
        PointerShape.Help -> desktop_linux_h.NativePointerShape_Help()
        PointerShape.Pointer -> desktop_linux_h.NativePointerShape_Pointer()
        PointerShape.Progress -> desktop_linux_h.NativePointerShape_Progress()
        PointerShape.Wait -> desktop_linux_h.NativePointerShape_Wait()
        PointerShape.Cell -> desktop_linux_h.NativePointerShape_Cell()
        PointerShape.Crosshair -> desktop_linux_h.NativePointerShape_Crosshair()
        PointerShape.Text -> desktop_linux_h.NativePointerShape_Text()
        PointerShape.VerticalText -> desktop_linux_h.NativePointerShape_VerticalText()
        PointerShape.Alias -> desktop_linux_h.NativePointerShape_Alias()
        PointerShape.Copy -> desktop_linux_h.NativePointerShape_Copy()
        PointerShape.Move -> desktop_linux_h.NativePointerShape_Move()
        PointerShape.NoDrop -> desktop_linux_h.NativePointerShape_NoDrop()
        PointerShape.NotAllowed -> desktop_linux_h.NativePointerShape_NotAllowed()
        PointerShape.Grab -> desktop_linux_h.NativePointerShape_Grab()
        PointerShape.Grabbing -> desktop_linux_h.NativePointerShape_Grabbing()
        PointerShape.EResize -> desktop_linux_h.NativePointerShape_EResize()
        PointerShape.NResize -> desktop_linux_h.NativePointerShape_NResize()
        PointerShape.NeResize -> desktop_linux_h.NativePointerShape_NeResize()
        PointerShape.NwResize -> desktop_linux_h.NativePointerShape_NwResize()
        PointerShape.SResize -> desktop_linux_h.NativePointerShape_SResize()
        PointerShape.SeResize -> desktop_linux_h.NativePointerShape_SeResize()
        PointerShape.SwResize -> desktop_linux_h.NativePointerShape_SwResize()
        PointerShape.WResize -> desktop_linux_h.NativePointerShape_WResize()
        PointerShape.EwResize -> desktop_linux_h.NativePointerShape_EwResize()
        PointerShape.NsResize -> desktop_linux_h.NativePointerShape_NsResize()
        PointerShape.NeswResize -> desktop_linux_h.NativePointerShape_NeswResize()
        PointerShape.NwseResize -> desktop_linux_h.NativePointerShape_NwseResize()
        PointerShape.ColResize -> desktop_linux_h.NativePointerShape_ColResize()
        PointerShape.RowResize -> desktop_linux_h.NativePointerShape_RowResize()
        PointerShape.AllScroll -> desktop_linux_h.NativePointerShape_AllScroll()
        PointerShape.ZoomIn -> desktop_linux_h.NativePointerShape_ZoomIn()
        PointerShape.ZoomOut -> desktop_linux_h.NativePointerShape_ZoomOut()
    }
}

private object VirtualKeysHelper {
    val MAP: Map<Int, VirtualKey> = mapOf(
        desktop_linux_h.NativeVirtualKey_Home() to VirtualKey.Home,
        desktop_linux_h.NativeVirtualKey_Back() to VirtualKey.Back,
        desktop_linux_h.NativeVirtualKey__0() to VirtualKey.`0`,
        desktop_linux_h.NativeVirtualKey__1() to VirtualKey.`1`,
        desktop_linux_h.NativeVirtualKey__2() to VirtualKey.`2`,
        desktop_linux_h.NativeVirtualKey__3() to VirtualKey.`3`,
        desktop_linux_h.NativeVirtualKey__4() to VirtualKey.`4`,
        desktop_linux_h.NativeVirtualKey__5() to VirtualKey.`5`,
        desktop_linux_h.NativeVirtualKey__6() to VirtualKey.`6`,
        desktop_linux_h.NativeVirtualKey__7() to VirtualKey.`7`,
        desktop_linux_h.NativeVirtualKey__8() to VirtualKey.`8`,
        desktop_linux_h.NativeVirtualKey__9() to VirtualKey.`9`,
        desktop_linux_h.NativeVirtualKey_Clear() to VirtualKey.Clear,
        desktop_linux_h.NativeVirtualKey_A() to VirtualKey.A,
        desktop_linux_h.NativeVirtualKey_B() to VirtualKey.B,
        desktop_linux_h.NativeVirtualKey_C() to VirtualKey.C,
        desktop_linux_h.NativeVirtualKey_D() to VirtualKey.D,
        desktop_linux_h.NativeVirtualKey_E() to VirtualKey.E,
        desktop_linux_h.NativeVirtualKey_F() to VirtualKey.F,
        desktop_linux_h.NativeVirtualKey_G() to VirtualKey.G,
        desktop_linux_h.NativeVirtualKey_H() to VirtualKey.H,
        desktop_linux_h.NativeVirtualKey_I() to VirtualKey.I,
        desktop_linux_h.NativeVirtualKey_J() to VirtualKey.J,
        desktop_linux_h.NativeVirtualKey_K() to VirtualKey.K,
        desktop_linux_h.NativeVirtualKey_L() to VirtualKey.L,
        desktop_linux_h.NativeVirtualKey_M() to VirtualKey.M,
        desktop_linux_h.NativeVirtualKey_N() to VirtualKey.N,
        desktop_linux_h.NativeVirtualKey_O() to VirtualKey.O,
        desktop_linux_h.NativeVirtualKey_P() to VirtualKey.P,
        desktop_linux_h.NativeVirtualKey_Q() to VirtualKey.Q,
        desktop_linux_h.NativeVirtualKey_R() to VirtualKey.R,
        desktop_linux_h.NativeVirtualKey_S() to VirtualKey.S,
        desktop_linux_h.NativeVirtualKey_T() to VirtualKey.T,
        desktop_linux_h.NativeVirtualKey_U() to VirtualKey.U,
        desktop_linux_h.NativeVirtualKey_V() to VirtualKey.V,
        desktop_linux_h.NativeVirtualKey_W() to VirtualKey.W,
        desktop_linux_h.NativeVirtualKey_X() to VirtualKey.X,
        desktop_linux_h.NativeVirtualKey_Y() to VirtualKey.Y,
        desktop_linux_h.NativeVirtualKey_Z() to VirtualKey.Z,
        desktop_linux_h.NativeVirtualKey_Comma() to VirtualKey.Comma,
        desktop_linux_h.NativeVirtualKey_Period() to VirtualKey.Period,
        desktop_linux_h.NativeVirtualKey_AltLeft() to VirtualKey.AltLeft,
        desktop_linux_h.NativeVirtualKey_AltRight() to VirtualKey.AltRight,
        desktop_linux_h.NativeVirtualKey_ShiftLeft() to VirtualKey.ShiftLeft,
        desktop_linux_h.NativeVirtualKey_ShiftRight() to VirtualKey.ShiftRight,
        desktop_linux_h.NativeVirtualKey_Tab() to VirtualKey.Tab,
        desktop_linux_h.NativeVirtualKey_Space() to VirtualKey.Space,
        desktop_linux_h.NativeVirtualKey_Enter() to VirtualKey.Enter,
        desktop_linux_h.NativeVirtualKey_BackSpace() to VirtualKey.BackSpace,
        desktop_linux_h.NativeVirtualKey_Grave() to VirtualKey.Grave,
        desktop_linux_h.NativeVirtualKey_Minus() to VirtualKey.Minus,
        desktop_linux_h.NativeVirtualKey_Equals() to VirtualKey.Equals,
        desktop_linux_h.NativeVirtualKey_LeftBracket() to VirtualKey.LeftBracket,
        desktop_linux_h.NativeVirtualKey_RightBracket() to VirtualKey.RightBracket,
        desktop_linux_h.NativeVirtualKey_Backslash() to VirtualKey.Backslash,
        desktop_linux_h.NativeVirtualKey_Semicolon() to VirtualKey.Semicolon,
        desktop_linux_h.NativeVirtualKey_Apostrophe() to VirtualKey.Apostrophe,
        desktop_linux_h.NativeVirtualKey_Slash() to VirtualKey.Slash,
        desktop_linux_h.NativeVirtualKey_Menu() to VirtualKey.Menu,
        desktop_linux_h.NativeVirtualKey_Search() to VirtualKey.Search,
        desktop_linux_h.NativeVirtualKey_PageUp() to VirtualKey.PageUp,
        desktop_linux_h.NativeVirtualKey_PageDown() to VirtualKey.PageDown,
        desktop_linux_h.NativeVirtualKey_Pictsymbols() to VirtualKey.Pictsymbols,
        desktop_linux_h.NativeVirtualKey_Escape() to VirtualKey.Escape,
        desktop_linux_h.NativeVirtualKey_Delete() to VirtualKey.Delete,
        desktop_linux_h.NativeVirtualKey_CtrlLeft() to VirtualKey.CtrlLeft,
        desktop_linux_h.NativeVirtualKey_CtrlRight() to VirtualKey.CtrlRight,
        desktop_linux_h.NativeVirtualKey_CapsLock() to VirtualKey.CapsLock,
        desktop_linux_h.NativeVirtualKey_ScrollLock() to VirtualKey.ScrollLock,
        desktop_linux_h.NativeVirtualKey_MetaLeft() to VirtualKey.MetaLeft,
        desktop_linux_h.NativeVirtualKey_MetaRight() to VirtualKey.MetaRight,
        desktop_linux_h.NativeVirtualKey_Sysrq() to VirtualKey.Sysrq,
        desktop_linux_h.NativeVirtualKey_Break() to VirtualKey.Break,
        desktop_linux_h.NativeVirtualKey_MoveHome() to VirtualKey.MoveHome,
        desktop_linux_h.NativeVirtualKey_MoveEnd() to VirtualKey.MoveEnd,
        desktop_linux_h.NativeVirtualKey_Insert() to VirtualKey.Insert,
        desktop_linux_h.NativeVirtualKey_Forward() to VirtualKey.Forward,
        desktop_linux_h.NativeVirtualKey_F1() to VirtualKey.F1,
        desktop_linux_h.NativeVirtualKey_F2() to VirtualKey.F2,
        desktop_linux_h.NativeVirtualKey_F10() to VirtualKey.F10,
        desktop_linux_h.NativeVirtualKey_F11() to VirtualKey.F11,
        desktop_linux_h.NativeVirtualKey_F12() to VirtualKey.F12,
        desktop_linux_h.NativeVirtualKey_F3() to VirtualKey.F3,
        desktop_linux_h.NativeVirtualKey_F4() to VirtualKey.F4,
        desktop_linux_h.NativeVirtualKey_F5() to VirtualKey.F5,
        desktop_linux_h.NativeVirtualKey_F6() to VirtualKey.F6,
        desktop_linux_h.NativeVirtualKey_F7() to VirtualKey.F7,
        desktop_linux_h.NativeVirtualKey_F8() to VirtualKey.F8,
        desktop_linux_h.NativeVirtualKey_F9() to VirtualKey.F9,
        desktop_linux_h.NativeVirtualKey_NumLock() to VirtualKey.NumLock,
        desktop_linux_h.NativeVirtualKey_Numpad0() to VirtualKey.Numpad0,
        desktop_linux_h.NativeVirtualKey_Numpad1() to VirtualKey.Numpad1,
        desktop_linux_h.NativeVirtualKey_Numpad2() to VirtualKey.Numpad2,
        desktop_linux_h.NativeVirtualKey_Numpad3() to VirtualKey.Numpad3,
        desktop_linux_h.NativeVirtualKey_Numpad4() to VirtualKey.Numpad4,
        desktop_linux_h.NativeVirtualKey_Numpad5() to VirtualKey.Numpad5,
        desktop_linux_h.NativeVirtualKey_Numpad6() to VirtualKey.Numpad6,
        desktop_linux_h.NativeVirtualKey_Numpad7() to VirtualKey.Numpad7,
        desktop_linux_h.NativeVirtualKey_Numpad8() to VirtualKey.Numpad8,
        desktop_linux_h.NativeVirtualKey_Numpad9() to VirtualKey.Numpad9,
        desktop_linux_h.NativeVirtualKey_NumpadDivide() to VirtualKey.NumpadDivide,
        desktop_linux_h.NativeVirtualKey_NumpadMultiply() to VirtualKey.NumpadMultiply,
        desktop_linux_h.NativeVirtualKey_NumpadSubtract() to VirtualKey.NumpadSubtract,
        desktop_linux_h.NativeVirtualKey_NumpadAdd() to VirtualKey.NumpadAdd,
        desktop_linux_h.NativeVirtualKey_NumpadDot() to VirtualKey.NumpadDot,
        desktop_linux_h.NativeVirtualKey_NumpadComma() to VirtualKey.NumpadComma,
        desktop_linux_h.NativeVirtualKey_NumpadEnter() to VirtualKey.NumpadEnter,
        desktop_linux_h.NativeVirtualKey_NumpadEquals() to VirtualKey.NumpadEquals,
        desktop_linux_h.NativeVirtualKey_NumpadLeftParen() to VirtualKey.NumpadLeftParen,
        desktop_linux_h.NativeVirtualKey_NumpadRightParen() to VirtualKey.NumpadRightParen,
        desktop_linux_h.NativeVirtualKey_ZoomIn() to VirtualKey.ZoomIn,
        desktop_linux_h.NativeVirtualKey_ZoomOut() to VirtualKey.ZoomOut,
        desktop_linux_h.NativeVirtualKey_Settings() to VirtualKey.Settings,
        desktop_linux_h.NativeVirtualKey_ZenkakuHankaku() to VirtualKey.ZenkakuHankaku,
        desktop_linux_h.NativeVirtualKey_Eisu() to VirtualKey.Eisu,
        desktop_linux_h.NativeVirtualKey_Muhenkan() to VirtualKey.Muhenkan,
        desktop_linux_h.NativeVirtualKey_Henkan() to VirtualKey.Henkan,
        desktop_linux_h.NativeVirtualKey_KatakanaHiragana() to VirtualKey.KatakanaHiragana,
        desktop_linux_h.NativeVirtualKey_Yen() to VirtualKey.Yen,
        desktop_linux_h.NativeVirtualKey_Ro() to VirtualKey.Ro,
        desktop_linux_h.NativeVirtualKey_Kana() to VirtualKey.Kana,
        desktop_linux_h.NativeVirtualKey_Cut() to VirtualKey.Cut,
        desktop_linux_h.NativeVirtualKey_Copy() to VirtualKey.Copy,
        desktop_linux_h.NativeVirtualKey_Paste() to VirtualKey.Paste,
        desktop_linux_h.NativeVirtualKey_Up() to VirtualKey.Up,
        desktop_linux_h.NativeVirtualKey_Down() to VirtualKey.Down,
        desktop_linux_h.NativeVirtualKey_Left() to VirtualKey.Left,
        desktop_linux_h.NativeVirtualKey_Right() to VirtualKey.Right,
        desktop_linux_h.NativeVirtualKey_Refresh() to VirtualKey.Refresh,
        desktop_linux_h.NativeVirtualKey_EmojiPicker() to VirtualKey.EmojiPicker,
        desktop_linux_h.NativeVirtualKey_New() to VirtualKey.New,
        desktop_linux_h.NativeVirtualKey_Close() to VirtualKey.Close,
        desktop_linux_h.NativeVirtualKey_Print() to VirtualKey.Print,
        desktop_linux_h.NativeVirtualKey_F13() to VirtualKey.F13,
        desktop_linux_h.NativeVirtualKey_F14() to VirtualKey.F14,
        desktop_linux_h.NativeVirtualKey_F15() to VirtualKey.F15,
        desktop_linux_h.NativeVirtualKey_F16() to VirtualKey.F16,
        desktop_linux_h.NativeVirtualKey_F17() to VirtualKey.F17,
        desktop_linux_h.NativeVirtualKey_F18() to VirtualKey.F18,
        desktop_linux_h.NativeVirtualKey_F19() to VirtualKey.F19,
        desktop_linux_h.NativeVirtualKey_F20() to VirtualKey.F20,
        desktop_linux_h.NativeVirtualKey_F21() to VirtualKey.F21,
        desktop_linux_h.NativeVirtualKey_F22() to VirtualKey.F22,
        desktop_linux_h.NativeVirtualKey_F23() to VirtualKey.F23,
        desktop_linux_h.NativeVirtualKey_F24() to VirtualKey.F24,
    )

    init {
        check(MAP.size == desktop_linux_h.NativeVirtualKey_Unknown())
    }
}

internal fun VirtualKey.Companion.fromNative(raw: Int): VirtualKey? {
    if (raw == desktop_linux_h.NativeVirtualKey_Unknown()) {
        return null
    }
    return VirtualKeysHelper.MAP[raw]
}

internal fun WindowCapabilities.Companion.fromNative(s: MemorySegment) = WindowCapabilities(
    windowMenu = NativeWindowCapabilities.window_menu(s),
    maximize = NativeWindowCapabilities.maximixe(s),
    fullscreen = NativeWindowCapabilities.fullscreen(s),
    minimize = NativeWindowCapabilities.minimize(s),
)

internal fun Color.Companion.fromNative(s: MemorySegment) = Color(
    red = NativeColor.red(s),
    green = NativeColor.green(s),
    blue = NativeColor.blue(s),
    alpha = NativeColor.alpha(s),
)

private fun DesktopTitlebarAction.Companion.fromNative(raw: Int): DesktopTitlebarAction {
    return when (raw) {
        desktop_linux_h.NativeDesktopTitlebarAction_Minimize() -> DesktopTitlebarAction.Minimize
        desktop_linux_h.NativeDesktopTitlebarAction_ToggleMaximize() -> DesktopTitlebarAction.ToggleMaximize
        desktop_linux_h.NativeDesktopTitlebarAction_Menu() -> DesktopTitlebarAction.Menu
        desktop_linux_h.NativeDesktopTitlebarAction_None() -> DesktopTitlebarAction.None
        else -> error("Unexpected desktop titlebar action value: $raw")
    }
}

private fun WindowDecorationMode.Companion.fromNative(raw: Int): WindowDecorationMode {
    return when (raw) {
        desktop_linux_h.NativeWindowDecorationMode_Client() -> WindowDecorationMode.Client
        desktop_linux_h.NativeWindowDecorationMode_Server() -> WindowDecorationMode.Server
        else -> error("Unexpected Window decoration mode: $raw")
    }
}

internal fun WindowDecorationMode.toNative(): Int {
    return when (this) {
        WindowDecorationMode.Client -> desktop_linux_h.NativeWindowDecorationMode_Client()
        WindowDecorationMode.Server -> desktop_linux_h.NativeWindowDecorationMode_Server()
    }
}

internal fun XdgDesktopSetting.Companion.fromNative(s: MemorySegment): XdgDesktopSetting {
    return when (val nativeTag = NativeXdgDesktopSetting.tag(s)) {
        desktop_linux_h.NativeXdgDesktopSetting_TitlebarLayout() -> {
            XdgDesktopSetting.TitlebarLayout(NativeXdgDesktopSetting.titlebar_layout(s).getUtf8String(0))
        }
        desktop_linux_h.NativeXdgDesktopSetting_ActionDoubleClickTitlebar() -> {
            XdgDesktopSetting.ActionDoubleClickTitlebar(
                DesktopTitlebarAction.fromNative(NativeXdgDesktopSetting.action_double_click_titlebar(s)),
            )
        }
        desktop_linux_h.NativeXdgDesktopSetting_ActionRightClickTitlebar() -> {
            XdgDesktopSetting.ActionRightClickTitlebar(
                DesktopTitlebarAction.fromNative(NativeXdgDesktopSetting.action_right_click_titlebar(s)),
            )
        }
        desktop_linux_h.NativeXdgDesktopSetting_ActionMiddleClickTitlebar() -> {
            XdgDesktopSetting.ActionMiddleClickTitlebar(
                DesktopTitlebarAction.fromNative(NativeXdgDesktopSetting.action_middle_click_titlebar(s)),
            )
        }
        desktop_linux_h.NativeXdgDesktopSetting_DoubleClickIntervalMs() -> XdgDesktopSetting.DoubleClickInterval(
            value = NativeXdgDesktopSetting.double_click_interval_ms(s).milliseconds,
        )
        desktop_linux_h.NativeXdgDesktopSetting_ColorScheme() -> XdgDesktopSetting.ColorScheme(
            when (NativeXdgDesktopSetting.color_scheme(s)) {
                desktop_linux_h.NativeXdgDesktopColorScheme_NoPreference() -> ColorSchemeValue.NoPreference
                desktop_linux_h.NativeXdgDesktopColorScheme_PreferDark() -> ColorSchemeValue.PreferDark
                desktop_linux_h.NativeXdgDesktopColorScheme_PreferLight() -> ColorSchemeValue.PreferLight
                else -> error("Unexpected color scheme ${NativeXdgDesktopSetting.color_scheme(s)}")
            },
        )
        desktop_linux_h.NativeXdgDesktopSetting_AccentColor() -> XdgDesktopSetting.AccentColor(
            Color.fromNative(NativeXdgDesktopSetting.accent_color(s)),
        )
        desktop_linux_h.NativeXdgDesktopSetting_FontAntialiasing() -> XdgDesktopSetting.FontAntialiasing(
            when (NativeXdgDesktopSetting.font_antialiasing(s)) {
                desktop_linux_h.NativeFontAntialiasing_None() -> FontAntialiasingValue.None
                desktop_linux_h.NativeFontAntialiasing_Grayscale() -> FontAntialiasingValue.Grayscale
                desktop_linux_h.NativeFontAntialiasing_Rgba() -> FontAntialiasingValue.Rgba
                else -> error("Unexpected font aliasing ${NativeXdgDesktopSetting.font_antialiasing(s)}")
            },
        )
        desktop_linux_h.NativeXdgDesktopSetting_FontHinting() -> XdgDesktopSetting.FontHinting(
            when (NativeXdgDesktopSetting.font_hinting(s)) {
                desktop_linux_h.NativeFontHinting_None() -> FontHintingValue.None
                desktop_linux_h.NativeFontHinting_Slight() -> FontHintingValue.Slight
                desktop_linux_h.NativeFontHinting_Medium() -> FontHintingValue.Medium
                desktop_linux_h.NativeFontHinting_Full() -> FontHintingValue.Full
                else -> error("Unexpected font hinting ${NativeXdgDesktopSetting.font_hinting(s)}")
            },
        )
        desktop_linux_h.NativeXdgDesktopSetting_FontRgbaOrder() -> XdgDesktopSetting.FontRgbaOrder(
            when (NativeXdgDesktopSetting.font_rgba_order(s)) {
                desktop_linux_h.NativeFontRgbaOrder_Rgb() -> FontRgbaOrderValue.Rgb
                desktop_linux_h.NativeFontRgbaOrder_Bgr() -> FontRgbaOrderValue.Bgr
                desktop_linux_h.NativeFontRgbaOrder_Vrgb() -> FontRgbaOrderValue.Vrgb
                desktop_linux_h.NativeFontRgbaOrder_Vbgr() -> FontRgbaOrderValue.Vbgr
                else -> error("Unexpected font rgba order ${NativeXdgDesktopSetting.font_rgba_order(s)}")
            },
        )
        desktop_linux_h.NativeXdgDesktopSetting_CursorBlink() -> XdgDesktopSetting.CursorBlink(NativeXdgDesktopSetting.cursor_blink(s))
        desktop_linux_h.NativeXdgDesktopSetting_CursorSize() -> XdgDesktopSetting.CursorSize(NativeXdgDesktopSetting.cursor_size(s))
        desktop_linux_h.NativeXdgDesktopSetting_CursorTheme() -> XdgDesktopSetting.CursorTheme(
            NativeXdgDesktopSetting.cursor_theme(s).getUtf8String(0),
        )
        desktop_linux_h.NativeXdgDesktopSetting_CursorBlinkTimeMs() -> XdgDesktopSetting.CursorBlinkTime(
            NativeXdgDesktopSetting.cursor_blink_time_ms(s).toDuration(DurationUnit.MILLISECONDS),
        )
        desktop_linux_h.NativeXdgDesktopSetting_CursorBlinkTimeoutMs() -> XdgDesktopSetting.CursorBlinkTimeout(
            NativeXdgDesktopSetting.cursor_blink_timeout_ms(s).toDuration(DurationUnit.MILLISECONDS),
        )
        desktop_linux_h.NativeXdgDesktopSetting_OverlayScrolling() -> XdgDesktopSetting.OverlayScrolling(
            NativeXdgDesktopSetting.overlay_scrolling(s),
        )
        desktop_linux_h.NativeXdgDesktopSetting_AudibleBell() -> XdgDesktopSetting.AudibleBell(NativeXdgDesktopSetting.audible_bell(s))
        else -> error("Unexpected setting $nativeTag")
    }
}

internal fun SoftwareDrawData.Companion.fromNative(nativeSoftwareDrawData: MemorySegment): SoftwareDrawData? {
    val nativeCanvas = NativeSoftwareDrawData.canvas(nativeSoftwareDrawData)
    if (nativeCanvas == MemorySegment.NULL) {
        return null
    }

    return SoftwareDrawData(canvas = nativeCanvas.address(), stride = NativeSoftwareDrawData.stride(nativeSoftwareDrawData))
}

internal fun WindowResizeEdge.toNative(): Int {
    return when (this) {
        WindowResizeEdge.Top -> desktop_linux_h.NativeWindowResizeEdge_Top()
        WindowResizeEdge.Bottom -> desktop_linux_h.NativeWindowResizeEdge_Bottom()
        WindowResizeEdge.Left -> desktop_linux_h.NativeWindowResizeEdge_Left()
        WindowResizeEdge.TopLeft -> desktop_linux_h.NativeWindowResizeEdge_TopLeft()
        WindowResizeEdge.BottomLeft -> desktop_linux_h.NativeWindowResizeEdge_BottomLeft()
        WindowResizeEdge.Right -> desktop_linux_h.NativeWindowResizeEdge_Right()
        WindowResizeEdge.TopRight -> desktop_linux_h.NativeWindowResizeEdge_TopRight()
        WindowResizeEdge.BottomRight -> desktop_linux_h.NativeWindowResizeEdge_BottomRight()
    }
}

internal fun TextInputContentPurpose.toNative(): Int {
    return when (this) {
        TextInputContentPurpose.Normal -> desktop_linux_h.NativeTextInputContentPurpose_Normal()
        TextInputContentPurpose.Alpha -> desktop_linux_h.NativeTextInputContentPurpose_Alpha()
        TextInputContentPurpose.Digits -> desktop_linux_h.NativeTextInputContentPurpose_Digits()
        TextInputContentPurpose.Number -> desktop_linux_h.NativeTextInputContentPurpose_Number()
        TextInputContentPurpose.Phone -> desktop_linux_h.NativeTextInputContentPurpose_Phone()
        TextInputContentPurpose.Url -> desktop_linux_h.NativeTextInputContentPurpose_Url()
        TextInputContentPurpose.Email -> desktop_linux_h.NativeTextInputContentPurpose_Email()
        TextInputContentPurpose.Name -> desktop_linux_h.NativeTextInputContentPurpose_Name()
        TextInputContentPurpose.Password -> desktop_linux_h.NativeTextInputContentPurpose_Password()
        TextInputContentPurpose.Pin -> desktop_linux_h.NativeTextInputContentPurpose_Pin()
        TextInputContentPurpose.Date -> desktop_linux_h.NativeTextInputContentPurpose_Date()
        TextInputContentPurpose.Time -> desktop_linux_h.NativeTextInputContentPurpose_Time()
        TextInputContentPurpose.Datetime -> desktop_linux_h.NativeTextInputContentPurpose_Datetime()
        TextInputContentPurpose.Terminal -> desktop_linux_h.NativeTextInputContentPurpose_Terminal()
    }
}

internal fun TextInputPreeditStringData.Companion.fromNative(s: MemorySegment): TextInputPreeditStringData {
    return TextInputPreeditStringData(
        text = fromOptionalNativeString(NativeTextInputPreeditStringData.text(s)),
        cursorBeginBytePos = NativeTextInputPreeditStringData.cursor_begin_byte_pos(s),
        cursorEndBytePos = NativeTextInputPreeditStringData.cursor_end_byte_pos(s),
    )
}

internal fun TextInputDeleteSurroundingTextData.Companion.fromNative(s: MemorySegment): TextInputDeleteSurroundingTextData {
    return TextInputDeleteSurroundingTextData(
        beforeLengthInBytes = NativeTextInputDeleteSurroundingTextData.before_length_in_bytes(s),
        afterLengthInBytes = NativeTextInputDeleteSurroundingTextData.after_length_in_bytes(s),
    )
}

internal fun TextInputContext.toNative(arena: Arena): MemorySegment {
    val result = NativeTextInputContext.allocate(arena)
    NativeTextInputContext.surrounding_text(result, arena.allocateUtf8String(surroundingText))
    NativeTextInputContext.cursor_codepoint_offset(result, cursorCodepointOffset)
    NativeTextInputContext.selection_start_codepoint_offset(result, selectionStartCodepointOffset)
    NativeTextInputContext.is_multiline(result, isMultiline)
    NativeTextInputContext.content_purpose(result, contentPurpose.toNative())
    NativeTextInputContext.cursor_rectangle(result, cursorRectangle.toNative(arena))
    NativeTextInputContext.change_caused_by_input_method(result, changeCausedByInputMethod)
    return result
}

internal fun DataTransferContent.Companion.fromNative(s: MemorySegment): DataTransferContent {
    val mimeTypesString = NativeDataTransferContent.mime_types(s).getUtf8String(0)
    val nativeU8Array = NativeDataTransferContent.data(s)
    val len = NativeBorrowedArray_u8.len(nativeU8Array)
    val buf = ByteArray(len.toInt())
    val dataPtr = NativeBorrowedArray_u8.ptr(nativeU8Array)
    for (i in 0 until len) {
        buf[i.toInt()] = dataPtr.getAtIndex(desktop_linux_h.C_CHAR, i)
    }
    return DataTransferContent(
        data = buf,
        mimeTypes = mimeTypesString.split(','),
    )
}

internal fun DataSource.Companion.fromNative(nativeDataSource: Int): DataSource = when (nativeDataSource) {
    desktop_linux_h.NativeDataSource_Clipboard() -> DataSource.Clipboard
    desktop_linux_h.NativeDataSource_DragAndDrop() -> DataSource.DragAndDrop
    else -> error("Unexpected data source type $nativeDataSource")
}

internal fun mimeTypesToNative(arena: Arena, mimeTypes: List<String>): MemorySegment {
    return arena.allocateUtf8String(mimeTypes.joinToString(","))
}

internal fun ByteArray?.toNative(arena: Arena): MemorySegment {
    val nativeDataArray = NativeBorrowedArray_u8.allocate(arena)
    if (this == null) {
        NativeBorrowedArray_u8.len(nativeDataArray, 0)
        NativeBorrowedArray_u8.ptr(nativeDataArray, MemorySegment.NULL)
    } else {
        NativeBorrowedArray_u8.len(nativeDataArray, size.toLong())
        val nativeArray = arena.allocate(MemoryLayout.sequenceLayout(size.toLong(), desktop_linux_h.C_CHAR))
        this.forEachIndexed { i, b ->
            nativeArray.setAtIndex(desktop_linux_h.C_CHAR, i.toLong(), b)
        }

        NativeBorrowedArray_u8.ptr(nativeDataArray, nativeArray)

        NativeBorrowedArray_u8.deinit(
            nativeDataArray,
            NativeBorrowedArray_u8.deinit.allocate({ _, _ ->
                arena.close()
            }, arena),
        )
    }

    return nativeDataArray
}

internal fun DragAndDropQueryData.Companion.fromNative(s: MemorySegment): DragAndDropQueryData {
    return DragAndDropQueryData(
        windowId = NativeDragAndDropQueryData.window_id(s),
        point = LogicalPoint.fromNative(NativeDragAndDropQueryData.point(s)),
    )
}

internal fun DragAction.toNative(): Int = when (this) {
    DragAction.Copy -> desktop_linux_h.NativeDragAction_Copy()
    DragAction.Move -> desktop_linux_h.NativeDragAction_Move()
    DragAction.Ask -> desktop_linux_h.NativeDragAction_Ask()
}

private fun readNativeU32Array(nativeU32Array: MemorySegment): List<Int> {
    val len = NativeBorrowedArray_u32.len(nativeU32Array)
    val dataPtr = NativeBorrowedArray_u32.ptr(nativeU32Array)
    val values = mutableListOf<Int>()
    for (i in 0 until len) {
        val raw = dataPtr.getAtIndex(desktop_linux_h.C_INT, i)
        Logger.debug { "readNativeU32ArrayFor: len=$len : dataPtr=$dataPtr, value of index $i : $raw" }
        values.add(raw)
    }
    return values
}

internal fun Event.Companion.fromNative(s: MemorySegment, app: Application): Event {
    return when (NativeEvent.tag(s)) {
        desktop_linux_h.NativeEvent_ApplicationStarted() -> {
            Event.ApplicationStarted
        }
        desktop_linux_h.NativeEvent_ApplicationWantsToTerminate() -> {
            Event.ApplicationWantsToTerminate
        }
        desktop_linux_h.NativeEvent_ApplicationWillTerminate() -> {
            Event.ApplicationWillTerminate
        }
        desktop_linux_h.NativeEvent_DisplayConfigurationChange() -> {
            Event.DisplayConfigurationChange(screens = app.allScreens())
        }
        desktop_linux_h.NativeEvent_XdgDesktopSettingChange() -> {
            val nativeEvent = NativeEvent.xdg_desktop_setting_change(s)
            Event.XdgDesktopSettingChange(
                setting = XdgDesktopSetting.fromNative(nativeEvent),
            )
        }

        desktop_linux_h.NativeEvent_DataTransferAvailable() -> {
            val nativeEvent = NativeEvent.data_transfer_available(s)
            val mimeTypesString = NativeDataTransferAvailableEvent.mime_types(nativeEvent).getUtf8String(0)
            Event.DataTransferAvailable(mimeTypes = mimeTypesString.split(","))
        }
        desktop_linux_h.NativeEvent_DataTransferCancelled() -> {
            val nativeEvent = NativeEvent.data_transfer_cancelled(s)
            Event.DataTransferCancelled(
                dataSource = DataSource.fromNative(NativeDataTransferCancelledEvent.data_source(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_DataTransfer() -> {
            val nativeEvent = NativeEvent.data_transfer(s)
            Event.DataTransfer(
                serial = NativeDataTransferContent.serial(nativeEvent),
                data = DataTransferContent.fromNative(nativeEvent),
            )
        }
        desktop_linux_h.NativeEvent_FileChooserResponse() -> {
            val nativeEvent = NativeEvent.file_chooser_response(s)
            val filesString = fromOptionalNativeString(NativeFileChooserResponse.newline_separated_files(nativeEvent))
            Event.FileChooserResponse(
                requestId = RequestId(NativeFileChooserResponse.request_id(nativeEvent)),
                files = filesString?.trimEnd()?.split("\r\n") ?: emptyList(),
            )
        }
        desktop_linux_h.NativeEvent_KeyboardLayoutChanged() -> {
            Event.KeyboardLayoutChanged
        }
        desktop_linux_h.NativeEvent_KeyDown() -> {
            val nativeEvent = NativeEvent.key_down(s)
            Event.KeyDown(
                keyCode = KeyCode(NativeKeyDownEvent.code(nativeEvent)),
                characters = fromOptionalNativeString(NativeKeyDownEvent.characters(nativeEvent)),
                key = KeySym(NativeKeyDownEvent.key(nativeEvent)),
                isRepeat = NativeKeyDownEvent.is_repeat(nativeEvent),
                virtualKey = VirtualKey.fromNative(NativeKeyDownEvent.vk(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_KeyUp() -> {
            val nativeEvent = NativeEvent.key_up(s)
            Event.KeyUp(
                key = KeySym(NativeKeyUpEvent.key(nativeEvent)),
                keyCode = KeyCode(NativeKeyUpEvent.code(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_TextInputAvailability() -> {
            val nativeEvent = NativeEvent.text_input(s)
            Event.TextInputAvailability(
                windowId = NativeTextInputAvailabilityEvent.window_id(nativeEvent),
                available = NativeTextInputAvailabilityEvent.available(nativeEvent),
            )
        }
        desktop_linux_h.NativeEvent_TextInput() -> {
            val nativeEvent = NativeEvent.text_input(s)
            Event.TextInput(
                preeditStringData = if (NativeTextInputEvent.has_preedit_string(nativeEvent)) {
                    TextInputPreeditStringData.fromNative(NativeTextInputEvent.preedit_string(nativeEvent))
                } else {
                    null
                },
                commitStringData = if (NativeTextInputEvent.has_commit_string(nativeEvent)) {
                    TextInputCommitStringData(
                        text = fromOptionalNativeString(NativeTextInputEvent.commit_string(nativeEvent)),
                    )
                } else {
                    null
                },
                deleteSurroundingTextData = if (NativeTextInputEvent.has_delete_surrounding_text(nativeEvent)) {
                    TextInputDeleteSurroundingTextData.fromNative(NativeTextInputEvent.delete_surrounding_text(nativeEvent))
                } else {
                    null
                },
            )
        }
        desktop_linux_h.NativeEvent_ModifiersChanged() -> {
            val nativeEvent = NativeEvent.modifiers_changed(s)
            val nativeModifiers = NativeModifiersChangedEvent.modifiers(nativeEvent)
            Event.ModifiersChanged(modifiers = keyModifiersFromNative(nativeModifiers))
        }
        desktop_linux_h.NativeEvent_MouseMoved() -> {
            val nativeEvent = NativeEvent.mouse_moved(s)
            Event.MouseMoved(
                windowId = NativeMouseMovedEvent.window_id(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(NativeMouseMovedEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(NativeMouseMovedEvent.timestamp(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_MouseEntered() -> {
            val nativeEvent = NativeEvent.mouse_entered(s)
            Event.MouseEntered(
                windowId = NativeMouseEnteredEvent.window_id(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(NativeMouseEnteredEvent.location_in_window(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_MouseExited() -> {
            val nativeEvent = NativeEvent.mouse_exited(s)
            Event.MouseExited(
                windowId = NativeMouseExitedEvent.window_id(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(NativeMouseExitedEvent.location_in_window(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_MouseUp() -> {
            val nativeEvent = NativeEvent.mouse_up(s)
            Event.MouseUp(
                windowId = NativeMouseUpEvent.window_id(nativeEvent),
                button = MouseButton(NativeMouseUpEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(NativeMouseUpEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(NativeMouseUpEvent.timestamp(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_MouseDown() -> {
            val nativeEvent = NativeEvent.mouse_down(s)
            Event.MouseDown(
                windowId = NativeMouseDownEvent.window_id(nativeEvent),
                button = MouseButton(NativeMouseDownEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(NativeMouseDownEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(NativeMouseDownEvent.timestamp(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_ScrollWheel() -> {
            val nativeEvent = NativeEvent.scroll_wheel(s)
            Event.ScrollWheel(
                windowId = NativeScrollWheelEvent.window_id(nativeEvent),
                scrollingDeltaX = NativeScrollWheelEvent.scrolling_delta_x(nativeEvent).toFloat(),
                scrollingDeltaY = NativeScrollWheelEvent.scrolling_delta_y(nativeEvent).toFloat(),
                locationInWindow = LogicalPoint.fromNative(NativeScrollWheelEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(NativeScrollWheelEvent.timestamp(nativeEvent)),
            )
        } desktop_linux_h.NativeEvent_WindowScreenChange() -> {
            val nativeEvent = NativeEvent.window_screen_change(s)
            Event.WindowScreenChange(
                windowId = NativeWindowScreenChangeEvent.window_id(nativeEvent),
                newScreenId = NativeWindowScreenChangeEvent.new_screen_id(nativeEvent),
            )
        }
        desktop_linux_h.NativeEvent_WindowConfigure() -> {
            val nativeEvent = NativeEvent.window_configure(s)
            Event.WindowConfigure(
                windowId = NativeWindowConfigureEvent.window_id(nativeEvent),
                size = LogicalSize.fromNative(NativeWindowConfigureEvent.size(nativeEvent)),
                active = NativeWindowConfigureEvent.active(nativeEvent),
                maximized = NativeWindowConfigureEvent.maximized(nativeEvent),
                fullscreen = NativeWindowConfigureEvent.fullscreen(nativeEvent),
                decorationMode = WindowDecorationMode.fromNative(NativeWindowConfigureEvent.decoration_mode(nativeEvent)),
                capabilities = WindowCapabilities.fromNative(NativeWindowConfigureEvent.capabilities(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_WindowKeyboardEnter() -> {
            val nativeEvent = NativeEvent.window_keyboard_enter(s)

            val keyCodes = readNativeU32Array(NativeWindowKeyboardEnterEvent.raw(nativeEvent)).map { KeyCode(it) }
            val keySyms = readNativeU32Array(NativeWindowKeyboardEnterEvent.keysyms(nativeEvent)).map { KeySym(it) }

            Event.WindowKeyboardEnter(
                windowId = NativeWindowKeyboardEnterEvent.window_id(nativeEvent),
                keyCodes,
                keySyms,
            )
        }
        desktop_linux_h.NativeEvent_WindowKeyboardLeave() -> {
            val nativeEvent = NativeEvent.window_keyboard_leave(s)
            Event.WindowKeyboardLeave(windowId = NativeWindowKeyboardLeaveEvent.window_id(nativeEvent))
        }
        desktop_linux_h.NativeEvent_WindowCloseRequest() -> {
            val nativeEvent = NativeEvent.window_close_request(s)
            Event.WindowCloseRequest(windowId = NativeWindowCloseRequestEvent.window_id(nativeEvent))
        }
        desktop_linux_h.NativeEvent_WindowDraw() -> {
            val nativeEvent = NativeEvent.window_draw(s)
            Event.WindowDraw(
                windowId = NativeWindowDrawEvent.window_id(nativeEvent),
                softwareDrawData = SoftwareDrawData.fromNative(NativeWindowDrawEvent.software_draw_data(nativeEvent)),
                size = PhysicalSize.fromNative(NativeWindowDrawEvent.physical_size(nativeEvent)),
                scale = NativeWindowDrawEvent.scale(nativeEvent),
            )
        }
        desktop_linux_h.NativeEvent_WindowScaleChanged() -> {
            val nativeEvent = NativeEvent.window_scale_changed(s)
            Event.WindowScaleChanged(
                windowId = NativeWindowScaleChangedEvent.window_id(nativeEvent),
                newScale = NativeWindowScaleChangedEvent.new_scale(nativeEvent),
            )
        }
        else -> {
            error("Unexpected Event tag")
        }
    }
}

internal fun FileDialog.CommonDialogParams.toNative(arena: Arena): MemorySegment {
    val result = NativeCommonFileDialogParams.allocate(arena)
    NativeCommonFileDialogParams.modal(result, modal)
    NativeCommonFileDialogParams.title(result, title?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL)
    NativeCommonFileDialogParams.accept_label(result, acceptLabel?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL)
    NativeCommonFileDialogParams.current_folder(result, currentFolder?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL)
    return result
}

internal fun FileDialog.OpenDialogParams.toNative(arena: Arena): MemorySegment {
    val result = NativeOpenFileDialogParams.allocate(arena)
    NativeOpenFileDialogParams.select_directories(result, selectDirectories)
    NativeOpenFileDialogParams.allows_multiple_selection(result, allowsMultipleSelections)
    return result
}

internal fun FileDialog.SaveDialogParams.toNative(arena: Arena): MemorySegment {
    val result = NativeSaveFileDialogParams.allocate(arena)
    NativeSaveFileDialogParams.name_field_string_value(
        result,
        nameFieldStringValue?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL,
    )
    return result
}
