# Win32 caption buttons — design

**Plan:** caption buttons.
**Crate:** `native/desktop-win32`.
**Status:** spec.

## 1. Goal

Add minimise / maximise-restore / close caption buttons to windows that use `WindowTitleBarKind::Custom` on Windows. The buttons are toolkit-managed `Windows.UI.Composition` visuals, with full state coverage (rest, hover, pressed, disabled for Minimize / Maximize, plus active / inactive modulation), full appearance coverage (light, dark, high contrast on / off), and Win32-correct hit-testing including Win11 snap-layout flyout integration.

The toolkit owns the buttons end-to-end. Click side-effects translate to the toolkit's existing `Window::request_close` / `Window::minimize` / `Window::maximize` / `Window::restore`. Apps don't get a per-button-press event — caption buttons behave like the system buttons they replace.

## 2. Scope

Caption-button rendering and pointer interaction for `WindowTitleBarKind::Custom` windows, including `HTMAXBUTTON` Snap Layouts integration, high contrast, DPI, and Windows 10/11 icon fallback. System-menu restoration is tracked separately in `TODO.md`.

Inherited preconditions:

- Non-system titlebar windows (`WindowTitleBarKind::Custom` / `WindowTitleBarKind::None`) keep `WS_CAPTION` but clear `WS_SYSMENU` in `WindowStyle::to_system`. This preserves native min/max/restore transition semantics while suppressing system caption controls; the toolkit owns Minimize / Maximize / Close rendering for `Custom` windows.
- `on_nccalcsize` uses the current window style for left / right / bottom frame adjustment and intentionally leaves the Custom top inset at 0 so the title-bar area remains client area. Preserve that; add only the maximized inset and strip resize after the client rect is computed.
- `on_nchittest` keeps the current first consultation with `DwmDefWindowProc` / `DefWindowProcW`. Insert strip hit-testing before preserving any default non-`HTCLIENT` result for points inside the strip. Outside the strip, preserve any non-`HTCLIENT` result before the Kotlin `NCHitTestEvent` callback and manual top-edge fallback. Split the current `!window.is_resizable()` early return so non-resizable Custom-titlebar windows still get caption-button hit-testing and title-bar drag fallback; only the resize-border math remains conditional on `is_resizable`.
- `Appearance` is queried alongside a `HighContrast` enum (`Off` / `On`) which the strip consumes.

## 3. Architecture

### 3.1 New modules

Two crate-internal modules, both `pub(crate)` only — no FFI surface, no `_api.rs` partner.

- **`composition.rs`** (new) — defines `D2dContext`, private to caption-button rasterisation. Holds the `IDWriteFactory` and `CompositionGraphicsDevice` (both survive device loss); CGD retains the D3D11 / D2D rendering device, swapped on device loss via `SetRenderingDevice`. Hides `BeginDraw` / `EndDraw` and device-loss handling behind a single `with_d2d_render_target(surface, |rt, offset| -> ...)` chokepoint.
- **`caption_buttons.rs`** — owns the per-window strip. Pure state-machine over typed inputs; no Win32 calls itself. The wndproc layer in `event_loop.rs` is the only place that touches both messages and the strip.

### 3.2 Hook points in existing code

- **`composition.rs`** exposes `pub(crate) fn ensure_d2d_context(compositor: Compositor) -> anyhow::Result<Rc<D2dContext>>` backed by a thread-local `OnceCell<Rc<D2dContext>>`. Called once per Custom-titlebar window from inside `CaptionButtonStrip::new`; later calls return the same `Rc<D2dContext>`. Failure is not memoised — a later Custom-titlebar window retries through the same cell.
- **`Window`** gains `caption_buttons: RefCell<Option<CaptionButtonStrip>>`, populated in `initialize_window` if and only if `style.title_bar_kind == Custom` and caption-button construction succeeds. Construction failure before the window is shown is fatal for `window_create`; a `Custom` titlebar window must not appear without visible caption buttons. The strip owns the `Rc<D2dContext>` and the `RenderingDeviceReplacedRegistration` guard.
- **`Window`** gains `nc_leave_tracking_armed: AtomicBool` and helper methods `ensure_nc_leave_tracking()` / `nc_leave_tracking_fired()` — see §3.5.
- **`Window`** already has `minimize` / `maximize` / `restore`, routed through `SendMessageW(WM_SYSCOMMAND, SC_*)`, used by the strip click path.
- **`event_loop.rs`** gains:
  - Strip consultation in `on_nchittest` after the initial `DwmDefWindowProc` / `DefWindowProcW` consultation, but before preserving a default non-`HTCLIENT` result for points inside the strip.
  - Current `on_nchittest` returns early when `!window.is_resizable()`. Split that guard: resize-border math stays conditional on `is_resizable`, but custom caption-button hit-testing and draggable titlebar fallback must still run for non-resizable Custom-titlebar windows.
  - The existing pointer handlers (`on_pointerupdate`, `on_pointerdown`, `on_pointerup`) — which already merge client and non-client pointer messages via the `WM_POINTERUPDATE | WM_NCPOINTERUPDATE` / `_DOWN` / `_UP` arms in the wndproc dispatch — are extended to dispatch into the free function `caption_kind_at_screen(window, screen)`, which converts screen coordinates to strip-local coordinates and runs the strip's geometric hit-test. The documented [`WM_NCPOINTERDOWN`](https://learn.microsoft.com/windows/win32/inputmsg/wm-ncpointerdown) contract — `HIWORD(wParam)` carries the prior `WM_NCHITTEST` result — is unreliable on Windows 11: implementation-observed, the upper word arrives muxed with `POINTER_FLAG_*` bits and never matches `HTCLOSE` / `HTMAXBUTTON` / `HTMINBUTTON` for `WM_NCPOINTER*` messages. Geometric hit-testing in the strip is the workaround; an earlier draft of this spec mandated the `HIWORD(wParam)` route.
  - On `WM_NCPOINTERUPDATE` the dispatch returns `Some(LRESULT(0))` for caption-button hit-test areas to suppress Kotlin-facing pointer events for the toolkit-owned gesture; non-caption NC areas (e.g. title-bar drag region) still fall through to the existing dispatch. The client-variant `WM_POINTERUPDATE` is gated by `strip.has_press_for(pointer_id)` while the strip owns a press: implicit pointer capture can drift from non-client to client mid-press, and an unsuppressed UPDATE on that path lets the host's drag source observe a held primary button without the matching DOWN, kicking off `DoDragDrop`. While suppressing the host event, the wndproc still forwards `strip.on_pointer_update(caption_kind_at_screen(...), ...)` so the WinUI capture rule (`Pressed → PressedDraggedOff` on drift, reverse on return) fires for client-side movement.
  - Beyond the primary path, `on_pointerup` drains tracked `Suppressed` sessions via `strip.consume_swallowed_release(pointer_id, button)` — keyed by `(pointer_id, PointerButton)` so the drain works regardless of where the implicit pointer capture delivers the UP (`WM_NCPOINTERUP` off the strip, or `WM_POINTERUP` in the client area). Releases whose press began outside the strip fall through to Kotlin: chrome ownership scopes to cycles whose DOWN was on the strip.
  - `WM_POINTERCAPTURECHANGED` gets a cleanup-only arm. The handler matches every owned session — `Active` plus any `Suppressed` mode, via `strip.has_press_for(pointer_id)` — and calls `strip.on_pointer_cancel(id)` without firing a caption-button action. Microsoft documents [`WM_POINTERCAPTURECHANGED`](https://learn.microsoft.com/windows/win32/inputmsg/wm-pointercapturechanged) as the cleanup signal and warns not to depend on paired pointer notifications, but does not state that it is always delivered for implicit `WM_NCPOINTER*` capture loss — hence the defensive cleanup arms below.
  - `WM_CANCELMODE` and `WM_ACTIVATE`-deactivate call `strip.cancel_any_press()` (no `pointer_id` available) and return `None`. [`WM_CANCELMODE`](https://learn.microsoft.com/windows/win32/winmsg/wm-cancelmode) doc says `DefWindowProc` "cancels internal processing of standard scroll bar input, cancels internal menu processing, and releases the mouse capture" — letting it run preserves that. These arms backstop focus-loss paths Windows does not deliver `WM_POINTERCAPTURECHANGED` on (ALT-TAB, `EnableWindow(FALSE)`, RDP disconnect); without them a press session can stay live on return.
  - `TrackMouseEvent(TME_NONCLIENT | TME_LEAVE)` is armed at the window/event-loop layer when an NC pointer update enters the window non-client area, not when it enters a specific caption button.
  - Existing `on_ncmouseleave` extended to call `strip.on_nc_mouse_leave()` and `window.nc_leave_tracking_fired()` before the existing `DwmDefWindowProc` pass-through.
  - `on_dpichanged` and `on_settingchange` extended to call the strip's invalidation methods.
  - `on_windowposchanged` extended to call `strip.on_max_state_change(IsZoomed(hwnd).as_bool())` when the cached maximize state has changed. The toolkit's wndproc dispatch already routes `WM_WINDOWPOSCHANGED`, and `on_windowposchanged` currently returns `Some(LRESULT(0))`; the dispatch path therefore does not rely on downstream `DefWindowProc`-generated `WM_SIZE`. Do not introduce a new `WM_SIZE` arm. [`IsZoomed`](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-iszoomed) is the documented Win32 API for checking maximized state.
  - `on_nccalcsize` extended (a) to apply the maximized client-area inset described in §3.6 *before* the existing `NCCalcSizeEvent` is emitted, and (b) to call `strip.on_resize(client_size, max_chrome_y)` after the inset-aware client-rect calculation and after `resize_backdrop_tint`. The strip's in-method `Commit()` then publishes the backdrop's queued size and the strip's new offset together — a single coupled commit rather than two visible frames.

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

`Window::add_visual` is updated to insert new visuals into `content_layer` rather than the root. Current `renderer_angle.rs` calls `window.add_visual()` before `eglCreateWindowSurface(..., visual.as_raw(), ...)`, so the redirect changes the parent collection used before ANGLE creates the EGL surface — no re-parenting of an already EGL-targeted visual. Microsoft docs do not explicitly state whether ANGLE's EGL window-surface path supports a `SpriteVisual` nested below an intermediate `ContainerVisual`; verified empirically by running the Skiko sample after redirecting `add_visual`.

**Z-order construction.** The three layers must be inserted into `composition_root.Children()` in a deterministic order so that `chrome_layer` sits visually above `content_layer` above `backdrop_layer`. Microsoft documents `VisualCollection` ordering as bottom-to-top and `InsertAtTop` as inserting a visual at the top of that collection. The implementation calls `InsertAtTop` three times in the sequence `backdrop_layer`, `content_layer`, `chrome_layer`, so chrome is inserted last and is the topmost rendered layer. The existing `backdrop_visual` becomes a child of `backdrop_layer` rather than `composition_root` directly; ANGLE's existing `add_visual` path is updated to insert into `content_layer.Children()` so this layer's `InsertAtTop` semantics remain unchanged for callers.

### 3.4 Threading

Single UI thread, consistent with the rest of the crate. `D2dContext` and `CaptionButtonStrip` are not `Send`. The only `Send + 'static` boundary is the `RenderingDeviceReplaced` callback (§6.2), which posts a private `WM_APP_*` message rather than touching `D2dContext`, `Window`, or `CaptionButtonStrip` directly.

### 3.5 Pointer / leave message routing

The crate uses `EnableMouseInPointer(true)`. The existing wndproc dispatch already merges `WM_NCPOINTER*` with their client-area counterparts (`WM_POINTERUPDATE | WM_NCPOINTERUPDATE` etc.), so the `WM_NCPOINTER*` contract is taken as established and no `WM_NCMOUSEMOVE` / `WM_NCLBUTTON*` fallback is implemented.

[`WM_POINTERLEAVE`](https://learn.microsoft.com/windows/win32/inputmsg/wm-pointerleave) fires *"when a pointer moves outside the boundaries of the window"* — covering NC→outside transitions for hovering pointers. The existing `on_pointerleave` handler clears `is_pointer_in_window` on this message regardless of which area the pointer was over.

**`PointerEntered` parity for entries via the strip.** The existing `on_pointerupdate` handler fires `Event::PointerEntered` whenever `is_pointer_in_window` is false on entry — for both `WM_POINTERUPDATE` and `WM_NCPOINTERUPDATE`, so System-titlebar windows get `PointerEntered` even when the pointer first appears over the title-bar drag region. Caption-button suppression must preserve that parity: when the pointer's first appearance over a caption button transitions `is_pointer_in_window` from false to true, the wndproc fires a synthesised `PointerEntered` (built from `pointer_info` exactly as the existing else-branch does) before returning `Some(LRESULT(0))`. Strip-internal `PointerUpdated` / `PointerDown` / `PointerUp` events stay suppressed.

Consequence: `WM_NCPOINTERUPDATE` / `WM_NCPOINTERDOWN` / `WM_NCPOINTERUP` are the primary input path. `WM_NCMOUSELEAVE` covers the case `WM_POINTERLEAVE` does not — NC→client transitions, where the strip's hover state must clear as the pointer slides off a caption button into the title-bar drag area or further into the client. To receive `WM_NCMOUSELEAVE`, the application must arm tracking via `TrackMouseEvent(TME_NONCLIENT | TME_LEAVE, hwndTrack=hwnd, dwHoverTime=0)`. Per the `WM_NCMOUSELEAVE` Remarks: *"All tracking requested by TrackMouseEvent is canceled when this message is generated."* — re-arm on each NC entry to receive the next. `WM_NCMOUSEMOVE` is intentionally unhandled; see TODO `WM_NCMOUSEMOVE fallback if WM_NCPOINTER* is missing on a supported config`.

`WM_POINTERCAPTURECHANGED` is handled only as a cancellation/cleanup signal — see §3.2 for the wndproc arm. For a non-client press that keeps the OS-provided implicit capture, `WM_NCPOINTERUP` remains the documented release message.

Tracking armed-state lives on `Window` (one flag per HWND), not on the strip. The event-loop layer calls `ensure_nc_leave_tracking()` when an NC pointer update enters the non-client area of a Custom-titlebar window, whether or not the hit-test is over a caption button:

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

This keeps the strip a pure state machine with no OS-call dependencies.

### 3.6 Maximized-window client-area inset

When a `WS_THICKFRAME`-having window is maximized, Windows positions it so that its window rect extends past the monitor's work area on all four sides. The overhang exists so the resize border / drop shadow remains off-screen when the window is maximized; the system clips the window to the monitor edge for display.

For the toolkit's non-system titlebar paths (`WindowTitleBarKind::Custom` / `WindowTitleBarKind::None`), the existing `on_nccalcsize` handler leaves the top inset at 0 so the title-bar area is part of the client area. **In maximized state two effects compound: (a) the OS positions the window so that its top sits `max_chrome_y` pixels above the visible monitor; (b) per [Visual.Offset](https://learn.microsoft.com/uwp/api/windows.ui.composition.visual.offset), composition (0,0) tracks the HWND window-rect top-left, so the content layer (and, for `Custom`, the strip's `composition_root`) at composition Y=0 sits the same distance off-monitor.**

The fix has two pieces. (1) `WM_NCCALCSIZE` adjusts `rgrc[0].top += max_chrome_y` so the client rect's top sits at the monitor edge (snippet below). (2) The composition tree shifts in lockstep — `Window::set_content_top_offset` offsets the content layer for both `Custom` and `None`, and `set_strip_position` additionally offsets the strip's `composition_root` for `Custom` — both by the same `max_chrome_y`. The publish path differs: `Custom`'s strip bundles the commit into its `on_resize` / `on_max_state_change`; `None` has no strip, so the wndproc calls `Window::commit_composition` directly. For non-resizable maximized windows `max_chrome_y` is `0`, the inset is skipped, and both layers stay at composition Y=0 matching the OS-positioned window-rect top. The wndproc derives `max_chrome_y` from [`GetSystemMetricsForDpi`](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getsystemmetricsfordpi) and passes it per call, so resize-while-maximized events preserve the offset.

`max_chrome_y` is the per-monitor DPI-scaled `SM_CYSIZEFRAME` only. The original spec drafted it as `SM_CYSIZEFRAME + SM_CXPADDEDBORDER`, mirroring [Windows Terminal's `_OnNcCalcSize`](https://github.com/microsoft/terminal/blob/e4e3f08efca9d0ffba330eee12edbcb16897ddcb/src/cascadia/WindowsTerminal/NonClientIslandWindow.cpp). Manual verification on Windows 11 showed the padded border over-insets for this toolkit's non-system titlebar style (`WS_CAPTION` retained, `WS_SYSMENU` cleared — see §2 / §4.6 for the style policy): the title-bar area sat below the monitor edge by exactly `SM_CXPADDEDBORDER` pixels. The divergence from Terminal is consistent with the [`SM_CXPADDEDBORDER`](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getsystemmetrics) docs, which describe it as "border padding for captioned windows" — a metric whose semantics depend on which caption-style bits are set. Terminal retains `WS_SYSMENU`; the toolkit does not.

The inset only runs when `window.is_resizable()` (i.e., `WS_THICKFRAME` is present); non-resizable windows don't get the maximized off-monitor overhang from Windows, so applying the inset would over-clip them. `WindowStyle::to_system` already clears `WS_THICKFRAME` when `!is_resizable`; mirror the gate at the inset site.

```text
if (wParam == TRUE && window.is_resizable() && IsZoomed(hwnd)) {
    if (window.has_non_system_title_bar()) {
        UINT dpi = GetDpiForWindow(hwnd);
        int max_chrome_y = GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi);
        // The non-system-titlebar handler leaves the top inset at 0 so
        // the title-bar area stays in the client rect. The maximized
        // off-monitor overhang on top is added back here so the strip
        // (Custom) and content layer (Custom / None) sit at the visible
        // monitor edge.
        rgrc[0].top += max_chrome_y;
    }
    // Auto-hide-taskbar claw-back applies to maximized resizable
    // non-system-titlebar windows only — see Notes below.
}
```

Notes:

- Detection uses [`IsZoomed`](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-iszoomed). Implementation-observed: `WS_MAXIMIZE` is set before `WM_NCCALCSIZE(TRUE)` arrives during the maximize transition; Microsoft does not document this ordering, but Windows Terminal's `_OnNcCalcSize` relies on it.
- DPI must come from [`GetDpiForWindow`](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getdpiforwindow) and the metrics from `GetSystemMetricsForDpi`. The non-DPI-aware variants return wrong values on per-monitor v2 windows ([Mixed-Mode DPI Scaling](https://learn.microsoft.com/windows/win32/hidpi/high-dpi-improvements-for-desktop-applications#new-dpi-related-apis)).
- The top resize-band used by the strip's hit-test (§5.6) is a separate computation: `SM_CXPADDEDBORDER + SM_CYSIZEFRAME`. There is no `SM_CYPADDEDBORDER`; the same `SM_CXPADDEDBORDER` value is added to both axes. Verified in Windows Terminal's `_GetResizeHandleHeight` at commit `e4e3f08efca9d0ffba330eee12edbcb16897ddcb`, which carries this comment verbatim: *"there isn't a SM_CYPADDEDBORDER for the Y axis."*
- To preserve the cursor's ability to trigger an auto-hide taskbar (which requires reaching the actual screen edge), the toolkit probes for an auto-hide taskbar on each monitor edge via [`SHAppBarMessage(ABM_GETSTATE)`](https://learn.microsoft.com/windows/win32/api/shellapi/nf-shellapi-shappbarmessage) + [`SHAppBarMessage(ABM_GETAUTOHIDEBAREX)`](https://learn.microsoft.com/windows/win32/api/shellapi/nf-shellapi-shappbarmessage) and claws back 2 px on the matching edge. Matches Windows Terminal's `_OnNcCalcSize` GH#1438 / GH#5209 handling (`AutohideTaskbarSize = 2`, `microsoft/terminal` at `e4e3f08efca…`). Non-system titlebar path (`Custom` / `None`) only — standard-frame windows retain the standard frame inset, leaving an OS-recognized non-client edge that triggers reveal without intervention.

## 4. Components

### 4.1 `D2dContext`

```rust
pub(crate) struct D2dContext {
    // Survives device loss; DirectWrite has no D3D dependency.
    dwrite_factory: IDWriteFactory,
    // Same WinRT object across device loss; underlying rendering device
    // gets swapped via SetRenderingDevice.
    composition_graphics_device: CompositionGraphicsDevice,
}

impl D2dContext {
    pub fn new(compositor: Compositor) -> anyhow::Result<Self>;
    // Eagerly constructs the D3D11 / D2D devices, DirectWrite factory, and
    // CompositionGraphicsDevice. The `Rc<D2dContext>` singleton wrapping
    // happens once at the `composition::ensure_d2d_context` accessor (§3.2).

    pub fn create_drawing_surface(
        &self,
        size: SizeInt32,
    ) -> anyhow::Result<CompositionDrawingSurface>;

    pub fn with_d2d_render_target<R>(
        &self,
        surface: &CompositionDrawingSurface,
        body: impl FnOnce(&ID2D1RenderTarget, POINT) -> anyhow::Result<R>,
    ) -> anyhow::Result<Option<R>>;
    // Internally requests `BeginDraw::<ID2D1DeviceContext>` (the documented
    // IID per the Composition native-interop sample) then upcasts to
    // `&ID2D1RenderTarget` for the closure.
    // Ok(None) when device loss is detected on either BeginDraw or EndDraw —
    // caller skips this frame. The trap matches three HRESULTs:
    // DXGI_ERROR_DEVICE_REMOVED, DXGI_ERROR_DEVICE_RESET, D2DERR_RECREATE_TARGET
    // (see §6.2). Callers must preserve dirty state when Ok(None) is returned;
    // no pixels were drawn. Other errors propagate as anyhow::Error.

    /// Cheap WinRT smart-pointer clone — the factory is a plain field, so this is infallible.
    pub fn dwrite_factory(&self) -> IDWriteFactory;

    pub fn add_rendering_device_replaced_callback<F>(
        &self,
        cb: F,
    ) -> anyhow::Result<RenderingDeviceReplacedRegistration>
    where
        F: Fn() + Send + 'static;
}
```

The strip holds `Rc<D2dContext>` for D2D / DirectWrite plus a `CompositorController` clone (cheap WinRT smart-pointer copy) sourced from `Window` — the controller's `Compositor()` accessor satisfies visual creation, and its `Commit()` method publishes frames. `composition::ensure_d2d_context` (§3.2) is the only entry point that constructs a `D2dContext`.

`add_rendering_device_replaced_callback` returns `anyhow::Result<RenderingDeviceReplacedRegistration>`; on success, the RAII guard stores a clone of the `CompositionGraphicsDevice` and the `i64` token returned by `windows = 0.62.2`; dropping it calls `RemoveRenderingDeviceReplaced`. The callback is `Send` and must only post a UI-thread command. It must not capture `Rc`, `RefCell`, or a strip directly.

Device loss is reactive-only — see §6.2 for the recovery sequence and `TODO.md` *Caption-button proactive device-loss detection* for the deferred proactive path.

### 4.2 `CaptionButtonStrip`

```rust
pub(crate) struct CaptionButtonStrip {
    composition_root: ContainerVisual,                     // child of window's chrome_layer
    buttons: Vec<CaptionButton>,                           // ordered by visual position
    visible_kinds: CaptionButtonKinds,                     // Close always; Min/Max per table below
    is_active: bool,
    is_window_maximized: bool,                             // for action_for(Maximize) + Maximize-glyph swap; layout uses live values from wndproc
    appearance: Appearance,
    high_contrast: HighContrast,                           // from appearance.rs
    metrics: CaptionButtonMetrics,                         // resolved from DPI
    pointer_over_kind: Option<CaptionButtonKind>,
    pointer_device:    Option<PointerDeviceKind>,
    press_session:     Option<PressSession>,
    top_offset_px: i32,                                    // strip Y-shift cached from on_nccalcsize for hit-test (§3.6 / §5.6)
    d2d_context: Rc<D2dContext>,                           // clone of the singleton from `composition::ensure_d2d_context`
    device_replaced_registration: RenderingDeviceReplacedRegistration,  // dropping it removes the RDR subscription (§6.2)
    compositor_controller: CompositorController,           // visual creation: Compositor() accessor; frame commit: Commit()
}

impl CaptionButtonStrip {
    // Constructor takes only what the strip needs — not &Window — to preserve
    // the §4.6 boundary that the strip is independent of Window's wider state.
    pub fn new(
        chrome_layer: &ContainerVisual,    // strip's parent visual; created by the caller
        initial_scale: f32,                // DPI scale for initial metrics
        style: &WindowStyle,               // caption-button visibility / availability
        compositor_controller: CompositorController,  // cloned from Window; provides both `Compositor()` and `Commit()`
        hwnd: HWND,                        // for the strip's WM_APP redraw post on `RenderingDeviceReplaced`
    ) -> anyhow::Result<Self>;
    // `is_active` and `is_window_maximized` seed to `false`; the first
    // `WM_ACTIVATE` / `WM_WINDOWPOSCHANGED` after `ShowWindow` update them.

    // Hit-testing — `caption_kind_at_screen` (caption_buttons.rs) converts the
    // screen point to client-space and dispatches here with the client point and
    // client-rect width; the strip computes strip-local geometry from there.
    // Custom strip hit-testing runs before preserving the default non-client
    // result outside the strip.
    pub fn hit_test(&self, client_point: PhysicalPoint, client_width: PhysicalPixels) -> Option<CaptionButtonKind>;
    /// Used by `on_nchittest` to gate `HTMINBUTTON` / `HTMAXBUTTON` for enabled
    /// buttons; visible disabled Min/Max collapse to `HTCAPTION` (see §5.6 step 2).
    pub fn is_enabled(&self, kind: CaptionButtonKind) -> bool;

    // Pointer routing — called from event_loop's WM_NCPOINTER* handlers and
    // capture-loss cleanup path.
    pub fn on_pointer_update(
        &mut self, kind: Option<CaptionButtonKind>, pointer_id: u32, device: PointerDeviceKind,
    ) -> anyhow::Result<()>;
    pub fn on_pointer_down(
        &mut self,
        kind: CaptionButtonKind,
        pointer_id: u32,
        device: PointerDeviceKind,
    ) -> anyhow::Result<()>;
    pub fn on_pointer_up(
        &mut self,
        kind_under_pointer: Option<CaptionButtonKind>,
        pointer_id: u32,
    ) -> anyhow::Result<Option<CaptionButtonAction>>;
    /// Per-pointer cancellation. Used on `WM_POINTERCAPTURECHANGED` for the
    /// pointer id Windows reports.
    pub fn on_pointer_cancel(&mut self, pointer_id: u32) -> anyhow::Result<()>;
    /// Unkeyed cancellation. Used on `WM_CANCELMODE` and `WM_ACTIVATE`-deactivate,
    /// neither of which carries a pointer id.
    pub fn cancel_any_press(&mut self) -> anyhow::Result<()>;

    // Press-session helpers (wndproc-level swallow path).
    pub(crate) const fn track_swallowed_press(
        &mut self,
        kind: CaptionButtonKind,
        pointer_id: u32,
        button: PointerButton,
    );
    pub(crate) fn consume_swallowed_release(&mut self, pointer_id: u32, button: PointerButton) -> bool;
    /// Matches the primary release path only — `Active` plus `Suppressed { held_button: Left }`.
    /// Used to gate `on_pointer_up` dispatch on the primary release branch.
    pub(crate) const fn has_active_press_for(&self, pointer_id: u32) -> bool;
    /// Matches every owned session — `Active` and every `Suppressed` mode.
    /// Used by the `WM_POINTERUPDATE` suppression gate (§3.2) and the
    /// `WM_POINTERCAPTURECHANGED` cleanup arm.
    pub(crate) const fn has_press_for(&self, pointer_id: u32) -> bool;

    pub fn on_nc_mouse_leave(&mut self) -> anyhow::Result<()>;

    // Theming / metric / state changes.
    pub fn on_activate(&mut self, is_active: bool) -> anyhow::Result<()>;
    pub fn on_dpi_change(&mut self, new_scale: f32) -> anyhow::Result<()>;
    pub fn on_appearance_change(
        &mut self, appearance: Appearance, high_contrast: HighContrast,
    ) -> anyhow::Result<()>;
    pub fn on_rendering_device_replaced(&mut self) -> anyhow::Result<()>;
    pub fn on_max_state_change(&mut self, is_maximized: bool) -> anyhow::Result<()>;
    pub fn on_resize(&mut self, client_size: PhysicalSize, max_chrome_y: i32) -> anyhow::Result<()>;
}

// The wndproc filters primary at the call site, but suppression is
// broader than activation. Two press modes drive the strip's existing
// `Option<PressSession>`:
//   - `PressSessionMode::Active` — primary press on enabled button.
//     Visual capture engaged; matched primary UP dispatches a
//     `CaptionButtonAction` if released over the same button. Driven
//     by `on_pointer_down`.
//   - `PressSessionMode::Suppressed { held_button }` — wndproc-level
//     swallow. No visual capture. `held_button = Left` for primary-
//     on-disabled (paired with the existing primary-release branch
//     via the tightened `has_active_press_for`); `held_button = Right
//     / Middle / XButton1 / XButton2` for non-primary presses (paired
//     with the new `consume_swallowed_release` drain). Driven by
//     `track_swallowed_press` for non-primary, by `on_pointer_down`
//     for primary-on-disabled.
//
// The `Option<PressSession>` invariant is preserved — concurrent
// multi-button or multi-pointer presses are dropped (matching the
// strip's existing single-press limitation). The dropped DOWN's
// matching UP will leak to Kotlin; this is a known limitation, not
// addressed here.

#[derive(Debug, Clone, Copy)]
pub(crate) enum CaptionButtonAction {
    Close,
    Minimize,
    Maximize,
    Restore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub(crate) enum CaptionButtonKind {
    // Discriminants are load-bearing: `CaptionButtonKinds` (in `caption_buttons.rs`)
    // uses `1 << kind as u8` as a bitmask. Reordering the variants silently
    // breaks every `CaptionButtonKinds::contains` / `iter_ordered` consumer.
    Minimize = 0,
    Maximize = 1,
    Close = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PointerDeviceKind { Mouse, Pen, Touch }

enum PressSessionMode {
    Active,                                      // primary press on enabled button; held button implicit (`Left`)
    Suppressed { held_button: PointerButton },   // wndproc swallow; `Left` for primary-on-disabled, `Right`/`Middle`/`XButton1`/`XButton2` for non-primary
}

struct PressSession {
    pointer_id: u32,
    captured_kind: CaptionButtonKind,
    mode: PressSessionMode,
}
```

The strip never invokes `Window::request_close` and friends directly — it returns `CaptionButtonAction` and the wndproc dispatches. This keeps the strip independent of `Window`'s broader state and unit-testable in isolation.

`CaptionButtonStrip::new` seeds `appearance` and `high_contrast` from `Appearance::get_current()` and `HighContrast::get_current()` so first paint matches the live system theme. Both queries are non-fatal: a failed query logs at warning level and falls back to `Appearance::Light` / `HighContrast::Off`, and the strip self-corrects on the next appearance / high-contrast event.

Minimize / Maximize follow Windows' paired-button behaviour per archived Microsoft KB Q130760: when neither style bit is present neither box appears; with one bit present both boxes appear and the absent one is disabled. The current *Window Styles* page documents the controlling style bits (`WS_MINIMIZEBOX`, `WS_MAXIMIZEBOX`, `WS_SYSMENU`) but not the paired-button outcome — treat it as **implementation-observed** (verified on Windows 11 against `WM_GETTITLEBARINFOEX`: hidden buttons surface as `STATE_SYSTEM_INVISIBLE`, disabled as `STATE_SYSTEM_UNAVAILABLE`).

Maximize requires `is_maximizable`. `Window::maximize()` is a no-op when `!is_maximizable`; `is_resizable` controls resize affordances and the maximized overhang inset, not Maximize button actionability.

| `WindowStyle` flags | Minimize button | Maximize button |
|---|---|---|
| `!is_minimizable && !is_maximizable` | hidden | hidden |
| `is_minimizable && !is_maximizable` | visible, enabled | visible, disabled |
| `!is_minimizable && is_maximizable` | visible, disabled | visible, enabled |
| `is_minimizable && is_maximizable` | visible, enabled | visible, enabled |

`Close` is always visible and enabled. Close-disable support is deferred because Win32 uses the system-menu `SC_CLOSE` state rather than a Min/Max-style window bit; see [TODO: Win32 Close-button disable support](../TODO.md#win32-close-button-disable-support).

For Minimize / Maximize, visibility and actionability are separate: if exactly one of `is_minimizable` / `is_maximizable` is true, both occupy strip geometry and the false one renders Disabled. `TITLEBARINFOEX` documents the corresponding accessibility states (index 2 Minimize, index 3 Maximize, `STATE_SYSTEM_UNAVAILABLE` / `STATE_SYSTEM_INVISIBLE`).

Disabled Minimize / Maximize buttons are suppressed from action and interaction, not from layout: they never enter Hovered / Pressed, `on_pointer_down` records a `Suppressed { held_button: Left }` primary session for them, and `on_pointer_up` never returns an action for them. Do not collapse disabled buttons out of the strip; collapse only when both Minimize and Maximize are hidden.

**Primary-button-only activation; full-press suppression.** The strip *acts* only on the primary action of a pointer ([`POINTER_CHANGE_FIRSTBUTTON_DOWN`](https://learn.microsoft.com/windows/win32/api/winuser/ne-winuser-pointer_button_change_type) / `..._UP` for mouse / pen, contact for touch). The wndproc computes `is_primary` from `POINTER_INFO.ButtonChangeType` and only invokes `strip.on_pointer_down` / `strip.on_pointer_up` for primary events. *Suppression* is broader. Acceptance criteria, by case:

| Button | Press location | Release location | Activation? | Kotlin passthrough? |
|---|---|---|---|---|
| Primary | Enabled strip button | Same enabled button | yes | no |
| Primary | Enabled strip button | Elsewhere (drag-off) | no | no |
| Primary | Elsewhere | Any enabled strip button | no | yes (UP only) |
| Primary | Disabled strip button | Anywhere | no | no |
| Non-primary | Any strip button | Anywhere | no | no |
| Non-primary | Elsewhere | Any strip button | no | yes (DOWN and UP both) |

These outcomes are produced by two wndproc branches (the primary-action path and `consume_swallowed_release`) plus the implicit fallthrough — case (4) needs no branch because chrome doesn't intervene when the press wasn't on the strip.

(1) **Primary-on-enabled.** `on_pointer_down` records `mode = Active`. The wndproc's `is_primary && strip_owns_press` branch (gated by `has_active_press_for`, now matching `Active | Suppressed { held_button: PointerButton::Left }`) invokes `on_pointer_up` — which dispatches a `CaptionButtonAction` if released over the same button.

(2) **Primary-on-disabled.** `on_pointer_down` records `mode = Suppressed { held_button: Left }`. The same `has_active_press_for` branch matches, so the same wndproc path runs; `on_pointer_up` returns `None` silently and no action fires.

(3) **Non-primary press over the strip, release anywhere.** The wndproc calls `strip.track_swallowed_press(kind, pointer_id, button)` to record `mode = Suppressed { held_button: button }`. The matching UP is drained by `strip.consume_swallowed_release(pointer_id, button)` — keyed on `(pointer_id, button)` rather than hit-test, so it works regardless of where the runtime's implicit pointer capture routes the UP.

(4) **Press elsewhere, release over the strip (any button).** No session was recorded; `consume_swallowed_release` returns false; the bottom-of-function dispatch emits the `PointerUp` event. Chrome ownership scopes to cycles whose DOWN was on the strip — applies equally to primary and non-primary.

`WM_POINTERCAPTURECHANGED` requires no `event_loop.rs` change: the existing call to `strip.on_pointer_cancel(pointer_id)` already drops any session for the cancelled pointer regardless of mode. The wndproc also short-circuits `on_pointerupdate`'s Kotlin-event dispatch for caption-button hit-test areas regardless of pointer state, so `Event::PointerUpdated` / `PointerEntered` / mid-contact button-change events never surface for the strip's region. The strip owns its hit-test rectangle end-to-end: Kotlin never sees `PointerDown` / `PointerUp` / `PointerUpdated` with `non_client_area = true` for points inside the strip, nor any release whose press began there. Right-click on a caption button remains an unhandled gesture — that surface is owned by the system-menu work tracked separately in `TODO.md`. The single-`Option<PressSession>` design retains the strip's existing single-press-at-a-time limitation: concurrent multi-button or multi-pointer presses (e.g. holding right while pressing left, or a touch + mouse hold) drop the second DOWN; its matching UP will leak.

Disabled visible Minimize / Maximize map to `HTCAPTION`. This matches Win32's native default: `DefWindowProcW(WM_NCHITTEST)` returns `HTCAPTION` for native Min/Max button rectangles (including the *enabled* ones in `min && max`), and `DwmDefWindowProc(WM_NCHITTEST)` does not handle these hit-tests — implementation-observed on Windows 11. The strip therefore must return `HTMINBUTTON` / `HTMAXBUTTON` for *enabled* buttons (Snap Layouts requires `HTMAXBUTTON` per the doc) and `HTCAPTION` for disabled ones. The strip still owns the pointer cycle for presses that begin on a disabled visible button: DOWN / UP are swallowed by the `Suppressed` session above, no hover or press visuals appear, and no action fires. Manual acceptance verifies disabled Min/Max fire no actions and disabled Maximize shows no Snap Layouts.

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
    backplate_brush: CompositionColorBrush,              // mutated on hover/press/active state
    glyph: SpriteVisual,
    glyph_brush: CompositionColorBrush,                  // mutated on theme/state — Source of the (unstored) CompositionMaskBrush
    glyph_surface: CompositionDrawingSurface,            // resized on DPI / re-rasterised on theme/HC/max-state
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Availability { Enabled, Disabled }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonInteraction {
    Idle,
    Hovered,                         // pointer over me, no press anywhere; mouse / pen only
    Pressed,                         // capture is mine and pointer is over me
    PressedDraggedOff,               // capture is mine and pointer left
}
```

The `glyph_surface` is rasterised in alpha-only / monochrome form (white pixels via D2D `DrawText` for maximum mask amplitude). The actual colour comes from the `glyph_brush` (a `CompositionColorBrush`) used as the **`Source`** of a [`CompositionMaskBrush`](https://learn.microsoft.com/uwp/api/windows.ui.composition.compositionmaskbrush) whose **`Mask`** is a `CompositionSurfaceBrush` wrapping the `glyph_surface`; the glyph `SpriteVisual` is set to the `CompositionMaskBrush`. State transitions only mutate the `glyph_brush`'s colour; glyph re-rasterisation happens only on DPI / theme / high-contrast / max-state changes.

The intermediate `CompositionSurfaceBrush` / `CompositionMaskBrush` aren't stored — `SpriteVisual.SetBrush(&mask_brush)` keeps the chain alive. `CaptionButtonVisuals` keeps `glyph_brush` (for `SetColor` on state change) and `glyph_surface` (for `Resize` + `BeginDraw`). Surface format is BGRA8 premultiplied; `DXGI_FORMAT_A8_UNORM` is a future optimisation.

### 4.4 `CaptionTheme` (private)

```rust
struct CaptionTheme {
    backplate_rest: windows::UI::Color,
    backplate_hover: windows::UI::Color,
    backplate_pressed: windows::UI::Color,
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
    fn resolve(appearance: Appearance, high_contrast: HighContrast) -> Self;
}
```

Construction strategy:

- **`HighContrast::On` branch.** Reads from `GetSysColor`. The slot mapping is sourced directly from Terminal's HighContrast block:
  - `COLOR_BTNFACE` (background) and `COLOR_BTNTEXT` (foreground) for rest state — **directly cited** in the *High contrast parameter* doc: *"Use the `GetSysColor` function to determine the appropriate foreground and background colors, using either a combination of `COLOR_WINDOWTEXT` and `COLOR_WINDOW` or a combination of `COLOR_BTNTEXT` and `COLOR_BTNFACE`."*
  - `COLOR_GRAYTEXT` for disabled foreground — documented in the *Button Messages* doc as *"Disabled (gray) text in buttons."* General-purpose, applicable to caption buttons by analogy.
  - `COLOR_HIGHLIGHT` (background) and `COLOR_HIGHLIGHTTEXT` (foreground) for hover and pressed states — sourced from `microsoft/terminal/src/cascadia/TerminalApp/MinMaxCloseControl.xaml@e4e3f08efca…` HighContrast block lines 102–105, which use `SystemColorHighlightColor` / `SystemColorHighlightTextColor` (the Win32 `GetSysColor` equivalents are `COLOR_HIGHLIGHT` / `COLOR_HIGHLIGHTTEXT`). Supporting evidence: *Compatibility/High-contrast mode* doc says *"`COLOR_HIGHLIGHTTEXT` is meant to be used with `COLOR_HIGHLIGHT` as a background"*.
- **`HighContrast::Off` branch.** Hard-coded table sourced from WinUI's [`microsoft-ui-xaml/controls/dev/CommonStyles/Common_themeresources_any.xaml`](https://github.com/microsoft/microsoft-ui-xaml/blob/main/src/controls/dev/CommonStyles/Common_themeresources_any.xaml) Fluent palette (`TextFillColor*` and `SubtleFillColor*` series), per the project's WinUI-first directive. State mapping: `TextFillColorPrimary` for rest+hover, `TextFillColorSecondary` for pressed, `TextFillColorDisabled` for disabled, `TextFillColorTertiary` for inactive (matching `TitleBarDeactivatedForegroundBrush` in microsoft-ui-xaml's `TitleBar` theme resources), `SubtleFillColorSecondary` for backplate hover, `SubtleFillColorTertiary` for backplate pressed. Each colour gets a one-line comment naming its WinUI source.
- **Inactive modulation is `Idle` / `PressedDraggedOff` only.** Hover and Pressed render with the active palette regardless of window activation, matching Terminal's [`MinMaxCloseControl`](https://github.com/microsoft/terminal/blob/e4e3f08efca9d0ffba330eee12edbcb16897ddcb/src/cascadia/TerminalApp/MinMaxCloseControl.xaml) `VisualState` last-set-wins behaviour and WinUI 3's `TitleBar` control. Earlier drafts of this spec early-returned the inactive palette before checking interaction state, suppressing hover and pressed on inactive windows; that behaviour did not match either reference and was corrected. Close hover/pressed reds apply regardless of activation.
- **Close-specific override** for hover and pressed: WinUI's "close button uses system red regardless of theme" rule (*Title bar customization* doc) replicated by sourcing literal values from Windows Terminal's `MinMaxCloseControl.xaml`. Hover backplate is `#C42B1C` opaque and hover foreground is `White` opaque; pressed backplate is `#C42B1C` with `Opacity 0.9` and pressed foreground is `White` with `Opacity 0.7`. Terminal is a Win32 host that doesn't depend on `AppWindowTitleBar`.

Use `windows::UI::Color { A, R, G, B }` directly — matching the existing `Window::set_backdrop_tint` precedent. `D2D1_COLOR_F` conversion happens only at draw time, never stored.

### 4.5 `CaptionButtonMetrics` (private)

```rust
#[derive(Debug, Clone, Copy)]
struct CaptionButtonMetrics {
    button_size_px: PhysicalSize,    // 46 × 32 epx → physical via DPI
    glyph_extent_px: PhysicalSize,   // 10 × 10 epx (the bounding box of the rendered glyph)
}
```

Storage uses the toolkit's `geometry::PhysicalSize` so `CaptionButtonMetrics::new` can reuse `LogicalSize::to_physical` for DPI rounding.

Values sourced from `microsoft/terminal:src/cascadia/TerminalApp/MinMaxCloseControl.xaml`:

| Dimension | Value | Source |
|---|---|---|
| Width | 46.0 epx | `Width="46.0"` |
| Height | 32.0 epx | WinUI `PreferredHeightOption.Standard` — File Explorer / Notepad / Settings default. Terminal opts into Tall (40 / 32) — see TODO 'Tall-mode title bars'. |
| Glyph rendered extent | 10 × 10 epx | Terminal's XAML wraps each glyph in a `Viewbox Width="10" Height="10"`; FontSize is unset and the Viewbox scales the glyph to that area. |
| Glyph font family | indirected via `SymbolThemeFontFamily` ThemeResource | Terminal uses `FontFamily="{ThemeResource SymbolThemeFontFamily}"`, which on modern Windows resolves to Segoe Fluent Icons with Segoe MDL2 Assets fallback. For DirectWrite, detect the system font collection at runtime: request `Segoe Fluent Icons` when DirectWrite reports it is present, otherwise use `Segoe MDL2 Assets`. |

The 10 × 10 epx is a **rendered extent**, not a font point size. With DirectWrite, we don't have a `Viewbox`; we instead pick a font size that produces a glyph whose visible black-box matches the 10 epx target, computed deterministically from design-unit metrics:

- Resolve glyph indices for the caption code points via [`IDWriteFontFace::GetGlyphIndices`](https://learn.microsoft.com/windows/win32/api/dwrite/nf-dwrite-idwritefontface-getglyphindices). The standard set is U+E921 (`ChromeMinimize`), U+E922 (`ChromeMaximize`), U+E923 (`ChromeRestore`), U+E8BB (`ChromeClose`); the high-contrast variants are U+EF2D (`ChromeMinimizeContrast`), U+EF2E (`ChromeMaximizeContrast`), U+EF2F (`ChromeRestoreContrast`), U+EF2C (`ChromeCloseContrast`) — all eight are listed on the [Segoe Fluent Icons](https://learn.microsoft.com/windows/apps/design/iconography/segoe-fluent-icons-font) reference page.
- Read the bbox via [`GetDesignGlyphMetrics`](https://learn.microsoft.com/windows/win32/api/dwrite/nf-dwrite-idwritefontface-getdesignglyphmetrics): `bbox.width = advanceWidth − leftSideBearing − rightSideBearing`. For height, use the **horizontal-layout** cell `(ascent + descent)` from [`GetMetrics`](https://learn.microsoft.com/windows/win32/api/dwrite/nf-dwrite-idwritefontface-getmetrics) and subtract the bearings: `bbox.height = (ascent + descent) − topSideBearing − bottomSideBearing`. `advanceHeight` is the *vertical*-layout advance per [`DWRITE_GLYPH_METRICS`](https://learn.microsoft.com/windows/win32/api/dwrite/ns-dwrite-dwrite_glyph_metrics) and only happens to equal `ascent + descent` for em-square icon fonts. Read `designUnitsPerEm` from `GetMetrics` for the per-axis fit formula below.
- Per-axis fit: `font_size_x = target_x * designUnitsPerEm / bbox.width`, similarly for Y; pick `min(font_size_x, font_size_y)`. The per-axis-min form is required for non-square targets; the square 10×10 target collapses to `target * designUnitsPerEm / max(bbox.width, bbox.height)`. The four caption glyphs in Segoe Fluent Icons share the same bbox in practice, so one `font_size_dip` is reused.
- Recompute `font_size_dip` whenever the font changes (Fluent ↔ MDL2 fallback, HC variants) — the bbox-to-em ratio differs between glyphs.

The actual draw call uses `ID2D1RenderTarget::DrawText` with an `IDWriteTextFormat` constructed at the computed `font_size_dip`.

### 4.6 Boundaries

- `D2dContext` knows nothing about caption buttons in its method signatures, but is private to caption-button rasterisation; if a future Composition+D2D consumer materialises in the crate, lift `D2dContext` to a shared module then.
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

The `Some(_) => Idle` branch is the WinUI capture-derived rule: when a press is owned by button A, button B does not enter Hovered state when the pointer moves over it.[^winui-capture]

Touch devices skip the Hovered state because touch input has no hover phase — `Idle → Pressed → action → Idle` directly.

### 5.2 Animation contract

Sourced from `microsoft/terminal:src/cascadia/TerminalApp/MinMaxCloseControl.xaml` `CommonStates` `VisualStateGroup`:

- Only the `PointerOver → Normal` and `PointerOver → Unfocused` transitions animate. Every other transition jumps via `Setter`.
- Backplate (`ButtonBaseElement.Background.Color`): **150 ms**.
- Glyph foreground (`ButtonIcon.Foreground.Color`): **100 ms**.
- No `EasingFunction` element is present in the storyboards. Our `ColorKeyFrameAnimation` calls match by passing no `CompositionEasingFunction` — `InsertKeyFrame(Single, Color)` interpolates linearly between keyframes in the default RGB color space.

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

**Animation restart race.** A defensive `InsertKeyFrame(0.0, brush.Color()?)` precedes `InsertKeyFrame(1.0, target)` so the animation always interpolates from a known start value. If a previous animation on the same brush is still mid-flight (rapid Hover→Idle→Hover→Idle), `brush.Color()?` returns the most-recently-set committed value, not the currently-rendered interpolated value, so the new animation snaps to that committed value before fading. This matches Terminal's behaviour; a fully correct fix requires `TryGetAnimationController` to read live progress, which is out of scope.

### 5.3 Inputs and what they trigger

| Input | Strip method | Cost |
|---|---|---|
| `WM_NCHITTEST` | `hit_test(point)` | cheap (geometry math) |
| `WM_NCPOINTERUPDATE` entering/staying in the window NC area | window arms NC leave tracking; strip receives `on_pointer_update(Some(kind), id, device)` over an enabled button or `on_pointer_update(None, id, device)` elsewhere | cheap (state + brush) |
| `WM_NCPOINTERDOWN` over an enabled button (primary) | `on_pointer_down(kind, id, device)` | cheap |
| `WM_NCPOINTERUP` or `WM_POINTERUP` after a caption-button press | `on_pointer_up(Some(kind) / None, id) -> Option<CaptionButtonAction>`; action fires when release is over the captured enabled button. UPs whose DOWN was on the strip are consumed (`Active` and `Suppressed`-`Left` via the primary branch, non-primary via `consume_swallowed_release`); releases whose press began outside the strip fall through to Kotlin as `PointerUp` events. | cheap |
| `WM_POINTERCAPTURECHANGED` | `on_pointer_cancel(id)` for any session whose `pointer_id` matches (gated by `has_press_for`, covering `Active` and every `Suppressed` mode); cleanup only, no action | cheap |
| `WM_CANCELMODE` | `cancel_any_press()`; returns `None` so `DefWindowProc` performs the standard cancel (mouse capture, scroll/menu tracking) | cheap |
| `WM_NCMOUSELEAVE` | `on_nc_mouse_leave()` (also: window clears tracking armed) | cheap |
| `WM_NCCALCSIZE` (after client-rect calc) | `Window::set_content_top_offset(max_chrome_y)` for any non-system titlebar; `Custom` then calls `on_resize(client_size, max_chrome_y)`, `None` calls `Window::commit_composition` | cheap (`SetOffset` per button + strip-position set + content-layer offset; single commit per kind) |
| `WM_DPICHANGED` | `on_dpi_change(new_scale)` re-rasterises glyphs at the new scale, then `on_resize(client_size, max_chrome_y_for_dpi(new_dpi))` re-anchors the strip. `new_dpi` comes from `wParam` (`GetDpiForWindow` may still report the old DPI); `client_size` from a fresh `GetClientRect` after the OS-suggested resize | **expensive** (re-rasterise all glyph surfaces, two commits) |
| `Appearance` event | `on_appearance_change(appearance, hc)` | **expensive iff foreground-rest colour changed** |
| `HighContrast` event | same as above with new `hc` | **expensive** (glyph code points swap E921↔EF2D etc.) |
| `WM_ACTIVATE` | `on_activate(is_active)`; on deactivate, also `cancel_any_press()` to backstop focus-loss paths Windows does not deliver `WM_POINTERCAPTURECHANGED` on (ALT-TAB, `EnableWindow(FALSE)`, RDP) | cheap (theme re-resolve, brush updates) |
| `WM_WINDOWPOSCHANGED` | `Window::set_content_top_offset(max_chrome_y)` for any non-system titlebar; `Custom` then calls `on_max_state_change(window.is_maximized())`, `None` calls `Window::commit_composition` | **expensive for Maximize button only** (E922 ↔ E923) on actual transitions; strip + content-layer Y-shift published by `on_max_state_change`'s commit (Custom) or `commit_composition` (None) |

The `WM_WINDOWPOSCHANGED` row covers both user-driven (button click) and programmatic (`Window::maximize` / `Window::restore`) maximize-state transitions.

"Expensive" rows go through `D2dContext::with_d2d_render_target` and a full glyph redraw on the affected button's surface. All others are O(1) brush colour updates.

### 5.4 Hit-test → action mapping

The wndproc routes pointer-up to the strip; the strip returns `Option<CaptionButtonAction>`; the wndproc dispatches:

| Action | Wndproc dispatch |
|---|---|
| `Close` | `Window::request_close()` (existing — `PostMessage WM_CLOSE`) |
| `Minimize` | `Window::minimize()` (existing — `SendMessageW(WM_SYSCOMMAND, SC_MINIMIZE, ...)`) |
| `Maximize` | `Window::maximize()` (existing — `SendMessageW(WM_SYSCOMMAND, SC_MAXIMIZE, ...)`) |
| `Restore` | `Window::restore()` (existing — `SendMessageW(WM_SYSCOMMAND, SC_RESTORE, ...)`) |

The Maximize button's action depends on current state at click time: the strip reads its own `is_window_maximized` field (kept in sync via `on_max_state_change` — see §5.3) and returns `Restore` if maximized, `Maximize` otherwise. The strip does not call back into `Window` or `IsZoomed` to resolve the action.

### 5.5 Frame commit coupling

The toolkit uses `CompositorController` so all Composition state changes flush under explicit application control ([`CompositorController` Remarks](https://learn.microsoft.com/uwp/api/windows.ui.composition.core.compositorcontroller)). The strip clones the controller from `Window` at construction.

Established toolkit pattern (verified in tree):

- `renderer_angle::swap_buffers` calls `self.compositor_controller.Commit()` after `eglSwapBuffers`. Commit is therefore tied to ANGLE buffer swap; it does **not** run on a timer or per-frame on its own.
- `Window::set_backdrop_tint` and `Window::remove_backdrop_tint` both call `self.compositor_controller.Commit()` directly because backdrop changes are not driven by ANGLE buffer swap.

Each public strip method updates visuals and calls `self.compositor_controller.Commit()` once at the end. The wndproc dispatches one strip method per Win32 message.

Re-rasterisation through `with_d2d_render_target` (DPI / theme / appearance / device-replaced paths) issues `EndDraw` per surface; the same in-method commit then publishes the new surface contents.

The strip's constructor commits once before returning so the buttons are visible the moment the window is shown.

### 5.6 New on_nchittest order

```
1. Run the current `DwmDefWindowProc(WM_NCHITTEST, ...)` / `DefWindowProcW` fallback path and record the result as `original_ht`. Do not return it yet for Custom-titlebar windows.
2. Dispatch into the strip via `caption_kind_at_screen(window, screen)`. The helper converts the screen point to client-space physical coordinates with `ScreenToClient` (or `MapWindowPoints` if mirroring support is added), compares against `GetClientRect`, derives strip-local coordinates from the actual client width and strip width, and runs the strip's geometric hit-test. **On a restored, resizable window `caption_kind_at_screen` first short-circuits to `None` over the top `SM_CXPADDEDBORDER + SM_CYSIZEFRAME` resize-border band, so step 3 falls through to `DefWindowProcW`'s `HTTOP` claim — the OS resize cursor and drag-resize work across the full strip width. Maximized or non-resizable windows skip this short-circuit because the OS does not deliver an off-screen resize border there.** Return `HTCLOSE` for Close, `HTMAXBUTTON` / `HTMINBUTTON` only for enabled Maximize / Minimize, and `HTCAPTION` for visible disabled Maximize / Minimize.
3. If the strip did not claim the point and `original_ht != HTCLIENT`, return `original_ht`.
4. Fire the existing Kotlin NCHitTestEvent callback so the app can carve interactive
   sub-regions (search box in title bar, etc.). If the callback handles the point,
   return HTCLIENT as the current code does.
5. If the callback did not handle the point, run the existing manual top-edge math (`resize_handle_height` / `title_bar_height`)
   to convert points near the top edge into HTTOP (resize) or HTCAPTION
   (drag region). This step is preserved from the current code, not removed.
6. Otherwise return HTCLIENT.
```

The manual top-edge math from the *Custom Window Frame Using DWM* recipe is preserved (`DwmDefWindowProc` / `DefWindowProcW` does not return `HTTOP` for a custom-frame window's top edge). The strip's hit-test slot in step 2 is the only additive change.

## 6. Error handling

Three tiers. Construction of a `Custom` titlebar window must fail closed if caption buttons cannot be created before show; runtime degradation after a successfully-created strip remains best-effort.

### 6.1 Construction-time

- **`composition::ensure_d2d_context` failure** (D3D11 / D2D / DWrite / `CompositionGraphicsDevice` creation): the error propagates out of `CaptionButtonStrip::new`, which propagates out of `initialize_window`, causing `WM_NCCREATE` / `CreateWindowExW` / `window_create` to fail through the existing `ffi_boundary` path. The thread-local singleton cell stays empty on failure — failure is *not* memoised. Subsequent `Custom`-titlebar windows retry through the same cell.
- **`CaptionButtonStrip::new` failure** (e.g., `CompositionGraphicsDevice::CreateDrawingSurface` returns failure): planned behavior is to propagate the error out of `initialize_window`; the custom window is not shown without caption buttons.
- Construction-time caption-button errors for `Custom` titlebars are fatal to window creation. Do not silently leave `Window.caption_buttons` as `None` after a successful `window_create`.

`initialize_window` errors surface to Kotlin as a generic `CreateWindowExW` failure via `LAST_EXCEPTION_MSGS`; the detailed `anyhow::Error` chain (specific HRESULT, file:line, context) lives in `log::error!` only. Auto-promoting to `WindowTitleBarKind::System` is rejected (requires runtime `WindowStyle` mutation, `SwitchWindowProcess`, `SWP_FRAMECHANGED`, and `chrome_layer` teardown — out of scope).

### 6.2 Runtime: device loss

Device loss is detected reactively inside `with_d2d_render_target` on both [`ID2D1RenderTarget::BeginDraw`](https://learn.microsoft.com/en-us/windows/win32/api/d2d1/nf-d2d1-id2d1rendertarget-begindraw) and [`ID2D1RenderTarget::EndDraw`](https://learn.microsoft.com/en-us/windows/win32/api/d2d1/nf-d2d1-id2d1rendertarget-enddraw). `EndDraw`'s doc explicitly returns `D2DERR_RECREATE_TARGET` when the underlying device is lost, so the chokepoint must trap there too — not only on `BeginDraw`. The proactive path through `ID3D11Device4::RegisterDeviceRemovedEvent` plus a threadpool wait callback is deferred — see `TODO.md` *Caption-button proactive device-loss detection*.

Recovery sequence (single path): a device-loss HRESULT on `BeginDraw` or `EndDraw` → `with_d2d_render_target` calls `D2dContext::rebuild_d2d_device` synchronously on the UI thread → builds a fresh `ID2D1Device` (D3D11 + D2D only — DirectWrite factory and `CompositionGraphicsDevice` survive device loss) → calls `ICompositionGraphicsDeviceInterop::SetRenderingDevice` on the existing `CompositionGraphicsDevice`, which retains the new device. The chokepoint returns `Ok(None)` so the calling glyph-rasterise function leaves `glyph_surface_dirty = true`; the next `RenderingDeviceReplaced` notification fires the strip's redraw subscription (below) which re-rasterises and commits. When `EndDraw` reports device loss after a successful body closure, the body's return value is dropped — the device is gone, so the closure's outputs are no longer trustworthy.

The expectation that `RenderingDeviceReplaced` fires on the UI thread holds only because the toolkit always invokes `SetRenderingDevice` from the UI thread — agile marshalling lets the callback fire on the caller's thread, not on a thread of the framework's choosing. Microsoft's Composition native-interop sample demonstrates the inverse pattern: when its `SetRenderingDevice` runs from a `SetThreadpoolWait` worker, the handler runs on that worker. See `TODO.md` *Verify `RenderingDeviceReplaced` fires synchronously*.

**HRESULT match.** Three documented loss-class HRESULTs are trapped: `DXGI_ERROR_DEVICE_REMOVED` (`0x887A0005`), `DXGI_ERROR_DEVICE_RESET` (`0x887A0007`), and `D2DERR_RECREATE_TARGET` (`0x8899000C`). The first two come from the underlying DXGI / D3D11 device; the third is D2D-specific and is the canonical signal `EndDraw` returns when the device is gone. The [Composition native interop](https://learn.microsoft.com/windows/apps/develop/composition/composition-native-interop) sample only covers the `DXGI_ERROR_DEVICE_REMOVED` case; the broader set comes from each API's documented return-values table.

**Surface lifetime.** `CompositionDrawingSurface` instances stay usable after `SetRenderingDevice` — only their contents need re-rasterising. The Composition native-interop sample reuses every surface across device replacement.

The subscription callback posts a private `WM_APP_*` message rather than calling `CaptionButtonStrip::on_rendering_device_replaced()` directly. The handler fires synchronously inside `SetRenderingDevice` (which runs inside `with_d2d_render_target`); a direct call would nest `BeginDraw` on the active surface.

`CompositionGraphicsDevice::RenderingDeviceReplaced` is the redraw notification after `SetRenderingDevice`, not the loss detector. The strip subscribes during construction and owns the `RenderingDeviceReplacedRegistration` guard as a field. The subscription is removed when `WM_NCDESTROY` clears `Window.caption_buttons` — before the HWND value can be recycled by a different window (potentially in another process). The wndproc handler invokes `CaptionButtonStrip::on_rendering_device_replaced()`:

1. Mark all glyph surfaces dirty (`glyph_surface_dirty = true` on every button).
2. Immediately re-rasterise dirty caption-button glyph surfaces in the UI-thread message handler, apply visuals, and `Commit` once.

The WinRT event callback does not draw itself; it only posts the private UI-thread message. The UI-thread handler re-rasterises eagerly so visual recovery is not dependent on later pointer movement.

If the new D2D device also fails to draw (cascading device loss), the strip logs and skips that frame. The affected glyph surfaces remain dirty; do not clear dirty flags when `with_d2d_render_target` returns `Ok(None)`. The strip does not run a local retry loop; it retries only on the next `BeginDraw` attempt or `RenderingDeviceReplaced` notification.

### 6.3 Runtime: per-call failures

- **`DirectWrite` failure** (`CreateTextFormat` / glyph layout / drawing failure after the Fluent-or-MDL2 font choice): logged. Glyph surface stays at its prior contents. Button remains hit-testable and clickable.
- **`StartAnimation` failure**: fall back to instant `Color` set on the `CompositionColorBrush`. Logged at warning level. Visual jumps instead of fading.
- **`TrackMouseEvent` failure**: logged at warning level. Hover-fade-out won't trigger when the pointer leaves the strip; corrects itself on next hover-in. Documented degradation; non-fatal.
- **`SetOffset` / `SetSize` / `SetBrush` failures**: logged; continue with stale visual; next state mutation retries.

## 7. Testing

### 7.1 Pure-Rust unit tests (no display, no GPU)

These cover load-bearing logic without composition or D3D dependency:

- **`resolve_interaction`** truth-table over `(Availability, kind, pointer_over_kind, pointer_device, press_session)`. ~30-40 cases. Catches regressions on the WinUI capture rule and the touch-skip-hover rule.
- **`on_pointer_cancel`**: clears only the matching press session and never returns a caption-button action.
- **`CaptionButton::transition_to(...)`**: which transitions animate vs jump.
- **`hit_test` / disabled-hit policy**: geometry math at varied DPI scales, across each Minimize / Maximize visibility and availability table row. Tests must prove disabled visible buttons remain in geometry but cannot produce Hovered / Pressed / action; wndproc tests must assert disabled visible Minimize / Maximize map to `HTCAPTION`, not `HTMINBUTTON` / `HTMAXBUTTON`.
- **`CaptionButtonMetrics::new`**: 46 × 32 epx round-trip through DPI scales 1.0 / 1.25 / 1.5 / 1.75 / 2.0 / 2.5.
- **`CaptionTheme::resolve`**: per `(Appearance, HighContrast, IsActive)` combo, expected colour-table cells. Catches Close-button override typos.
- **Layout for the Minimize / Maximize visibility and availability table** from `WindowStyle` flags.

These tests live in `caption_buttons.rs` `#[cfg(test)] mod tests { ... }`. No Window or Application required.

### 7.2 Tests that require live composition (skipped under `cargo test`)

- Anything exercising real `D2dContext` (D3D11), real `CompositionGraphicsDevice`, real `BeginDraw` / `EndDraw`. Need a desktop session and GPU on a UI thread.
- Snapshot / golden-image tests for rasterised glyphs. Out of scope; recorded as a follow-on if visual regressions warrant it.

### 7.3 Manual test plan exercised via the sample app

`:sample:runSkikoSampleWin32` already runs with `WindowTitleBarKind.Custom` in this repo. Acceptance checklist for review:

- Rest / Hover / Pressed for enabled buttons, plus Disabled for Minimize / Maximize table rows, in light and dark themes.
- Disabled Minimize / Maximize never fire actions, never show hover / press visuals, swallow their DOWN / UP cycle, and disabled Maximize does not show the Win11 Snap Layout flyout.
- Window inactive vs active — colour modulation visible on `Idle` and `PressedDraggedOff` only. Hover and pressed on caption buttons of an inactive window render with the active palette (matches Terminal's `MinMaxCloseControl` and WinUI 3's `TitleBar`).
- **High contrast** — `HighContrast { Off, On }` is binary; the strip does not distinguish among the four shipped HC themes (HC #1, HC #2, HC White, HC Black). The actual colours come from `GetSysColor` reads (§4.4) and therefore vary per theme even though the toolkit's enum is binary. Manually verify: (a) glyph code points swap to the contrast variants U+EF2C / EF2D / EF2E / EF2F when HC turns On; (b) cycling through all four shipped HC themes under HC On — the strip's backplate, glyph foreground, hover, and pressed colours read from `COLOR_BTNFACE` / `COLOR_BTNTEXT` / `COLOR_HIGHLIGHT` / `COLOR_HIGHLIGHTTEXT` / `COLOR_GRAYTEXT`, so each theme paints a visibly different palette and no colour is hard-coded; (c) a side-by-side visual comparison against a native Win32 chrome window (e.g., File Explorer) under each HC theme — caption-button colours match. Divergence from native colour pairs (e.g., hover not using `HIGHLIGHT` / `HIGHLIGHTTEXT`) is a bug, not acceptable variance.
- Maximize → Restore via the button — glyph swap U+E922 ↔ U+E923.
- **Maximized window edge inset** — when the window is maximized on each of (a) a single-monitor setup, (b) a primary monitor with the taskbar at bottom, (c) a primary monitor with the taskbar at top, (d) a secondary monitor at a different DPI scale than the primary: every caption button is fully visible (no clipping at right or top edge), the strip's right edge sits flush against the inset client area, the Close button's top edge sits flush with the visible monitor edge (no clip, no excess gap above the buttons). The close button hover region maps correctly under the cursor, and Snap Layouts pop on the maximize button at the visually-correct location. Verifies §3.6 inset and strip-side Y-shift.
- **Maximized Custom / None content placement** — under both `WindowTitleBarKind::Custom` and `WindowTitleBarKind::None`, render Skiko content at client y=0 (e.g. a coloured rect), maximize, and confirm the rect's top sits flush with the visible monitor edge for each titlebar mode.
- Snap-layout flyout appears on Win11 hover over the maximize button.
- Mouse, touch, and pen input — touch correctly skips the hover state.
- DPI change by dragging across monitors with different scales — glyph re-rasterised crisply at the new scale; deterministic font size from `GetDesignGlyphMetrics` per §4.5 produces glyphs whose visible bbox matches the 10-epx target on each monitor; the strip re-anchors so its right edge stays flush with the new client width and `max_chrome_y` is recomputed at the new DPI.
- Drag the title bar (drag region between strip and left edge) to move the window.
- Press a button, drag off, release outside — no action fires; button visually returns to Rest.
- Press button A, drag over button B — B does *not* react (WinUI capture rule).
- **Right-click / middle-click on a caption button** — no action fires; button stays in Rest. Only primary-button presses activate (§4.2).
- **Press cycle survives NC→client drift** — press a caption button, drag the cursor into the client area without releasing, then release. `WM_POINTERUP` (client variant) claims the cycle; no stuck press visual on the button, and the host's drag source does not start `DoDragDrop`.
- **Focus-loss cancels the press** — press a caption button and hold; ALT-TAB away (or trigger `EnableWindow(FALSE)` via a modal). Return focus: button is in Rest, no stuck press visual, the next click starts a fresh cycle.
- **System-titlebar window (default `WindowStyle`)** — system Min / Max / Close still respond; configure the sample without `Custom` opt-in to verify.
- Caption glyphs use Segoe Fluent Icons when DirectWrite reports the family is present in the system font collection, with Segoe MDL2 Assets fallback when Fluent Icons is unavailable; manual review still verifies the chosen family renders the expected glyphs on the test OS.
- Glyph size and alignment match Terminal's 10 epx `Viewbox` reference closely enough that Minimize / Maximize / Restore / Close do not look heavier, smaller, or vertically displaced relative to native caption buttons.
- **Hover-fade animation under controlled commit** — on a window with no active ANGLE rendering (apps using on-demand rendering), hover a caption button then move away; the 150 ms backplate / 100 ms glyph fade plays smoothly at 60 Hz. If stutter is observed, escalate per §8 / TODO `Caption-button animation cadence — CommitNeeded fallback`.
- **Auto-hide taskbar reveal under maximized window** — on a monitor with an auto-hide taskbar, maximize both a Custom-titlebar and a System-titlebar window; verify the taskbar still reveals when the cursor reaches the screen edge in each case.

Observations on system-menu restoration, keyboard system-command parity, and UIA/accessibility from the sample run go to `TODO.md`.

This list goes into the sample app's `README` so the manual run-through is reproducible.

## 8. Open questions

1. **Animation cadence under `CompositorController`.** Microsoft documents that Visual-Layer animations run on the system compositor's own thread independent of the UI thread, but does not directly address whether `ColorKeyFrameAnimation` requires per-frame `Commit()` when the compositor is the controlled-commit `CompositorController` variant. The simplest reading is taken (§5.5): commit once at `StartAnimation` and let the compositor's thread advance the animation, matching the existing toolkit's explicit-commit-per-mutation pattern. §7.3 acceptance verifies hover-fade smoothness; if stutter is observed, see TODO `Caption-button animation cadence — CommitNeeded fallback`.

Future work tracked separately in `TODO.md`: RTL mirroring; `WM_NCMOUSEMOVE` fallback; system-menu restoration; Tall-mode title bars; Close-button disable; proactive device-loss detection; `RenderingDeviceReplaced` thread-affinity probe.

## 9. References

### Microsoft documentation

- [Custom Window Frame Using DWM](https://learn.microsoft.com/windows/win32/dwm/customframe) — the canonical Win32 custom-frame recipe (`WM_NCCALCSIZE`, `DwmExtendFrameIntoClientArea`, `WM_NCHITTEST`, `DwmDefWindowProc`).
- [WM_NCCALCSIZE message](https://learn.microsoft.com/windows/win32/winmsg/wm-nccalcsize) — confirms standard-frame removal does not affect DWM-extended frames; non-system titlebar windows keep `WS_CAPTION` for transition semantics but clear `WS_SYSMENU`, the non-system path uses custom non-client handling, and the `Custom` path adds toolkit-owned caption-button visuals.
- [WM_NCHITTEST message](https://learn.microsoft.com/windows/win32/inputdev/wm-nchittest) — return-value table for `HTCLOSE` (20), `HTMAXBUTTON` (9), `HTMINBUTTON` (8).
- [Support snap layouts for desktop apps on Windows 11](https://learn.microsoft.com/windows/apps/desktop/modernize/ui/apply-snap-layout-menu) — `HTMAXBUTTON` is the documented contract for Win11 snap-layout flyout.
- [DwmDefWindowProc function](https://learn.microsoft.com/windows/win32/api/dwmapi/nf-dwmapi-dwmdefwindowproc) — first consultation in `WM_NCHITTEST` and `WM_NCMOUSELEAVE` for custom frames.
- [Composition native interoperation with DirectX and Direct2D](https://learn.microsoft.com/en-us/windows/uwp/composition/composition-native-interop) — the canonical interop pattern (`ICompositorInterop`, `CompositionGraphicsDevice`, `CompositionDrawingSurface`, `BeginDraw` / `EndDraw`).
- [ID3D11Device4::RegisterDeviceRemovedEvent](https://learn.microsoft.com/en-us/windows/win32/api/d3d11_4/nf-d3d11_4-id3d11device4-registerdeviceremovedevent) — asynchronous D3D device-removal notification and Composition recovery remarks.
- [ICompositionGraphicsDeviceInterop::SetRenderingDevice](https://learn.microsoft.com/en-us/windows/win32/api/windows.ui.composition.interop/nf-windows-ui-composition-interop-icompositiongraphicsdeviceinterop-setrenderingdevice) — installs the replacement rendering device on the existing Composition graphics device.
- [CompositionGraphicsDevice.RenderingDeviceReplaced](https://learn.microsoft.com/en-us/uwp/api/windows.ui.composition.compositiongraphicsdevice.renderingdevicereplaced?view=winrt-26100) — redraw notification after the rendering device has been replaced.
- [ScreenToClient](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-screentoclient) — screen-to-client coordinate conversion for `WM_NCHITTEST` strip geometry.
- [VisualCollection.InsertAtTop](https://learn.microsoft.com/en-us/uwp/api/windows.ui.composition.visualcollection.insertattop?view=winrt-26100) — `VisualCollection` ordering is bottom-to-top; `InsertAtTop` inserts at the top of that collection.
- [CompositionMaskBrush Class](https://learn.microsoft.com/uwp/api/windows.ui.composition.compositionmaskbrush) — recolors a pre-rasterised glyph by combining a `Source` colour brush with a `Mask` surface brush; introduced in Windows 10 1607 / 10.0.14393.
- [Segoe Fluent Icons font](https://learn.microsoft.com/windows/apps/design/iconography/segoe-fluent-icons-font) — glyph code points (E921, E922, E923, E8BB) and high-contrast variants (EF2D, EF2E, EF2F, EF2C).
- [Segoe MDL2 Assets icons](https://learn.microsoft.com/en-us/windows/apps/design/iconography/segoe-ui-symbol-font) — fallback icon font source for Windows versions without Segoe Fluent Icons.
- [Title bar customization](https://learn.microsoft.com/windows/apps/develop/title-bar) — *"The button background color is not applied to the Close button hover and pressed states. The close button always uses the system-defined color for those states."*
- [High contrast parameter](https://learn.microsoft.com/windows/win32/winauto/high-contrast-parameter) — `GetSysColor` slot mapping for high-contrast colours; documents only the `BTN*` and `WINDOW*` foreground/background pairs.
- [Compatibility / High-contrast mode](https://learn.microsoft.com/windows/compatibility/high-contrast-mode) — *"`COLOR_HIGHLIGHTTEXT` is meant to be used with `COLOR_HIGHLIGHT` as a background"*; supports the convention of using `HIGHLIGHT` / `HIGHLIGHTTEXT` for selected/hover state.
- [Button Messages — Button Color Messages](https://learn.microsoft.com/windows/win32/controls/button-messages#button-color-messages) — `COLOR_GRAYTEXT` is *"Disabled (gray) text in buttons."*
- [Window Styles](https://learn.microsoft.com/windows/win32/winmsg/window-styles) — `WS_SYSMENU` *"The `WS_CAPTION` style must also be specified."*; `WS_MAXIMIZEBOX` / `WS_MINIMIZEBOX` *"The `WS_SYSMENU` style must also be specified."* — constraints referenced by `WindowStyle::to_system` and the custom-caption-button policy.
- [Microsoft KB Archive Q130760](https://www.betaarchive.com/wiki/index.php/Microsoft_KB_Archive/130760) — documents the paired minimize / maximize box behaviour: neither box appears if both styles are omitted; if only one style is present, both boxes appear and the omitted style's box is disabled.
- [TITLEBARINFOEX structure](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-titlebarinfoex) and [WM_GETTITLEBARINFOEX](https://learn.microsoft.com/en-us/windows/win32/menurc/wm-gettitlebarinfoex) — title-bar element indexes, per-element rectangles, and `STATE_SYSTEM_UNAVAILABLE` / `STATE_SYSTEM_INVISIBLE` states.
- [ColorKeyFrameAnimation Class](https://learn.microsoft.com/uwp/api/windows.ui.composition.colorkeyframeanimation?view=winrt-28000) — available since Windows 10 build 10586 (1511 / Nov 2015 Update).
- [UIElement.CapturePointer (WinUI)](https://learn.microsoft.com/windows/windows-app-sdk/api/winrt/microsoft.ui.xaml.uielement.capturepointer?view=windows-app-sdk-1.8) — *"the second element doesn't fire `PointerEntered` events for a captured pointer when the captured pointer enters it."* `PointerOver` visual state is driven by the same plumbing.
- [TrackMouseEvent](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-trackmouseevent) and [WM_NCMOUSELEAVE message](https://learn.microsoft.com/windows/win32/inputdev/wm-ncmouseleave) — non-client leave tracking and the rule that tracking is canceled when leave is generated.
- [WM_NCPOINTERDOWN](https://learn.microsoft.com/windows/win32/inputmsg/wm-ncpointerdown), [WM_NCPOINTERUPDATE](https://learn.microsoft.com/windows/win32/inputmsg/wm-ncpointerupdate), [WM_NCPOINTERUP](https://learn.microsoft.com/windows/win32/inputmsg/wm-ncpointerup) — non-client pointer messages; the documented contract is *"`HIWORD(wParam)`: hit-test value returned from processing the `WM_NCHITTEST` message"*. Implementation-observed on Windows 11: the upper word arrives muxed with `POINTER_FLAG_*` bits and never matches `HTCLOSE` / `HTMAXBUTTON` / `HTMINBUTTON` for these messages, so the toolkit dispatches via geometric hit-test (`caption_kind_at_screen`) instead. `WM_NCPOINTERDOWN` documents the implicit non-client capture contract: *"The pointer is implicitly captured to the window so that the window continues to receive input for the pointer until it breaks contact."* `WM_NCPOINTERUP` describes the release end of that same session.
- [WM_POINTERCAPTURECHANGED](https://learn.microsoft.com/windows/win32/inputmsg/wm-pointercapturechanged), [WM_POINTERUP](https://learn.microsoft.com/windows/win32/inputmsg/wm-pointerup) — capture-loss cleanup guidance and the warning not to depend on paired pointer notifications.
- [WM_CANCELMODE](https://learn.microsoft.com/windows/win32/winmsg/wm-cancelmode) — cancellation signal. Remarks: *"When the `WM_CANCELMODE` message is sent, the `DefWindowProc` function cancels internal processing of standard scroll bar input, cancels internal menu processing, and releases the mouse capture."* Return value: *"If an application processes this message, it should return zero."* The toolkit returns `None` so `DefWindowProc` runs that standard cleanup in addition to the strip's `cancel_any_press`.
- [windows-rs `IDWriteFactory`](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Graphics/DirectWrite/struct.IDWriteFactory.html) — `IDWriteFactory` implements `Clone` in `windows` 0.62.2.
- [docs.rs `windows = 0.62.2` feature flags](https://docs.rs/crate/windows/0.62.2/features) — confirms the real feature names used by `Cargo.toml`.
- [microsoft-ui-xaml/specs/TitleBar/titlebar-functional-spec.md](https://github.com/microsoft/microsoft-ui-xaml/blob/main/specs/TitleBar/titlebar-functional-spec.md) — confirms WinUI delegates caption-button rendering to `AppWindowTitleBar`.

### Source-code references

- [microsoft/terminal MinMaxCloseControl.xaml (pinned commit `e4e3f08efca9d0ffba330eee12edbcb16897ddcb`)](https://github.com/microsoft/terminal/blob/e4e3f08efca9d0ffba330eee12edbcb16897ddcb/src/cascadia/TerminalApp/MinMaxCloseControl.xaml) — source for caption-button metrics, glyph code points, `VisualState`s, and storyboards used by this spec.
- [Min/Max/Close buttons should be 32px · Issue #9093](https://github.com/microsoft/terminal/issues/9093) — Terminal's rationale for the 40-windowed / 32-maximised height pair.
- [Chromium `CustomFrameView::NonClientHitTest` (pinned tag `125.0.6422.160`)](https://chromium.googlesource.com/chromium/src/+/refs/tags/125.0.6422.160/ui/views/window/custom_frame_view.cc) — custom-frame source example returning `HTCLOSE` / `HTMAXBUTTON` / `HTMINBUTTON` for custom caption-button bounds.

### Crate-internal references

- `native/desktop-win32/docs/AGENTS.md` — agent orientation; WinRT-only-where-necessary discipline.
- `native/desktop-win32/docs/ARCHITECTURE.md` § Composition — `Windows.UI.Composition` via `ICompositorDesktopInterop`, controlled-commit `CompositorController`.
- `native/desktop-win32/docs/SUBSYSTEMS.md` — Window, Renderer (ANGLE), Pointer, Appearance subsystems.
- `native/desktop-win32/docs/TODO.md` — backlog for deferred caption-button-adjacent work, including Win32 Close-button disable support.

[^winui-capture]: [*UIElement.CapturePointer (WinUI)*](https://learn.microsoft.com/windows/windows-app-sdk/api/winrt/microsoft.ui.xaml.uielement.capturepointer?view=windows-app-sdk-1.8): *"the second element doesn't fire `PointerEntered` events for a captured pointer when the captured pointer enters it."*
