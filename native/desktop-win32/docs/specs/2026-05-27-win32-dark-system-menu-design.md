# Win32 dark-mode system menu — design

**Date**: 2026-05-27

## 1. Purpose

Make the native `HMENU` system-menu popup honour Windows 11's dark theme. The popup opens from `WindowTitleBarKind::Custom` and `WindowTitleBarKind::None` windows via Alt+Space and a right-click on the title bar (see [2026-05-26-win32-system-menu-restoration-design.md](2026-05-26-win32-system-menu-restoration-design.md)). By default it paints with the classic light `COLOR_MENU` palette regardless of the system theme.

To recolour it, drive the OS's immersive dark-mode pipeline through the undocumented `uxtheme.dll` ordinal #135 `SetPreferredAppMode` — the same mechanism Chromium ([`base/win/dark_mode_support.cc`](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/base/win/dark_mode_support.cc)), Edge, Electron, WinAppSDK, and the Windows Settings app rely on. The mode is process-global, so each window's menu is themed at the moment it opens: the popup path forces `ForceDark`/`ForceLight` to match the opening window's own state, shows the menu, then restores the prior mode.

### What this design is NOT

- No Skia-rendered popup replacement (Chromium's `views::MenuRunner` path).
- No XAML Islands or WinUI `MenuFlyout`.
- No HMENU owner-draw.

## 2. Mechanism

Two independent Windows APIs govern the two surfaces (popup and title-bar frame):

| API | Scope | Documented? | Role |
|-----|-------|-------------|------|
| `SetPreferredAppMode(Force*)` — `uxtheme.dll` ordinal #135 | Process | No | Sole governor of the `HMENU` popup colour. Scoped around each `TrackPopupMenu` and paired with `FlushMenuThemes` (#136) so the menu re-themes for the opening window (§3, §4). |
| `DwmSetWindowAttribute(DWMWA_USE_IMMERSIVE_DARK_MODE, …)` | Per HWND | Yes (Win11 22000+) | Governs the title-bar dark frame only. Called from [`Window::set_immersive_dark_mode`](../../src/win32/window.rs). |

The popup remains a native `HMENU`: same handle, same hit-testing, same accessibility tree, same `TrackPopupMenu` codepath.

### Caveats

- **The ordinals are undocumented.** Microsoft's [undocumented-APIs policy](https://learn.microsoft.com/windows/compatibility/undocumented-apis) warns against ordinal lookups, but #135 and #136 have been stable since Windows 10 1903 across the consumers named in §1. Resolution failure is non-fatal (§6); worst case, the menu stays light.
- **`SetPreferredAppMode` is process-global.** The popup path forces the opening window's mode only for the duration of its `TrackPopupMenu` and restores the prior mode afterward, so each window's menu matches that window and the process is left as it was found. `TrackPopupMenu` is modal, so two windows never theme a menu at once. Title-bar frames are independent — the DWM bit is per-HWND.
- **`ForceDark` / `ForceLight` ignore the system theme.** The popup colour follows the Kotlin caller, not the user's Windows theme setting.
- **The popup is not visually Windows 11.** It is a classic flat menu — no Mica, no rounded corners, no reveal animation. Those treatments exist only on XAML `MenuFlyout`, not on native HMENU.

## 3. Module layout

The code lives in [`appearance.rs`](../../src/win32/appearance.rs) alongside the `Appearance` and `HighContrast` types, exposing one `pub(crate)` entry point — a scope function that forces a menu palette for the duration of a closure:

```rust
pub(crate) fn with_preferred_app_mode<R>(appearance: Appearance, f: impl FnOnce() -> R) -> R;
```

### 3.1 Internal state

```rust
type RawProcFn = unsafe extern "system" fn() -> isize;
// Raw `i32` both ways: pass `PreferredAppMode as i32` going in, feed the returned previous
// mode straight back when restoring, without rebuilding a `#[repr(i32)]` enum from it.
type SetPreferredAppModeFn = unsafe extern "system" fn(i32) -> i32;
type FlushMenuThemesFn = unsafe extern "system" fn();

#[repr(i32)]
#[allow(dead_code)]
enum PreferredAppMode {
    Default = 0,
    AllowDark = 1,
    ForceDark = 2,
    ForceLight = 3,
    Max = 4,
}
```

Both pointers are resolved once and cached together in a function-scoped `static OnceLock<Option<UxThemeFns>>` (the `UxThemeFns` struct and its loader follow in §3.2). Resolution is all-or-nothing: #135 and #136 shipped together in Windows 10 1903, below our 22000 gate, so they are present as a pair. `SetPreferredAppMode` flips the *process* preferred mode, and `FlushMenuThemes` (ordinal #136) drops the cached menu theme so the next `TrackPopupMenu` re-themes. The guard in §3.2 pairs them: it forces a mode and flushes on creation, then restores the previous mode and flushes again on drop.

### 3.2 `with_preferred_app_mode` and the scope guard

`with_preferred_app_mode` runs a closure with the menu palette forced to `appearance`. A private RAII guard forces the mode (and flushes) when constructed and restores the previous mode (and flushes) on drop, so the closure — the `TrackPopupMenu` call — sees the forced palette and the process mode is left as it was.

```rust
struct PreferredAppModeGuard {
    fns: &'static UxThemeFns,
    previous: i32,
}

impl PreferredAppModeGuard {
    fn set(appearance: Appearance) -> Option<Self> {
        let fns = ux_theme_fns()?;
        let mode = match appearance {
            Appearance::Dark => PreferredAppMode::ForceDark,
            Appearance::Light => PreferredAppMode::ForceLight,
        };
        let previous = unsafe { (fns.set_preferred_app_mode)(mode as i32) };
        unsafe { (fns.flush_menu_themes)() };
        Some(Self { fns, previous })
    }
}

impl Drop for PreferredAppModeGuard {
    fn drop(&mut self) {
        unsafe { (self.fns.set_preferred_app_mode)(self.previous) };
        unsafe { (self.fns.flush_menu_themes)() };
    }
}

pub(crate) fn with_preferred_app_mode<R>(appearance: Appearance, f: impl FnOnce() -> R) -> R {
    let _guard = PreferredAppModeGuard::set(appearance);
    f()
}
```

When uxtheme is unavailable (pre-22000, resolution failure) `set` returns `None` and the closure still runs, so the menu opens unthemed.

The version probe, single DLL load, and per-ordinal resolution sit in `ux_theme_fns`, which caches the pair in one `OnceLock` and returns `None` unless both resolve:

```rust
struct UxThemeFns {
    set_preferred_app_mode: SetPreferredAppModeFn,
    flush_menu_themes: FlushMenuThemesFn,
}

fn ux_theme_fns() -> Option<&'static UxThemeFns> {
    static FNS: OnceLock<Option<UxThemeFns>> = OnceLock::new();
    FNS.get_or_init(|| {
        if !utils::is_windows_11_build_22000_or_higher() {
            return None;
        }
        let module = match unsafe { LoadLibraryExW(w!("uxtheme.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32) } {
            Ok(module) => module,
            Err(err) => {
                log::warn!("LoadLibraryExW(uxtheme.dll) failed: {err}");
                return None;
            }
        };
        let set_preferred_app_mode = resolve_ordinal(module, 135, "SetPreferredAppMode")?;
        let flush_menu_themes = resolve_ordinal(module, 136, "FlushMenuThemes")?;
        Some(UxThemeFns {
            // SAFETY: ordinal #135 signature per Chromium base/win/dark_mode_support.cc
            // and ysc3839/win32-darkmode. Both stub and target are `extern "system"`.
            set_preferred_app_mode: unsafe { std::mem::transmute::<RawProcFn, SetPreferredAppModeFn>(set_preferred_app_mode) },
            // SAFETY: ordinal #136 signature per ysc3839/win32-darkmode (`void()`).
            flush_menu_themes: unsafe { std::mem::transmute::<RawProcFn, FlushMenuThemesFn>(flush_menu_themes) },
        })
    })
    .as_ref()
}

fn resolve_ordinal(module: HMODULE, n: u16, name: &str) -> Option<RawProcFn> {
    // MAKEINTRESOURCEA: the ordinal sits in the low word of an otherwise-null PCSTR.
    let ordinal = PCSTR(n as usize as *const u8);
    let raw = unsafe { GetProcAddress(module, ordinal) };
    if raw.is_none() {
        log::warn!("uxtheme ordinal #{n} ({name}) missing");
    }
    raw
}
```

`LOAD_LIBRARY_SEARCH_SYSTEM32` blocks the loader from resolving `uxtheme.dll` from a planted copy on the application's search path. The `ordinal` cast is the `MAKEINTRESOURCEA` form `GetProcAddress` expects — the value in the low word, no string.

## 4. Wire-up — [`window.rs`](../../src/win32/window.rs)

`show_system_menu` owns the popup colour. It reads the window's dark/light state — `self.immersive_dark`, maintained by the existing `set_immersive_dark_mode` alongside the DWM frame and caption strip — and wraps just the `TrackPopupMenu` call, so the menu is themed from the window that opened it. `GetLastError` is read inside the closure, before the guard's restore runs (which calls `SetPreferredAppMode` again and sets the last error):

```rust
let appearance = if self.immersive_dark.get() { Appearance::Dark } else { Appearance::Light };
let (cmd, last_error) = appearance::with_preferred_app_mode(appearance, || {
    unsafe { SetLastError(WIN32_ERROR(0)) };
    let cmd = unsafe {
        TrackPopupMenu(h_menu, TPM_RIGHTBUTTON | TPM_RETURNCMD, screen_pt.x.0, screen_pt.y.0, None, hwnd, None)
    };
    (cmd, unsafe { GetLastError() })
});
```

## 5. Cargo.toml

No new dependencies. The imports come from the `windows` features already enabled in [Cargo.toml](../../Cargo.toml), in particular `Win32_System_LibraryLoader`. The code uses:

- `std::sync::OnceLock`, `std::mem::transmute`, and the workspace `log` macros.
- `windows::Win32::System::LibraryLoader::{LoadLibraryExW, GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32}` for the ordinal lookup, plus `windows::Win32::Foundation::HMODULE` for the resolved-module handle.
- `windows_core::{PCSTR, w}` for the wide-string DLL name (`w!`) and the `MAKEINTRESOURCEA` ordinal cast (`PCSTR`) in `resolve_ordinal`.
- `super::utils::is_windows_11_build_22000_or_higher` for the version gate.

## 6. Error handling

Every failure path is non-fatal; `with_preferred_app_mode` still runs the closure, so the menu just opens unthemed (light).

| Failure | Reaction |
|---------|----------|
| Pre-22000 Windows | The version gate returns `None`, cached in `OnceLock`; `set` yields no guard and the closure runs unthemed. |
| `LoadLibraryExW("uxtheme.dll")` fails | A single `log::warn!` carrying the `WinError`; `OnceLock` caches `None`. |
| Ordinal #135 or #136 missing | A single `log::warn!` naming the ordinal; `ux_theme_fns` caches `None` (resolved as a pair). |
| `SetPreferredAppMode` previous mode | Captured as a raw `i32` and fed back verbatim when the guard drops — never rebuilt into a `PreferredAppMode`, so an out-of-range value cannot trigger UB. |

## 7. Tests

**Unit tests are not viable** — ordinal resolution and uxtheme behaviour can only be exercised against a live Windows 11 22000+ system.

**Manual verification** (sample app). Steps 1–8 run on Windows 11 22000+; step 9 is the pre-22000 fallback check.

1. Launch [`SkikoSampleWin32`](../../../../sample/src/main/kotlin/org/jetbrains/desktop/sample/win32/SkikoSampleWin32.kt) and ensure the window is dark — automatic when the system theme is dark, otherwise press `D` (step 5).
2. Right-click the title bar of a `Custom`-title-bar window. The system-menu popup should appear with a dark background, light item text, and visible separators.
3. Press Alt+Space on the same window. The popup should match step 2.
4. Call `setImmersiveDarkMode(false)` from Kotlin. The title bar reverts to light, and the window's system menu now opens light regardless of the Windows system theme.
5. **Re-toggle.** Press the sample's `D` key (in [`SkikoWindowWin32`](../../../../sample/src/main/kotlin/org/jetbrains/desktop/sample/win32/SkikoWindowWin32.kt)) to flip dark↔light repeatedly, reopening the system menu after each flip on a window whose menu was already opened once. The popup must recolour on every reopen.
6. Toggle the Windows system theme. Popups should *not* track the change — `ForceDark` / `ForceLight` overrides the system setting until the next `setImmersiveDarkMode` call.
7. Multi-window: set window A dark and window B light. Open each window's system menu in both orders — each popup matches its own window, independent of order. After a popup closes, a menu opened by an unrelated control (or the other window) is unaffected, confirming the process mode was restored.
8. Verify that `SC_*` dispatch still works (existing minimize, maximize, restore, and close behaviour from the system-menu restoration design).
9. On Windows 10 (any build < 22000), confirm menus stay light, nothing is logged, and the toolkit functions normally.

**Regression.** The existing `system_menu_enable_table` decision-table tests stay green.

**Lint.** `./gradlew lint` must pass per [CLAUDE.md](../../../../CLAUDE.md).

## 8. Out of scope

- **Windows 10 (pre-22000) support.** Ordinals #135/#136 work from Windows 10 1903+, but the version gate in `ux_theme_fns` returns early below build 22000, so the menu opens unthemed there.
- **Per-window `AllowDarkModeForWindow` (ordinal #133).** This ordinal opts an HWND into dark theming for its own child Win32 controls (buttons, scrollbars, edits, listviews). KDT renders via Windows.UI.Composition + Skia and hosts no Win32 child controls, so the call would be a no-op here.
- **Following the system theme (`AllowDark` + `RefreshImmersiveColorPolicyState`, ordinal #104).** Chromium and ysc3839 set `AllowDark`/`Default` and refresh #104 on `WM_SETTINGCHANGE("ImmersiveColorSet")` to track the system theme. This design forces `ForceDark`/`ForceLight` per `setImmersiveDarkMode` call, so #104 does not apply.
- **Rounded corners, Mica, reveal animation.** These treatments exist only on XAML `MenuFlyout`, not on native HMENU.
- **Menu-bar dark theming** (`WM_UAHDRAWMENU` / `WM_UAHDRAWMENUITEM`). The toolkit ships no menu bar.
- **New Kotlin API.** The design reuses the existing `Window.setImmersiveDarkMode(enabled)`.

## References

- [SetWindowAttribute / DWMWA_USE_IMMERSIVE_DARK_MODE](https://learn.microsoft.com/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute)
- [Apply Windows themes (Win32 dark mode scope)](https://learn.microsoft.com/windows/apps/desktop/modernize/ui/apply-windows-themes)
- [Undocumented APIs policy](https://learn.microsoft.com/windows/compatibility/undocumented-apis)
- [LoadLibraryExW + LOAD_LIBRARY_SEARCH_SYSTEM32](https://learn.microsoft.com/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryexw)
- [Chromium `base/win/dark_mode_support.cc`](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/base/win/dark_mode_support.cc) — reference implementation of ordinals 133 / 135.
- [WinAppSDK Discussion #2967 — titlebar context menu dark theme](https://github.com/microsoft/WindowsAppSDK/discussions/2967) — same ordinal usage in WinUI / WinAppSDK.
- [ysc3839/win32-darkmode](https://github.com/ysc3839/win32-darkmode) — canonical reference for the undocumented uxtheme dark-mode ordinals.
- [2026-05-26-win32-system-menu-restoration-design.md](2026-05-26-win32-system-menu-restoration-design.md) — system-menu plumbing this design extends.
- [2026-04-30-win32-caption-buttons-design.md](2026-04-30-win32-caption-buttons-design.md) — caption-button strip context.
