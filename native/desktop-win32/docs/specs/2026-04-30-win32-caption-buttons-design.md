# Win32 caption buttons — design

**Spec:** caption-button strip for `WindowTitleBarKind::Custom`.
**Crate:** `native/desktop-win32`.

## 1. Goal

Add minimise / maximise-restore / close caption buttons to windows that use `WindowTitleBarKind::Custom` on Windows. The buttons are toolkit-managed `Windows.UI.Composition` visuals, with full state coverage (rest, hover, pressed, disabled for Minimize / Maximize, plus active / inactive modulation), full appearance coverage (light, dark, high contrast on / off), and Win32-correct hit-testing including Win11 snap-layout flyout integration.

The toolkit owns the buttons end-to-end. Click side-effects translate to `Window::request_close` / `Window::minimize` / `Window::maximize` / `Window::restore`. Apps don't get a per-button-press event — caption buttons behave like the system buttons they replace.

## 2. Scope

Caption-button rendering and pointer interaction for `WindowTitleBarKind::Custom` windows, including `HTMAXBUTTON` Snap Layouts integration, high contrast, DPI, and Windows 10/11 icon fallback. System-menu restoration is tracked separately in `TODO.md`.

Preconditions:

- Non-system titlebar windows (`WindowTitleBarKind::Custom` / `WindowTitleBarKind::None`) keep `WS_CAPTION` but clear `WS_SYSMENU` in `WindowStyle::to_system`. This preserves native min/max/restore transition semantics while suppressing system caption controls; the toolkit owns Minimize / Maximize / Close rendering for `Custom` windows.
- `on_nccalcsize` uses the window style for left / right / bottom frame adjustment and intentionally leaves the Custom top inset at 0 so the title-bar area remains client area. The maximized inset and strip resize are applied after the client rect is computed.
- `on_nchittest` consults `DwmDefWindowProc` / `DefWindowProcW` first, then consults the strip before preserving any non-`HTCLIENT` default for points inside the strip. Outside the strip, any non-`HTCLIENT` result is preserved before the Kotlin `NCHitTestEvent` callback and manual top-edge fallback. The `!window.is_resizable()` guard applies only to resize-border math; non-resizable Custom-titlebar windows still receive caption-button hit-testing and title-bar drag fallback.
- `Appearance` is queried alongside a `HighContrast` enum (`Off` / `On`) which the strip consumes.

## 3. Architecture

### 3.1 Modules

Two crate-internal modules, both `pub(crate)` only — no FFI surface, no `_api.rs` partner.

- **`composition.rs`** — defines `CompositionContext`, private to caption-button rasterisation. Holds the `IDWriteFactory` and `CompositionGraphicsDevice` (both survive device loss); CGD retains the D3D11 / D2D rendering device, swapped on device loss via `SetRenderingDevice`. Hides `BeginDraw` / `EndDraw` and device-loss handling behind a single `with_d2d_render_target(surface, |rt, offset| -> ...)` chokepoint.
- **`caption_buttons.rs`** — owns the per-window strip. Pure state-machine over typed inputs; no Win32 calls itself. The wndproc layer in `event_loop.rs` is the only place that touches both messages and the strip.

### 3.2 Hook points

- **`composition.rs`** exposes `pub(crate) fn ensure_composition_context(compositor: Compositor) -> anyhow::Result<Rc<CompositionContext>>` backed by a thread-local `OnceCell<Rc<CompositionContext>>`. Called once per Custom-titlebar window from inside `CaptionButtonStrip::new`; later calls return the same `Rc<CompositionContext>`. Failure is not memoised — a later Custom-titlebar window retries through the same cell.
- **`Window`** holds `caption_buttons: RefCell<Option<CaptionButtonStrip>>`, populated in `initialize_window` if and only if `style.title_bar_kind == Custom` and caption-button construction succeeds. Construction failure before the window is shown is fatal for `window_create`; a `Custom` titlebar window must not appear without visible caption buttons. The strip owns the `Rc<CompositionContext>` and the `RenderingDeviceReplacedRegistration` guard.
- **`Window`** holds `nc_leave_tracking_armed: AtomicBool` and helper methods `ensure_nc_leave_tracking()` / `nc_leave_tracking_fired()` — see §3.5.
- **`Window`** has `minimize` / `maximize` / `restore`, routed through `SendMessageW(WM_SYSCOMMAND, SC_*)`, used by the strip click path.
- **`event_loop.rs`** handles:
  - Strip consultation in `on_nchittest` after the initial `DwmDefWindowProc` / `DefWindowProcW` consultation, but before preserving a default non-`HTCLIENT` result for points inside the strip.
  - The `on_nchittest` resize-border guard is split: resize-border math is conditional on `is_resizable`, but custom caption-button hit-testing and draggable titlebar fallback run for non-resizable Custom-titlebar windows too.
  - The pointer handlers (`on_pointerupdate`, `on_pointerdown`, `on_pointerup`) dispatch geometrically into `caption_kind_at_screen(window, screen)`, which converts screen coordinates to strip-local coordinates and runs the strip's geometric hit-test. The `HIWORD(wParam)` value documented for `WM_NCPOINTER*` is unreliable on Win11 — it arrives muxed with `POINTER_FLAG_*` bits and does not match `HTCLOSE` / `HTMAXBUTTON` / `HTMINBUTTON`.
  - On `WM_NCPOINTERUPDATE` the dispatch returns `Some(LRESULT(0))` for caption-button hit-test areas to suppress Kotlin-facing pointer events for the toolkit-owned gesture; non-caption NC areas (e.g. title-bar drag region) still fall through to the standard dispatch. The client-variant `WM_POINTERUPDATE` is gated by `strip.has_press_for(pointer_id)` while the strip owns a press: implicit pointer capture can drift from non-client to client mid-press, and an unsuppressed UPDATE on that path lets the host's drag source observe a held primary button without the matching DOWN, kicking off `DoDragDrop`. While suppressing the host event, the wndproc still forwards `strip.on_pointer_update(caption_kind_at_screen(...), ...)` so the WinUI capture rule (`Pressed → PressedDraggedOff` on drift, reverse on return) fires for client-side movement.
  - `on_pointerup` drains tracked `Suppressed` sessions via `strip.consume_swallowed_release(pointer_id, button)` — keyed by `(pointer_id, PointerButton)` so the drain works regardless of where the implicit pointer capture delivers the UP (`WM_NCPOINTERUP` off the strip, or `WM_POINTERUP` in the client area). Releases whose press began outside the strip fall through to Kotlin: chrome ownership scopes to cycles whose DOWN was on the strip.
  - `WM_POINTERCAPTURECHANGED` matches every owned session — `Active` plus any `Suppressed` mode, via `strip.has_press_for(pointer_id)` — and calls `strip.on_pointer_cancel(id)` without firing a caption-button action. Microsoft documents [`WM_POINTERCAPTURECHANGED`](https://learn.microsoft.com/windows/win32/inputmsg/wm-pointercapturechanged) as the cleanup signal and warns not to depend on paired pointer notifications.
  - `WM_CANCELMODE` and `WM_ACTIVATE`-deactivate call `strip.cancel_any_press()` (no `pointer_id` available) and return `None`. [`WM_CANCELMODE`](https://learn.microsoft.com/windows/win32/winmsg/wm-cancelmode) doc says `DefWindowProc` "cancels internal processing of standard scroll bar input, cancels internal menu processing, and releases the mouse capture" — letting it run preserves that. These arms backstop focus-loss paths Windows does not deliver `WM_POINTERCAPTURECHANGED` on (ALT-TAB, `EnableWindow(FALSE)`, RDP disconnect).
  - `TrackMouseEvent(TME_NONCLIENT | TME_LEAVE)` is armed when an NC pointer update enters the window non-client area, not when it enters a specific caption button.
  - `on_ncmouseleave` calls `strip.on_nc_mouse_leave()` and `window.nc_leave_tracking_fired()` before the `DwmDefWindowProc` pass-through.
  - `on_dpichanged` and `on_settingchange` call the strip's invalidation methods. `WM_DPICHANGED` calls `Window::set_dpi_metrics(new_dpi)` (wparam-derived) before any nested handler runs so downstream readers consume the freshly cached metrics without re-syscalling.
  - `on_windowposchanged` calls `strip.on_max_state_change(IsZoomed(hwnd).as_bool())` when the cached maximize state has changed. `on_windowposchanged` returns `Some(LRESULT(0))`; no separate `WM_SIZE` arm. [`IsZoomed`](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-iszoomed) is the documented Win32 API for checking maximized state.
  - `on_nccalcsize` (a) applies the maximized client-area inset described in §3.6 *before* emitting `NCCalcSizeEvent`, and (b) calls `strip.on_resize(max_chrome_y)` after the inset-aware client-rect calculation. Strip mutations queue; the driver's `CommitNeeded` fast-path publishes inline on the UI thread before the wndproc returns. The backdrop tint `SpriteVisual` auto-tracks via `RelativeSizeAdjustment(1,1)` and does not require a per-tick `SetSize`.

### 3.3 Composition tree

The 3-layer split ensures deterministic z-order and keeps the caption strip reliably above ANGLE content:

```
composition_root (ContainerVisual)
├── backdrop_layer (ContainerVisual, bottom)
│   └── backdrop_tint SpriteVisual
├── content_layer  (ContainerVisual, middle)
│   └── ANGLE SpriteVisual (inserted by Window::add_visual)
└── chrome_layer   (ContainerVisual, top)
    └── CaptionButtonStrip's parent ContainerVisual
```

`Window::add_visual` inserts new visuals into `content_layer` rather than the root. `InsertAtTop` is called three times in sequence (`backdrop_layer`, `content_layer`, `chrome_layer`) so chrome is inserted last and is the topmost rendered layer. Microsoft documents `VisualCollection` ordering as bottom-to-top and [`VisualCollection.InsertAtTop`](https://learn.microsoft.com/en-us/uwp/api/windows.ui.composition.visualcollection.insertattop?view=winrt-26100) as inserting at the top of that collection.

### 3.4 Threading

Single UI thread, consistent with the rest of the crate. `CompositionContext` and `CaptionButtonStrip` are not `Send`. The only `Send + 'static` boundary is the `RenderingDeviceReplaced` callback (§6.2), which posts a private `WM_APP_*` message rather than touching `CompositionContext`, `Window`, or `CaptionButtonStrip` directly.

### 3.5 Pointer / leave message routing

`EnableMouseInPointer(true)` routes `WM_MOUSE*` through `WM_POINTER*`. The wndproc dispatch merges `WM_NCPOINTER*` with their client-area counterparts. No `WM_NCMOUSEMOVE` / `WM_NCLBUTTON*` fallback.

[`WM_POINTERLEAVE`](https://learn.microsoft.com/windows/win32/inputmsg/wm-pointerleave) fires *"when a pointer moves outside the boundaries of the window"* — covering NC→outside transitions for hovering pointers. `on_pointerleave` clears `is_pointer_in_window` on this message regardless of which area the pointer was over.

**`PointerEntered` parity for entries via the strip.** `on_pointerupdate` fires `Event::PointerEntered` whenever `is_pointer_in_window` transitions false→true. When the first appearance is over a caption button, the wndproc fires a synthesised `PointerEntered` before returning `Some(LRESULT(0))` to suppress the strip-internal events.

**`WM_NCMOUSELEAVE`** covers NC→client transitions, where the strip's hover state must clear as the pointer slides off a caption button into the title-bar drag area or further into the client. To receive `WM_NCMOUSELEAVE`, the application must arm tracking via `TrackMouseEvent(TME_NONCLIENT | TME_LEAVE, hwndTrack=hwnd, dwHoverTime=0)`. Per the `WM_NCMOUSELEAVE` Remarks: *"All tracking requested by TrackMouseEvent is canceled when this message is generated."* — re-arm on each NC entry to receive the next. `WM_NCMOUSEMOVE` is intentionally unhandled; see TODO `WM_NCMOUSEMOVE fallback if WM_NCPOINTER* is missing on a supported config`.

Tracking armed-state lives on `Window` (one flag per HWND), not on the strip, keeping the strip a pure state machine:

```rust
impl Window {
    pub(crate) fn ensure_nc_leave_tracking(&self) -> anyhow::Result<()> {
        if !self.nc_leave_tracking_armed.swap(true, Ordering::Relaxed) {
            let mut tme = TRACKMOUSEEVENT {
                cbSize: size_of::<TRACKMOUSEEVENT>().try_into()?,
                dwFlags: TME_NONCLIENT | TME_LEAVE,
                hwndTrack: self.hwnd(),
                dwHoverTime: 0,
            };
            unsafe { TrackMouseEvent(&raw mut tme) }?;
        }
        Ok(())
    }

    pub(crate) fn nc_leave_tracking_fired(&self) {
        self.nc_leave_tracking_armed.store(false, Ordering::Relaxed);
    }
}
```

### 3.6 Maximized-window client-area inset

When a `WS_THICKFRAME`-having window is maximized, Windows positions it so that its window rect extends past the monitor's work area on all four sides, so the resize border / drop shadow remains off-screen. For non-system titlebar paths, `on_nccalcsize` leaves the top inset at 0, so composition (0,0) also sits off-monitor.

`WM_NCCALCSIZE` adjusts `rgrc[0].top += max_chrome_y` so the client rect's top sits at the monitor edge. The composition tree shifts in lockstep — `Window::set_content_top_offset` offsets the content layer for both `Custom` and `None`, and additionally offsets the strip's `composition_root` for `Custom`. For non-resizable maximized windows `max_chrome_y` is `0` and both layers stay at composition Y=0 matching the OS-positioned window-rect top.

`max_chrome_y` is the per-monitor DPI-scaled `SM_CYSIZEFRAME` only, not `SM_CYSIZEFRAME + SM_CXPADDEDBORDER`. Windows Terminal's [`_OnNcCalcSize`](https://github.com/microsoft/terminal/blob/e4e3f08efca9d0ffba330eee12edbcb16897ddcb/src/cascadia/WindowsTerminal/NonClientIslandWindow.cpp) uses `SM_CYSIZEFRAME + SM_CXPADDEDBORDER`. The [`SM_CXPADDEDBORDER`](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getsystemmetrics) docs describe it as "border padding for captioned windows" — semantics that depend on which caption-style bits are set. Terminal retains `WS_SYSMENU`; this toolkit clears it. Adding `SM_CXPADDEDBORDER` over-insets for this toolkit's style (`WS_CAPTION` retained, `WS_SYSMENU` cleared): the title-bar area sits below the monitor edge by exactly `SM_CXPADDEDBORDER` pixels.

The inset runs only when `window.is_resizable()` (`WS_THICKFRAME` present); non-resizable windows don't get the maximized off-monitor overhang.

```text
if (wParam == TRUE && window.is_resizable() && IsZoomed(hwnd)) {
    if (window.has_non_system_title_bar()) {
        UINT dpi = GetDpiForWindow(hwnd);
        int max_chrome_y = GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi);
        rgrc[0].top += max_chrome_y;
    }
    // Auto-hide-taskbar claw-back applies to maximized resizable
    // non-system-titlebar windows only — see Notes below.
}
```

Notes:

- `WS_MAXIMIZE` is set before `WM_NCCALCSIZE(TRUE)` arrives during the maximize transition; Microsoft does not document this ordering, but detection via [`IsZoomed`](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-iszoomed) works in practice and matches Windows Terminal's approach.
- DPI must come from [`GetDpiForWindow`](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getdpiforwindow) and metrics from `GetSystemMetricsForDpi`. The non-DPI-aware variants return wrong values on per-monitor v2 windows ([Mixed-Mode DPI Scaling](https://learn.microsoft.com/windows/win32/hidpi/high-dpi-improvements-for-desktop-applications#new-dpi-related-apis)).
- The top resize-band used by the strip's hit-test (§5.6) is a separate computation: `SM_CXPADDEDBORDER + SM_CYSIZEFRAME`. There is no `SM_CYPADDEDBORDER`; the same `SM_CXPADDEDBORDER` value is added to both axes — per Windows Terminal's `_GetResizeHandleHeight` at commit `e4e3f08efca9d0ffba330eee12edbcb16897ddcb`: *"there isn't a SM_CYPADDEDBORDER for the Y axis."*
- To preserve the cursor's ability to trigger an auto-hide taskbar, the toolkit probes for an auto-hide taskbar on each monitor edge via [`SHAppBarMessage(ABM_GETSTATE)`](https://learn.microsoft.com/windows/win32/api/shellapi/nf-shellapi-shappbarmessage) + [`SHAppBarMessage(ABM_GETAUTOHIDEBAREX)`](https://learn.microsoft.com/windows/win32/api/shellapi/nf-shellapi-shappbarmessage) and claws back 2 px on the matching edge, matching Windows Terminal's GH#1438 / GH#5209 handling (`AutohideTaskbarSize = 2`).

## 4. Components

### 4.1 `CompositionContext`

Defined in `composition.rs`. Holds `IDWriteFactory` and `CompositionGraphicsDevice`; both survive device loss. The CGD's underlying rendering device is swapped on loss via `SetRenderingDevice`.

Public surface (all `pub(crate)`):

```rust
pub fn new(compositor: Compositor) -> anyhow::Result<Self>;
pub fn create_drawing_surface(&self, size: SizeInt32) -> anyhow::Result<CompositionDrawingSurface>;
pub fn with_d2d_render_target<R>(
    &self,
    surface: &CompositionDrawingSurface,
    body: impl FnOnce(&ID2D1RenderTarget, POINT) -> anyhow::Result<R>,
) -> anyhow::Result<Option<R>>;
pub fn dwrite_factory(&self) -> IDWriteFactory;
pub fn add_rendering_device_replaced_callback<F>(&self, cb: F) -> anyhow::Result<RenderingDeviceReplacedRegistration>
where F: Fn() + Send + 'static;
```

`with_d2d_render_target` returns `Ok(None)` on device loss (`DXGI_ERROR_DEVICE_REMOVED`, `DXGI_ERROR_DEVICE_RESET`, `D2DERR_RECREATE_TARGET`); other errors propagate. Callers must preserve dirty state when `Ok(None)` is returned.

`ensure_composition_context(compositor: Compositor) -> anyhow::Result<Rc<CompositionContext>>` is the only construction entry point, backed by a thread-local `OnceCell`. The strip holds `Rc<CompositionContext>` for D2D / DirectWrite access. The strip does not hold a `CompositorController` and never calls `Commit()` — publishing is handled by `CompositorDriver` via `CommitNeeded`.

Device loss recovery — see §6.2. Proactive detection via `ID3D11Device4::RegisterDeviceRemovedEvent` is deferred; see `TODO.md` *Caption-button proactive device-loss detection*.

### 4.2 `CaptionButtonStrip`

Owns the per-window visual strip and its press-session state machine.

**Visibility / availability table** (from `WindowStyle` flags):

| `WindowStyle` flags | Minimize button | Maximize button |
|---|---|---|
| `!is_minimizable && !is_maximizable` | hidden | hidden |
| `is_minimizable && !is_maximizable` | visible, enabled | visible, disabled |
| `!is_minimizable && is_maximizable` | visible, disabled | visible, enabled |
| `is_minimizable && is_maximizable` | visible, enabled | visible, enabled |

`Close` is always visible and enabled. Close-disable support is deferred; see [TODO.md](../TODO.md#win32-close-button-disable-support). Minimize / Maximize follow Win32's paired-button behaviour (Microsoft KB Q130760): when exactly one of `is_minimizable` / `is_maximizable` is true, both buttons appear and the absent one renders Disabled.

Disabled Minimize / Maximize buttons are visible and occupy strip geometry, but never enter Hovered / Pressed and return no action. `on_pointer_down` records a `Suppressed { held_button: Left }` session for them; `on_pointer_up` returns `None`. Disabled visible Minimize / Maximize map to `HTCAPTION` in `on_nchittest`, not `HTMINBUTTON` / `HTMAXBUTTON` — matching Win32's native default and preventing Snap Layouts from appearing on a disabled Maximize.

**Press sessions.** Two modes populate `press_sessions: Vec<PressSession>`:

- `PressSessionMode::Active` — primary press on an enabled button. Visual capture engaged; matched primary UP dispatches a `CaptionButtonAction` if released over the same button.
- `PressSessionMode::Suppressed { held_button }` — wndproc-level swallow. No visual capture. `held_button = Left` for primary-on-disabled; `Right` / `Middle` / `XButton1` / `XButton2` for non-primary presses.

At most one `Active` (or `Suppressed{Left}`) entry exists at a time; any number of non-primary `Suppressed` entries may coexist.

**Suppression truth table:**

| Button | Press location | Release location | Activation? | Kotlin passthrough? |
|---|---|---|---|---|
| Primary | Enabled strip button | Same enabled button | yes | no |
| Primary | Enabled strip button | Elsewhere (drag-off) | no | no |
| Primary | Elsewhere | Any enabled strip button | no | yes (UP only) |
| Primary | Disabled strip button | Anywhere | no | no |
| Non-primary | Any strip button | Anywhere | no | no |
| Non-primary | Elsewhere | Any strip button | no | yes (DOWN and UP both) |

`WM_POINTERCAPTURECHANGED` requires no special handling: `strip.on_pointer_cancel(pointer_id)` drops any session for the cancelled pointer regardless of mode.

Right-click on a caption button is an unhandled gesture — that surface is owned by the system-menu work tracked in `TODO.md`.

Public surface (all `pub(crate)`):

```rust
pub fn new(
    chrome_layer: &ContainerVisual,
    initial_scale: f32,
    style: &WindowStyle,
    compositor: &Compositor,
    hwnd: HWND,
    initial_is_active: bool,
    initial_is_maximized: bool,
    initial_top_offset_px: i32,
) -> anyhow::Result<Self>;

// Hit-testing
pub fn hit_test(&self, client_point: PhysicalPoint, client_width: PhysicalPixels) -> Option<CaptionButtonKind>;
pub fn is_enabled(&self, kind: CaptionButtonKind) -> bool;

// Pointer routing
pub fn on_pointer_update(&mut self, kind: Option<CaptionButtonKind>, device: PointerDeviceKind);
pub fn on_pointer_down(&mut self, kind: CaptionButtonKind, pointer_id: u32, device: PointerDeviceKind);
pub fn on_pointer_up(&mut self, kind_under_pointer: Option<CaptionButtonKind>, pointer_id: u32) -> Option<CaptionButtonAction>;
pub fn on_pointer_cancel(&mut self, pointer_id: u32);
pub fn cancel_any_press(&mut self);

// Press-session helpers
pub fn track_swallowed_press(&mut self, kind: CaptionButtonKind, pointer_id: u32, button: PointerButton);
pub fn consume_swallowed_release(&mut self, pointer_id: u32, button: PointerButton) -> bool;
pub fn has_active_press_for(&self, pointer_id: u32) -> bool;
pub fn has_press_for(&self, pointer_id: u32) -> bool;

pub fn on_nc_mouse_leave(&mut self);

// Theming / state changes
pub fn on_activate(&mut self, is_active: bool);
pub fn on_dpi_change(&mut self, new_scale: f32, max_chrome_y: i32) -> anyhow::Result<()>;
pub fn on_appearance_change(&mut self, appearance: Appearance, high_contrast: HighContrast);
pub fn on_rendering_device_replaced(&mut self);
pub fn on_max_state_change(&mut self, is_maximized: bool);
pub fn on_resize(&mut self, max_chrome_y: i32);
```

`CaptionButtonStrip::new` seeds `appearance` and `high_contrast` from `Appearance::get_current()` and `HighContrast::get_current()`. Both queries are non-fatal: a failed query logs at warning level and falls back to `Appearance::Light` / `HighContrast::Off`, and the strip self-corrects on the next appearance event.

The strip never invokes `Window::request_close` and friends directly — it returns `CaptionButtonAction` and the wndproc dispatches. This keeps the strip independent of `Window`'s broader state and unit-testable in isolation.

### 4.3 `CaptionButton` (private to `caption_buttons.rs`)

Each button holds a `backplate: SpriteVisual` with a `CompositionColorBrush` and a `glyph: SpriteVisual` whose brush is a `CompositionMaskBrush` combining a `CompositionSurfaceBrush` (wrapping the pre-rasterised `glyph_surface`) as `Mask` with a `CompositionColorBrush` as `Source`. State transitions only mutate the `Source` colour; glyph re-rasterisation happens only on DPI / theme / high-contrast / max-state changes.

The intermediate `CompositionSurfaceBrush` / `CompositionMaskBrush` aren't stored — `SpriteVisual.SetBrush(&mask_brush)` keeps the chain alive. Surface format is BGRA8 premultiplied.

`ButtonInteraction` states: `Idle`, `Hovered` (pointer over, no press; mouse / pen only), `Pressed` (capture mine, pointer over), `PressedDraggedOff` (capture mine, pointer left).

### 4.4 `CaptionTheme` (private)

Theme colours per `(Appearance, HighContrast)`:

- **`HighContrast::On`**: reads from `GetSysColor`. Slots: `COLOR_BTNFACE` / `COLOR_BTNTEXT` for rest, `COLOR_HIGHLIGHT` / `COLOR_HIGHLIGHTTEXT` for hover and pressed, `COLOR_GRAYTEXT` for disabled. Sourced from Terminal's HighContrast block and the [*High contrast parameter*](https://learn.microsoft.com/windows/win32/winauto/high-contrast-parameter) doc.
- **`HighContrast::Off`**: hard-coded table from WinUI's [`microsoft-ui-xaml` Fluent palette](https://github.com/microsoft/microsoft-ui-xaml/blob/main/src/controls/dev/CommonStyles/Common_themeresources_any.xaml) (`TextFillColor*` / `SubtleFillColor*` series).
- **Inactive modulation** applies to `Idle` / `PressedDraggedOff` only. Hover and Pressed render with the active palette regardless of window activation, matching Terminal's `MinMaxCloseControl` and WinUI 3's `TitleBar` control.
- **Close-specific override**: hover backplate `#C42B1C` opaque / foreground `White` opaque; pressed backplate `#C42B1C` α=0xE6 / foreground `White` α=0xB3. Per [*Title bar customization*](https://learn.microsoft.com/windows/apps/develop/title-bar): *"The close button always uses the system-defined color for those states."*

### 4.5 `CaptionButtonMetrics` (private)

Button dimensions sourced from `microsoft/terminal:src/cascadia/TerminalApp/MinMaxCloseControl.xaml`:

| Dimension | Value | Source |
|---|---|---|
| Width | 46.0 epx | `Width="46.0"` |
| Height | 32.0 epx | WinUI `PreferredHeightOption.Standard` |
| Glyph rendered extent | 10 × 10 epx | Terminal's `Viewbox Width="10" Height="10"` |
| Glyph font family | `Segoe Fluent Icons` with `Segoe MDL2 Assets` fallback | Detected at runtime via DirectWrite system font collection |

The 10 × 10 epx is a **rendered extent**, not a font point size. Font size is computed from design-unit metrics:

1. Resolve glyph indices via [`IDWriteFontFace::GetGlyphIndices`](https://learn.microsoft.com/windows/win32/api/dwrite/nf-dwrite-idwritefontface-getglyphindices). Standard set: U+E921 (`ChromeMinimize`), U+E922 (`ChromeMaximize`), U+E923 (`ChromeRestore`), U+E8BB (`ChromeClose`); high-contrast variants: U+EF2D / EF2E / EF2F / EF2C — all listed on the [Segoe Fluent Icons](https://learn.microsoft.com/windows/apps/design/iconography/segoe-fluent-icons-font) reference.
2. Read the bbox via [`GetDesignGlyphMetrics`](https://learn.microsoft.com/windows/win32/api/dwrite/nf-dwrite-idwritefontface-getdesignglyphmetrics). For height, use the horizontal-layout cell `(ascent + descent)` from [`GetMetrics`](https://learn.microsoft.com/windows/win32/api/dwrite/nf-dwrite-idwritefontface-getmetrics) minus bearings.
3. Per-axis fit: `font_size = target * designUnitsPerEm / max(bbox.width, bbox.height)`.

`font_size_dip` is recomputed whenever the font changes (Fluent ↔ MDL2, HC variants).

### 4.6 Boundaries

- `CompositionContext` knows nothing about caption buttons in its method signatures; if a future D2D consumer materialises, lift it to a shared module.
- `CaptionButtonStrip` knows nothing about Win32 messages or wndproc. Inputs are typed; outputs are typed (`Option<CaptionButtonAction>`).
- `CaptionButton`, `CaptionTheme`, `CaptionButtonMetrics`, `Availability`, `ButtonInteraction`, `PressSession` are private to `caption_buttons.rs`.
- `Window` only knows how to insert a `ContainerVisual` into its `chrome_layer` and call the strip's lifecycle methods. The strip's right-edge auto-tracks via `composition_root.RelativeOffsetAdjustment(1,0,0) + AnchorPoint(1,0)`; only `Offset.Y` mutates per resize tick.

## 5. Data flow

### 5.1 Visual state derivation

Per-button interaction state is derived from strip-level state each frame. The `Some(Active) => Idle` branch for a non-captured button is the WinUI capture-derived rule: when a press is owned by button A, button B does not enter Hovered state. Touch skips Hovered entirely (`Idle → Pressed → action → Idle`).

`resolve_interaction` truth table — inputs: `(Availability, kind, pointer_over_kind, pointer_device, press_session)`:

| press_session | is_pointer_over_self | result |
|---|---|---|
| None | true, Mouse/Pen | `Hovered` |
| None | true, Touch | `Idle` |
| None | false | `Idle` |
| `Active`, captured_kind == kind | true | `Pressed` |
| `Active`, captured_kind == kind | false | `PressedDraggedOff` |
| `Active`, captured_kind != kind | — | `Idle` |
| `Suppressed` (any) | true, Mouse/Pen | `Hovered` |
| `Suppressed` (any) | false | `Idle` |
| Disabled (any state) | — | `Idle` |

### 5.2 Animation contract

Sourced from `microsoft/terminal:MinMaxCloseControl.xaml` `CommonStates` `VisualStateGroup`. Only `PointerOver → Normal` and `PointerOver → Unfocused` transitions animate; all others jump via `Setter`.

| Transition | Animated? | Duration |
|---|---|---|
| `Idle → Hovered` | jump | — |
| `Hovered → Idle` | **animate** | backplate 150 ms, glyph 100 ms, no easing |
| `Hovered → Pressed` | jump | — |
| `Pressed → Hovered` (action fires) | jump | — |
| `Pressed → PressedDraggedOff` | jump | — |
| `PressedDraggedOff → Pressed` | jump | — |
| `PressedDraggedOff → Idle` (no action) | jump | — |
| any → `Disabled` | jump | — |
| `is_active` flip | jump | — |
| `is_window_maximized` flip | jump | — |

`ColorKeyFrameAnimation` via `CompositionColorBrush.StartAnimation("Color", animation)`. A defensive `InsertKeyFrame(0.0, brush.Color()?)` precedes `InsertKeyFrame(1.0, target)` so the animation always interpolates from the last committed value.

### 5.3 Inputs and what they trigger

| Input | Strip method | Cost |
|---|---|---|
| `WM_NCHITTEST` | `hit_test(point)` | cheap (geometry math) |
| `WM_NCPOINTERUPDATE` entering NC area | window arms NC leave tracking; strip receives `on_pointer_update(kind, device)` | cheap (state + brush) |
| `WM_NCPOINTERDOWN` over enabled button (primary) | `on_pointer_down(kind, id, device)` | cheap |
| `WM_NCPOINTERUP` / `WM_POINTERUP` after strip press | `on_pointer_up(kind, id) → Option<CaptionButtonAction>` | cheap |
| `WM_POINTERCAPTURECHANGED` | `on_pointer_cancel(id)` for any matching session | cheap |
| `WM_CANCELMODE` | `cancel_any_press()`; returns `None` so `DefWindowProc` performs standard cancel | cheap |
| `WM_NCMOUSELEAVE` | `on_nc_mouse_leave()`; window clears tracking armed | cheap |
| `WM_NCCALCSIZE` (after client-rect calc) | `set_content_top_offset(max_chrome_y)`; `Custom` calls `on_resize(max_chrome_y)`. `CommitNeeded` fast-path publishes inline. | cheap |
| `WM_DPICHANGED` | `set_dpi_metrics(new_dpi)` first; `Custom`: `set_content_top_offset` → `on_dpi_change(scale, max_chrome_y)` | **expensive** (re-rasterise all glyph surfaces) |
| `Appearance` event | `on_appearance_change(appearance, hc)` | **expensive iff foreground-rest colour changed** |
| `HighContrast` event | same as above with new `hc` | **expensive** (glyph code points swap E921↔EF2D etc.) |
| `WM_ACTIVATE` | `on_activate(is_active)`; deactivate also calls `cancel_any_press()` | cheap (theme re-resolve, brush updates) |
| `WM_WINDOWPOSCHANGED` | `Custom`: `on_max_state_change(is_maximized)` | **expensive for Maximize button only** on actual transitions (E922 ↔ E923) |

"Expensive" rows go through `with_d2d_render_target` and a full glyph redraw. All others are O(1) brush colour updates.

### 5.4 Hit-test → action mapping

| Action | Wndproc dispatch |
|---|---|
| `Close` | `Window::request_close()` (`PostMessage WM_CLOSE`) |
| `Minimize` | `Window::minimize()` (`SendMessageW(WM_SYSCOMMAND, SC_MINIMIZE, ...)`) |
| `Maximize` | `Window::maximize()` (`SendMessageW(WM_SYSCOMMAND, SC_MAXIMIZE, ...)`) |
| `Restore` | `Window::restore()` (`SendMessageW(WM_SYSCOMMAND, SC_RESTORE, ...)`) |

The Maximize button returns `Restore` if `is_window_maximized`, `Maximize` otherwise. The strip reads its own `is_window_maximized` field — no `IsZoomed` call at click time.

### 5.5 Frame commit coupling

Publishing uses `CompositorDriver` (see `2026-05-22-win32-compositor-driver-design.md`):

- **UI-thread fast-path**: `CommitNeeded` fires on the UI thread → `Commit()` inline in the same wndproc invocation.
- **Dispatcher drain**: `CommitNeeded` fires off-thread → marshals to UI thread via `DispatcherQueue::TryEnqueueWithPriority(High, …)`.

The strip never calls `Commit()` directly. `renderer_angle::swap_buffers` is the sole explicit `Commit()` trigger outside the driver: `compositor_driver.publish_and_resume_autocommit()` commits only when the resize pause-gate was active, ensuring `visual.SetSize` and the matching `eglSwapBuffers` Present land on the same DWM tick.

### 5.6 `on_nchittest` dispatch order

```
1. Run DwmDefWindowProc(WM_NCHITTEST, ...) / DefWindowProcW fallback; record
   result as `original_ht`. Do not return yet for Custom-titlebar windows.
2. Dispatch into the strip via caption_kind_at_screen(window, screen).
   - On a restored, resizable window: short-circuit to None over the top
     SM_CXPADDEDBORDER + SM_CYSIZEFRAME resize-border band so step 3 returns
     HTTOP (OS resize cursor and drag-resize across the full strip width).
   - Maximized or non-resizable windows skip this short-circuit.
   - Return HTCLOSE for Close, HTMAXBUTTON / HTMINBUTTON for enabled
     Maximize / Minimize, HTCAPTION for visible disabled Maximize / Minimize.
3. If strip did not claim the point and original_ht != HTCLIENT, return original_ht.
4. Fire the Kotlin NCHitTestEvent callback so the app can carve interactive
   sub-regions. If handled, return HTCLIENT.
5. Run the manual top-edge math (resize_handle_height / title_bar_height)
   to convert near-top points into HTTOP (resize) or HTCAPTION (drag).
6. Otherwise return HTCLIENT.
```

## 6. Error handling

Three tiers. Construction of a `Custom` titlebar window must fail closed if caption buttons cannot be created before show; runtime degradation after a successfully-created strip is best-effort.

### 6.1 Construction-time

- **`ensure_composition_context` failure** (D3D11 / D2D / DWrite / `CompositionGraphicsDevice` creation): propagates out of `CaptionButtonStrip::new` → `initialize_window` → `WM_NCCREATE` / `window_create`. The thread-local singleton cell stays empty; subsequent `Custom`-titlebar windows retry.
- **D3D11 driver-type fallback.** `build_d2d_device` attempts `D3D_DRIVER_TYPE_HARDWARE` first; on failure retries with `D3D_DRIVER_TYPE_WARP`. Headless / Remote Desktop / degraded-GPU sessions complete window creation against WARP. Once WARP is chosen, the `CompositionContext` stays on WARP for the lifetime of the UI thread; no automatic upgrade-back to HARDWARE.
- **`CaptionButtonStrip::new` failure**: propagates out of `initialize_window`; the custom window is not shown without caption buttons. Do not silently leave `Window.caption_buttons` as `None` after a successful `window_create`.

### 6.2 Runtime: device loss

Device loss is detected reactively inside `with_d2d_render_target` on both [`ID2D1RenderTarget::BeginDraw`](https://learn.microsoft.com/en-us/windows/win32/api/d2d1/nf-d2d1-id2d1rendertarget-begindraw) and [`ID2D1RenderTarget::EndDraw`](https://learn.microsoft.com/en-us/windows/win32/api/d2d1/nf-d2d1-id2d1rendertarget-enddraw). Three HRESULTs are trapped: `DXGI_ERROR_DEVICE_REMOVED`, `DXGI_ERROR_DEVICE_RESET`, `D2DERR_RECREATE_TARGET`.

Recovery sequence: device-loss HRESULT → `with_d2d_render_target` calls `CompositionContext::rebuild_d2d_device` synchronously on the UI thread (builds fresh `ID2D1Device`, inherits HARDWARE→WARP fallback, calls `ICompositionGraphicsDeviceInterop::SetRenderingDevice` on the existing CGD) → returns `Ok(None)` so the caller leaves `glyph_surface_dirty = true` → next `RenderingDeviceReplaced` notification fires the strip's subscription → re-rasterises. `CompositionDrawingSurface` instances stay usable across `SetRenderingDevice`; only their contents need re-rasterising.

The `RenderingDeviceReplaced` callback posts a private `WM_APP_*` message rather than calling `CaptionButtonStrip::on_rendering_device_replaced()` directly (the callback fires synchronously inside `SetRenderingDevice` / `with_d2d_render_target`; a direct call would nest `BeginDraw`).

The strip subscribes during construction and owns the `RenderingDeviceReplacedRegistration` guard. The subscription is removed when `WM_NCDESTROY` clears `Window.caption_buttons`. The UI-thread handler re-rasterises eagerly so visual recovery is not dependent on later pointer movement.

If the new D2D device also fails (cascading device loss), the strip logs and skips that frame; dirty flags are not cleared. Retry happens on the next `BeginDraw` attempt or `RenderingDeviceReplaced` notification.

`RenderingDeviceReplaced` fires on the thread that called `SetRenderingDevice` (agile marshalling, not framework-chosen thread). The toolkit always calls `SetRenderingDevice` from the UI thread. See `TODO.md` *Verify `RenderingDeviceReplaced` fires synchronously*.

### 6.3 Runtime: per-call failures

- **DirectWrite failure** (`CreateTextFormat` / glyph layout / drawing): logged. Glyph surface stays at prior contents. Button remains hit-testable and clickable.
- **`StartAnimation` failure**: fall back to instant `Color` set on the `CompositionColorBrush`. Logged at warning level.
- **`TrackMouseEvent` failure**: logged at warning level. Hover-fade-out won't trigger on pointer leave; corrects on next hover-in.
- **`SetOffset` / `SetSize` / `SetBrush` failures**: logged; continue with stale visual; next state mutation retries.

## 7. Testing

### 7.1 Pure-Rust unit tests (no display, no GPU)

- **`resolve_interaction`** truth-table over `(Availability, kind, pointer_over_kind, pointer_device, press_session)`. ~30-40 cases.
- **`on_pointer_cancel`**: clears only the matching press session and never returns a caption-button action.
- **`CaptionButton::transition_to(...)`**: which transitions animate vs jump.
- **`hit_test` / disabled-hit policy**: geometry math at varied DPI scales, across each Minimize / Maximize visibility and availability table row. Disabled visible buttons remain in geometry but cannot produce Hovered / Pressed / action; disabled visible Minimize / Maximize map to `HTCAPTION`, not `HTMINBUTTON` / `HTMAXBUTTON`.
- **`CaptionButtonMetrics::new`**: 46 × 32 epx round-trip through DPI scales 1.0 / 1.25 / 1.5 / 1.75 / 2.0 / 2.5.
- **`CaptionTheme::resolve`**: per `(Appearance, HighContrast, IsActive)` combo, expected colour-table cells.
- **Layout for the Minimize / Maximize visibility and availability table** from `WindowStyle` flags.

Tests are in `caption_buttons.rs` under `#[cfg(test)] mod tests { ... }`.

### 7.2 Tests that require live composition (skipped under `cargo test`)

- Anything exercising real `D2dContext` (D3D11), real `CompositionGraphicsDevice`, real `BeginDraw` / `EndDraw`. Need a desktop session and GPU on a UI thread.
- Snapshot / golden-image tests for rasterised glyphs. Out of scope; recorded as follow-on if visual regressions warrant.

### 7.3 Manual test plan exercised via the sample app

`:sample:runSkikoSampleWin32` runs with `WindowTitleBarKind.Custom`. Acceptance checklist:

- Rest / Hover / Pressed for enabled buttons, plus Disabled for Minimize / Maximize table rows, in light and dark themes.
- Disabled Minimize / Maximize never fire actions, never show hover / press visuals, swallow their DOWN / UP cycle, and disabled Maximize does not show the Win11 Snap Layout flyout.
- Window inactive vs active — colour modulation visible on `Idle` and `PressedDraggedOff` only. Hover and pressed on caption buttons of an inactive window render with the active palette.
- **High contrast** — glyph code points swap to U+EF2C / EF2D / EF2E / EF2F when HC turns On; cycling through all four shipped HC themes produces visibly different palettes (colours read from `GetSysColor`, none hard-coded); side-by-side comparison against a native Win32 chrome window — caption-button colours match.
- Maximize → Restore via the button — glyph swap U+E922 ↔ U+E923.
- **Maximized window edge inset** — on single-monitor, taskbar at bottom, taskbar at top, secondary monitor at different DPI: every caption button is fully visible, strip right edge sits flush against the inset client area, Close button top edge sits flush with the visible monitor edge. Snap Layouts pop at the visually-correct location.
- **Maximized Custom / None content placement** — render Skiko content at client y=0, maximize, confirm the rect's top sits flush with the visible monitor edge for each titlebar mode.
- Snap-layout flyout appears on Win11 hover over the maximize button.
- Mouse, touch, and pen input — touch correctly skips the hover state.
- DPI change by dragging across monitors — glyph re-rasterised crisply; strip re-anchors flush with new client width.
- Drag the title bar (drag region between strip and left edge) to move the window.
- Press a button, drag off, release outside — no action fires; button returns to Rest.
- Press button A, drag over button B — B does not react (WinUI capture rule).
- **Right-click / middle-click on a caption button** — no action fires; button stays in Rest.
- **Press cycle survives NC→client drift** — press, drag into client area, release. No stuck press visual; host drag source does not start `DoDragDrop`.
- **Focus-loss cancels the press** — press and hold; ALT-TAB away. Return focus: button is in Rest, no stuck press visual.
- **System-titlebar window** — system Min / Max / Close still respond.
- Caption glyphs use Segoe Fluent Icons when present, with Segoe MDL2 Assets fallback.
- **Hover-fade animation under controlled commit** — hover a caption button then move away on a window with no active ANGLE rendering; the 150 ms backplate / 100 ms glyph fade plays smoothly at 60 Hz.
- **Auto-hide taskbar reveal under maximized window** — taskbar still reveals when the cursor reaches the screen edge.

Observations on system-menu restoration, keyboard system-command parity, and UIA/accessibility from the sample run go to `TODO.md`.

## 8. Open questions

1. **Animation cadence under `CompositorController`.** Microsoft documents Visual-Layer animations run on the system compositor's own thread independent of the UI thread, but does not directly address whether `ColorKeyFrameAnimation` requires per-frame `Commit()` under the controlled-commit variant. The simplest reading: commit once at `StartAnimation` and let the compositor's thread advance the animation. §7.3 acceptance verifies hover-fade smoothness; if stutter is observed, see TODO `Caption-button animation cadence — CommitNeeded fallback`.

Future work tracked in `TODO.md`: RTL mirroring; `WM_NCMOUSEMOVE` fallback; system-menu restoration; Tall-mode title bars; Close-button disable; proactive device-loss detection; `RenderingDeviceReplaced` thread-affinity probe.

## 9. References

### Microsoft documentation

- [Custom Window Frame Using DWM](https://learn.microsoft.com/windows/win32/dwm/customframe) — canonical Win32 custom-frame recipe.
- [WM_NCCALCSIZE message](https://learn.microsoft.com/windows/win32/winmsg/wm-nccalcsize)
- [WM_NCHITTEST message](https://learn.microsoft.com/windows/win32/inputdev/wm-nchittest) — return-value table for `HTCLOSE` (20), `HTMAXBUTTON` (9), `HTMINBUTTON` (8).
- [Support snap layouts for desktop apps on Windows 11](https://learn.microsoft.com/windows/apps/desktop/modernize/ui/apply-snap-layout-menu) — `HTMAXBUTTON` is the documented contract for Win11 snap-layout flyout.
- [DwmDefWindowProc function](https://learn.microsoft.com/windows/win32/api/dwmapi/nf-dwmapi-dwmdefwindowproc)
- [Composition native interoperation with DirectX and Direct2D](https://learn.microsoft.com/en-us/windows/uwp/composition/composition-native-interop) — canonical interop pattern.
- [ID3D11Device4::RegisterDeviceRemovedEvent](https://learn.microsoft.com/en-us/windows/win32/api/d3d11_4/nf-d3d11_4-id3d11device4-registerdeviceremovedevent)
- [ICompositionGraphicsDeviceInterop::SetRenderingDevice](https://learn.microsoft.com/en-us/windows/win32/api/windows.ui.composition.interop/nf-windows-ui-composition-interop-icompositiongraphicsdeviceinterop-setrenderingdevice)
- [CompositionGraphicsDevice.RenderingDeviceReplaced](https://learn.microsoft.com/en-us/uwp/api/windows.ui.composition.compositiongraphicsdevice.renderingdevicereplaced?view=winrt-26100)
- [ScreenToClient](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-screentoclient)
- [VisualCollection.InsertAtTop](https://learn.microsoft.com/en-us/uwp/api/windows.ui.composition.visualcollection.insertattop?view=winrt-26100)
- [CompositionMaskBrush Class](https://learn.microsoft.com/uwp/api/windows.ui.composition.compositionmaskbrush)
- [Segoe Fluent Icons font](https://learn.microsoft.com/windows/apps/design/iconography/segoe-fluent-icons-font) — glyph code points (E921, E922, E923, E8BB) and high-contrast variants (EF2D, EF2E, EF2F, EF2C).
- [Segoe MDL2 Assets icons](https://learn.microsoft.com/en-us/windows/apps/design/iconography/segoe-ui-symbol-font)
- [Title bar customization](https://learn.microsoft.com/windows/apps/develop/title-bar)
- [High contrast parameter](https://learn.microsoft.com/windows/win32/winauto/high-contrast-parameter)
- [Compatibility / High-contrast mode](https://learn.microsoft.com/windows/compatibility/high-contrast-mode)
- [Button Messages — Button Color Messages](https://learn.microsoft.com/windows/win32/controls/button-messages#button-color-messages)
- [Window Styles](https://learn.microsoft.com/windows/win32/winmsg/window-styles)
- [Microsoft KB Archive Q130760](https://www.betaarchive.com/wiki/index.php/Microsoft_KB_Archive/130760) — paired minimize / maximize box behaviour.
- [TITLEBARINFOEX structure](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-titlebarinfoex) and [WM_GETTITLEBARINFOEX](https://learn.microsoft.com/en-us/windows/win32/menurc/wm-gettitlebarinfoex)
- [ColorKeyFrameAnimation Class](https://learn.microsoft.com/uwp/api/windows.ui.composition.colorkeyframeanimation?view=winrt-28000) — available since Windows 10 build 10586.
- [UIElement.CapturePointer (WinUI)](https://learn.microsoft.com/windows/windows-app-sdk/api/winrt/microsoft.ui.xaml.uielement.capturepointer?view=windows-app-sdk-1.8) — *"the second element doesn't fire `PointerEntered` events for a captured pointer when the captured pointer enters it."*
- [TrackMouseEvent](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-trackmouseevent) and [WM_NCMOUSELEAVE message](https://learn.microsoft.com/windows/win32/inputdev/wm-ncmouseleave)
- [WM_NCPOINTERDOWN](https://learn.microsoft.com/windows/win32/inputmsg/wm-ncpointerdown), [WM_NCPOINTERUPDATE](https://learn.microsoft.com/windows/win32/inputmsg/wm-ncpointerupdate), [WM_NCPOINTERUP](https://learn.microsoft.com/windows/win32/inputmsg/wm-ncpointerup) — documented `HIWORD(wParam)` is the hit-test value; implementation-observed on Windows 11 to arrive muxed with `POINTER_FLAG_*` bits and never matching `HTCLOSE` / `HTMAXBUTTON` / `HTMINBUTTON`, hence geometric hit-test via `caption_kind_at_screen`. `WM_NCPOINTERDOWN` documents implicit non-client capture.
- [WM_POINTERCAPTURECHANGED](https://learn.microsoft.com/windows/win32/inputmsg/wm-pointercapturechanged), [WM_POINTERUP](https://learn.microsoft.com/windows/win32/inputmsg/wm-pointerup)
- [WM_CANCELMODE](https://learn.microsoft.com/windows/win32/winmsg/wm-cancelmode)
- [IsZoomed](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-iszoomed)
- [GetDpiForWindow](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getdpiforwindow) and [`GetSystemMetricsForDpi`](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getsystemmetricsfordpi)
- [SM_CXPADDEDBORDER](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getsystemmetrics)
- [docs.rs `windows = 0.62.2` feature flags](https://docs.rs/crate/windows/0.62.2/features)
- [microsoft-ui-xaml/specs/TitleBar/titlebar-functional-spec.md](https://github.com/microsoft/microsoft-ui-xaml/blob/main/specs/TitleBar/titlebar-functional-spec.md)

### Source-code references

- [microsoft/terminal MinMaxCloseControl.xaml (pinned commit `e4e3f08efca9d0ffba330eee12edbcb16897ddcb`)](https://github.com/microsoft/terminal/blob/e4e3f08efca9d0ffba330eee12edbcb16897ddcb/src/cascadia/TerminalApp/MinMaxCloseControl.xaml)
- [Min/Max/Close buttons should be 32px · Issue #9093](https://github.com/microsoft/terminal/issues/9093)
- [Chromium `CustomFrameView::NonClientHitTest` (pinned tag `125.0.6422.160`)](https://chromium.googlesource.com/chromium/src/+/refs/tags/125.0.6422.160/ui/views/window/custom_frame_view.cc)

### Crate-internal references

- `native/desktop-win32/docs/AGENTS.md`
- `native/desktop-win32/docs/ARCHITECTURE.md` § Composition
- `native/desktop-win32/docs/SUBSYSTEMS.md`
- `native/desktop-win32/docs/TODO.md`
