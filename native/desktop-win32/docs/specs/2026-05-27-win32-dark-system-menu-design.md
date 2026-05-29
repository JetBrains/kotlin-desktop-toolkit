# Win32 dark-mode system menu — design

**Date**: 2026-05-27

## 1. Purpose

Make the native `HMENU` system-menu popup honour Windows 11's dark theme. The popup opens from `WindowTitleBarKind::Custom` and `WindowTitleBarKind::None` windows via Alt+Space and a right-click on the title bar (see [2026-05-26-win32-system-menu-restoration-design.md](2026-05-26-win32-system-menu-restoration-design.md)). By default it paints with the classic light `COLOR_MENU` palette regardless of the system theme.

To recolour it, drive the OS's immersive dark-mode pipeline through the undocumented `uxtheme.dll` ordinal #135 `SetPreferredAppMode` — the same mechanism Chromium ([`base/win/dark_mode_support.cc`](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/base/win/dark_mode_support.cc)), Edge, Electron, WinAppSDK, and the Windows Settings app rely on. Each `Window.setImmersiveDarkMode(enabled)` call sets `PreferredAppMode::ForceDark` when `enabled`, `ForceLight` otherwise; the next `TrackPopupMenu` paints the popup in that colour.

### What this design is NOT

- No Skia-rendered popup replacement (Chromium's `views::MenuRunner` path).
- No XAML Islands or WinUI `MenuFlyout`.
- No HMENU owner-draw.

## 2. Mechanism

Two independent Windows APIs govern the two surfaces (popup and title-bar frame):

| API | Scope | Documented? | Role |
|-----|-------|-------------|------|
| `SetPreferredAppMode(Force*)` — `uxtheme.dll` ordinal #135 | Process | No | Sole governor of the `HMENU` popup colour; paired with `FlushMenuThemes` (#136) so already-open menus pick up a change (§3.1). |
| `DwmSetWindowAttribute(DWMWA_USE_IMMERSIVE_DARK_MODE, …)` | Per HWND | Yes (Win11 22000+) | Governs the title-bar dark frame only. Called from [`Window::set_immersive_dark_mode`](../../src/win32/window.rs). |

The popup remains a native `HMENU`: same handle, same hit-testing, same accessibility tree, same `TrackPopupMenu` codepath.

### Caveats

- **The ordinals are undocumented.** Microsoft's [undocumented-APIs policy](https://learn.microsoft.com/windows/compatibility/undocumented-apis) warns against ordinal lookups, but #135 and #136 have been stable since Windows 10 1903 across the consumers named in §1. Resolution failure is non-fatal (§6); worst case, the menu stays light.
- **`SetPreferredAppMode` is process-wide.** Calling `set_immersive_dark_mode` on any window flips the popup mode for every menu opened by the process. A multi-window app with mixed light and dark windows therefore sees last-call-wins on popup colour; every call re-asserts the mode for its window's appearance, so a window can reclaim the popup colour at any time (`set_immersive_dark_mode` does not early-return on an unchanged state). Title-bar frames stay correct per window because the DWM bit is per-HWND. A follow-up in §9 describes how to scope the mode per popup so concurrent windows no longer contend.
- **`ForceDark` / `ForceLight` ignore the system theme.** The popup colour follows the Kotlin caller, not the user's Windows theme setting.
- **The popup is not visually Windows 11.** It is a classic flat menu — no Mica, no rounded corners, no reveal animation. Those treatments exist only on XAML `MenuFlyout`, not on native HMENU.

## 3. Module layout

The code lives in [`appearance.rs`](../../src/win32/appearance.rs) alongside the `Appearance` and `HighContrast` types, exposing one `pub(crate)` entry point:

```rust
pub(crate) fn set_preferred_app_mode(appearance: Appearance);
```

### 3.1 Internal state

```rust
type RawProcFn = unsafe extern "system" fn() -> isize;
// Returns the previous mode as `i32`, not `PreferredAppMode`: an out-of-range enum from FFI would be UB.
type SetPreferredAppModeFn = unsafe extern "system" fn(PreferredAppMode) -> i32;
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

Both pointers are resolved once and cached together in a function-scoped `static OnceLock<Option<UxThemeFns>>` (the `UxThemeFns` struct and its loader follow in §3.2). Resolution is all-or-nothing: #135 and #136 shipped together in Windows 10 1903, below our 22000 gate, so they are present as a pair. `SetPreferredAppMode` flips the *process* preferred mode but leaves an already-opened menu — including the cached per-window system-menu `HMENU` — painted in its previous theme. `FlushMenuThemes` (ordinal #136) drops that cached menu theme so the next `TrackPopupMenu` re-themes; it is called immediately after every `SetPreferredAppMode`.

### 3.2 `set_preferred_app_mode`

```rust
pub(crate) fn set_preferred_app_mode(appearance: Appearance) {
    let Some(fns) = ux_theme_fns() else {
        return;
    };
    let mode = match appearance {
        Appearance::Dark => PreferredAppMode::ForceDark,
        Appearance::Light => PreferredAppMode::ForceLight,
    };
    unsafe { (fns.set_preferred_app_mode)(mode) };
    // SetPreferredAppMode leaves already-themed menus on their old cache; flush so
    // the next TrackPopupMenu re-themes the system-menu HMENU.
    unsafe { (fns.flush_menu_themes)() };
}
```

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

`Window::set_immersive_dark_mode` toggles `DWMWA_USE_IMMERSIVE_DARK_MODE`, derives an `Appearance` from the new state, and propagates it to the caption-button strip. The popup-mode call joins that pipeline at the same point:

```rust
pub fn set_immersive_dark_mode(&self, enabled: bool) -> WinResult<()> {
    // …existing DWM toggle…
    let appearance = if enabled { Appearance::Dark } else { Appearance::Light };
    self.with_strip_mut(|strip| strip.on_appearance_change(appearance));
    appearance::set_preferred_app_mode(appearance);
    self.immersive_dark.set(enabled); // commit state after the side effects land
    Ok(())
}
```

The process-wide mode flip happens whenever a Kotlin caller sets immersive dark/light on a window — `set_immersive_dark_mode` no longer early-returns on an unchanged state, so a window always re-asserts its popup colour (the caption strip still skips repainting when the appearance is unchanged).

## 5. Cargo.toml

No new dependencies. The imports come from the `windows` features already enabled in [Cargo.toml](../../Cargo.toml), in particular `Win32_System_LibraryLoader`. The code uses:

- `std::sync::OnceLock`, `std::mem::transmute`, and the workspace `log` macros.
- `windows::Win32::System::LibraryLoader::{LoadLibraryExW, GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32}` for the ordinal lookup, plus `windows::Win32::Foundation::HMODULE` for the resolved-module handle.
- `windows_core::{PCSTR, w}` for the wide-string DLL name (`w!`) and the `MAKEINTRESOURCEA` ordinal cast (`PCSTR`) in `resolve_ordinal`.
- `super::utils::is_windows_11_build_22000_or_higher` for the version gate.

## 6. Error handling

Every failure path is non-fatal; the toolkit falls back to a light popup.

| Failure | Reaction |
|---------|----------|
| Pre-22000 Windows | The version gate returns `None`, which `OnceLock` caches. Subsequent calls take the cached path with no further probe. |
| `LoadLibraryExW("uxtheme.dll")` fails | `.inspect_err` emits a single `log::warn!` carrying the underlying `WinError`; `OnceLock` caches `None`. |
| Ordinal #135 or #136 missing | A single `log::warn!` naming the ordinal; `ux_theme_fns` caches `None` (resolved as a pair). The toolkit falls back to a light popup. |
| `SetPreferredAppMode` returns a value | Discarded — the return register is read as `i32`, never as a `PreferredAppMode` enum, so an out-of-range value cannot trigger UB. The return is the previous app mode, not a failure code. |

## 7. Tests

**Unit tests are not viable** — ordinal resolution and uxtheme behaviour can only be exercised against a live Windows 11 22000+ system.

**Manual verification** (sample app). Steps 1–8 run on Windows 11 22000+; step 9 is the pre-22000 fallback check.

1. Launch [`SkikoSampleWin32`](../../../../sample/src/main/kotlin/org/jetbrains/desktop/sample/win32/SkikoSampleWin32.kt) and ensure the window is dark — automatic when the system theme is dark, otherwise press `D` (step 5).
2. Right-click the title bar of a `Custom`-title-bar window. The system-menu popup should appear with a dark background, light item text, and visible separators.
3. Press Alt+Space on the same window. The popup should match step 2.
4. Call `setImmersiveDarkMode(false)` from Kotlin. The title bar reverts to light; the next menu opens in `ForceLight` mode (light popup) regardless of the Windows system theme.
5. **Re-toggle.** Press the sample's `D` key (in [`SkikoWindowWin32`](../../../../sample/src/main/kotlin/org/jetbrains/desktop/sample/win32/SkikoWindowWin32.kt)) to flip dark↔light repeatedly, reopening the system menu after each flip on a window whose menu was already opened once. The popup must recolour on every reopen.
6. Toggle the Windows system theme. Popups should *not* track the change — `ForceDark` / `ForceLight` overrides the system setting until the next `setImmersiveDarkMode` call.
7. In a multi-window scenario, set window A to dark and window B to light. Confirm last-call-wins on popup colour: opening the menu on either window paints in the mode last requested.
8. Verify that `SC_*` dispatch still works (existing minimize, maximize, restore, and close behaviour from the system-menu restoration design).
9. On Windows 10 (any build < 22000), confirm menus stay light, nothing is logged, and the toolkit functions normally.

**Regression.** The existing `system_menu_enable_table` decision-table tests stay green.

**Lint.** `./gradlew lint` must pass per [CLAUDE.md](../../../../CLAUDE.md).

## 8. Out of scope

- **Windows 10 (pre-22000) support.** Ordinals #135/#136 work from Windows 10 1903+, but the popup call sits inside `Window::set_immersive_dark_mode`, which early-returns on pre-22000.
- **Per-window `AllowDarkModeForWindow` (ordinal #133).** This ordinal opts an HWND into dark theming for its own child Win32 controls (buttons, scrollbars, edits, listviews). KDT renders via Windows.UI.Composition + Skia and hosts no Win32 child controls, so the call would be a no-op here.
- **Following the system theme (`AllowDark` + `RefreshImmersiveColorPolicyState`, ordinal #104).** Chromium and ysc3839 set `AllowDark`/`Default` and refresh #104 on `WM_SETTINGCHANGE("ImmersiveColorSet")` to track the system theme. This design forces `ForceDark`/`ForceLight` per `setImmersiveDarkMode` call, so #104 does not apply.
- **Rounded corners, Mica, reveal animation.** These treatments exist only on XAML `MenuFlyout`, not on native HMENU.
- **Menu-bar dark theming** (`WM_UAHDRAWMENU` / `WM_UAHDRAWMENUITEM`). The toolkit ships no menu bar.
- **New Kotlin API.** The design reuses the existing `Window.setImmersiveDarkMode(enabled)`.

## 9. Follow-ups (not in this design)

- **Per-popup scoping for multi-window apps.** Inside `Window::show_system_menu` (called from `WM_NCRBUTTONUP` and the Alt+Space arm), capture the previous app mode, set the window's intended mode via `set_preferred_app_mode`, call `TrackPopupMenu`, and then restore the previous mode. This removes the last-call-wins behaviour at the cost of two extra ordinal calls per popup.

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
