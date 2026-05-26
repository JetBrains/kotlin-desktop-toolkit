# Win32 system-menu restoration — design

**Date**: 2026-05-26

## 1. Purpose

Restore three system-menu affordances on `WindowTitleBarKind::Custom` and `WindowTitleBarKind::None` windows while keeping `WS_SYSMENU` cleared at runtime.

| Affordance | Trigger | Title-bar kinds | Anchor (screen coords) |
|------------|---------|-----------------|------------------------|
| Alt+Space | `WM_SYSCOMMAND` with `wParam & 0xFFF0 == SC_KEYMENU` and `lParam == ' '` | `Custom`, `None` | `(visible.left, visible.top)` — top-left of the visible window frame. `visible` = `DWMWA_EXTENDED_FRAME_BOUNDS`. |
| Title-bar right-click | `WM_NCRBUTTONUP` with `wParam == HTCAPTION` | `Custom` (`None` has no synthetic drag band) | `lParam` (cursor) |
| Caption-button right-click | — | — | Out of scope. The strip's pointer-message swallow path ([on_pointerdown](../../src/win32/event_loop.rs#L887-L907) / [on_pointerup](../../src/win32/event_loop.rs#L947-L955)) consumes `WM_(NC)POINTERDOWN/UP` for non-primary buttons and returns `Some(LRESULT(0))` without forwarding to `DefWindowProc`; the legacy `WM_NCRBUTTONUP` is then never delivered. The swallow path is button-kind-agnostic — it gates on `caption_kind_at_screen(...).is_some()`, so every visible caption button is covered (enabled or disabled). Spot-checked empirically on the disabled-`Maximize` case. |

Visible bounds come from `Window::get_physical_rect` (wraps `DwmGetWindowAttribute(DWMWA_EXTENDED_FRAME_BOUNDS)`). `GetWindowRect` is the fallback path only — its outer rect includes the invisible resize border that DWM uses for the drop shadow.

## 2. Runtime style stays narrowed

`WindowStyle::to_system` clears `WS_SYSMENU` for `Custom` / `None` ([window_api.rs:55-57](../../src/win32/window_api.rs#L55-L57)). With the bit kept set, Windows draws native caption buttons over the toolkit-drawn strip even when `WM_NCCALCSIZE` reduces the title-bar margin to zero — verified empirically.

### How the `HMENU` survives the style narrow

The window menu is lazy. Per [GetSystemMenu docs](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getsystemmenu) and Raymond Chen's ["Why doesn't my window get system menu commands?"](https://devblogs.microsoft.com/oldnewthing/20100528-00/?p=13893), the window uses a shared global-default menu until `GetSystemMenu(hwnd, FALSE)` is called once, at which point it's promoted to a per-window copy. Docs say the previous menu "is destroyed" only when `GetSystemMenu(hwnd, TRUE)` is called — the toolkit never invokes that, so the copy lives until the window is destroyed.

Sequence:

1. `CreateWindowExW` ([window.rs:179](../../src/win32/window.rs#L179)) uses `WS_OVERLAPPEDWINDOW`. `WS_SYSMENU` is set.
2. `initialize_window` calls `seed_system_menu(hwnd)` for `Custom` / `None` — the dummy `GetSystemMenu(hwnd, FALSE)` that materialises the per-window copy.
3. The same function narrows the style via `SetWindowLongPtrW(GWL_STYLE, to_system())`, clearing `WS_SYSMENU`.
4. Every later `GetSystemMenu(hwnd, FALSE)` returns the cached copy. Microsoft docs don't explicitly say the call works after `WS_SYSMENU` is cleared, but empirical testing confirms it does — and `ensure_system_menu`'s `WS_SYSMENU`-toggle fallback covers the case if it ever stops working.

## 3. `Window::show_system_menu(screen_pt)`

```rust
pub(crate) fn show_system_menu(&self, screen_pt: PhysicalPoint) -> anyhow::Result<()> {
    let hwnd = self.hwnd();
    let h_menu = ensure_system_menu(hwnd)?;

    sync_system_menu_state(h_menu, &self.style.borrow(), self.is_maximized(), self.is_minimized());

    let _ = unsafe { SetForegroundWindow(hwnd) };
    // TrackPopupMenu with TPM_RETURNCMD returns 0 for both cancel and failure;
    // distinguish via GetLastError.
    unsafe { SetLastError(WIN32_ERROR(0)) };
    let cmd = unsafe {
        TrackPopupMenu(h_menu, TPM_RIGHTBUTTON | TPM_RETURNCMD,
                       screen_pt.x.0, screen_pt.y.0, None, hwnd, None)
    };
    let last_error = unsafe { GetLastError() };
    let _ = unsafe { PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) };

    if cmd.0 == 0 && last_error != ERROR_SUCCESS {
        anyhow::bail!("TrackPopupMenu failed: {last_error:?}");
    }
    if let Ok(cmd_id) = u32::try_from(cmd.0) && cmd_id != 0 {
        self.send_system_command(cmd_id & 0xFFF0);
    }
    Ok(())
}
```

Dispatch goes through the existing [`send_system_command`](../../src/win32/window.rs#L381) so native min/max/restore animations are preserved.

### 3.1 `sync_system_menu_state`

Pure decision table over `(WindowStyle, is_maximized, is_minimized)`, applied via `EnableMenuItem(MF_BYCOMMAND | MF_ENABLED|MF_GRAYED)`. Reapplied before each show because the `HMENU` is reused.

| `SC_*` item | Enabled iff |
|-------------|-------------|
| `SC_RESTORE` | `is_maximized \|\| is_minimized` |
| `SC_MOVE` | `!is_maximized` |
| `SC_SIZE` | `is_resizable && !is_maximized && !is_minimized` |
| `SC_MINIMIZE` | `is_minimizable && !is_minimized` |
| `SC_MAXIMIZE` | `is_maximizable && !is_maximized` |
| `SC_CLOSE` | always (extension point for future `is_closable`) |

### 3.2 `ensure_system_menu`

```rust
pub(crate) fn ensure_system_menu(hwnd: HWND) -> anyhow::Result<HMENU> {
    let h_menu = unsafe { GetSystemMenu(hwnd, false) };
    if !h_menu.is_invalid() {
        return Ok(h_menu);
    }
    log::warn!("GetSystemMenu fast path returned NULL; falling back to WS_SYSMENU toggle");

    let prev = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) };
    let with_sysmenu: u32 = u32::try_from(prev).unwrap() | WS_SYSMENU.0;
    unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, with_sysmenu.try_into().unwrap()) };
    let h_menu = unsafe { GetSystemMenu(hwnd, false) };
    unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, prev) };

    if h_menu.is_invalid() {
        anyhow::bail!("GetSystemMenu returned NULL even after WS_SYSMENU toggle");
    }
    Ok(h_menu)
}
```

The toggle path is defence-in-depth — should never run post-seed. The style restore is unconditional so the bit cannot leak on the error path. `SWP_FRAMECHANGED` is deliberately omitted (no visible frame redraw).

## 4. Wndproc arms ([event_loop.rs](../../src/win32/event_loop.rs))

```rust
WM_NCRBUTTONUP => on_ncrbuttonup(window, wparam, lparam),
WM_SYSCOMMAND  => on_syscommand(window, wparam, lparam),
```

```rust
fn on_ncrbuttonup(window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if !window.has_custom_title_bar() { return None; }
    if wparam.0 as u32 != HTCAPTION { return None; }
    let pt = PhysicalPoint::new(GET_X_LPARAM!(lparam.0), GET_Y_LPARAM!(lparam.0));
    match window.show_system_menu(pt) {
        Ok(()) => Some(LRESULT(0)),
        Err(err) => { log::warn!("show_system_menu (WM_NCRBUTTONUP) failed: {err}"); None }
    }
}

fn on_syscommand(window: &Window, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if !window.has_non_system_title_bar() { return None; }
    let cmd = (wparam.0 & 0xFFF0) as u32;
    if cmd != SC_KEYMENU || lparam.0 != ' ' as isize { return None; }
    let anchor = alt_space_anchor(window);
    match window.show_system_menu(anchor) {
        Ok(()) => Some(LRESULT(0)),
        Err(err) => { log::warn!("show_system_menu (Alt+Space) failed: {err}"); None }
    }
}
```

On show failure both arms return `None` so `DefWindowProc` gets the message — pre-restoration behaviour, no crash.

`on_keyevent` is unchanged. `WM_SYSKEYDOWN/UP` (including VK_SPACE under ALT) still forwards to Kotlin. If Kotlin doesn't consume it, the wndproc returns `None`, `DefWindowProc` runs, and the resulting `WM_SYSCOMMAND/SC_KEYMENU` is caught above — same override semantics as `System`-kind windows.

## 5. Tests

- **Unit**: `system_menu_enable_table` decision table (`#[cfg(test)] mod tests` in `system_menu.rs`). Pure function, six test cases.
- **Manual (sample app)**: for each title-bar kind × window state, verify menu opens at the expected anchor with OS-localised strings, each item dispatches the correct `SC_*` command, and no `GetSystemMenu fast path returned NULL` warn appears.
- **`./gradlew lint`** — required green per [CLAUDE.md](../../../../CLAUDE.md).

## 6. Out of scope

- Caption-button right-click → menu (native Win11 doesn't either).
- `TPM_LAYOUTRTL` — covered by the RTL-mirroring TODO entry.
- `is_closable` — separate TODO entry; `sync_system_menu_state` is the hook point.
- FFI surface to open the menu programmatically.

## References

- [GetSystemMenu](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getsystemmenu)
- [TrackPopupMenu](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-trackpopupmenu)
- [WM_SYSCOMMAND](https://learn.microsoft.com/windows/win32/menurc/wm-syscommand) — `SC_KEYMENU` lParam is the ALT+key character
- [EnableMenuItem](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-enablemenuitem)
- Raymond Chen, [Why doesn't my window get system menu commands?](https://devblogs.microsoft.com/oldnewthing/20100528-00/?p=13893)
- Caption-button design: [2026-04-30-win32-caption-buttons-design.md](2026-04-30-win32-caption-buttons-design.md)
