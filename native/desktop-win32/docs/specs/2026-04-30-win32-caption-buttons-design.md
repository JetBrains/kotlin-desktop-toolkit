# Win32 caption buttons — design

**Plan:** 3 of 5 (caption buttons).
**Crate:** `native/desktop-win32`.
**Status:** spec, awaiting user review.

## 1. Goal

Add minimise / maximise-restore / close caption buttons to windows that use `WindowTitleBarKind::Custom` on Windows. The buttons are toolkit-managed `Windows.UI.Composition` visuals, with full state coverage (rest, hover, pressed, disabled, plus active / inactive modulation), full appearance coverage (light, dark, high contrast on / off), and Win32-correct hit-testing including Win11 snap-layout flyout integration.

The toolkit owns the buttons end-to-end. Click side-effects translate to the toolkit's existing `Window::request_close` / `Window::minimize` / `Window::maximize` and a new `Window::restore`. Apps don't get a per-button-press event — caption buttons behave like the system buttons they replace.

## 2. Plan structure

This work is part of a five-plan effort:

| # | Plan | Brief | Status |
|---|---|---|---|
| 1 | Doc cleanup + custom-titlebar gating | Remove `WS_CAPTION` for `Custom`; simplify `on_nccalcsize` and `on_nchittest`; fix SUBSYSTEMS.md / TODO.md | small spec, no detour |
| 2 | High-contrast modelling | Introduce `HighContrast` enum; handle `SPI_SETHIGHCONTRAST` in `on_settingchange`; add appearance-event coverage | small spec, one inline decision |
| 3 | Caption buttons | This document. | spec written |
| 4 | System-menu restoration | Restore Alt+Space / right-click-on-title-bar system menu (lost when `WS_CAPTION` is removed in Plan 1) | brainstormed later |
| 5 | Tall-mode title bar + maximised metric transitions | Introduce the `WindowTitleBarHeight` enum (`Standard` / `Tall`); plumb it through `WindowStyle` and FFI; implement Tall rendering (40 epx windowed); add the maximised height shrink (40 → 32) and the suspected ~8 epx strip y-offset for Tall maximised. Plan 3's hard-coded 32 epx height is replaced with a `resolve_button_height(WindowTitleBarHeight, is_maximized)` lookup. | brainstormed later |

**Plan 3 depends on Plan 1 and Plan 2 having merged.** Plan 3 assumes:

- Custom-titlebar windows have `WS_CAPTION` removed at creation. The system therefore has no caption buttons to draw, by the documented style-flag dependency chain: per *Window Styles*, `WS_SYSMENU` "must also be specified [with] `WS_CAPTION`," and `WS_MAXIMIZEBOX` / `WS_MINIMIZEBOX` each "must also be specified [with] `WS_SYSMENU`." With `WS_CAPTION` absent, none of `WS_SYSMENU` / `WS_MAXIMIZEBOX` / `WS_MINIMIZEBOX` take effect, so DWM has no system caption buttons to render in the title-bar area. (Inferred consequence; the chain is the documented part. There is no MS doc explicitly stating "no `WS_CAPTION` ⇒ no DWM caption buttons" — but the dependency tree implies it.)
- `on_nccalcsize` no longer special-cases the top inset for Custom mode; `AdjustWindowRectEx` returns the right value because the style flag is already gone.
- `on_nchittest` falls through `DwmDefWindowProc` for resize borders; the strip's hit-test is the second consultation, before the Kotlin `NCHitTestEvent` callback, before the drag-region (`HTCAPTION`) fallback.
- `Appearance` is queried alongside a `HighContrast` enum (`Off` / `On`) which the strip consumes.

Plan 1 and 2 must merge before Plan 3 starts implementation.

## 3. Architecture

### 3.1 New modules

Two crate-internal modules, both `pub(crate)` only — no FFI surface, no `_api.rs` partner.

- **`composition.rs`** (new) — defines `CompositionDevice`, the toolkit's single Composition gateway. Owns the `Compositor` and `CompositorController` (currently held directly by `Application` and cloned into `Window` and `AngleDevice`), plus the D3D11 / D2D / DirectWrite stack and the `CompositionGraphicsDevice` introduced by this plan. Hides `BeginDraw` / `EndDraw` and device-loss handling behind a single `with_d2d_context(surface, |context, offset| -> ...)` chokepoint.
- **`caption_buttons.rs`** — owns the per-window strip. Pure state-machine over typed inputs; no Win32 calls itself. The wndproc layer in `event_loop.rs` is the only place that touches both messages and the strip.

### 3.2 Hook points in existing code

Plan 3 introduces `CompositionDevice` and migrates the existing call sites that today reach for `Compositor` / `CompositorController` directly:

- **`Application`** today holds `compositor_controller: CompositorController` (`application.rs`). It is replaced with `composition_device: Rc<CompositionDevice>`, which encapsulates that controller plus its `Compositor` plus the new D3D11 / D2D / DirectWrite stack. Application accessors that returned the `Compositor` or the `CompositorController` are replaced with accessors that hand out the `CompositionDevice` reference.
- **`Window`** today holds `compositor_controller: CompositorController` (`window.rs:66`), used in `Window::add_visual` to call `compositor_controller.Compositor()?.CreateSpriteVisual()?` (`window.rs:151`). The field becomes `composition_device: Rc<CompositionDevice>`, and `add_visual` calls `self.composition_device.compositor()?.CreateSpriteVisual()?` instead. The new `chrome_layer` is also created via `self.composition_device.compositor()?.CreateContainerVisual()?`.
- **`renderer_angle.rs`**'s only direct use of `CompositorController` is `self.compositor_controller.Commit()` after `eglSwapBuffers` (`renderer_angle.rs:148`). The `SpriteVisual` it targets comes from `window.add_visual()` (`renderer_angle.rs:90`). `AngleDevice::create_for_window`'s second parameter changes from `CompositorController` to `Rc<CompositionDevice>`; the stored field becomes `composition_device: Rc<CompositionDevice>`; the commit call becomes `self.composition_device.compositor_commit()`.
- **`Application`**'s lazy-init policy for the D3D11/D2D/DWrite portion of `CompositionDevice`: those fields are not constructed at `Application::init` time. Instead, `CompositionDevice::ensure_d2d_initialised()` is called from `CaptionButtonStrip::new` and populates them on demand. Successful D2D init is cached for the lifetime of the `CompositionDevice`; failure is *not* memoised — re-attempt on each new Custom-titlebar window. Apps that never use Custom titlebars never pay the D3D11/D2D/DWrite startup cost.
- **`Window`** gains `caption_buttons: RefCell<Option<CaptionButtonStrip>>`, populated in `initialize_window` if and only if `style.title_bar_kind == Custom` and `CompositionDevice::ensure_d2d_initialised()` succeeded. Construction failure logs and leaves the field `None`.
- **`Window`** gains `nc_leave_tracking_armed: AtomicBool` and helper methods `ensure_nc_leave_tracking()` / `nc_leave_tracking_fired()` — see §3.5.
- **`Window`** gains `restore()` (calls `ShowWindow(SW_RESTORE)`), used by the strip's maximize-button click path. Internal-only for now; FFI export deferred.
- **`event_loop.rs`** gains:
  - Strip consultation in `on_nchittest` after `DwmDefWindowProc` and before the Kotlin callback.
  - The existing pointer handlers (`on_pointerupdate`, `on_pointerdown`, `on_pointerup`) — which already merge client and non-client pointer messages via the `WM_POINTERUPDATE | WM_NCPOINTERUPDATE` / `_DOWN` / `_UP` arms in the wndproc dispatch — are extended to read `HIWORD(wParam)` and route to the strip when the hit-test value is one of `HTCLOSE` / `HTMAXBUTTON` / `HTMINBUTTON`. **No new handlers are added; the routing is added inside the existing handlers.** The `is_non_client` discriminator already used in `on_pointerup` (`event_loop.rs:521-523`) lets the strip path apply only to non-client interactions.
  - Existing `on_ncmouseleave` extended to call `strip.on_nc_mouse_leave()` and `window.nc_leave_tracking_fired()` before the existing `DwmDefWindowProc` pass-through.
  - `on_dpichanged` and `on_settingchange` extended to call the strip's invalidation methods.
  - `on_size` (when `wParam == SIZE_MAXIMIZED` or `SIZE_RESTORED`) extended to call `strip.on_max_state_change(...)`.
  - `on_nccalcsize` extended to call `strip.on_resize(client_size)` after the existing client-rect calculation and before `resize_backdrop_tint`.

The `CompositionDevice` migration is part of Plan 3's scope. Splitting it would create a partial-refactor window where some call sites use the wrapper and others use the bare `Compositor` / `CompositorController` — confusing in-between state. Plan 3 introduces the wrapper, migrates `application.rs` / `window.rs` / `renderer_angle.rs` to it, and adds the D3D11/D2D/DWrite extensions in the same change.

### 3.3 Composition tree restructure

Today, `Window::composition_root` directly holds the backdrop `SpriteVisual` (bottom) and ANGLE's `SpriteVisual` on top. `Window::add_visual` inserts new visuals via `InsertAtTop`, which makes z-order order-of-creation-dependent. With caption buttons added, the strip needs to be reliably above ANGLE.

The fix is a 3-layer split:

```
composition_root (ContainerVisual)
├── backdrop_layer (ContainerVisual, bottom)
│   └── backdrop_tint SpriteVisual (existing)
├── content_layer  (ContainerVisual, middle)
│   └── ANGLE SpriteVisual (added by Window::add_visual)
└── chrome_layer   (ContainerVisual, top)
    └── CaptionButtonStrip's parent ContainerVisual
```

`Window::add_visual` is updated to insert into `content_layer` rather than the root. ANGLE's `SpriteVisual` is the surface ANGLE's EGL window-surface targets; the targeting is bound to the `SpriteVisual` object identity, not to its position in the tree. The Composition object model permits arbitrary `ContainerVisual.Children` nesting (`Visual` instances are addressable per-object via `VisualCollection` insertion methods), so re-parenting the ANGLE `SpriteVisual` from `composition_root` directly to `content_layer` does not invalidate the EGL targeting. Inferred from the Composition object model — there is no MS doc that explicitly addresses ANGLE-targeted `SpriteVisual`s under re-parenting. Verify by running the existing Skiko sample after the restructure. No external API change.

**Z-order construction.** The three layers must be inserted into `composition_root.Children()` in a deterministic order so that `chrome_layer` sits visually above `content_layer` above `backdrop_layer`. `VisualCollection` is documented as ordered bottom-to-top; `InsertAtTop` appends to the end of the collection, which is the visually topmost element. The implementation calls `InsertAtTop` three times in the sequence `backdrop_layer`, `content_layer`, `chrome_layer`, so chrome (last inserted) ends up at the top of the collection and is the topmost rendered. The existing `backdrop_visual` becomes a child of `backdrop_layer` rather than `composition_root` directly; ANGLE's existing `add_visual` path is updated to insert into `content_layer.Children()` so this layer's `InsertAtTop` semantics remain unchanged for callers.

### 3.4 Threading

Single UI thread, consistent with the rest of the crate. `CompositionDevice` and `CaptionButtonStrip` are not `Send`. Device-loss callbacks fire on the dispatcher queue (UI thread) per the Composition+D2D interop sample.

### 3.5 Pointer / leave message routing

The crate uses `EnableMouseInPointer(true)`. The `EnableMouseInPointer` doc states only that mouse input is routed to `WM_POINTER` messages — it does not explicitly cover the non-client variants. Empirical evidence that `WM_NCPOINTER*` messages do fire under `EnableMouseInPointer(true)`: the existing wndproc dispatch merges them into the same handlers as their client-area counterparts (`event_loop.rs:108-112`: `WM_POINTERUPDATE | WM_NCPOINTERUPDATE`, `WM_POINTERDOWN | WM_NCPOINTERDOWN`, `WM_POINTERUP | WM_NCPOINTERUP`), and the body of `on_pointerup` distinguishes the two via an `is_non_client` flag (`event_loop.rs:521-523`) — which would be dead code if the NC variants didn't fire. We rely on this empirical contract.

The pointer-message family does *not* include a `WM_NCPOINTERLEAVE` in the MS docs (the non-client variants documented are `WM_NCPOINTERUPDATE` 0x0241, `WM_NCPOINTERDOWN` 0x0242, `WM_NCPOINTERUP` 0x0243). `WM_POINTERLEAVE` exists but is documented as covering the client area. Verifying a negative is impossible from docs alone, but the documented set has no NC leave message.

Consequence: leave detection uses the legacy `WM_NCMOUSELEAVE`, which still fires under `EnableMouseInPointer(true)` (the existing `on_ncmouseleave` in `event_loop.rs:371-378` confirms this). To receive it, the application must arm tracking via `TrackMouseEvent(TME_NONCLIENT | TME_LEAVE, hwndTrack=hwnd, dwHoverTime=0)`. Per the `WM_NCMOUSELEAVE` Remarks: *"All tracking requested by TrackMouseEvent is canceled when this message is generated."* — the OS clears tracking unilaterally on each leave; we must re-arm to receive the next one.

Tracking armed-state lives on `Window` (one flag per HWND), not on the strip:

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

This keeps the strip a pure state machine with no OS-call dependencies, and future-proofs for any other component that needs NC leave tracking on the same window.

## 4. Components

### 4.1 `CompositionDevice`

```rust
pub(crate) struct CompositionDevice {
    compositor_controller: CompositorController,    // constructed in ::new(); never exposed
    // The D3D11/D2D/DWrite/CompositionGraphicsDevice quartet is wrapped in an
    // Option<...> (or RefCell<Option<...>>) so it can be lazily initialised
    // via ensure_d2d_initialised() — see §3.2.
    d2d: RefCell<Option<D2dStack>>,
    // device-loss bookkeeping (RegisterDeviceRemovedEvent + threadpool wait)
    // RenderingDeviceReplaced subscription token storage
}

struct D2dStack {
    d3d_device: ID3D11Device,
    d2d_device: ID2D1Device,
    dwrite_factory: IDWriteFactory,
    composition_graphics_device: CompositionGraphicsDevice,
}

impl CompositionDevice {
    pub fn new() -> anyhow::Result<Rc<Self>>;
    // Constructs CompositorController internally. Precondition: a WinRT
    // DispatcherQueue must already exist on the calling (UI) thread —
    // CompositorController::new() requires it. Application sets up the
    // DispatcherQueueController in application_init before this is called.

    pub fn create_drawing_surface(
        &self,
        size: SizeInt32,
    ) -> anyhow::Result<CompositionDrawingSurface>;

    pub fn with_d2d_context<R>(
        &self,
        surface: &CompositionDrawingSurface,
        body: impl FnOnce(&ID2D1DeviceContext, POINT) -> anyhow::Result<R>,
    ) -> anyhow::Result<Option<R>>;
    // Ok(None) on DXGI_ERROR_DEVICE_REMOVED — caller skips this frame.
    // Other errors propagate as anyhow::Error.

    pub fn dwrite_factory(&self) -> &IDWriteFactory;
    pub fn compositor(&self) -> WinResult<Compositor>;     // wraps CompositorController::Compositor()
    pub fn compositor_commit(&self) -> WinResult<()>;      // wraps CompositorController::Commit()

    pub fn add_device_replaced_callback(
        &self,
        cb: Box<dyn Fn() + 'static>,
    ) -> RegistrationToken;
}
```

`CompositionDevice::new` constructs its own `CompositorController` (via `CompositorController::new()`) and stores it. The `CompositorController` is never exposed; only the wrapper holds it. The device exposes:

- `compositor()` — wraps `CompositorController::Compositor()`, fallible per the WinRT method's `Result` return. Used by the strip (and migrated call sites in `window.rs` / `renderer_angle.rs`) to create `SpriteVisual` / `ContainerVisual` / `CompositionColorBrush` / `ColorKeyFrameAnimation`.
- `compositor_commit()` — wraps `CompositorController::Commit()`. Hides the underlying `CompositorController` so callers can't accidentally invoke other controller APIs that would interfere with the application-level commit cadence.

`CompositionDevice` is the single owner of the toolkit's Composition stack lifecycle. `Application` no longer constructs `CompositorController` directly; it calls `CompositionDevice::new()` after the dispatcher queue is set up (the dispatcher is a `CompositorController::new` precondition). The strip's `d2d: Rc<CompositionDevice>` field is its single Composition dependency.

Design rationale: the Composition+D2D interop sample's `BeginDraw` / `EndDraw` lifecycle is easy to get wrong (forgetting `EndDraw`, caching the device-context pointer past `EndDraw`, calling `BeginDraw` on a second surface before ending the first). Hiding both behind a closure-shaped `with_d2d_context` means callers can't make those mistakes.

### 4.2 `CaptionButtonStrip`

```rust
pub(crate) struct CaptionButtonStrip {
    composition_root: ContainerVisual,                     // child of window's chrome_layer
    buttons: Vec<CaptionButton>,                           // ordered by visual position
    visible_kinds: CaptionButtonKinds,                     // bitset from WindowStyle flags
    rtl: bool,
    is_active: bool,
    is_window_maximized: bool,
    appearance: Appearance,
    high_contrast: HighContrast,                           // from Plan 2
    metrics: CaptionButtonMetrics,                         // resolved from DPI
    pointer_over_kind: Option<CaptionButtonKind>,
    pointer_device:    Option<PointerDeviceKind>,
    press_session:     Option<PressSession>,
    d2d: Rc<CompositionDevice>,
}

impl CaptionButtonStrip {
    // Constructor takes only what the strip needs — not &Window — to preserve
    // the §4.6 boundary that the strip is independent of Window's wider state.
    pub fn new(
        chrome_layer: &ContainerVisual,    // strip's parent visual; created by the caller
        initial_scale: f32,                // DPI scale for initial metrics
        style: &WindowStyle,               // visible/disabled buttons + RTL flag
        d2d: Rc<CompositionDevice>,
    ) -> anyhow::Result<Self>;

    // Hit-testing — called from on_nchittest after DwmDefWindowProc returned HTCLIENT.
    pub fn hit_test(&self, point_in_window: PhysicalPoint) -> Option<CaptionButtonKind>;

    // Pointer routing — called from event_loop's WM_NCPOINTER* handlers.
    pub fn on_pointer_update(
        &mut self, kind: CaptionButtonKind, pointer_id: u32, device: PointerDeviceKind,
    ) -> anyhow::Result<()>;
    pub fn on_pointer_down(
        &mut self, kind: CaptionButtonKind, pointer_id: u32, device: PointerDeviceKind,
    ) -> anyhow::Result<()>;
    pub fn on_pointer_up(
        &mut self, kind: CaptionButtonKind, pointer_id: u32,
    ) -> anyhow::Result<Option<CaptionButtonAction>>;
    pub fn on_nc_mouse_leave(&mut self) -> anyhow::Result<()>;

    // Theming / metric / state changes.
    pub fn on_activate(&mut self, is_active: bool) -> anyhow::Result<()>;
    pub fn on_dpi_change(&mut self, new_scale: f32) -> anyhow::Result<()>;
    pub fn on_appearance_change(
        &mut self, appearance: Appearance, high_contrast: HighContrast,
    ) -> anyhow::Result<()>;
    pub fn on_max_state_change(&mut self, is_maximized: bool) -> anyhow::Result<()>;
    pub fn on_resize(&mut self, client_size: PhysicalSize) -> anyhow::Result<()>;
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum CaptionButtonAction {
    Close,
    Minimize,
    Maximize,
    Restore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CaptionButtonKind {
    Minimize,
    Maximize,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PointerDeviceKind { Mouse, Pen, Touch }

struct PressSession {
    pointer_id: u32,
    captured_kind: CaptionButtonKind,
    device: PointerDeviceKind,
}
```

The strip never invokes `Window::request_close` and friends directly — it returns `CaptionButtonAction` and the wndproc dispatches. This keeps the strip independent of `Window`'s broader state and unit-testable in isolation.

### 4.3 `CaptionButton` (private to `caption_buttons.rs`)

```rust
struct CaptionButton {
    kind: CaptionButtonKind,
    availability: Availability,
    visuals: CaptionButtonVisuals,
    last_applied_interaction: ButtonInteraction,
    glyph_surface_dirty: bool,
}

struct CaptionButtonVisuals {
    backplate: SpriteVisual,
    backplate_brush: CompositionColorBrush,
    glyph: SpriteVisual,
    glyph_brush: CompositionColorBrush,
    glyph_surface: CompositionDrawingSurface,
    glyph_surface_brush: CompositionSurfaceBrush,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Availability { Enabled, Disabled }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonInteraction {
    Idle,
    Hovered,                         // pointer over me, no press anywhere; mouse / pen only
    Pressed,                         // capture is mine and pointer is over me
    PressedDraggedOff,               // capture is mine and pointer left
}
```

The `glyph_surface` is rasterised in alpha-only / monochrome form. The actual colour comes from the `glyph_brush` (a `CompositionColorBrush`) blended through the surface. State transitions only mutate the brush colours; glyph re-rasterisation happens only on DPI / theme / high-contrast / max-state changes. (Implementation detail to confirm: whether `CompositionDrawingSurface` supports `DXGI_FORMAT_A8_UNORM` directly. If not, fall back to BGRA8 with white pixels and rely on premultiplied-alpha composition. Either path preserves the rasterise-once-per-glyph-variant property.)

### 4.4 `CaptionTheme` (private)

```rust
struct CaptionTheme {
    backplate_rest: windows::UI::Color,
    backplate_hover: windows::UI::Color,
    backplate_pressed: windows::UI::Color,
    backplate_inactive: windows::UI::Color,
    foreground_rest: windows::UI::Color,
    foreground_hover: windows::UI::Color,
    foreground_pressed: windows::UI::Color,
    foreground_disabled: windows::UI::Color,
    foreground_inactive: windows::UI::Color,
    // Close-specific overrides for the documented system-red rule.
    close_backplate_hover: windows::UI::Color,
    close_backplate_pressed: windows::UI::Color,
    close_foreground_hover: windows::UI::Color,
    close_foreground_pressed: windows::UI::Color,
}

impl CaptionTheme {
    fn resolve(appearance: Appearance, high_contrast: HighContrast, is_active: bool) -> Self;
}
```

Construction strategy:

- **`HighContrast::On` branch.** Reads from `GetSysColor`. The slot mapping is *partly* documented and *partly* convention:
  - `COLOR_BTNFACE` (background) and `COLOR_BTNTEXT` (foreground) for rest state — **directly cited** in the *High contrast parameter* doc: *"Use the `GetSysColor` function to determine the appropriate foreground and background colors, using either a combination of `COLOR_WINDOWTEXT` and `COLOR_WINDOW` or a combination of `COLOR_BTNTEXT` and `COLOR_BTNFACE`."*
  - `COLOR_GRAYTEXT` for disabled foreground — documented in the *Button Messages* doc as *"Disabled (gray) text in buttons."* General-purpose, applicable to caption buttons by analogy.
  - `COLOR_HIGHLIGHT` (background) and `COLOR_HIGHLIGHTTEXT` (foreground) for hover and pressed states — **convention, not directly cited** in the *High contrast parameter* doc. Supporting evidence: *Compatibility/High-contrast mode* doc says *"`COLOR_HIGHLIGHTTEXT` is meant to be used with `COLOR_HIGHLIGHT` as a background"*; Visual Studio's high-contrast colour-usage guide treats `Highlight` / `HighlightText` as the canonical hover/pressed pair. We follow this convention; if a specific MS doc later prescribes different slots for caption-button hover/pressed in high contrast, revisit.
- **`HighContrast::Off` branch.** Hard-coded table sourced from `microsoft/terminal:src/cascadia/TerminalApp/MinMaxCloseControl.xaml` (and adjacent `*.xaml` files holding the `CaptionButtonBackground*` / `CaptionButtonForeground*` `ThemeResource` definitions). Each colour is annotated in a comment with its repo path + commit revision so future audits can trace provenance. **WinUI itself doesn't expose these values** — `microsoft/microsoft-ui-xaml`'s TitleBar control delegates caption-button rendering to `Microsoft.UI.Windowing.AppWindowTitleBar`, which is not source-citable. Windows Terminal is the highest-fidelity public reference.
- **Close-specific override** for hover and pressed: a WinUI / Windows App SDK UX convention that we replicate. The exact phrasing — *"The button background color is not applied to the Close button hover and pressed states. The close button always uses the system-defined color for those states."* — appears in the *Title bar customization → Full customization* doc, but that doc describes the WinUI / Windows App SDK `AppWindowTitleBar` API, which `desktop-win32` deliberately does **not** depend on (per AGENTS.md / ARCHITECTURE.md "Win32-first" discipline). The Win32 docs do not document a "close button is system red on hover" rule directly; we replicate the WinUI behaviour by sourcing the actual hex values from Windows Terminal, which is itself a Win32 host and a non-WinAppSDK reference implementation. Treating the WinUI doc as a source of UX *convention* (not as an API contract our crate consumes) is consistent with the crate's discipline.

Use `windows::UI::Color { A, R, G, B }` directly — matching the existing `Window::set_backdrop_tint` precedent (`window.rs:271-286`). `D2D1_COLOR_F` conversion happens only at draw time, never stored.

### 4.5 `CaptionButtonMetrics` (private)

```rust
struct CaptionButtonMetrics {
    button_size: SizeInt32,         // 46 × 32 epx → physical via DPI
    glyph_extent: SizeInt32,        // 10 × 10 epx (the bounding box of the rendered glyph)
    strip_top_offset: i32,          // 0 epx — strip is top-anchored
}
```

Values sourced from `microsoft/terminal:src/cascadia/TerminalApp/MinMaxCloseControl.xaml`:

| Dimension | Value | Source |
|---|---|---|
| Width | 46.0 epx | `Width="46.0"` |
| Height | 32.0 epx | `<x:Double x:Key="CaptionButtonHeightMaximized">32.0</x:Double>` — Terminal swaps between two heights via a `WindowTitleBarHeight`-shaped state machine (windowed: 40, maximised: 32). Plan 3 hard-codes the 32 epx height; Plan 5 introduces the windowed/maximised split. |
| Glyph rendered extent | 10 × 10 epx | Terminal's XAML wraps each glyph in a `Viewbox Width="10" Height="10"`; FontSize is unset and the Viewbox scales the glyph to that area. |
| Glyph font family | indirected via `SymbolThemeFontFamily` ThemeResource | Terminal uses `FontFamily="{ThemeResource SymbolThemeFontFamily}"`, which on modern Windows resolves to Segoe Fluent Icons (Win11) with Segoe MDL2 Assets fallback (Win10). For DirectWrite, request `Segoe Fluent Icons` and rely on the OS's font-fallback chain. |

Plan 3 has a single fixed height of 32 epx in both windowed and maximised states. Tall mode (40 epx windowed), the windowed↔maximised height transition, and the suspected ~8 epx strip y-offset for Tall maximised are all deferred to Plan 5, which introduces the `WindowTitleBarHeight` enum and the corresponding `resolve_button_height` machinery.

The 10 × 10 epx is a **rendered extent**, not a font point size. With DirectWrite, we don't have a `Viewbox`; we instead pick a `IDWriteTextFormat` font size that produces a glyph whose visible black-box matches the 10 epx target. Since Segoe Fluent Icons / Segoe MDL2 Assets glyphs at code points U+E921 / E922 / E923 / E8BB occupy roughly the full em-box, requesting an em size of ~10 device-independent pixels (in DIPs at the current DPI) is a reasonable starting point. Verify visually during implementation; the precise font size is *not* documented anywhere I've located, so the spec leaves it as a tunable rather than a fixed value.

### 4.6 Boundaries

- `CompositionDevice` knows nothing about caption buttons. It's a generic facility usable by any future Composition+D2D consumer in the crate.
- `CaptionButtonStrip` knows nothing about Win32 messages or wndproc. Inputs are typed (kind, pointer id, device kind, theme, etc.); outputs are typed (`Option<CaptionButtonAction>`).
- `CaptionButton`, `CaptionTheme`, `CaptionButtonMetrics`, `Availability`, `ButtonInteraction`, `PressSession` are private to `caption_buttons.rs`.
- `Window` only knows how to insert a `ContainerVisual` into its `chrome_layer` and how to call the strip's lifecycle methods. No new public methods leak through FFI.

## 5. Data flow

### 5.1 Visual state derivation

Per-button state is derived (not stored as flags) from strip-level state:

```rust
fn resolve_interaction(
    kind: CaptionButtonKind,
    availability: Availability,
    pointer_over_kind: Option<CaptionButtonKind>,
    pointer_device: Option<PointerDeviceKind>,
    press_session: Option<&PressSession>,
) -> ButtonInteraction {
    if availability == Availability::Disabled {
        return ButtonInteraction::Idle;
    }
    let is_pointer_over_self = pointer_over_kind == Some(kind);
    match press_session {
        Some(s) if s.captured_kind == kind => {
            if is_pointer_over_self { ButtonInteraction::Pressed }
            else                    { ButtonInteraction::PressedDraggedOff }
        }
        Some(_) => ButtonInteraction::Idle,    // capture is on another button — this one stays Idle
        None if is_pointer_over_self => match pointer_device {
            Some(PointerDeviceKind::Touch) => ButtonInteraction::Idle,
            _                              => ButtonInteraction::Hovered,
        },
        None => ButtonInteraction::Idle,
    }
}
```

The `Some(_) => Idle` branch is the WinUI-documented capture rule: when a press is owned by button A, button B does not enter Hovered state when the pointer moves over it. This matches `UIElement.PointerEntered`'s documented behaviour ("If another element has captured the pointer, PointerEntered won't fire even if the captured pointer enters an element's bounds") and the `UIElement.CapturePointer` Remarks ("the second element doesn't fire `PointerEntered` events for a captured pointer when the captured pointer enters it"). `PointerOver` visual state is driven by the same plumbing, so without the event firing the visual state can't change.

Touch devices skip the Hovered state because touch input has no hover phase — `Idle → Pressed → action → Idle` directly.

### 5.2 Animation contract

Sourced from `microsoft/terminal:src/cascadia/TerminalApp/MinMaxCloseControl.xaml` `CommonStates` `VisualStateGroup`:

- Only the `PointerOver → Normal` and `PointerOver → Unfocused` transitions animate. Every other transition jumps via `Setter`.
- Backplate (`ButtonBaseElement.Background.Color`): **150 ms**.
- Glyph foreground (`ButtonIcon.Foreground.Color`): **100 ms**.
- No `EasingFunction` element is present in the storyboards. We follow Terminal's choice and use no easing — i.e., interpolate uniformly across the duration. (The XAML default for `ColorAnimation` without an `EasingFunction` is not something I've located documentation for in the verification pass; the spec's contract is "match Terminal's behaviour by omitting the easing function" rather than "use the XAML default, whatever that is.")

Mapped to our `ButtonInteraction` transitions:

| Transition | Animated? | Duration |
|---|---|---|
| `Idle → Hovered` | jump | — |
| `Hovered → Idle` | **animate** | backplate 150 ms, glyph 100 ms, no easing function |
| `Hovered → Pressed` | jump | — |
| `Pressed → Hovered` (action fires here) | jump | — |
| `Pressed → PressedDraggedOff` | jump | — |
| `PressedDraggedOff → Pressed` | jump | — |
| `PressedDraggedOff → Idle` (no action) | jump | — |
| any → `Disabled` | jump | — |
| `is_active` flip | jump | — |
| `is_window_maximized` flip (max glyph swap) | jump | — |

Implementation: `ColorKeyFrameAnimation` started via `Compositor.CreateColorKeyFrameAnimation` and applied via `CompositionColorBrush.StartAnimation("Color", animation)`. Per the API doc, `ColorKeyFrameAnimation` is available since Windows 10 build 10586 (version 1511) — well below the toolkit's effective minimum.

### 5.3 Inputs and what they trigger

| Input | Strip method | Cost |
|---|---|---|
| `WM_NCHITTEST` | `hit_test(point)` | cheap (geometry math) |
| `WM_NCPOINTERUPDATE` over a button | `on_pointer_update(kind, id, device)` (also: window arms NC leave tracking) | cheap (state + brush) |
| `WM_NCPOINTERDOWN` over a button | `on_pointer_down(kind, id, device)` | cheap |
| `WM_NCPOINTERUP` over a button | `on_pointer_up(kind, id) → Option<CaptionButtonAction>` | cheap |
| `WM_NCMOUSELEAVE` | `on_nc_mouse_leave()` (also: window clears tracking armed) | cheap |
| `WM_NCCALCSIZE` (after client-rect calc) | `on_resize(client_size)` | cheap (`SetOffset` per button) |
| `WM_DPICHANGED` | `on_dpi_change(new_scale)` | **expensive** (re-rasterise all glyph surfaces) |
| `Appearance` event | `on_appearance_change(appearance, hc)` | **expensive iff foreground-rest colour changed** |
| `HighContrast` event (Plan 2) | same as above with new `hc` | **expensive** (glyph code points swap E921↔EF2D etc.) |
| `WM_ACTIVATE` | `on_activate(is_active)` | cheap (theme re-resolve, brush updates) |
| `WM_SIZE` with `wParam == SIZE_MAXIMIZED` / `SIZE_RESTORED` | `on_max_state_change(is_maximized)` | **expensive for Maximize button only** (E922 ↔ E923) |

The `WM_SIZE` row covers both user-driven (button click → toolkit `ShowWindow(SW_*)`, then system fires `WM_SIZE`) and programmatic (app calls `Window::maximize`/`Window::restore`) paths — both flow through `WM_SIZE`, so the strip needs only one handler. `SIZE_MINIMIZED` doesn't trigger this path (the window is hidden when minimised; redrawing is irrelevant).

"Expensive" rows go through `CompositionDevice::with_d2d_context` and a full glyph redraw on the affected button's surface. All others are O(1) brush colour updates.

### 5.4 Hit-test → action mapping

The wndproc routes pointer-up to the strip; the strip returns `Option<CaptionButtonAction>`; the wndproc dispatches:

| Action | Wndproc dispatch |
|---|---|
| `Close` | `Window::request_close()` (existing — `PostMessage WM_CLOSE`) |
| `Minimize` | `Window::minimize()` (existing — `ShowWindow(SW_SHOWMINIMIZED)`) |
| `Maximize` | `Window::maximize()` (existing — `ShowWindow(SW_SHOWMAXIMIZED)`) |
| `Restore` | `Window::restore()` (new — `ShowWindow(SW_RESTORE)`) |

The Maximize button's action depends on current state at click time: the strip reads its own `is_window_maximized` field (kept in sync via `on_max_state_change` — see §5.3) and returns `Restore` if maximized, `Maximize` otherwise. The field is the single source of truth inside the strip; the strip does not call back into `Window` or `IsZoomed` to resolve the action, preserving the §4.6 boundary that the strip is independent of `Window`'s wider state.

### 5.5 Frame commit coupling

The `CompositorController` is owned by `CompositionDevice` (per §3.2 / §4.1) and never exposed. The strip calls `self.d2d.compositor_commit()` after every state mutation (brush colour change, offset update, surface re-rasterisation). This decouples the strip's visual update cadence from the app's rendering cadence — apps using on-demand rendering still see snappy button transitions. Cost is expected to be negligible on the UI thread; commits are sequential (the toolkit is single-threaded). If the per-mutation commit cost ever appears on a profile during implementation, an alternative is to coalesce by deferring `compositor_commit` until the wndproc handler returns; for now the simplicity of "every mutation commits" outweighs the speculative perf concern.

### 5.6 New on_nchittest order (after Plan 1 lands)

```
1. DwmDefWindowProc(WM_NCHITTEST, ...) — returns resize-border codes (HTTOP / HTLEFT / etc.) where
   it does, otherwise FALSE/HTCLIENT.
2. If result is HTCLIENT, ask the strip via strip.hit_test(point).
   On Some(kind), return the corresponding HTCLOSE / HTMAXBUTTON / HTMINBUTTON.
3. If still HTCLIENT, fire the existing Kotlin NCHitTestEvent callback so the app can carve interactive
   sub-regions (search box in title bar, etc.).
4. If still HTCLIENT, run the existing manual top-edge math (resize_handle_height / title_bar_height
   from event_loop.rs:354-368) to convert points near the top edge into HTTOP (resize) or HTCAPTION
   (drag region). This step is preserved from the current code, not removed.
5. Otherwise return HTCLIENT.
```

**Important caveat about steps 1 and 4.** Microsoft's *Custom Window Frame Using DWM* canonical recipe shows a `HitTestNCA` example that **manually** computes the top resize zone — i.e., it does not rely on `DwmDefWindowProc` or `DefWindowProcW` to return `HTTOP` for the top edge of a custom-frame window. The current crate code matches that recipe (`event_loop.rs:354-368`). My earlier draft of this spec optimistically claimed the math could be dropped after `WS_CAPTION` removal. That claim was unverified and likely wrong: the MS recipe does the math precisely because the system's hit-test does *not* return resize codes for a custom frame. **The manual top-edge math stays.** Plan 1's simplification of `on_nchittest` is therefore narrower than I first thought: drop the special-case for the top inset in `on_nccalcsize` (no longer needed once `WS_CAPTION` is gone), but keep the manual hit-test math for resize/caption regions. The strip's hit-test slot in §5.6 step 2 is the meaningful additive change.

Step 1's behaviour is also inferred — without `WS_CAPTION` the system-caption-button styles are inert (§2 dependency chain), so `DwmDefWindowProc` has no caption buttons to hit-test. Resize-border behaviour from `DwmDefWindowProc` / `DefWindowProcW` for a `WS_CAPTION`-less / `WS_THICKFRAME`-present window is empirically variable and is the reason step 4 retains the manual fallback. Verify both during Plan 1 implementation.

## 6. Error handling

Three tiers, with caption buttons treated as best-effort throughout — never block the window from coming up.

### 6.1 Construction-time

- **`CompositionDevice::ensure_d2d_initialised` failure** (D3D11 / D2D / DWrite / `CompositionGraphicsDevice` creation): logged via `anyhow::Error` through `ffi_boundary`. The `CompositionDevice`'s internal D2D cache stays `None`. Subsequent windows with `Custom` titlebar retry — failure is *not* memoised, so transient driver hiccups don't permanently disable buttons. `CompositionDevice::new` itself only constructs the `CompositorController` and is not expected to fail outside of catastrophic conditions; if it does, that's an `Application::init` failure tier (out of caption-button error scope).
- **`CaptionButtonStrip::new` failure** (e.g., `CompositionGraphicsDevice::CreateDrawingSurface` returns failure): logged. `Window.caption_buttons` stays `None`. Window comes up without caption buttons; drag region, resize, Alt+F4, taskbar context menu still work.
- All construction-time errors are non-fatal to window creation. Kotlin never sees them as exceptions.

**TODO (deferred to a later plan):** when caption-button construction fails, auto-promote the window to `WindowTitleBarKind::System` so the user gets visible system-drawn caption buttons as an obvious indication that the toolkit's custom path is broken. Implementation requires runtime `WindowStyle` mutation (which doesn't exist today): `SetWindowLongPtrW(GWL_STYLE, +WS_CAPTION)`, undoing `DwmExtendFrameIntoClientArea`, switching `on_nccalcsize` to System-mode handling, `SetWindowPos(... SWP_FRAMECHANGED ...)`, and tearing down `chrome_layer` from the composition tree. A literal `// TODO(plan-N): on failure, promote this window to WindowTitleBarKind::System` comment lives at the failure site.

### 6.2 Runtime: device loss

`DXGI_ERROR_DEVICE_REMOVED` from `BeginDraw` returns `Ok(None)` from `with_d2d_context`. The `CompositionGraphicsDevice`'s `RenderingDeviceReplaced` event fires asynchronously when a new D3D11/D2D device is wired in via `ICompositionGraphicsDeviceInterop::SetRenderingDevice`. Each strip subscribes in its constructor:

1. Mark all glyph surfaces dirty (`glyph_surface_dirty = true` on every button).
2. Schedule a redraw via the dispatcher to re-rasterise the dirty surfaces and `Commit` once.

Eager (immediate) re-rasterisation is preferred over lazy — driver crashes are rare; when they happen, the user expects visual recovery without needing to wiggle the mouse to "wake up" the buttons. Cost is bounded at ~3 D2D draws per device-loss event per window.

If the new D2D device also fails to draw (cascading device loss), the strip logs and skips that frame; the next `RenderingDeviceReplaced` will retry. Device-loss events are rate-limited by the OS, so no retry storm.

### 6.3 Runtime: per-call failures

- **`DirectWrite` failure** (font missing, malformed code point — practically impossible since Segoe Fluent Icons / Segoe MDL2 Assets are OS-shipped): logged. Glyph surface stays at its prior contents. Button remains hit-testable and clickable.
- **`StartAnimation` failure**: fall back to instant `Color` set on the `CompositionColorBrush`. Logged at warning level. Visual jumps instead of fading.
- **`TrackMouseEvent` failure**: logged at warning level. Hover-fade-out won't trigger when the pointer leaves the strip; corrects itself on next hover-in. Documented degradation; non-fatal.
- **`SetOffset` / `SetSize` / `SetBrush` failures**: extremely rare. Logged; continue with stale visual; next state mutation retries.

### 6.4 What's *not* in the error budget

- Window creation failure due to caption-button issues. Never.
- Caption-button-related panics. Anything reaching `ffi_boundary`'s `catch_unwind` is a bug, not a planned path.

## 7. Testing

### 7.1 Pure-Rust unit tests (no display, no GPU)

These cover load-bearing logic without composition or D3D dependency:

- **`resolve_interaction`** truth-table over `(Availability, kind, pointer_over_kind, pointer_device, press_session)`. ~30-40 cases. Catches regressions on the WinUI capture rule and the touch-skip-hover rule.
- **`CaptionButton::transition_to(...)`**: which transitions animate vs jump.
- **`hit_test`**: geometry math at varied DPI scales, with RTL on/off, with disabled buttons suppressed.
- **`CaptionButtonMetrics::new`**: 46 × 32 epx round-trip through DPI scales 1.0 / 1.25 / 1.5 / 1.75 / 2.0 / 2.5.
- **`CaptionTheme::resolve`**: per `(Appearance, HighContrast, IsActive)` combo, expected colour-table cells. Catches Close-button override typos.
- **Layout for visible / disabled buttons** from `WindowStyle` flags.

These tests live in `caption_buttons.rs` `#[cfg(test)] mod tests { ... }`. No Window or Application required.

### 7.2 Tests that require live composition (skipped under `cargo test`)

- Anything exercising real `CompositionDevice` (D3D11), real `CompositionGraphicsDevice`, real `BeginDraw` / `EndDraw`. Need a desktop session and GPU on a UI thread.
- Snapshot / golden-image tests for rasterised glyphs. Out of scope for this plan; recorded as a follow-on if visual regressions warrant it.

### 7.3 Manual test plan exercised via the sample app

`:sample:runSkikoSampleWin32` gets a Custom-titlebar mode (or extends an existing one). Acceptance checklist for review:

- All four states (Rest / Hover / Pressed / Disabled) per button, in light and dark themes.
- Window inactive vs active — colour modulation visible.
- High contrast on / off — glyph code points and palette swap.
- Maximize → Restore via the button — glyph swap U+E922 ↔ U+E923.
- Snap-layout flyout appears on Win11 hover over the maximize button.
- Mouse, touch, and pen input — touch correctly skips the hover state.
- DPI change by dragging across monitors with different scales — glyph re-rasterised crisply.
- Drag the title bar (drag region between strip and left edge) to move the window.
- Press a button, drag off, release outside — no action fires; button visually returns to Rest.
- Press button A, drag over button B — B does *not* react (WinUI capture rule).
- Win+arrow snap shortcuts and taskbar right-click → Restore / Move / Size / Min / Max / Close all still work.

This list goes into the sample app's `README` so the manual run-through is reproducible.

## 8. Open questions / future work

1. **Exact strip y-offset when a Tall window is maximised.** Plan 5. Suspected ~8 epx downward shift to align visible button bottoms with screen edge under Win11 invisible-resize-border math; verify against Windows Terminal source or empirical measurement.
2. **Whether Plan 3's hard-coded 32 epx height needs a maximised adjustment.** Plan 5 confirms or revises. Current Plan 3 assumption: no — 32 epx in both windowed and maximised states. (Plan 5's `WindowTitleBarHeight::Standard` is expected to keep the same behaviour; only `Tall` introduces the windowed/maximised split.)
3. **Glyph surface format**: `DXGI_FORMAT_A8_UNORM` vs BGRA8 with white pixels and premultiplied-alpha composition. Decided during implementation; either preserves the rasterise-once-per-glyph-variant property.
4. **Auto-fall-back-to-`WindowTitleBarKind::System`** on caption-button construction failure. Future plan; depends on a runtime `WindowStyle` mutation API the toolkit doesn't yet have.
5. **Per-Close-button override colours**: confirm Terminal's `CaptionButtonBackgroundPointerOver` / `…Pressed` overrides specifically for the close button match the documented system-red rule. Done during spec implementation; values pinned by repo path + commit.
6. **Animation easing**: Terminal's storyboards omit `EasingFunction`; we follow suit and use no easing. The XAML default for absent `EasingFunction` is uncited; we treat "match Terminal's choice" as the contract rather than asserting the XAML semantics.
7. **Glyph DirectWrite font size to match the 10 epx Terminal Viewbox extent.** Terminal uses a XAML `Viewbox` for visual scaling rather than a fixed font size. We have to pick an explicit `IDWriteTextFormat` font size. Starting estimate is ~10 DIPs; tune visually during implementation.
8. **Empirical verification needed for the WS_CAPTION-removed → no-DWM-caption-buttons claim.** The Plan 1 / Plan 3 architecture rests on it; the dependency chain (§2) is documented but the *consequence* for DWM rendering is not. Verify visually in Plan 1's implementation and document the result in `SUBSYSTEMS.md`.
9. **Empirical verification needed for the `EnableMouseInPointer` + `WM_NCPOINTER*` empirical contract.** The `EnableMouseInPointer` doc only addresses client-area `WM_POINTER` routing. The toolkit's existing handler relies on `WM_NCPOINTER*` firing under `EnableMouseInPointer`. The behaviour is empirical; if a future Windows release changed it, the strip's input routing would break silently.
10. **`DwmDefWindowProc(WM_NCHITTEST)` behaviour over the title-bar area when `WS_CAPTION` is absent.** Inferred to return FALSE / HTCLIENT (no caption buttons present); not documented. Verify during implementation.
11. **Asymmetric per-window outcome when D2D init fails on the first Custom-titlebar window but later succeeds.** The "failure is not memoised" policy (§3.2) means a second window can succeed where the first failed — but the first window stays without buttons indefinitely (the strip is created in `initialize_window` and is never re-tried after the window's lifecycle starts). Acceptable for v1 (driver crashes during the first window of an app session are rare); a fix would require re-attempting strip construction on a subsequent event (e.g., `WM_DPICHANGED` or an explicit "recover" hook). Worth flagging if reports come in.

## 9. References

### Microsoft documentation

- [Custom Window Frame Using DWM](https://learn.microsoft.com/windows/win32/dwm/customframe) — the canonical Win32 custom-frame recipe (`WM_NCCALCSIZE`, `DwmExtendFrameIntoClientArea`, `WM_NCHITTEST`, `DwmDefWindowProc`).
- [WM_NCCALCSIZE message](https://learn.microsoft.com/windows/win32/winmsg/wm-nccalcsize) — confirms standard-frame removal does not affect DWM-extended frames; Plan 1 removes `WS_CAPTION` so the system caption buttons don't draw.
- [WM_NCHITTEST message](https://learn.microsoft.com/windows/win32/inputdev/wm-nchittest) — return-value table for `HTCLOSE` (20), `HTMAXBUTTON` (9), `HTMINBUTTON` (8).
- [Support snap layouts for desktop apps on Windows 11](https://learn.microsoft.com/windows/apps/desktop/modernize/ui/apply-snap-layout-menu) — `HTMAXBUTTON` is the documented contract for Win11 snap-layout flyout.
- [DwmDefWindowProc function](https://learn.microsoft.com/windows/win32/api/dwmapi/nf-dwmapi-dwmdefwindowproc) — first consultation in `WM_NCHITTEST` and `WM_NCMOUSELEAVE` for custom frames.
- [Composition native interoperation with DirectX and Direct2D](https://learn.microsoft.com/en-us/windows/uwp/composition/composition-native-interop) — the canonical interop pattern (`ICompositorInterop`, `CompositionGraphicsDevice`, `CompositionDrawingSurface`, `BeginDraw` / `EndDraw`, `RenderingDeviceReplaced`).
- [Segoe Fluent Icons font](https://learn.microsoft.com/windows/apps/design/iconography/segoe-fluent-icons-font) — glyph code points (E921, E922, E923, E8BB) and high-contrast variants (EF2D, EF2E, EF2F, EF2C).
- [Title bar customization](https://learn.microsoft.com/windows/apps/develop/title-bar) — *"The button background color is not applied to the Close button hover and pressed states. The close button always uses the system-defined color for those states."*
- [High contrast parameter](https://learn.microsoft.com/windows/win32/winauto/high-contrast-parameter) — `GetSysColor` slot mapping for high-contrast colours; documents only the `BTN*` and `WINDOW*` foreground/background pairs.
- [Compatibility / High-contrast mode](https://learn.microsoft.com/windows/compatibility/high-contrast-mode) — *"`COLOR_HIGHLIGHTTEXT` is meant to be used with `COLOR_HIGHLIGHT` as a background"*; supports the convention of using `HIGHLIGHT` / `HIGHLIGHTTEXT` for selected/hover state.
- [Button Messages — Button Color Messages](https://learn.microsoft.com/windows/win32/controls/button-messages#button-color-messages) — `COLOR_GRAYTEXT` is *"Disabled (gray) text in buttons."*
- [Window Styles](https://learn.microsoft.com/windows/win32/winmsg/window-styles) — `WS_SYSMENU` *"The `WS_CAPTION` style must also be specified."*; `WS_MAXIMIZEBOX` / `WS_MINIMIZEBOX` *"The `WS_SYSMENU` style must also be specified."* — the dependency chain Plan 1 leverages.
- [ColorKeyFrameAnimation Class](https://learn.microsoft.com/uwp/api/windows.ui.composition.colorkeyframeanimation?view=winrt-28000) — available since Windows 10 build 10586 (1511 / Nov 2015 Update).
- [UIElement.CapturePointer (WinUI)](https://learn.microsoft.com/windows/windows-app-sdk/api/winrt/microsoft.ui.xaml.uielement.capturepointer?view=windows-app-sdk-1.8) — *"the second element doesn't fire `PointerEntered` events for a captured pointer when the captured pointer enters it."*
- [UIElement.PointerEntered (WinUI)](https://learn.microsoft.com/windows/windows-app-sdk/api/winrt/microsoft.ui.xaml.uielement.pointerentered?view=windows-app-sdk-1.8) — `PointerOver` visual state is loaded by the same plumbing.
- [WM_NCMOUSELEAVE message](https://learn.microsoft.com/windows/win32/inputdev/wm-ncmouseleave) — *"All tracking requested by TrackMouseEvent is canceled when this message is generated."*
- [WM_NCPOINTERDOWN](https://learn.microsoft.com/windows/win32/inputmsg/wm-ncpointerdown), [WM_NCPOINTERUPDATE](https://learn.microsoft.com/windows/win32/inputmsg/wm-ncpointerupdate) — pointer-message routing under `EnableMouseInPointer(true)`; `HIWORD(wParam)` carries the prior `WM_NCHITTEST` result.
- [microsoft-ui-xaml/specs/TitleBar/titlebar-functional-spec.md](https://github.com/microsoft/microsoft-ui-xaml/blob/main/specs/TitleBar/titlebar-functional-spec.md) — confirms WinUI delegates caption-button rendering to `AppWindowTitleBar`.

### Source-code references

- [microsoft/terminal MinMaxCloseControl.xaml (main)](https://github.com/microsoft/terminal/blob/main/src/cascadia/TerminalApp/MinMaxCloseControl.xaml) — caption-button metrics, glyph code points, `VisualState`s, storyboards.
- [Min/Max/Close buttons should be 32px · Issue #9093](https://github.com/microsoft/terminal/issues/9093) — Terminal's rationale for the 40-windowed / 32-maximised height pair.

### Crate-internal references

- `native/desktop-win32/docs/AGENTS.md` — agent orientation; WinRT-only-where-necessary discipline.
- `native/desktop-win32/docs/ARCHITECTURE.md` § Composition — `Windows.UI.Composition` via `ICompositorDesktopInterop`, controlled-commit `CompositorController`.
- `native/desktop-win32/docs/SUBSYSTEMS.md` — Window, Renderer (ANGLE), Pointer, Appearance subsystems.
- `native/desktop-win32/docs/TODO.md` — items #89 (`WindowTitleBarKind::Custom` reachability — corrected during this plan), #93 (high-contrast modelling — Plan 2).
