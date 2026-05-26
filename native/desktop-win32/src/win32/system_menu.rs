//! System-menu helpers for `WindowTitleBarKind::Custom` / `None` windows.
//! See `docs/specs/2026-05-26-win32-system-menu-restoration-design.md`.

use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        EnableMenuItem, GWL_STYLE, GetSystemMenu, GetWindowLongPtrW, HMENU, MF_BYCOMMAND, MF_ENABLED, MF_GRAYED, SC_CLOSE, SC_MAXIMIZE,
        SC_MINIMIZE, SC_MOVE, SC_RESTORE, SC_SIZE, SetWindowLongPtrW, WS_SYSMENU,
    },
};

use crate::win32::window_api::WindowStyle;

/// Enable state for each predefined `SC_*` item. Mirrors `DefWindowProc`'s
/// standard grays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SystemMenuEnableTable {
    pub restore: bool,
    pub move_: bool,
    pub size: bool,
    pub minimize: bool,
    pub maximize: bool,
    pub close: bool,
}

#[must_use]
pub(crate) const fn system_menu_enable_table(style: &WindowStyle, is_maximized: bool, is_minimized: bool) -> SystemMenuEnableTable {
    SystemMenuEnableTable {
        restore: is_maximized || is_minimized,
        move_: !is_maximized,
        size: style.is_resizable && !is_maximized && !is_minimized,
        minimize: style.is_minimizable && !is_minimized,
        maximize: style.is_maximizable && !is_maximized,
        close: true,
    }
}

/// Materialise the per-window system-menu copy. The first
/// `GetSystemMenu(hwnd, FALSE)` promotes the window from the shared global-default
/// menu to its own copy; the copy then persists until the window is destroyed.
/// Must be called while `WS_SYSMENU` is still set — i.e. before
/// `initialize_window` narrows the style.
pub(crate) fn seed_system_menu(hwnd: HWND) {
    let _ = unsafe { GetSystemMenu(hwnd, false) };
}

/// Return the window-menu `HMENU` for `hwnd`. Fast path returns the per-window
/// copy created by [`seed_system_menu`]. If the seed never ran, falls back to
/// toggling `WS_SYSMENU` around `GetSystemMenu` (no `SWP_FRAMECHANGED`); the
/// style is restored unconditionally so the bit cannot leak.
pub(crate) fn ensure_system_menu(hwnd: HWND) -> anyhow::Result<HMENU> {
    let h_menu = unsafe { GetSystemMenu(hwnd, false) };
    if !h_menu.is_invalid() {
        return Ok(h_menu);
    }

    log::warn!("GetSystemMenu fast path returned NULL; falling back to WS_SYSMENU toggle");

    let prev_style_bits = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) };
    let prev_style: u32 = prev_style_bits.try_into().unwrap();
    let with_sysmenu = prev_style | WS_SYSMENU.0;
    unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, with_sysmenu.try_into().unwrap()) };
    let h_menu = unsafe { GetSystemMenu(hwnd, false) };
    unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, prev_style_bits) };

    if h_menu.is_invalid() {
        anyhow::bail!("GetSystemMenu returned NULL even after WS_SYSMENU toggle");
    }
    Ok(h_menu)
}

/// Apply [`system_menu_enable_table`] to a live `HMENU`. Called before each
/// show because the same `HMENU` is reused for the window's lifetime.
pub(crate) fn sync_system_menu_state(h_menu: HMENU, style: &WindowStyle, is_maximized: bool, is_minimized: bool) {
    let t = system_menu_enable_table(style, is_maximized, is_minimized);

    let apply = |cmd: u32, enabled: bool| {
        let flag = MF_BYCOMMAND | if enabled { MF_ENABLED } else { MF_GRAYED };
        let _ = unsafe { EnableMenuItem(h_menu, cmd, flag) };
    };

    apply(SC_RESTORE, t.restore);
    apply(SC_MOVE, t.move_);
    apply(SC_SIZE, t.size);
    apply(SC_MINIMIZE, t.minimize);
    apply(SC_MAXIMIZE, t.maximize);
    apply(SC_CLOSE, t.close);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::win32::window_api::{WindowStyle, WindowSystemBackdropType, WindowTitleBarKind};

    fn style_with(is_resizable: bool, is_minimizable: bool, is_maximizable: bool) -> WindowStyle {
        WindowStyle {
            title_bar_kind: WindowTitleBarKind::Custom,
            is_resizable,
            is_minimizable,
            is_maximizable,
            system_backdrop_type: WindowSystemBackdropType::Auto,
        }
    }

    #[test]
    fn restored_resizable_window_enables_all_except_restore() {
        let t = system_menu_enable_table(&style_with(true, true, true), false, false);
        assert_eq!(
            t,
            SystemMenuEnableTable {
                restore: false,
                move_: true,
                size: true,
                minimize: true,
                maximize: true,
                close: true,
            }
        );
    }

    #[test]
    fn maximized_window_enables_restore_and_minimize_but_not_move_size_or_maximize() {
        let t = system_menu_enable_table(&style_with(true, true, true), true, false);
        assert_eq!(
            t,
            SystemMenuEnableTable {
                restore: true,
                move_: false,
                size: false,
                minimize: true,
                maximize: false,
                close: true,
            }
        );
    }

    #[test]
    fn minimized_window_enables_only_restore_move_maximize_close() {
        let t = system_menu_enable_table(&style_with(true, true, true), false, true);
        assert_eq!(
            t,
            SystemMenuEnableTable {
                restore: true,
                move_: true,
                size: false,
                minimize: false,
                maximize: true,
                close: true,
            }
        );
    }

    #[test]
    fn non_resizable_window_disables_size() {
        let t = system_menu_enable_table(&style_with(false, true, true), false, false);
        assert!(!t.size);
        assert!(t.move_);
    }

    #[test]
    fn non_minimizable_window_disables_minimize() {
        let t = system_menu_enable_table(&style_with(true, false, true), false, false);
        assert!(!t.minimize);
        assert!(t.maximize);
    }

    #[test]
    fn non_maximizable_window_disables_maximize() {
        let t = system_menu_enable_table(&style_with(true, true, false), false, false);
        assert!(t.minimize);
        assert!(!t.maximize);
    }
}
