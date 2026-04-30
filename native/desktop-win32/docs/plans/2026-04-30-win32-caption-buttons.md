# Win32 Caption Buttons Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add toolkit-managed caption buttons (close / maximise-restore / minimise) to `WindowTitleBarKind::Custom` windows, rendered as `Windows.UI.Composition` visuals with full state coverage (rest, hover, pressed, disabled for Minimize / Maximize + active/inactive modulation), full appearance coverage (light, dark, high-contrast), and Win32-correct hit-testing including Win11 snap-layout flyout.

**Close-button disable scope:** Close is always visible and enabled. Win32 Close-button disable support is deferred because it is controlled through the system menu rather than a Min/Max-style window bit; see `native/desktop-win32/docs/TODO.md#win32-close-button-disable-support`.

**Architecture:** Introduces `D2dContext` — a private (caption-button-only) gateway for D3D11 / D2D / DirectWrite / `CompositionGraphicsDevice`, exposed via a UI-thread `OnceCell<Rc<D2dContext>>` singleton in `composition.rs`. The window's composition root is split into three explicit z-layers (`backdrop_layer` / `content_layer` / `chrome_layer`). Per-window `CaptionButtonStrip` lives in `chrome_layer`; it's a pure state machine over typed inputs, dispatched from the wndproc layer. Click side-effects translate to existing `Window::request_close` / `Window::minimize` / `Window::maximize` / `Window::restore` — no new app-facing events.

**Tech Stack:** Rust + `windows` crate v0.62 (`Windows.UI.Composition`, `Windows.UI.Composition.Core`, `Windows.UI.Composition.Desktop`, `Windows.Win32.System.WinRT.Composition` interop, `Windows.Win32.Graphics.Direct3D11`, `Windows.Win32.Graphics.Direct2D`, `Windows.Win32.Graphics.DirectWrite`). Existing ANGLE-on-D3D11 renderer.

**Reference spec:** `native/desktop-win32/docs/specs/2026-04-30-win32-caption-buttons-design.md`.

**Reference candidate for concrete values (per the per-feature fallback rule in `ARCHITECTURE.md`):** Windows Terminal `MinMaxCloseControl.xaml` pinned at commit `e4e3f08efca9d0ffba330eee12edbcb16897ddcb`: <https://github.com/microsoft/terminal/blob/e4e3f08efca9d0ffba330eee12edbcb16897ddcb/src/cascadia/TerminalApp/MinMaxCloseControl.xaml>.

---

## File layout

| File | Status | Responsibility |
|---|---|---|
| `native/desktop-win32/src/win32/composition.rs` | new | `D2dContext` (private to caption-button rasterisation). Holds `IDWriteFactory` and `CompositionGraphicsDevice`, plus the device-loss recovery machinery. Exposes `new(compositor)`, `dwrite_factory()`, `create_drawing_surface()`, `with_d2d_render_target()`, `add_rendering_device_replaced_callback()`. |
| `native/desktop-win32/src/win32/caption_buttons.rs` | new | `CaptionButtonStrip` (public to crate), `CaptionButton` / `CaptionTheme` / `CaptionButtonMetrics` / `Availability` / `ButtonInteraction` / `PressSession` / `PointerDeviceKind` (private). Pure state machine; no Win32 calls itself. Includes `#[cfg(test)] mod tests` for the pure logic. |
| `native/desktop-win32/src/win32/pointer.rs` | modify | Add `PointerInfo::pointer_id()` accessor that returns `self.get_native_pointer_info().pointerId`. Consistent with the existing `get_pointer_state()` / `get_timestamp()` / etc. accessor pattern; lets the wndproc layer (Task 6.3) route to the strip without re-decoding `wparam`. |
| `native/desktop-win32/src/win32/window.rs` | modify | Add `nc_leave_tracking_armed`, `ensure_nc_leave_tracking` / `nc_leave_tracking_fired`, `caption_buttons` fields and `chrome_layer` accessor; restructure `initialize_content` for the 3-layer split; redirect `add_visual` to insert into `content_layer`. Extend the existing `WM_NCDESTROY` arm to drop `window.caption_buttons` before the HWND is destroyed. Add an `if !self.is_resizable() { return; }` early-return to `Window::maximize()` so non-resizable windows can't be maximized programmatically (matches strip-layer policy in spec §4.2). |
| `native/desktop-win32/src/win32/event_loop.rs` | modify | Extend `on_nchittest` to consult strip. Extend existing `on_pointerupdate` / `on_pointerdown` / `on_pointerup` (which already merge `WM_*POINTER*` and `WM_NC*POINTER*`) to route to strip when `HIWORD(wParam)` is a caption-button hit-test value, filtering on primary-button-only via `pointer_info.get_pointer_button_change()`. Add cleanup-only `WM_POINTERCAPTURECHANGED`. Add `pub(crate) WM_APP_CAPTION_BUTTONS_RENDERING_DEVICE_REPLACED` constant (Task 1.2 step 3) + wndproc dispatch arm (Task 6.4 step 2). Extend `on_activate` / `on_ncmouseleave` / `on_dpichanged` / `on_settingchange` / `on_windowposchanged` / `on_nccalcsize`. |
| `native/desktop-win32/src/win32/mod.rs` | modify | Add `pub mod composition;` and `pub mod caption_buttons;`. |
| `native/desktop-win32/Cargo.toml` | modify | Add real `windows = 0.62.2` features for `Graphics_DirectX`, `Win32_Graphics_Direct2D`, `Win32_Graphics_Direct2D_Common`, `Win32_Graphics_Direct3D`, `Win32_Graphics_Direct3D11`, `Win32_Graphics_DirectWrite`, `Win32_Graphics_Dxgi`. |

---

## Phase 1 — `D2dContext` introduction and UI-thread singleton

> **Phase 1 commit policy:** Tasks 1.1 through 1.6 stage their changes incrementally; the single Phase 1 commit lands at Task 1.6 step 5.

### Task 1.1: Add Cargo feature flags for D3D11 / D2D / DWrite / DXGI / Composition surface formats

**Files:** Modify: `native/desktop-win32/Cargo.toml`

- [ ] **Step 1: Read existing `[dependencies.windows]` features**

```powershell
Select-String -Path D:/repos/kotlin-desktop-toolkit/native/desktop-win32/Cargo.toml -Pattern "windows"
```

- [ ] **Step 2: Add the new feature flags**

Append to the `windows` crate's `features = [ ... ]` array:

```toml
"Graphics_DirectX",
"Win32_Graphics_Direct2D",
"Win32_Graphics_Direct2D_Common",
"Win32_Graphics_Direct3D",
"Win32_Graphics_Direct3D11",
"Win32_Graphics_DirectWrite",
"Win32_Graphics_Dxgi",
```

`Graphics_DirectX` gates `DirectXPixelFormat`/`DirectXAlphaMode` (used by `CreateDrawingSurface2`). The native Composition interop interfaces stay under the existing `Win32_System_WinRT_Composition` feature.

- [ ] **Step 3: Verify `cargo check` succeeds**

Run: `cd D:/repos/kotlin-desktop-toolkit/native && cargo check -p desktop-win32`
Expected: PASS (just adding features doesn't compile new code yet).

- [ ] **Step 4: Stage**

```bash
git add native/desktop-win32/Cargo.toml
```

### Task 1.2: Create `composition.rs` skeleton with `D2dContext`

**Files:**
- Create: `native/desktop-win32/src/win32/composition.rs`
- Modify: `native/desktop-win32/src/win32/mod.rs`
- Modify: `native/desktop-win32/src/win32/event_loop.rs`

- [ ] **Step 1: Add module declaration in `mod.rs`**

Add this line alongside the existing `pub mod` siblings (alphabetical order is the existing convention; insert between the `clipboard_api` line and the `cursor` line):

```rust
pub mod composition;
```

- [ ] **Step 2: Create the skeleton file**

`native/desktop-win32/src/win32/composition.rs`:

```rust
//! `D2dContext` — caption-button D2D / DirectWrite gateway.
//!
//! Holds the `IDWriteFactory` and `CompositionGraphicsDevice`. The CGD
//! retains the D3D11 / D2D rendering device, swapped on device loss via
//! `SetRenderingDevice`. Hides `BeginDraw`/`EndDraw`/device-loss behind a
//! closure-shaped `with_d2d_render_target` chokepoint.
//!
//! See `docs/specs/2026-04-30-win32-caption-buttons-design.md` § 4.1 for the
//! full design rationale.

use windows::UI::Composition::Compositor;

pub(crate) struct D2dContext {
    // Fields populated in Task 1.4.
}

impl D2dContext {
    /// Eager construction. Lazy-init and the singleton `Rc<D2dContext>` cell
    /// live at `composition::ensure_d2d_context` (Task 1.3).
    pub fn new(_compositor: Compositor) -> anyhow::Result<Self> {
        // Task 1.4 fills this in.
        Ok(Self {})
    }
}
```

- [ ] **Step 3: Add the wndproc redraw-message constant in `event_loop.rs`**

Add `WM_APP` to the existing `WindowsAndMessaging` import group, then append above `thread_local!`:

```rust
pub(crate) const WM_APP_CAPTION_BUTTONS_RENDERING_DEVICE_REPLACED: u32 = WM_APP + 0x31;
```

The constant is referenced by `caption_buttons.rs` (Task 5.1) and the wndproc dispatch arm (Task 6.4 step 2); land it here so Phase 5 builds.

- [ ] **Step 4: Verify `cargo check` succeeds**

Run: `cd D:/repos/kotlin-desktop-toolkit/native && cargo check -p desktop-win32`
Expected: PASS.

- [ ] **Step 5: Stage**

```bash
git add native/desktop-win32/src/win32/composition.rs native/desktop-win32/src/win32/mod.rs native/desktop-win32/src/win32/event_loop.rs
```

### Task 1.3: Add the `ensure_d2d_context` UI-thread singleton accessor

**Files:** Modify: `native/desktop-win32/src/win32/composition.rs`

`composition.rs` exposes a thread-local `OnceCell<Rc<D2dContext>>` and a free function `ensure_d2d_context(compositor) -> Rc<D2dContext>` that lazy-initialises the cell. The strip calls this directly (Task 5.1); `Application` and `Window` are unchanged. Mirrors the `appearance.rs:40-50` `OnceLock<UISettings>` pattern (with `thread_local!` because `D2dContext` is `!Send`).

- [ ] **Step 1: Add the singleton imports**

Add to the top of `composition.rs` (alongside the existing `use windows::UI::Composition::Compositor;` from Task 1.2):

```rust
use std::cell::OnceCell;
use std::rc::Rc;
```

- [ ] **Step 2: Append the singleton accessor at the end of the file**

After the `impl D2dContext` block, append:

```rust
thread_local! {
    /// UI-thread singleton for caption-button rasterisation. Lazy on first
    /// `ensure_d2d_context` call; failure is not memoised — `D2dContext::new`'s
    /// `Err` propagates via `?` before `get_or_init` runs, leaving the cell
    /// empty for retry.
    static D2D_CONTEXT: OnceCell<Rc<D2dContext>> = const { OnceCell::new() };
}

pub(crate) fn ensure_d2d_context(compositor: Compositor) -> anyhow::Result<Rc<D2dContext>> {
    D2D_CONTEXT.with(|cell| {
        if let Some(ctx) = cell.get() {
            return Ok(Rc::clone(ctx));
        }
        let ctx = D2dContext::new(compositor)?;
        let cached = cell.get_or_init(|| Rc::new(ctx));
        Ok(Rc::clone(cached))
    })
}
```

- [ ] **Step 3: Verify `cargo check`**

Run: `cd D:/repos/kotlin-desktop-toolkit/native && cargo check -p desktop-win32`
Expected: PASS. The `D2dContext::new` stub from Task 1.2 returns `Ok(Self {})` so the singleton compiles without the heavy D3D/D2D/DWrite/CGD construction (Task 1.4 fills it in).

- [ ] **Step 4: Stage**

```bash
git add native/desktop-win32/src/win32/composition.rs
```


### Task 1.4: Add D3D11 / D2D / DirectWrite / `CompositionGraphicsDevice` initialisation

**Files:** Modify: `native/desktop-win32/src/win32/composition.rs`.

- [ ] **Step 1: Replace the imports and the empty struct (leave Task 1.3's `D2D_CONTEXT` thread-local and `ensure_d2d_context` fn untouched)**

Replace the import block at the top of `composition.rs` (keeps `OnceCell`, `Rc`, `Compositor` from Task 1.2 / 1.3):

```rust
use std::cell::OnceCell;
use std::rc::Rc;

use windows::{
    UI::Composition::{Compositor, CompositionGraphicsDevice},
    Win32::{
        Foundation::HMODULE,
        Graphics::{
            Direct2D::{
                D2D1CreateFactory, D2D1_FACTORY_TYPE_SINGLE_THREADED, ID2D1Device, ID2D1Factory1,
            },
            Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1},
            Direct3D11::{
                D3D11CreateDevice, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, ID3D11Device,
            },
            DirectWrite::{
                DWriteCreateFactory, DWRITE_FACTORY_TYPE_SHARED, IDWriteFactory,
            },
            Dxgi::{IDXGIAdapter, IDXGIDevice},
        },
        System::WinRT::Composition::ICompositorInterop,
    },
};
use windows_core::Interface;
```

Replace the empty `D2dContext` struct definition (the stub from Task 1.2) with:

```rust
pub(crate) struct D2dContext {
    dwrite_factory: IDWriteFactory,
    composition_graphics_device: CompositionGraphicsDevice,
}
```

- [ ] **Step 2: Replace `D2dContext::new` with the eager constructor**

Replace `impl D2dContext` (currently the empty stub from Task 1.2) with:

```rust
impl D2dContext {
    /// Eagerly constructs the D3D11 / D2D devices, the DirectWrite factory,
    /// and the `CompositionGraphicsDevice`. The `Rc<D2dContext>` singleton
    /// wrapping happens once at `composition::ensure_d2d_context` (Task 1.3).
    pub fn new(compositor: Compositor) -> anyhow::Result<Self> {
        let d2d_device = build_d2d_device()?;
        let dwrite_factory: IDWriteFactory =
            unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };
        let compositor_interop: ICompositorInterop = compositor.cast()?;
        let composition_graphics_device =
            unsafe { compositor_interop.CreateGraphicsDevice(&d2d_device)? };
        Ok(Self {
            dwrite_factory,
            composition_graphics_device,
        })
    }

    pub fn dwrite_factory(&self) -> IDWriteFactory {
        self.dwrite_factory.clone()
    }
}

fn build_d2d_device() -> anyhow::Result<ID2D1Device> {
    let feature_levels = [D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0];
    let mut d3d_device: Option<ID3D11Device> = None;
    let mut returned_level = D3D_FEATURE_LEVEL_11_0;
    unsafe {
        D3D11CreateDevice(
            // Turbofish disambiguates `None` to `Option<&IDXGIAdapter>`
            // because `padapter: P0: Param<IDXGIAdapter>`.
            None::<&IDXGIAdapter>,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            Some(&feature_levels),
            D3D11_SDK_VERSION,
            Some(&raw mut d3d_device),
            Some(&raw mut returned_level),
            None,
        )?;
    }
    let d3d_device = d3d_device.ok_or_else(|| anyhow::anyhow!("D3D11CreateDevice returned no device"))?;
    let d2d_factory: ID2D1Factory1 = unsafe {
        D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?
    };
    let dxgi_device: IDXGIDevice = d3d_device.cast()?;
    Ok(unsafe { d2d_factory.CreateDevice(&dxgi_device)? })
}
```

- [ ] **Step 3: Run `cargo check`**

Run: `cd D:/repos/kotlin-desktop-toolkit/native && cargo check -p desktop-win32`
Expected: PASS.

- [ ] **Step 4: Stage**

```bash
git add native/desktop-win32/src/win32/composition.rs
```

### Task 1.5: Add `with_d2d_render_target` chokepoint and `create_drawing_surface` helper

**Files:** Modify: `native/desktop-win32/src/win32/composition.rs`.

- [ ] **Step 1: Add the imports**

Append to the existing imports:

```rust
use windows::{
    Graphics::SizeInt32,
    UI::Composition::CompositionDrawingSurface,
};
use windows::Win32::{
    Foundation::POINT,
    Graphics::{
        Direct2D::{ID2D1DeviceContext, ID2D1RenderTarget},
        Dxgi::DXGI_ERROR_DEVICE_REMOVED,
    },
    System::WinRT::Composition::ICompositionDrawingSurfaceInterop,
};
use windows::Graphics::DirectX::{DirectXAlphaMode, DirectXPixelFormat};
```

- [ ] **Step 2: Add the helpers to `impl D2dContext`**

Inside `impl D2dContext`, after `dwrite_factory`:

```rust
    pub fn create_drawing_surface(&self, size: SizeInt32) -> anyhow::Result<CompositionDrawingSurface> {
        Ok(self.composition_graphics_device.CreateDrawingSurface2(
            size,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            DirectXAlphaMode::Premultiplied,
        )?)
    }

    /// Run a Direct2D drawing closure against the given Composition surface.
    ///
    /// `Ok(None)` means the underlying D3D11 device was lost; the caller
    /// should skip this frame and leave any dirty flags set so the next
    /// invocation re-rasterises. The `DXGI_ERROR_DEVICE_REMOVED` branch is
    /// wired below; Task 1.6 adds the `rebuild_d2d_device` body it calls.
    /// Other errors propagate.
    pub fn with_d2d_render_target<R>(
        &self,
        surface: &CompositionDrawingSurface,
        body: impl FnOnce(&ID2D1RenderTarget, POINT) -> anyhow::Result<R>,
    ) -> anyhow::Result<Option<R>> {
        use anyhow::Context as _;

        let surface_interop: ICompositionDrawingSurfaceInterop = surface.cast()?;
        let mut offset = POINT::default();
        let context = match unsafe { surface_interop.BeginDraw::<ID2D1DeviceContext>(None, &raw mut offset) } {
            Ok(context) => context,
            Err(err) if err.code() == DXGI_ERROR_DEVICE_REMOVED => return Ok(None),
            Err(err) => return Err(err.into()),
        };
        let rt: &ID2D1RenderTarget = (&context).into();
        // EndDraw must run even on body failure — an open BeginDraw breaks
        // future rasterisations on this surface. Body error wins; EndDraw
        // error attaches as context.
        let body_result = body(rt, offset);
        let end_draw_result = unsafe { surface_interop.EndDraw() };
        match (body_result, end_draw_result) {
            (Ok(value), Ok(())) => Ok(Some(value)),
            (Err(body_err), Err(end_draw_err)) => Err(body_err
                .context(format!("EndDraw also failed: {end_draw_err}"))),
            (Err(body_err), Ok(())) => Err(body_err),
            (Ok(_), Err(end_draw_err)) => Err(end_draw_err.into()),
        }
    }
```

- [ ] **Step 3: Verify `cargo check`**

Run: `cd D:/repos/kotlin-desktop-toolkit/native && cargo check -p desktop-win32`
Expected: PASS.

- [ ] **Step 4: Stage**

```bash
git add native/desktop-win32/src/win32/composition.rs
```

### Task 1.6: Add reactive device-loss recovery and the redraw notification subscription

**Files:** Modify: `native/desktop-win32/src/win32/composition.rs`.

- [ ] **Step 1: Update the `BeginDraw` device-loss path in `with_d2d_render_target`**

Update `with_d2d_render_target` so a `BeginDraw` error whose HRESULT is `DXGI_ERROR_DEVICE_REMOVED` calls `rebuild_d2d_device` and returns `Ok(None)`:

```rust
let context = match unsafe { surface_interop.BeginDraw::<ID2D1DeviceContext>(None, &raw mut offset) } {
    Ok(context) => context,
    Err(err) if err.code() == DXGI_ERROR_DEVICE_REMOVED => {
        self.rebuild_d2d_device()?;
        return Ok(None);
    }
    Err(err) => return Err(err.into()),
};
```

The match is `DXGI_ERROR_DEVICE_REMOVED` only — see spec §6.2 for the HRESULT contract.

- [ ] **Step 2: Add the shared replacement routine**

Implement `rebuild_d2d_device`. It rebuilds D3D11 + D2D and calls `ICompositionGraphicsDeviceInterop::SetRenderingDevice` on the existing `CompositionGraphicsDevice`. On failure, CGD keeps the previous rendering device.

```rust
/// Rebuild the D3D/D2D devices after device loss is detected.
///
/// Called from the `BeginDraw` `DXGI_ERROR_DEVICE_REMOVED` path inside
/// `with_d2d_render_target`. Do not call this from `RenderingDeviceReplaced`;
/// that event is only the redraw notification after `SetRenderingDevice`.
pub(crate) fn rebuild_d2d_device(&self) -> anyhow::Result<()> {
    let d2d_device = build_d2d_device()?;
    let cgd_interop: windows::Win32::System::WinRT::Composition::ICompositionGraphicsDeviceInterop =
        self.composition_graphics_device.cast()?;
    unsafe { cgd_interop.SetRenderingDevice(&d2d_device)?; }
    Ok(())
}
```

- [ ] **Step 3: Add the redraw-notification subscription**

Inside `impl D2dContext`, append a subscription helper for existing surfaces:

```rust
    /// Subscribe to `RenderingDeviceReplaced` (the redraw notification fired
    /// synchronously inside `SetRenderingDevice`). Callbacks must post a
    /// `WM_APP_*` message rather than call the strip directly — the event
    /// fires nested inside `with_d2d_render_target → rebuild_d2d_device`, so a
    /// direct re-entry would nest `BeginDraw` on the active surface.
    pub(crate) fn add_rendering_device_replaced_callback<F>(
        &self,
        callback: F,
    ) -> anyhow::Result<RenderingDeviceReplacedRegistration>
    where
        F: Fn() + Send + 'static,
    {
        let handler = windows::Foundation::TypedEventHandler::<
            CompositionGraphicsDevice,
            windows::UI::Composition::RenderingDeviceReplacedEventArgs,
        >::new(move |_, _| {
            callback();
            Ok(())
        });
        let token = self.composition_graphics_device.RenderingDeviceReplaced(&handler)?;
        Ok(RenderingDeviceReplacedRegistration {
            composition_graphics_device: self.composition_graphics_device.clone(),
            token,
        })
    }
```

The RAII guard's `Drop` calls `RemoveRenderingDeviceReplaced(token)`. The token type is `i64` (not `EventRegistrationToken`) per the v0.62.2 binding.

```rust
pub(crate) struct RenderingDeviceReplacedRegistration {
    composition_graphics_device: CompositionGraphicsDevice,
    token: i64,
}

impl Drop for RenderingDeviceReplacedRegistration {
    fn drop(&mut self) {
        let _ = self
            .composition_graphics_device
            .RemoveRenderingDeviceReplaced(self.token);
    }
}
```

- [ ] **Step 4: Verify `cargo check`**

Run: `cd D:/repos/kotlin-desktop-toolkit/native && cargo check -p desktop-win32`
Expected: PASS.

- [ ] **Step 5: Commit Phase 1**

The Phase 1 commit lands all of Tasks 1.1 through 1.6 (each prior task only staged):

```bash
git add native/desktop-win32/src/win32/composition.rs
git commit -m "feat(win32): introduce D2dContext with reactive device-loss recovery"
```

---

## Phase 2 — Composition tree restructure

### Task 2.1: Restructure `initialize_content` into 3-layer split

**Files:** Modify: `native/desktop-win32/src/win32/window.rs`

- [ ] **Step 1: Add `backdrop_layer`, `content_layer`, `chrome_layer` fields to `Window`**

In the `Window` struct, replace `composition_root: RefCell<Option<ContainerVisual>>,` with:

```rust
    composition_root: RefCell<Option<ContainerVisual>>,
    backdrop_layer:   RefCell<Option<ContainerVisual>>,
    content_layer:    RefCell<Option<ContainerVisual>>,
    chrome_layer:     RefCell<Option<ContainerVisual>>,
```

In `Window::new`'s struct literal, initialise the new fields to `RefCell::new(None)` alongside `composition_root`.

- [ ] **Step 2: Replace `initialize_content` body**

Replace the existing `initialize_content`:

```rust
fn initialize_content(window: &Window, hwnd: HWND) -> anyhow::Result<()> {
    let compositor = window.compositor_controller.Compositor()?;
    let compositor_interop: ICompositorDesktopInterop = compositor.cast()?;
    let desktop_window_target = unsafe { compositor_interop.CreateDesktopWindowTarget(hwnd, false) }?;

    let root_visual = compositor.CreateContainerVisual()?;
    root_visual.SetBackfaceVisibility(CompositionBackfaceVisibility::Hidden)?;

    let backdrop_layer = compositor.CreateContainerVisual()?;
    let content_layer  = compositor.CreateContainerVisual()?;
    let chrome_layer   = compositor.CreateContainerVisual()?;

    // Order matters: last InsertAtTop wins.
    let root_children = root_visual.Children()?;
    root_children.InsertAtTop(&backdrop_layer)?;
    root_children.InsertAtTop(&content_layer)?;
    root_children.InsertAtTop(&chrome_layer)?;

    let backdrop_visual = compositor.CreateSpriteVisual()?;
    backdrop_layer.Children()?.InsertAtBottom(&backdrop_visual)?;

    desktop_window_target.SetRoot(&root_visual)?;

    window.backdrop_tint.replace(Some(backdrop_visual));
    window.composition_target.replace(Some(desktop_window_target));
    window.composition_root.replace(Some(root_visual));
    window.backdrop_layer.replace(Some(backdrop_layer));
    window.content_layer.replace(Some(content_layer));
    window.chrome_layer.replace(Some(chrome_layer));
    Ok(())
}
```

- [ ] **Step 3: Verify `cargo check`**

Run: `cd D:/repos/kotlin-desktop-toolkit/native && cargo check -p desktop-win32`
Expected: PASS (no API change yet that depends on the new layers; ANGLE still inserts via the now-stale `add_visual` which adds to `composition_root` — fixed in 2.2).

- [ ] **Step 4: Stage**

```bash
git add native/desktop-win32/src/win32/window.rs
```

### Task 2.2: Redirect `Window::add_visual` to insert into `content_layer`

**Files:** Modify: `native/desktop-win32/src/win32/window.rs`

- [ ] **Step 1: Update the `add_visual` body**

Replace `Window::add_visual`:

```rust
    #[inline]
    pub(crate) fn add_visual(&self) -> anyhow::Result<SpriteVisual> {
        let sprite_visual = self.compositor_controller.Compositor()?.CreateSpriteVisual()?;
        self.content_layer
            .borrow()
            .as_ref()
            .context("Window has not been created yet")?
            .Children()?
            .InsertAtTop(&sprite_visual)?;
        Ok(sprite_visual)
    }
```

- [ ] **Step 2: Verify `cargo check`, then run the existing Skiko sample to confirm ANGLE still renders correctly**

Run:
```bash
cd D:/repos/kotlin-desktop-toolkit/native && cargo check -p desktop-win32
cd D:/repos/kotlin-desktop-toolkit && ./gradlew :sample:runSkikoSampleWin32
```
Expected: the sample window opens and Skia content renders — same as before. (Manual visual check.)

- [ ] **Step 3: Stage**

```bash
git add native/desktop-win32/src/win32/window.rs
```

### Task 2.3: Add `chrome_layer()` accessor on `Window`

**Files:** Modify: `native/desktop-win32/src/win32/window.rs`

- [ ] **Step 1: Add the accessor**

After `Window::add_visual`, add:

```rust
    #[inline]
    pub(crate) fn chrome_layer(&self) -> anyhow::Result<ContainerVisual> {
        self.chrome_layer
            .borrow()
            .as_ref()
            .context("Window has not been created yet")?
            .clone()
            .pipe(Ok)
    }
```

If the codebase doesn't already use the `pipe` extension trait, replace the function body with:

```rust
        let layer = self.chrome_layer.borrow();
        let layer = layer.as_ref().context("Window has not been created yet")?;
        Ok(layer.clone())
```

- [ ] **Step 2: Verify `cargo check`**

Run: `cd D:/repos/kotlin-desktop-toolkit/native && cargo check -p desktop-win32`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add native/desktop-win32/src/win32/window.rs
git commit -m "refactor(win32): split window composition tree into backdrop/content/chrome layers"
```

---

## Phase 3 — Window-level extensions

### Task 3.1: Add `Window::ensure_nc_leave_tracking` / `nc_leave_tracking_fired`

**Files:** Modify: `native/desktop-win32/src/win32/window.rs`

- [ ] **Step 1: Import `TrackMouseEvent`, `TRACKMOUSEEVENT`, `TME_LEAVE`, `TME_NONCLIENT`**

Add a new `use` line at the top of `window.rs`:

```rust
use windows::Win32::UI::Input::KeyboardAndMouse::{
    TrackMouseEvent, TRACKMOUSEEVENT, TME_LEAVE, TME_NONCLIENT,
};
```

(`Win32_UI_Input_KeyboardAndMouse` is already enabled in `Cargo.toml`.)

- [ ] **Step 2: Add the field to the `Window` struct**

Add to the struct (alongside `pointer_in_window: AtomicBool`):

```rust
    nc_leave_tracking_armed: AtomicBool,
```

In `Window::new`'s struct literal, initialise to `AtomicBool::new(false)`.

- [ ] **Step 3: Add the methods**

In `impl Window`:

```rust
    pub(crate) fn ensure_nc_leave_tracking(&self) -> anyhow::Result<()> {
        if !self.nc_leave_tracking_armed.swap(true, Ordering::Relaxed) {
            let mut tme = TRACKMOUSEEVENT {
                cbSize: size_of::<TRACKMOUSEEVENT>().try_into()?,
                dwFlags: TME_NONCLIENT | TME_LEAVE,
                hwndTrack: self.hwnd(),
                dwHoverTime: 0,
            };
            unsafe { TrackMouseEvent(&raw mut tme)? };
        }
        Ok(())
    }

    pub(crate) fn nc_leave_tracking_fired(&self) {
        self.nc_leave_tracking_armed.store(false, Ordering::Relaxed);
    }
```

- [ ] **Step 4: Verify and stage**

Run: `cd D:/repos/kotlin-desktop-toolkit/native && cargo check -p desktop-win32`
Expected: PASS.

```bash
git add native/desktop-win32/src/win32/window.rs
```

### Task 3.2: Add `Window::caption_buttons` field (placeholder; populated in Phase 5)

**Files:** Modify: `native/desktop-win32/src/win32/window.rs`

- [ ] **Step 1: Add the field**

Add to the `Window` struct (after `nc_leave_tracking_armed`):

```rust
    pub(crate) caption_buttons: RefCell<Option<crate::win32::caption_buttons::CaptionButtonStrip>>,
```

In `Window::new`'s struct literal, initialise `caption_buttons` to `RefCell::new(None)`.

- [ ] **Step 2: `cargo check` will fail until `caption_buttons` module exists — defer commit to Phase 4**

(You can stage the change but don't commit yet; the final Phase 4 commit at Task 4.3 step 4 lands these fields together with the new module + tests.)

---

## Phase 4 — `caption_buttons.rs` types and pure logic (TDD)

### Task 4.1: Types + `resolve_interaction` + visible-button derivation

**Files:**
- Create: `native/desktop-win32/src/win32/caption_buttons.rs`
- Modify: `native/desktop-win32/src/win32/mod.rs`

- [ ] **Step 1: Add the module declaration**

In `mod.rs`, add `pub mod caption_buttons;` alphabetically (between `application` and `clipboard`).

- [ ] **Step 2: Create the file with type stubs**

`native/desktop-win32/src/win32/caption_buttons.rs`:

```rust
//! Caption-button strip for `WindowTitleBarKind::Custom` windows.
//!
//! See `docs/specs/2026-04-30-win32-caption-buttons-design.md` for the design.
//!
//! This module is a pure state machine over typed inputs. It does not call
//! Win32 APIs; the wndproc layer in `event_loop.rs` is the only place that
//! bridges Win32 messages and this module.

use std::rc::Rc;

use windows::UI::Composition::{Compositor, ContainerVisual, SpriteVisual, CompositionColorBrush, CompositionDrawingSurface, CompositionSurfaceBrush};
use windows::UI::Composition::Core::CompositorController;

use super::composition::D2dContext;
use super::geometry::{PhysicalPoint, PhysicalSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub(crate) enum CaptionButtonKind {
    // Discriminants are load-bearing: `CaptionButtonKinds::with` /
    // `::contains` use `1 << kind as u8` as a bitmask. Reordering or
    // inserting a variant silently breaks every consumer.
    Minimize = 0,
    Maximize = 1,
    Close = 2,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum CaptionButtonAction {
    Close,
    Minimize,
    Maximize,
    Restore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PointerDeviceKind {
    Mouse,
    Pen,
    Touch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Availability {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonInteraction {
    Idle,
    Hovered,
    Pressed,
    PressedDraggedOff,
}

#[derive(Debug, Clone, Copy)]
struct PressSession {
    pointer_id: u32,
    captured_kind: CaptionButtonKind,
    device: PointerDeviceKind,
}

/// Bitset of which caption-button kinds are visible on a window. Derived from
/// `WindowStyle` flags at strip-construction time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct CaptionButtonKinds(u8);

impl CaptionButtonKinds {
    pub fn empty() -> Self { Self(0) }
    pub fn with(self, kind: CaptionButtonKind) -> Self {
        Self(self.0 | (1 << kind as u8))
    }
    pub fn contains(self, kind: CaptionButtonKind) -> bool {
        (self.0 & (1 << kind as u8)) != 0
    }

    /// Yields the visible kinds in left-to-right system order
    /// (Minimize → Maximize → Close). Single source of truth used by
    /// both hit-testing (`StripGeometry::hit_test`) and layout
    /// (`CaptionButtonStrip::relayout`, `CaptionButtonStrip::new`).
    pub fn iter_ordered(self) -> impl Iterator<Item = CaptionButtonKind> {
        [
            CaptionButtonKind::Minimize,
            CaptionButtonKind::Maximize,
            CaptionButtonKind::Close,
        ]
        .into_iter()
        .filter(move |kind| self.contains(*kind))
    }
}

// CaptionButtonStrip definition lives here once Phase 5 adds it.
pub(crate) struct CaptionButtonStrip {
    _placeholder: (),
}
```

- [ ] **Step 3: `cargo check` should now pass (Task 3.2 + the new file together)**

Run: `cd D:/repos/kotlin-desktop-toolkit/native && cargo check -p desktop-win32`
Expected: PASS.

- [ ] **Step 4: Add `resolve_interaction` tests**

Append to `caption_buttons.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn session(kind: CaptionButtonKind, device: PointerDeviceKind) -> PressSession {
        PressSession { pointer_id: 1, captured_kind: kind, device }
    }

    #[test]
    fn disabled_button_is_idle_regardless_of_input() {
        let r = resolve_interaction(CaptionButtonKind::Maximize, Availability::Disabled, Some(CaptionButtonKind::Maximize), Some(PointerDeviceKind::Mouse), None);
        assert_eq!(r, ButtonInteraction::Idle);
    }

    #[test]
    fn no_pointer_no_press_is_idle() {
        let r = resolve_interaction(CaptionButtonKind::Close, Availability::Enabled, None, None, None);
        assert_eq!(r, ButtonInteraction::Idle);
    }

    #[test]
    fn pointer_over_self_with_mouse_is_hovered() {
        let r = resolve_interaction(CaptionButtonKind::Close, Availability::Enabled, Some(CaptionButtonKind::Close), Some(PointerDeviceKind::Mouse), None);
        assert_eq!(r, ButtonInteraction::Hovered);
    }

    #[test]
    fn pointer_over_self_with_pen_is_hovered() {
        let r = resolve_interaction(CaptionButtonKind::Close, Availability::Enabled, Some(CaptionButtonKind::Close), Some(PointerDeviceKind::Pen), None);
        assert_eq!(r, ButtonInteraction::Hovered);
    }

    #[test]
    fn pointer_over_self_with_touch_skips_hover() {
        let r = resolve_interaction(CaptionButtonKind::Close, Availability::Enabled, Some(CaptionButtonKind::Close), Some(PointerDeviceKind::Touch), None);
        assert_eq!(r, ButtonInteraction::Idle);
    }

    #[test]
    fn captured_self_with_pointer_inside_is_pressed() {
        let s = session(CaptionButtonKind::Close, PointerDeviceKind::Mouse);
        let r = resolve_interaction(CaptionButtonKind::Close, Availability::Enabled, Some(CaptionButtonKind::Close), Some(PointerDeviceKind::Mouse), Some(&s));
        assert_eq!(r, ButtonInteraction::Pressed);
    }

    #[test]
    fn captured_self_with_pointer_outside_is_pressed_dragged_off() {
        let s = session(CaptionButtonKind::Close, PointerDeviceKind::Mouse);
        let r = resolve_interaction(CaptionButtonKind::Close, Availability::Enabled, None, Some(PointerDeviceKind::Mouse), Some(&s));
        assert_eq!(r, ButtonInteraction::PressedDraggedOff);
    }

    #[test]
    fn captured_other_button_keeps_self_idle_winui_capture_rule() {
        // Press is on Minimize; pointer moves over Close. Close stays Idle.
        let s = session(CaptionButtonKind::Minimize, PointerDeviceKind::Mouse);
        let r = resolve_interaction(CaptionButtonKind::Close, Availability::Enabled, Some(CaptionButtonKind::Close), Some(PointerDeviceKind::Mouse), Some(&s));
        assert_eq!(r, ButtonInteraction::Idle);
    }

    #[test]
    fn captured_other_with_touch_keeps_self_idle() {
        // Same WinUI capture rule under touch input — the second button
        // still receives no Hovered/PointerEntered while another button
        // owns capture.
        let s = session(CaptionButtonKind::Minimize, PointerDeviceKind::Touch);
        let r = resolve_interaction(CaptionButtonKind::Close, Availability::Enabled, Some(CaptionButtonKind::Close), Some(PointerDeviceKind::Touch), Some(&s));
        assert_eq!(r, ButtonInteraction::Idle);
    }
}
```

- [ ] **Step 5: Implement `resolve_interaction`**

Add (above the `#[cfg(test)]` block):

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
            if is_pointer_over_self { ButtonInteraction::Pressed } else { ButtonInteraction::PressedDraggedOff }
        }
        Some(_) => ButtonInteraction::Idle,
        None if is_pointer_over_self => match pointer_device {
            Some(PointerDeviceKind::Touch) => ButtonInteraction::Idle,
            _ => ButtonInteraction::Hovered,
        },
        None => ButtonInteraction::Idle,
    }
}
```

- [ ] **Step 6: Add visible-button-derivation tests**

Inside `mod tests`:

```rust
    use crate::win32::window_api::{WindowStyle, WindowSystemBackdropType, WindowTitleBarKind};

    fn style_with(is_min: bool, is_max: bool, is_resize: bool) -> WindowStyle {
        WindowStyle {
            title_bar_kind: WindowTitleBarKind::Custom,
            is_resizable: is_resize,
            is_minimizable: is_min,
            is_maximizable: is_max,
            system_backdrop_type: WindowSystemBackdropType::Auto,
        }
    }

    #[test]
    fn all_three_buttons_visible_for_default_overlapped_window() {
        let kinds = CaptionButtonKinds::from_style(&style_with(true, true, true));
        assert!(kinds.contains(CaptionButtonKind::Minimize));
        assert!(kinds.contains(CaptionButtonKind::Maximize));
        assert!(kinds.contains(CaptionButtonKind::Close));
    }

    #[test]
    fn close_is_always_visible_even_when_min_max_disallowed() {
        let kinds = CaptionButtonKinds::from_style(&style_with(false, false, false));
        assert!(!kinds.contains(CaptionButtonKind::Minimize));
        assert!(!kinds.contains(CaptionButtonKind::Maximize));
        assert!(kinds.contains(CaptionButtonKind::Close));
    }

    #[test]
    fn min_and_max_are_both_visible_when_only_minimize_is_allowed() {
        let kinds = CaptionButtonKinds::from_style(&style_with(true, false, true));
        assert!(kinds.contains(CaptionButtonKind::Minimize));
        assert!(kinds.contains(CaptionButtonKind::Maximize));
        assert!(kinds.contains(CaptionButtonKind::Close));
    }

    #[test]
    fn min_and_max_are_both_visible_when_only_maximize_is_allowed() {
        let kinds = CaptionButtonKinds::from_style(&style_with(false, true, true));
        assert!(kinds.contains(CaptionButtonKind::Minimize));
        assert!(kinds.contains(CaptionButtonKind::Maximize));
        assert!(kinds.contains(CaptionButtonKind::Close));
    }

    #[test]
    fn min_only_style_disables_maximize_but_keeps_minimize_enabled() {
        let style = style_with(true, false, true);
        assert_eq!(availability_from_style(CaptionButtonKind::Minimize, &style), Availability::Enabled);
        assert_eq!(availability_from_style(CaptionButtonKind::Maximize, &style), Availability::Disabled);
    }

    #[test]
    fn max_only_style_disables_minimize_but_keeps_maximize_enabled() {
        let style = style_with(false, true, true);
        assert_eq!(availability_from_style(CaptionButtonKind::Minimize, &style), Availability::Disabled);
        assert_eq!(availability_from_style(CaptionButtonKind::Maximize, &style), Availability::Enabled);
    }

    #[test]
    fn non_resizable_disables_maximize_even_when_maximizable_bit_is_set() {
        // Spec §4.2 Maximize policy: requires is_resizable && is_maximizable.
        let style = style_with(true, true, false);
        assert_eq!(availability_from_style(CaptionButtonKind::Minimize, &style), Availability::Enabled);
        assert_eq!(availability_from_style(CaptionButtonKind::Maximize, &style), Availability::Disabled);
    }
```

- [ ] **Step 7: Implement `CaptionButtonKinds::from_style` and `availability_from_style`**

Inside `impl CaptionButtonKinds`:

```rust
    pub fn from_style(style: &crate::win32::window_api::WindowStyle) -> Self {
        let mut kinds = Self::empty().with(CaptionButtonKind::Close);
        if style.is_minimizable || style.is_maximizable {
            kinds = kinds.with(CaptionButtonKind::Minimize);
            kinds = kinds.with(CaptionButtonKind::Maximize);
        }
        kinds
    }
```

Add this helper beside `CaptionButtonKinds::from_style`; the constructor in Task 5.1 must use it rather than repeating the match inline:

```rust
fn availability_from_style(
    kind: CaptionButtonKind,
    style: &crate::win32::window_api::WindowStyle,
) -> Availability {
    match kind {
        CaptionButtonKind::Minimize if !style.is_minimizable => Availability::Disabled,
        // Maximize requires both `is_resizable` and `is_maximizable` per spec §4.2:
        // non-resizable windows have no maximize semantics in this toolkit.
        CaptionButtonKind::Maximize if !(style.is_resizable && style.is_maximizable) => Availability::Disabled,
        _ => Availability::Enabled,
    }
}
```

- [ ] **Step 8: Run tests + stage**

Run: `cd D:/repos/kotlin-desktop-toolkit/native && cargo test -p desktop-win32 caption_buttons`
Expected: 16 tests pass (9 `resolve_interaction` + 7 visible-button-derivation).

```bash
git add native/desktop-win32/src/win32/window.rs native/desktop-win32/src/win32/caption_buttons.rs native/desktop-win32/src/win32/mod.rs
```

### Task 4.2: `CaptionTheme` palette + `CaptionButtonMetrics` + their tests

**Files:** Modify: `native/desktop-win32/src/win32/caption_buttons.rs`

- [ ] **Step 1: Add the import for `Appearance` and `HighContrast`**

```rust
use super::appearance::{Appearance, HighContrast};
```

- [ ] **Step 2: Add the struct + resolve fn**

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
    close_backplate_hover: windows::UI::Color,
    close_backplate_pressed: windows::UI::Color,
    close_foreground_hover: windows::UI::Color,
    close_foreground_pressed: windows::UI::Color,
}

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> windows::UI::Color {
    windows::UI::Color { A: a, R: r, G: g, B: b }
}

impl CaptionTheme {
    fn resolve(appearance: Appearance, hc: HighContrast) -> Self {
        match (hc, appearance) {
            (HighContrast::On, _) => Self::high_contrast(),
            (HighContrast::Off, Appearance::Light) => Self::light(),
            (HighContrast::Off, Appearance::Dark)  => Self::dark(),
        }
    }

    // WinUI Fluent palette: `microsoft/microsoft-ui-xaml@5f9e851133b…`.
    // Close-specific reds: `microsoft/terminal@e4e3f08efca…` MinMaxCloseControl.xaml
    // (`Opacity 0.9` → α=0xE6; `Opacity 0.7` → α=0xB3 — valid only because the source RGB is fully opaque; both rounded to nearest).
    fn light() -> Self {
        Self {
            backplate_rest: rgba(0, 0, 0, 0),
            backplate_hover: rgba(0, 0, 0, 0x09),                // SubtleFillColorSecondary
            backplate_pressed: rgba(0, 0, 0, 0x06),              // SubtleFillColorTertiary
            backplate_inactive: rgba(0, 0, 0, 0),
            foreground_rest: rgba(0, 0, 0, 0xE4),                // TextFillColorPrimary
            foreground_hover: rgba(0, 0, 0, 0xE4),
            foreground_pressed: rgba(0, 0, 0, 0x9E),             // TextFillColorSecondary
            foreground_disabled: rgba(0, 0, 0, 0x5C),            // TextFillColorDisabled
            foreground_inactive: rgba(0, 0, 0, 0x5C),
            close_backplate_hover: rgba(0xC4, 0x2B, 0x1C, 0xFF),
            close_backplate_pressed: rgba(0xC4, 0x2B, 0x1C, 0xE6),
            close_foreground_hover: rgba(0xFF, 0xFF, 0xFF, 0xFF),
            close_foreground_pressed: rgba(0xFF, 0xFF, 0xFF, 0xB3),
        }
    }

    fn dark() -> Self {
        Self {
            backplate_rest: rgba(0, 0, 0, 0),
            backplate_hover: rgba(0xFF, 0xFF, 0xFF, 0x0F),       // SubtleFillColorSecondary
            backplate_pressed: rgba(0xFF, 0xFF, 0xFF, 0x0A),     // SubtleFillColorTertiary
            backplate_inactive: rgba(0, 0, 0, 0),
            foreground_rest: rgba(0xFF, 0xFF, 0xFF, 0xFF),       // TextFillColorPrimary
            foreground_hover: rgba(0xFF, 0xFF, 0xFF, 0xFF),
            foreground_pressed: rgba(0xFF, 0xFF, 0xFF, 0xC5),    // TextFillColorSecondary
            foreground_disabled: rgba(0xFF, 0xFF, 0xFF, 0x5D),   // TextFillColorDisabled
            foreground_inactive: rgba(0xFF, 0xFF, 0xFF, 0x5D),
            close_backplate_hover: rgba(0xC4, 0x2B, 0x1C, 0xFF),
            close_backplate_pressed: rgba(0xC4, 0x2B, 0x1C, 0xE6),
            close_foreground_hover: rgba(0xFF, 0xFF, 0xFF, 0xFF),
            close_foreground_pressed: rgba(0xFF, 0xFF, 0xFF, 0xB3),
        }
    }

    fn high_contrast() -> Self {
        use windows::Win32::Graphics::Gdi::{
            COLOR_BTNFACE, COLOR_BTNTEXT, COLOR_GRAYTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT,
        };
        let face = sys_color(COLOR_BTNFACE);
        let text = sys_color(COLOR_BTNTEXT);
        let highlight = sys_color(COLOR_HIGHLIGHT);
        let highlight_text = sys_color(COLOR_HIGHLIGHTTEXT);
        let grayed = sys_color(COLOR_GRAYTEXT);
        Self {
            backplate_rest: face,
            backplate_hover: highlight,
            backplate_pressed: highlight,
            backplate_inactive: face,
            foreground_rest: text,
            foreground_hover: highlight_text,
            foreground_pressed: highlight_text,
            foreground_disabled: grayed,
            foreground_inactive: grayed,
            close_backplate_hover: highlight,
            close_backplate_pressed: highlight,
            close_foreground_hover: highlight_text,
            close_foreground_pressed: highlight_text,
        }
    }
}

fn sys_color(index: SYS_COLOR_INDEX) -> windows::UI::Color {
    // SAFETY: `index` is a documented Win32 system-color identifier from
    // `windows::Win32::Graphics::Gdi::COLOR_*`. `GetSysColor` is thread-safe.
    let colorref = unsafe { GetSysColor(index) };
    windows::UI::Color {
        A: 0xFF,
        R: (colorref & 0xFF) as u8,
        G: ((colorref >> 8) & 0xFF) as u8,
        B: ((colorref >> 16) & 0xFF) as u8,
    }
}
```

`GetSysColor`, `SYS_COLOR_INDEX`, and the `COLOR_*` constants live in `windows::Win32::Graphics::Gdi` (already enabled by the toolkit's `Win32_Graphics_Gdi` Cargo feature). Add `GetSysColor` and `SYS_COLOR_INDEX` to the file-level `use` block alongside the other `windows` imports introduced by Phase 4.

- [ ] **Step 3: Add tests**

```rust
    #[test]
    fn light_theme_resolves_to_black_foreground_rest() {
        let theme = CaptionTheme::resolve(Appearance::Light, HighContrast::Off);
        assert_eq!(theme.foreground_rest, rgba(0, 0, 0, 0xE4));
    }

    #[test]
    fn dark_theme_resolves_to_white_foreground_rest() {
        let theme = CaptionTheme::resolve(Appearance::Dark, HighContrast::Off);
        assert_eq!(theme.foreground_rest.R, 0xFF);
    }

    #[test]
    fn close_button_hover_red_is_systemwide_in_both_themes() {
        let light = CaptionTheme::resolve(Appearance::Light, HighContrast::Off);
        let dark  = CaptionTheme::resolve(Appearance::Dark,  HighContrast::Off);
        assert_eq!(light.close_backplate_hover, dark.close_backplate_hover);
        assert_eq!(light.close_backplate_hover.R, 0xC4);
    }

    #[test]
    fn close_pressed_alpha_is_e6_in_both_themes() {
        // Brush.Opacity 0.9 → alpha 0xE6 (0.9 × 255 = 229.5, rounded to nearest).
        // Identical light/dark because the source `#C42B1C` is fully opaque.
        let light = CaptionTheme::resolve(Appearance::Light, HighContrast::Off);
        let dark  = CaptionTheme::resolve(Appearance::Dark,  HighContrast::Off);
        assert_eq!(light.close_backplate_pressed.A, 0xE6);
        assert_eq!(dark.close_backplate_pressed.A, 0xE6);
    }

    #[test]
    fn pressed_backplate_is_more_subtle_than_hover_in_off_branch() {
        // Fluent invariant: SubtleFillColorTertiary (pressed) has lower alpha
        // than SubtleFillColorSecondary (hover).
        let light = CaptionTheme::resolve(Appearance::Light, HighContrast::Off);
        assert!(light.backplate_pressed.A < light.backplate_hover.A);
        let dark = CaptionTheme::resolve(Appearance::Dark, HighContrast::Off);
        assert!(dark.backplate_pressed.A < dark.backplate_hover.A);
    }

    #[test]
    fn disabled_foreground_differs_from_rest_foreground() {
        let light = CaptionTheme::resolve(Appearance::Light, HighContrast::Off);
        assert_ne!(light.foreground_disabled, light.foreground_rest);
        let dark = CaptionTheme::resolve(Appearance::Dark, HighContrast::Off);
        assert_ne!(dark.foreground_disabled, dark.foreground_rest);
    }

    #[test]
    fn high_contrast_pressed_matches_hover() {
        // HC reads system colours, so absolute bytes vary; what holds across
        // all four shipped HC themes is hover == pressed.
        let hc = CaptionTheme::resolve(Appearance::Light, HighContrast::On);
        assert_eq!(hc.backplate_hover, hc.backplate_pressed);
        assert_eq!(hc.foreground_hover, hc.foreground_pressed);
    }
```

- [ ] **Step 4: Add `CaptionButtonMetrics` tests**

```rust
    #[test]
    fn metrics_at_100_percent_dpi_match_epx_values_directly() {
        let m = CaptionButtonMetrics::new(1.0);
        assert_eq!(m.button_size_px.width.0, 46);
        assert_eq!(m.button_size_px.height.0, 32);
        assert_eq!(m.glyph_extent_px.width.0, 10);
        assert_eq!(m.glyph_extent_px.height.0, 10);
    }

    #[test]
    fn metrics_at_150_percent_dpi_round_correctly() {
        let m = CaptionButtonMetrics::new(1.5);
        // Values match LogicalSize::to_physical's rounding
        // (floor(v.mul_add(scale, 0.5))).
        assert_eq!((m.button_size_px.width.0, m.button_size_px.height.0), (69, 48));
        assert_eq!((m.glyph_extent_px.width.0, m.glyph_extent_px.height.0), (15, 15));
    }

    #[test]
    fn metrics_at_200_percent_dpi_double() {
        let m = CaptionButtonMetrics::new(2.0);
        assert_eq!((m.button_size_px.width.0, m.button_size_px.height.0), (92, 64));
        assert_eq!((m.glyph_extent_px.width.0, m.glyph_extent_px.height.0), (20, 20));
    }
```

- [ ] **Step 5: Implement `CaptionButtonMetrics`**

```rust
#[derive(Debug, Clone, Copy)]
pub(crate) struct CaptionButtonMetrics {
    pub button_size_px: PhysicalSize,
    pub glyph_extent_px: PhysicalSize,
}

impl CaptionButtonMetrics {
    pub fn new(scale: f32) -> Self {
        Self {
            button_size_px: LogicalSize::new(46.0, 32.0).to_physical(scale),
            glyph_extent_px: LogicalSize::new(10.0, 10.0).to_physical(scale),
        }
    }
}
```

Add `LogicalSize` to the existing `super::geometry` import in `caption_buttons.rs`.

- [ ] **Step 6: Run tests + stage**

Run: `cargo test -p desktop-win32 caption_buttons`
Expected: 26 tests pass (16 from Task 4.1 + 7 palette + 3 metrics).

```bash
git add native/desktop-win32/src/win32/caption_buttons.rs
```

### Task 4.3: TDD — `hit_test` geometry + `hittest_for_caption_button_kind` / `caption_button_kind_for_hittest`

**Files:** Modify: `native/desktop-win32/src/win32/caption_buttons.rs`

- [ ] **Step 1: Add a struct stub for the strip's geometry-only fields**

(Full `CaptionButtonStrip` lands in Phase 5; for hit-test testing we extract a small struct.)

```rust
struct StripGeometry {
    /// Width of the coordinate space `point` is in. Pass full client width
    /// for client-space points (unit tests) or `strip_width_px` for
    /// strip-local points (`CaptionButtonStrip::hit_test`).
    reference_width_px: i32,
    metrics: CaptionButtonMetrics,
    visible_kinds: CaptionButtonKinds,
}

impl StripGeometry {
    fn hit_test(&self, point: PhysicalPoint) -> Option<CaptionButtonKind> {
        let bw = self.metrics.button_size_px.width.0;
        let bh = self.metrics.button_size_px.height.0;
        if point.y.0 < 0 || point.y.0 >= bh { return None; }
        let count = self.visible_kinds.0.count_ones() as i32;
        let strip_left = self.reference_width_px - bw * count;
        for (i, kind) in self.visible_kinds.iter_ordered().enumerate() {
            let x_left = strip_left + (i as i32) * bw;
            if point.x.0 >= x_left && point.x.0 < x_left + bw {
                return Some(kind);
            }
        }
        None
    }
}
```

- [ ] **Step 2: Add tests**

```rust
    use super::PhysicalPoint;
    use super::super::geometry::PhysicalPixels;

    fn pt(x: i32, y: i32) -> PhysicalPoint {
        PhysicalPoint { x: PhysicalPixels(x), y: PhysicalPixels(y) }
    }

    fn ltr_geometry(width: i32, kinds: CaptionButtonKinds) -> StripGeometry {
        StripGeometry { reference_width_px: width, metrics: CaptionButtonMetrics::new(1.0), visible_kinds: kinds }
    }

    #[test]
    fn hits_close_in_rightmost_46px() {
        let g = ltr_geometry(800, CaptionButtonKinds::empty().with(CaptionButtonKind::Minimize).with(CaptionButtonKind::Maximize).with(CaptionButtonKind::Close));
        assert_eq!(g.hit_test(pt(799, 16)), Some(CaptionButtonKind::Close));
        assert_eq!(g.hit_test(pt(754, 16)), Some(CaptionButtonKind::Close));
    }

    #[test]
    fn hits_maximize_left_of_close() {
        let g = ltr_geometry(800, CaptionButtonKinds::empty().with(CaptionButtonKind::Minimize).with(CaptionButtonKind::Maximize).with(CaptionButtonKind::Close));
        assert_eq!(g.hit_test(pt(753, 16)), Some(CaptionButtonKind::Maximize));
        assert_eq!(g.hit_test(pt(708, 16)), Some(CaptionButtonKind::Maximize));
    }

    #[test]
    fn hits_minimize_left_of_maximize() {
        let g = ltr_geometry(800, CaptionButtonKinds::empty().with(CaptionButtonKind::Minimize).with(CaptionButtonKind::Maximize).with(CaptionButtonKind::Close));
        assert_eq!(g.hit_test(pt(707, 16)), Some(CaptionButtonKind::Minimize));
        assert_eq!(g.hit_test(pt(662, 16)), Some(CaptionButtonKind::Minimize));
    }

    #[test]
    fn no_hit_outside_strip_height() {
        let g = ltr_geometry(800, CaptionButtonKinds::empty().with(CaptionButtonKind::Close));
        assert_eq!(g.hit_test(pt(799, 32)), None);  // y == height is outside
        assert_eq!(g.hit_test(pt(799, -1)), None);
    }

    #[test]
    fn no_hit_for_hidden_button_kinds_not_in_visible_set() {
        let g = ltr_geometry(800, CaptionButtonKinds::empty().with(CaptionButtonKind::Close));
        // Maximize would have lived at x in [708, 753] if visible. With Maximize hidden,
        // that x range collapses — the layout shifts left; (753) is now outside Close.
        assert_eq!(g.hit_test(pt(753, 16)), None);
    }
```

- [ ] **Step 3: Add `hittest_for_caption_button_kind` + tests**

Spec §7.1 requires automated coverage for the disabled-Min/Max → `HTCAPTION` rule. Extract the policy into a pure helper so it can be unit-tested without a wndproc:

```rust
/// Map a caption-button hit to the WM_NCHITTEST return code per §4.2:
/// enabled Min/Max/Close → HTMINBUTTON / HTMAXBUTTON / HTCLOSE; visible
/// disabled Min/Max → HTCAPTION (drag region) so the rectangle stays in the
/// title-bar surface and Snap Layouts is *not* advertised on a disabled
/// Maximize. Takes `is_enabled: bool` (rather than the private `Availability`
/// enum) so `event_loop.rs` can call it without leaking the enum.
pub(crate) fn hittest_for_caption_button_kind(kind: CaptionButtonKind, is_enabled: bool) -> u32 {
    use windows::Win32::UI::WindowsAndMessaging::{HTCAPTION, HTCLOSE, HTMAXBUTTON, HTMINBUTTON};
    match (kind, is_enabled) {
        (CaptionButtonKind::Close, _) => HTCLOSE,
        (CaptionButtonKind::Minimize, true) => HTMINBUTTON,
        (CaptionButtonKind::Maximize, true) => HTMAXBUTTON,
        (CaptionButtonKind::Minimize | CaptionButtonKind::Maximize, false) => HTCAPTION,
    }
}
```

Inside `mod tests`:

```rust
    use windows::Win32::UI::WindowsAndMessaging::{HTCAPTION, HTCLOSE, HTMAXBUTTON, HTMINBUTTON};

    #[test]
    fn enabled_min_max_close_return_their_dedicated_codes() {
        assert_eq!(hittest_for_caption_button_kind(CaptionButtonKind::Minimize, true), HTMINBUTTON);
        assert_eq!(hittest_for_caption_button_kind(CaptionButtonKind::Maximize, true), HTMAXBUTTON);
        assert_eq!(hittest_for_caption_button_kind(CaptionButtonKind::Close, true), HTCLOSE);
    }

    #[test]
    fn disabled_min_max_collapse_to_htcaption_not_their_codes() {
        // Snap Layouts must not appear on a disabled Maximize button; the
        // rectangle stays in the title-bar drag region.
        assert_eq!(hittest_for_caption_button_kind(CaptionButtonKind::Minimize, false), HTCAPTION);
        assert_eq!(hittest_for_caption_button_kind(CaptionButtonKind::Maximize, false), HTCAPTION);
        // Close is always enabled, but the policy still returns HTCLOSE for completeness.
        assert_eq!(hittest_for_caption_button_kind(CaptionButtonKind::Close, false), HTCLOSE);
    }
```

- [ ] **Step 3-bis: Add `caption_button_kind_for_hittest` (inverse helper) + test**

The three pointer arms in Task 6.3 each need to recover a `CaptionButtonKind` from `HIWORD(WM_NCPOINTER* wParam)`; centralize that policy here.

```rust
/// Inverse of `hittest_for_caption_button_kind`: recover the strip's
/// button kind from the `HIWORD(WM_NCPOINTER* wParam)` hit-test code, or
/// `None` for non-caption-button hit-tests so the wndproc falls through.
pub(crate) fn caption_button_kind_for_hittest(hit_test: u32) -> Option<CaptionButtonKind> {
    use windows::Win32::UI::WindowsAndMessaging::{HTCLOSE, HTMAXBUTTON, HTMINBUTTON};
    match hit_test {
        HTCLOSE => Some(CaptionButtonKind::Close),
        HTMAXBUTTON => Some(CaptionButtonKind::Maximize),
        HTMINBUTTON => Some(CaptionButtonKind::Minimize),
        _ => None,
    }
}
```

Inside `mod tests`:

```rust
    #[test]
    fn hittest_to_kind_matches_three_caption_codes() {
        use windows::Win32::UI::WindowsAndMessaging::{HTCLIENT, HTCLOSE, HTMAXBUTTON, HTMINBUTTON};
        assert_eq!(caption_button_kind_for_hittest(HTCLOSE), Some(CaptionButtonKind::Close));
        assert_eq!(caption_button_kind_for_hittest(HTMAXBUTTON), Some(CaptionButtonKind::Maximize));
        assert_eq!(caption_button_kind_for_hittest(HTMINBUTTON), Some(CaptionButtonKind::Minimize));
        assert_eq!(caption_button_kind_for_hittest(HTCLIENT), None);
    }
```

- [ ] **Step 4: Run tests, confirm pass, commit**

Run: `cargo test -p desktop-win32 caption_buttons`
Expected: 34 tests pass.

```bash
git add native/desktop-win32/src/win32/caption_buttons.rs
git commit -m "feat(win32): caption_buttons pure-logic module + Window strip / NC-leave-tracking fields"
```

---

## Phase 5 — `CaptionButtonStrip` rendering, animation, lifecycle

### Task 5.1: Replace `CaptionButtonStrip` placeholder with the full struct + constructor

**Files:** Modify: `native/desktop-win32/src/win32/caption_buttons.rs`

- [ ] **Step 1: Replace the placeholder with the full struct and constructor**

Replace `pub(crate) struct CaptionButtonStrip { _placeholder: () }` with:

```rust
pub(crate) struct CaptionButtonStrip {
    composition_root: ContainerVisual,
    buttons: Vec<CaptionButton>,
    visible_kinds: CaptionButtonKinds,
    is_active: bool,
    is_window_maximized: bool,
    appearance: Appearance,
    high_contrast: HighContrast,
    metrics: CaptionButtonMetrics,
    pointer_over_kind: Option<CaptionButtonKind>,
    pointer_device: Option<PointerDeviceKind>,
    press_session: Option<PressSession>,
    d2d_context: Rc<D2dContext>,                                       // clone of the singleton from `composition::ensure_d2d_context`
    device_replaced_registration: RenderingDeviceReplacedRegistration, // dropping it removes the RDR subscription (§6.2)
    // CompositorController gives both `Compositor()` (for visual creation
    // during invalidation paths) and `Commit()` (for frame publication).
    compositor_controller: CompositorController,
}

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
    glyph_brush: CompositionColorBrush,                   // Source of the SpriteVisual's CompositionMaskBrush
    glyph_surface: CompositionDrawingSurface,
}

impl CaptionButtonStrip {
    pub fn new(
        chrome_layer: &ContainerVisual,
        initial_scale: f32,
        style: &crate::win32::window_api::WindowStyle,
        compositor_controller: CompositorController,
        hwnd: HWND,
    ) -> anyhow::Result<Self> {
        let compositor = compositor_controller.Compositor()?;
        let d2d_context = crate::win32::composition::ensure_d2d_context(compositor.clone())?;
        // Subscribe to RenderingDeviceReplaced; see spec §6.2 for the
        // reentrancy contract.
        let device_replaced_registration = {
            let hwnd_value = hwnd.0 as isize;
            d2d_context.add_rendering_device_replaced_callback(move || unsafe {
                let _ = PostMessageW(
                    Some(HWND(hwnd_value as _)),
                    crate::win32::event_loop::WM_APP_CAPTION_BUTTONS_RENDERING_DEVICE_REPLACED,
                    WPARAM(0),
                    LPARAM(0),
                );
            })?
        };
        let composition_root = compositor.CreateContainerVisual()?;
        chrome_layer.Children()?.InsertAtTop(&composition_root)?;

        let visible_kinds = CaptionButtonKinds::from_style(style);
        let metrics = CaptionButtonMetrics::new(initial_scale);

        let mut buttons = Vec::new();
        for kind in visible_kinds.iter_ordered() {
            let availability = availability_from_style(kind, style);
            buttons.push(create_caption_button(&compositor, &composition_root, &d2d_context, kind, availability, &metrics)?);
        }

        // Seed appearance + HC from live state (spec §4.2); on failure, fall
        // back to Light / Off — the next appearance event self-corrects.
        let appearance = Appearance::get_current()
            .inspect_err(|err| log::warn!("CaptionButtonStrip: failed to read initial appearance, defaulting to Light: {err}"))
            .unwrap_or(Appearance::Light);
        let high_contrast = HighContrast::get_current()
            .inspect_err(|err| log::warn!("CaptionButtonStrip: failed to read initial high-contrast state, defaulting to Off: {err}"))
            .unwrap_or(HighContrast::Off);

        let mut strip = Self {
            composition_root,
            buttons,
            visible_kinds,
            is_active: false,
            is_window_maximized: false,
            appearance,
            high_contrast,
            metrics,
            pointer_over_kind: None,
            pointer_device: None,
            press_session: None,
            d2d_context,
            device_replaced_registration,
            compositor_controller,
        };
        strip.relayout()?;
        strip.rasterise_all_glyphs()?;
        strip.apply_visuals_to_all_buttons()?;
        strip.compositor_controller.Commit()?;
        Ok(strip)
    }

}
```

Imports the strip needs (in addition to existing): `HWND`, `WPARAM`, `LPARAM` from `windows::Win32::Foundation`; `PostMessageW` from `windows::Win32::UI::WindowsAndMessaging`; `RenderingDeviceReplacedRegistration` from `crate::win32::composition`.

- [ ] **Step 2: Add the `create_caption_button` helper**

```rust
fn create_caption_button(
    compositor: &windows::UI::Composition::Compositor,
    parent: &ContainerVisual,
    d2d_context: &Rc<D2dContext>,
    kind: CaptionButtonKind,
    availability: Availability,
    metrics: &CaptionButtonMetrics,
) -> anyhow::Result<CaptionButton> {
    use windows::Graphics::SizeInt32;
    use windows_numerics::Vector2;

    let backplate = compositor.CreateSpriteVisual()?;
    let backplate_brush = compositor.CreateColorBrushWithColor(rgba(0, 0, 0, 0))?;
    backplate.SetBrush(&backplate_brush)?;
    backplate.SetSize(Vector2::new(metrics.button_size_px.width.0 as f32, metrics.button_size_px.height.0 as f32))?;
    parent.Children()?.InsertAtTop(&backplate)?;

    // Mask-brush topology: see spec §4.3.
    let glyph_surface = d2d_context.create_drawing_surface(SizeInt32 {
        Width: metrics.glyph_extent_px.width.0,
        Height: metrics.glyph_extent_px.height.0,
    })?;
    let glyph_surface_brush = compositor.CreateSurfaceBrushWithSurface(&glyph_surface)?;
    let glyph_brush = compositor.CreateColorBrushWithColor(rgba(0, 0, 0, 0xFF))?;
    let glyph_mask_brush = compositor.CreateMaskBrush()?;
    glyph_mask_brush.SetSource(&glyph_brush)?;
    glyph_mask_brush.SetMask(&glyph_surface_brush)?;
    let glyph = compositor.CreateSpriteVisual()?;
    glyph.SetBrush(&glyph_mask_brush)?;
    glyph.SetSize(Vector2::new(metrics.glyph_extent_px.width.0 as f32, metrics.glyph_extent_px.height.0 as f32))?;
    backplate.Children()?.InsertAtTop(&glyph)?;

    Ok(CaptionButton {
        kind,
        availability,
        visuals: CaptionButtonVisuals {
            backplate,
            backplate_brush,
            glyph,
            glyph_brush,
            glyph_surface,
        },
        last_applied_interaction: ButtonInteraction::Idle,
        glyph_surface_dirty: true,
    })
}
```

- [ ] **Step 3: Add stub bodies for `relayout`, `rasterise_all_glyphs`, `apply_visuals_to_all_buttons`**

```rust
impl CaptionButtonStrip {
    fn relayout(&mut self) -> anyhow::Result<()> { Ok(()) }            // implemented in 5.2
    fn rasterise_all_glyphs(&mut self) -> anyhow::Result<()> { Ok(()) } // implemented in 5.4
    fn apply_visuals_to_all_buttons(&mut self) -> anyhow::Result<()> { Ok(()) } // implemented in 5.5
}
```

- [ ] **Step 4: Verify and stage**

Run: `cargo build -p desktop-win32`
Expected: PASS (the strip exists but does nothing visible yet).

```bash
git add native/desktop-win32/src/win32/caption_buttons.rs
```

### Task 5.2: Implement `relayout` — position buttons along the right edge

**Files:** Modify: `native/desktop-win32/src/win32/caption_buttons.rs`

- [ ] **Step 1: Replace the `relayout` stub**

```rust
    fn relayout(&mut self) -> anyhow::Result<()> {
        use windows_numerics::Vector2;
        let bw = self.metrics.button_size_px.width.0;
        let bh = self.metrics.button_size_px.height.0;
        let total_width = bw * self.buttons.len() as i32;
        // Buttons line up at increasing x within the strip's parent;
        // `set_strip_position` places the parent at top-right of `chrome_layer`.
        self.composition_root.SetSize(Vector2::new(total_width as f32, bh as f32))?;
        for (i, button) in self.buttons.iter_mut().enumerate() {
            let x = (i as i32) * bw;
            button.visuals.backplate.SetOffset(windows_numerics::Vector3 { X: x as f32, Y: 0.0, Z: 0.0 })?;
            button.visuals.backplate.SetSize(Vector2::new(bw as f32, bh as f32))?;
            let gw = self.metrics.glyph_extent_px.width.0;
            let gh = self.metrics.glyph_extent_px.height.0;
            let gx = (bw - gw) / 2;
            let gy = (bh - gh) / 2;
            button.visuals.glyph.SetOffset(windows_numerics::Vector3 { X: gx as f32, Y: gy as f32, Z: 0.0 })?;
            button.visuals.glyph.SetSize(Vector2::new(gw as f32, gh as f32))?;
        }
        Ok(())
    }
```

- [ ] **Step 2: Add `set_strip_position` for the wndproc to position the strip in client coordinates**

```rust
    pub(crate) fn set_strip_position(
        &self,
        client_size: PhysicalSize,
        max_chrome_y: i32,
    ) -> anyhow::Result<()> {
        let bw = self.metrics.button_size_px.width.0;
        let total_width = bw * self.buttons.len() as i32;
        let x = client_size.width.0 - total_width;
        self.composition_root.SetOffset(windows_numerics::Vector3 {
            X: x as f32,
            Y: max_chrome_y as f32,
            Z: 0.0,
        })?;
        Ok(())
    }
```

- [ ] **Step 3: Verify and stage**

Run: `cargo build -p desktop-win32`
Expected: PASS.

```bash
git add native/desktop-win32/src/win32/caption_buttons.rs
```

### Task 5.3: Glyph rasterisation with DirectWrite + D2D

**Files:** Modify: `native/desktop-win32/src/win32/caption_buttons.rs`

- [ ] **Step 1: Implement `rasterise_all_glyphs`**

```rust
    fn rasterise_all_glyphs(&mut self) -> anyhow::Result<()> {
        for button in self.buttons.iter_mut() {
            if button.glyph_surface_dirty {
                if rasterise_glyph(&self.d2d_context, &button.visuals.glyph_surface, button.kind, self.is_window_maximized, self.high_contrast, &self.metrics)? {
                    button.glyph_surface_dirty = false;
                }
            }
        }
        Ok(())
    }
```

- [ ] **Step 2: Add `rasterise_glyph` free function**

```rust
fn rasterise_glyph(
    d2d_context: &Rc<D2dContext>,
    surface: &CompositionDrawingSurface,
    kind: CaptionButtonKind,
    is_maximised: bool,
    hc: HighContrast,
    metrics: &CaptionButtonMetrics,
) -> anyhow::Result<bool> {
    let glyph_char = glyph_for(kind, is_maximised, hc);
    let dwrite = d2d_context.dwrite_factory();
    let (font_family, font_face) = caption_glyph_font_family(&dwrite)?;
    let font_size = compute_glyph_font_size(&font_face, glyph_char, metrics.glyph_extent_px)?;
    let format = unsafe {
        dwrite.CreateTextFormat(
            font_family,
            None::<&windows::Win32::Graphics::DirectWrite::IDWriteFontCollection>,
            windows::Win32::Graphics::DirectWrite::DWRITE_FONT_WEIGHT_REGULAR,
            windows::Win32::Graphics::DirectWrite::DWRITE_FONT_STYLE_NORMAL,
            windows::Win32::Graphics::DirectWrite::DWRITE_FONT_STRETCH_NORMAL,
            font_size,
            windows::core::w!("en-US"),
        )?
    };
    unsafe {
        format.SetTextAlignment(windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_ALIGNMENT_CENTER)?;
        format.SetParagraphAlignment(windows::Win32::Graphics::DirectWrite::DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
    }
    let mut text_buf = [0u16; 2];
    let text: &[u16] = glyph_char.encode_utf16(&mut text_buf);
    let drew = d2d_context.with_d2d_render_target(surface, |rt, offset| {
        unsafe {
            // Pin to 96 DPI so 1 DIP == 1 pixel; `compute_glyph_font_size`
            // assumes that mapping.
            rt.SetDpi(96.0, 96.0);
            let clear_color = windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
            rt.Clear(Some(&raw const clear_color));
            let brush = rt.CreateSolidColorBrush(
                &windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 },
                None,
            )?;
            let rect = windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F {
                left: offset.x as f32,
                top: offset.y as f32,
                right: offset.x as f32 + metrics.glyph_extent_px.width.0 as f32,
                bottom: offset.y as f32 + metrics.glyph_extent_px.height.0 as f32,
            };
            rt.DrawText(text, &format, &rect, &brush, windows::Win32::Graphics::Direct2D::D2D1_DRAW_TEXT_OPTIONS_NONE, windows::Win32::Graphics::DirectWrite::DWRITE_MEASURING_MODE_NATURAL);
        }
        Ok(())
    })?.is_some();
    Ok(drew)
}

fn glyph_for(kind: CaptionButtonKind, is_maximised: bool, hc: HighContrast) -> char {
    match (kind, is_maximised, hc) {
        (CaptionButtonKind::Minimize, _, HighContrast::Off) => '\u{E921}',
        (CaptionButtonKind::Maximize, false, HighContrast::Off) => '\u{E922}',
        (CaptionButtonKind::Maximize, true,  HighContrast::Off) => '\u{E923}',
        (CaptionButtonKind::Close,    _, HighContrast::Off) => '\u{E8BB}',
        (CaptionButtonKind::Minimize, _, HighContrast::On) => '\u{EF2D}',
        (CaptionButtonKind::Maximize, false, HighContrast::On) => '\u{EF2E}',
        (CaptionButtonKind::Maximize, true,  HighContrast::On) => '\u{EF2F}',
        (CaptionButtonKind::Close,    _, HighContrast::On) => '\u{EF2C}',
    }
}

/// Resolve the system font collection's first available caption-glyph family
/// (Segoe Fluent Icons, falling back to Segoe MDL2 Assets) and produce a
/// concrete `IDWriteFontFace` for it. Returning the face here lets the caller
/// run glyph-metric queries without re-opening the font collection or
/// re-resolving the family — a single round-trip per rasterise.
fn caption_glyph_font_family(
    dwrite: &windows::Win32::Graphics::DirectWrite::IDWriteFactory,
) -> anyhow::Result<(
    windows::core::PCWSTR,
    windows::Win32::Graphics::DirectWrite::IDWriteFontFace,
)> {
    use windows::Win32::Graphics::DirectWrite::{
        DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_REGULAR,
    };
    let mut collection = None;
    unsafe { dwrite.GetSystemFontCollection(&raw mut collection, false)? };
    let collection = collection.ok_or_else(|| anyhow::anyhow!("DirectWrite returned no system font collection"))?;
    for family_name in [
        windows::core::w!("Segoe Fluent Icons"),
        windows::core::w!("Segoe MDL2 Assets"),
    ] {
        let mut index = 0u32;
        let mut exists = windows_core::BOOL(0);
        unsafe { collection.FindFamilyName(family_name, &raw mut index, &raw mut exists)? };
        if exists.as_bool() {
            let family = unsafe { collection.GetFontFamily(index)? };
            let font = unsafe {
                family.GetFirstMatchingFont(
                    DWRITE_FONT_WEIGHT_REGULAR,
                    DWRITE_FONT_STRETCH_NORMAL,
                    DWRITE_FONT_STYLE_NORMAL,
                )?
            };
            let face = unsafe { font.CreateFontFace()? };
            return Ok((family_name, face));
        }
    }
    anyhow::bail!("neither Segoe Fluent Icons nor Segoe MDL2 Assets is present in the system font collection");
}

/// Compute the DirectWrite font size (DIPs) at which the glyph's visible
/// black-box fits within `target_extent_px`. Algorithm in spec §4.5. Caller
/// supplies the already-resolved `IDWriteFontFace` (produced by
/// `caption_glyph_font_family`) so this function does not reopen the system
/// font collection.
fn compute_glyph_font_size(
    face: &windows::Win32::Graphics::DirectWrite::IDWriteFontFace,
    glyph_char: char,
    target_extent_px: PhysicalSize,
) -> anyhow::Result<f32> {
    use windows::Win32::Graphics::DirectWrite::{DWRITE_FONT_METRICS, DWRITE_GLYPH_METRICS};

    let codepoint = glyph_char as u32;
    let mut glyph_index: u16 = 0;
    unsafe { face.GetGlyphIndices(&raw const codepoint, 1, &raw mut glyph_index)? };
    if glyph_index == 0 {
        // GetGlyphIndices returns 0 for `.notdef` (missing in font's CMAP).
        anyhow::bail!("caption glyph U+{:04X} maps to .notdef in selected font", codepoint);
    }

    let mut glyph_metrics = DWRITE_GLYPH_METRICS::default();
    unsafe {
        face.GetDesignGlyphMetrics(&raw const glyph_index, 1, &raw mut glyph_metrics, false)?;
    }
    let mut font_metrics = DWRITE_FONT_METRICS::default();
    unsafe { face.GetMetrics(&raw mut font_metrics) };

    let design_units_per_em = i32::from(font_metrics.designUnitsPerEm);
    if design_units_per_em <= 0 {
        anyhow::bail!("DirectWrite returned designUnitsPerEm = {design_units_per_em}");
    }

    let bbox_w = (glyph_metrics.advanceWidth as i32)
        - glyph_metrics.leftSideBearing
        - glyph_metrics.rightSideBearing;
    // Horizontal-layout cell height per DWRITE_FONT_METRICS — `ascent + descent`,
    // not `glyph_metrics.advanceHeight` (which is the *vertical* advance).
    let cell_height_du = i32::from(font_metrics.ascent) + i32::from(font_metrics.descent);
    let bbox_h = cell_height_du
        - glyph_metrics.topSideBearing
        - glyph_metrics.bottomSideBearing;
    if bbox_w <= 0 || bbox_h <= 0 {
        anyhow::bail!("DirectWrite returned non-positive glyph bbox: {bbox_w}x{bbox_h}");
    }

    // Per-axis font_size that fits each axis; take the smaller.
    // `rasterise_glyph` pins the render target's DPI to 96, so 1 DIP == 1
    // pixel inside the draw block.
    let dpem = design_units_per_em as f32;
    let font_size_x = (target_extent_px.width.0 as f32) * dpem / (bbox_w as f32);
    let font_size_y = (target_extent_px.height.0 as f32) * dpem / (bbox_h as f32);
    Ok(font_size_x.min(font_size_y))
}
```

- [ ] **Step 3: Verify and stage**

Run: `cargo build -p desktop-win32`
Expected: PASS.

```bash
git add native/desktop-win32/src/win32/caption_buttons.rs
```

### Task 5.4: Apply theme + `Hovered → Idle` animation to all buttons

**Files:** Modify: `native/desktop-win32/src/win32/caption_buttons.rs`

This task lands the animation-aware versions of `apply_visuals_to_all_buttons` and `apply_button_visuals` directly. The `Hovered → Idle` transition is the only animated transition (per spec §5.2: 150ms backplate, 100ms glyph, no easing); every other transition jumps via `SetColor`.

- [ ] **Step 1: Implement `apply_visuals_to_all_buttons`**

```rust
    fn apply_visuals_to_all_buttons(&mut self) -> anyhow::Result<()> {
        // Re-rasterise dirty glyphs (spec §6.2 reactive device-loss heal).
        self.rasterise_all_glyphs()?;

        let theme = CaptionTheme::resolve(self.appearance, self.high_contrast);
        for button in self.buttons.iter_mut() {
            let new_interaction = resolve_interaction(
                button.kind,
                button.availability,
                self.pointer_over_kind,
                self.pointer_device,
                self.press_session.as_ref(),
            );
            apply_button_visuals(button, new_interaction, &theme, self.is_active)?;
        }
        Ok(())
    }
```

- [ ] **Step 2: Add `apply_button_visuals`, `animate_color`, and `colours_for` free functions**

```rust
fn apply_button_visuals(
    button: &mut CaptionButton,
    new_interaction: ButtonInteraction,
    theme: &CaptionTheme,
    is_active: bool,
) -> anyhow::Result<()> {
    let (backplate, foreground) = colours_for(button.kind, button.availability, new_interaction, theme, is_active);
    let prev = button.last_applied_interaction;
    // Spec §5.2: animate only `Hovered → Idle` on Enabled buttons; everything else jumps.
    let is_hover_leave = prev == ButtonInteraction::Hovered
        && new_interaction == ButtonInteraction::Idle
        && button.availability == Availability::Enabled;

    if is_hover_leave {
        animate_color(&button.visuals.backplate_brush, backplate, std::time::Duration::from_millis(150))?;
        animate_color(&button.visuals.glyph_brush,     foreground, std::time::Duration::from_millis(100))?;
    } else {
        button.visuals.backplate_brush.SetColor(backplate)?;
        button.visuals.glyph_brush.SetColor(foreground)?;
    }
    button.last_applied_interaction = new_interaction;
    Ok(())
}

fn animate_color(
    brush: &CompositionColorBrush,
    target: windows::UI::Color,
    duration: std::time::Duration,
) -> anyhow::Result<()> {
    let anim = brush.Compositor()?.CreateColorKeyFrameAnimation()?;
    anim.SetDuration(windows::Foundation::TimeSpan { Duration: (duration.as_nanos() / 100) as i64 })?;
    // Both keyframes set explicitly (docs don't pin the implicit start);
    // matches Terminal's behaviour on rapid restart.
    anim.InsertKeyFrame(0.0, brush.Color()?)?;
    anim.InsertKeyFrame(1.0, target)?;
    brush.StartAnimation(windows::core::h!("Color"), &anim)?;
    Ok(())
}

fn colours_for(
    kind: CaptionButtonKind,
    availability: Availability,
    interaction: ButtonInteraction,
    theme: &CaptionTheme,
    is_active: bool,
) -> (windows::UI::Color, windows::UI::Color) {
    if availability == Availability::Disabled {
        return (theme.backplate_rest, theme.foreground_disabled);
    }
    if !is_active {
        return (theme.backplate_inactive, theme.foreground_inactive);
    }
    if kind == CaptionButtonKind::Close {
        match interaction {
            ButtonInteraction::Hovered => return (theme.close_backplate_hover, theme.close_foreground_hover),
            ButtonInteraction::Pressed => return (theme.close_backplate_pressed, theme.close_foreground_pressed),
            _ => {}
        }
    }
    match interaction {
        ButtonInteraction::Idle | ButtonInteraction::PressedDraggedOff => (theme.backplate_rest, theme.foreground_rest),
        ButtonInteraction::Hovered => (theme.backplate_hover, theme.foreground_hover),
        ButtonInteraction::Pressed => (theme.backplate_pressed, theme.foreground_pressed),
    }
}
```

- [ ] **Step 3: Verify and stage**

Run: `cargo build -p desktop-win32`
Expected: PASS.

```bash
git add native/desktop-win32/src/win32/caption_buttons.rs
```

### Task 5.5: Strip's pointer / leave handlers

**Files:** Modify: `native/desktop-win32/src/win32/caption_buttons.rs`

- [ ] **Step 1: Add the pointer-routing methods**

```rust
impl CaptionButtonStrip {
    pub fn hit_test(&self, point: PhysicalPoint) -> Option<CaptionButtonKind> {
        StripGeometry {
            reference_width_px: self.strip_width_px(),
            metrics: self.metrics,   // CaptionButtonMetrics derives Copy
            visible_kinds: self.visible_kinds,
        }
        .hit_test(point)
    }

    pub fn strip_width_px(&self) -> i32 {
        self.metrics.button_size_px.width.0 * self.buttons.len() as i32
    }

    pub fn on_pointer_update(&mut self, kind: Option<CaptionButtonKind>, _pointer_id: u32, device: PointerDeviceKind) -> anyhow::Result<()> {
        // Pointer is over `kind` (None = left strip area but still in NC).
        if self.pointer_over_kind != kind || self.pointer_device != Some(device) {
            self.pointer_over_kind = kind;
            self.pointer_device = Some(device);
            self.apply_visuals_to_all_buttons()?;
            self.compositor_controller.Commit()?;
        }
        Ok(())
    }

    pub fn on_pointer_down(
        &mut self,
        kind: CaptionButtonKind,
        pointer_id: u32,
        device: PointerDeviceKind,
    ) -> anyhow::Result<()> {
        if self.press_session.is_some() { return Ok(()); }   // already pressing; ignore
        // Don't capture disabled buttons.
        if self.button_for(kind).map(|b| b.availability) != Some(Availability::Enabled) { return Ok(()); }
        self.press_session = Some(PressSession { pointer_id, captured_kind: kind, device });
        self.pointer_over_kind = Some(kind);
        self.pointer_device = Some(device);
        self.apply_visuals_to_all_buttons()?;
        self.compositor_controller.Commit()?;
        Ok(())
    }

    pub fn on_pointer_up(
        &mut self,
        kind_under_pointer: Option<CaptionButtonKind>,
        pointer_id: u32,
    ) -> anyhow::Result<Option<CaptionButtonAction>> {
        let session = match self.press_session {
            Some(s) if s.pointer_id == pointer_id => s,
            _ => return Ok(None),
        };
        self.press_session = None;
        let action = if Some(session.captured_kind) == kind_under_pointer {
            Some(self.action_for(session.captured_kind))
        } else {
            None
        };
        self.pointer_over_kind = kind_under_pointer;
        self.apply_visuals_to_all_buttons()?;
        self.compositor_controller.Commit()?;
        Ok(action)
    }

    pub fn on_pointer_cancel(&mut self, pointer_id: u32) -> anyhow::Result<()> {
        let should_cancel = matches!(self.press_session, Some(s) if s.pointer_id == pointer_id);
        if should_cancel {
            self.press_session = None;
            self.apply_visuals_to_all_buttons()?;
            self.compositor_controller.Commit()?;
        }
        Ok(())
    }

    pub fn on_nc_mouse_leave(&mut self) -> anyhow::Result<()> {
        self.pointer_over_kind = None;
        self.pointer_device = None;
        self.apply_visuals_to_all_buttons()?;
        self.compositor_controller.Commit()?;
        Ok(())
    }

    fn button_for(&self, kind: CaptionButtonKind) -> Option<&CaptionButton> {
        self.buttons.iter().find(|b| b.kind == kind)
    }

    pub fn is_enabled(&self, kind: CaptionButtonKind) -> bool {
        self.button_for(kind).map(|b| b.availability) == Some(Availability::Enabled)
    }

    /// True iff the strip owns a press session for `pointer_id`. The id
    /// match is load-bearing under multi-pointer input — a mouse release
    /// while a touch holds capture must not trip the wndproc's strip-owned
    /// suppression and swallow Kotlin's `PointerUp`.
    pub(crate) fn has_active_press_for(&self, pointer_id: u32) -> bool {
        matches!(self.press_session, Some(s) if s.pointer_id == pointer_id)
    }

    fn action_for(&self, kind: CaptionButtonKind) -> CaptionButtonAction {
        match kind {
            CaptionButtonKind::Close => CaptionButtonAction::Close,
            CaptionButtonKind::Minimize => CaptionButtonAction::Minimize,
            CaptionButtonKind::Maximize => if self.is_window_maximized { CaptionButtonAction::Restore } else { CaptionButtonAction::Maximize },
        }
    }
}
```

- [ ] **Step 2: Verify and stage**

Run: `cargo build -p desktop-win32`
Expected: PASS.

```bash
git add native/desktop-win32/src/win32/caption_buttons.rs
```

### Task 5.6: Strip's invalidation handlers (activate/dpi/appearance/device-replaced/max state/resize)

**Files:** Modify: `native/desktop-win32/src/win32/caption_buttons.rs`

- [ ] **Step 1: Add the methods**

```rust
impl CaptionButtonStrip {
    pub fn on_activate(&mut self, is_active: bool) -> anyhow::Result<()> {
        if self.is_active != is_active {
            self.is_active = is_active;
            // Spec §5.2: `is_active` flips never animate. Resetting per-button
            // history keeps the `Hovered → Idle` predicate false across the flip.
            for button in self.buttons.iter_mut() {
                button.last_applied_interaction = ButtonInteraction::Idle;
            }
            self.apply_visuals_to_all_buttons()?;
            self.compositor_controller.Commit()?;
        }
        Ok(())
    }

    pub fn on_dpi_change(&mut self, new_scale: f32) -> anyhow::Result<()> {
        self.metrics = CaptionButtonMetrics::new(new_scale);
        for button in self.buttons.iter_mut() {
            // Resize the glyph surface to the new physical pixels.
            let new_size = windows::Graphics::SizeInt32 {
                Width: self.metrics.glyph_extent_px.width.0,
                Height: self.metrics.glyph_extent_px.height.0,
            };
            button.visuals.glyph_surface.Resize(new_size)?;
            button.glyph_surface_dirty = true;
        }
        self.relayout()?;
        self.apply_visuals_to_all_buttons()?;
        self.compositor_controller.Commit()?;
        Ok(())
    }

    pub fn on_appearance_change(&mut self, appearance: Appearance, hc: HighContrast) -> anyhow::Result<()> {
        let glyph_invalidate = self.high_contrast != hc;
        self.appearance = appearance;
        self.high_contrast = hc;
        if glyph_invalidate {
            for button in self.buttons.iter_mut() { button.glyph_surface_dirty = true; }
        }
        self.apply_visuals_to_all_buttons()?;
        self.compositor_controller.Commit()?;
        Ok(())
    }

    pub fn on_rendering_device_replaced(&mut self) -> anyhow::Result<()> {
        for button in self.buttons.iter_mut() {
            button.glyph_surface_dirty = true;
        }
        self.rasterise_all_glyphs()?;
        self.apply_visuals_to_all_buttons()?;
        self.compositor_controller.Commit()?;
        Ok(())
    }

    pub fn on_max_state_change(&mut self, is_maximized: bool) -> anyhow::Result<()> {
        if self.is_window_maximized != is_maximized {
            self.is_window_maximized = is_maximized;
            for button in self.buttons.iter_mut() {
                if button.kind == CaptionButtonKind::Maximize {
                    button.glyph_surface_dirty = true;
                }
            }
            self.rasterise_all_glyphs()?;
            self.compositor_controller.Commit()?;
        }
        Ok(())
    }

    pub fn on_resize(
        &self,
        client_size: PhysicalSize,
        max_chrome_y: i32,
    ) -> anyhow::Result<()> {
        self.set_strip_position(client_size, max_chrome_y)?;
        self.compositor_controller.Commit()?;
        Ok(())
    }
}
```

- [ ] **Step 2: Verify and commit Phase 5**

Run: `cargo build -p desktop-win32`
Expected: PASS.

```bash
git add native/desktop-win32/src/win32/caption_buttons.rs
git commit -m "feat(win32): CaptionButtonStrip rendering, theme, pointer state, and Hovered→Idle animation"
```

---

## Phase 6 — Wndproc integration

### Task 6.1: Window initializes the strip on `WindowTitleBarKind::Custom`

**Files:** Modify: `native/desktop-win32/src/win32/window.rs`

> **Adjacent code-touch:** add `if !self.is_resizable() { return; }` at the top of `Window::maximize()` to match spec §4.2's policy.

- [ ] **Step 1: After `initialize_content` completes, attempt to construct the strip**

Locate `initialize_window`. After `initialize_content(window, hwnd)?;` add:

```rust
    if window.has_custom_title_bar() {
        let chrome_layer = window.chrome_layer()?;
        let strip = crate::win32::caption_buttons::CaptionButtonStrip::new(
            &chrome_layer,
            window.get_scale(),
            &*window.style.borrow(),
            window.compositor_controller.clone(),
            hwnd,
        )?;
        let mut client_rect = RECT::default();
        unsafe { GetClientRect(hwnd, &raw mut client_rect)? };
        strip.set_strip_position(
            PhysicalSize::new(
                client_rect.right - client_rect.left,
                client_rect.bottom - client_rect.top,
            ),
            0,
        )?;
        window.caption_buttons.replace(Some(strip));
    }
```

The initial `set_strip_position` is required so first paint and first `WM_NCHITTEST` agree before any later resize / `WM_NCCALCSIZE`. Add any missing `GetClientRect`, `RECT`, `PhysicalSize`, `HWND` imports.

- [ ] **Step 2: Verify and stage**

Run: `cargo build -p desktop-win32`
Expected: PASS.

```bash
git add native/desktop-win32/src/win32/window.rs
```

### Task 6.2: `event_loop.rs` — extend `on_nchittest` to consult the strip

**Files:** Modify: `native/desktop-win32/src/win32/event_loop.rs`

- [ ] **Step 1: Insert strip consultation before preserving the default non-client result**

In `on_nchittest`, narrow the existing early-return guard so non-resizable Custom-titlebar windows still reach the strip routing:

```rust
    if !window.has_custom_title_bar() {
        return None;
    }
```

The strip is hit-test-routable on every Custom-titlebar window, including non-resizable ones. The `is_resizable()` gate moves to the resize-border math itself (below).

Then modify the post-`DwmDefWindowProc` path where `on_nchittest` checks `original_ht != HTCLIENT`. **Delete `event_loop.rs:375-379` — the HTCLIENT early-return AND the duplicated `let mouse_x` / `let mouse_y` declarations** (the inserted snippet's leading two lines re-declare them):**

```rust
    if original_ht != LRESULT(HTCLIENT as _) {
        return Some(original_ht);
    }
    let mouse_x = GET_X_LPARAM!(lparam.0);
    let mouse_y = GET_Y_LPARAM!(lparam.0);
```

(Retain the existing `let original_ht = { ... DwmDefWindowProc / DefWindowProcW ... };` capture at `event_loop.rs:367-374` above; only lines 375-379 are deleted.) The new code below restores the `if original_ht != HTCLIENT` check *after* the strip-routing block has had a chance to claim points inside the strip's bounds.

Final body order: `let original_ht = ...` → strip-routing block (with its own `let mouse_x` / `let mouse_y`) → `if original_ht != HTCLIENT { return Some(original_ht); }` → existing `NCHitTestEvent` dispatch.

```rust
    let mouse_x = GET_X_LPARAM!(lparam.0);
    let mouse_y = GET_Y_LPARAM!(lparam.0);

    // Consult the caption-button strip first.
    if let Some(strip) = window.caption_buttons.borrow().as_ref() {
        // Convert from screen-relative to client-space, then to strip-relative
        // coordinates using the actual client width and strip width.
        let mut client_point = POINT { x: mouse_x, y: mouse_y };
        let _ = unsafe { ScreenToClient(hwnd, &raw mut client_point) };
        let mut client_rect = RECT::default();
        let _ = unsafe { GetClientRect(hwnd, &raw mut client_rect) };
        let strip_left = client_rect.right - strip.strip_width_px();
        if client_point.x >= strip_left
            && client_point.x < client_rect.right
            && client_point.y >= client_rect.top
            && client_point.y < client_rect.bottom
        {
            let strip_relative = PhysicalPoint {
                x: PhysicalPixels(client_point.x - strip_left),
                y: PhysicalPixels(client_point.y - client_rect.top),
            };
            if let Some(kind) = strip.hit_test(strip_relative) {
                // Pure-helper unit-tested in Task 4.3 (`hittest_for_caption_button_kind`).
                // Trusts the archived-KB plus local empirical policy: visible
                // disabled Min/Max collapse to HTCAPTION (no Snap flyout).
                return Some(LRESULT(
                    hittest_for_caption_button_kind(kind, strip.is_enabled(kind)) as _,
                ));
            }
        }
    }

    if original_ht != LRESULT(HTCLIENT as _) {
        return Some(original_ht);
    }

    let event = NCHitTestEvent { mouse_x, mouse_y };
    let handled = event_loop.handle_event(window, event);
    if handled.is_some() {
        return Some(LRESULT(HTCLIENT as _));
    }
    // ... existing manual top-edge resize-border math follows ...
```

In the manual top-edge / title-bar fallback block, compute `resize_handle_height` (and the derived `title_bar_height`) unconditionally; gate only the `is_on_resize_border` check on `window.is_resizable()`. This keeps the title-bar drag-region height consistent across resizable and non-resizable Custom-titlebar windows.

```rust
    let current_dpi = unsafe { GetDpiForWindow(hwnd) };
    let resize_handle_height = unsafe {
        GetSystemMetricsForDpi(SM_CXPADDEDBORDER, current_dpi)
            + GetSystemMetricsForDpi(SM_CYSIZEFRAME, current_dpi)
    };
    let title_bar_height = resize_handle_height + unsafe { GetSystemMetricsForDpi(SM_CYSIZE, current_dpi) };
    let is_on_resize_border = window.is_resizable() && mouse_y < (window_rect.top + resize_handle_height) as _;
```

(Use the `strip_width_px` accessor from Task 5.5. Imports: `GetClientRect` (`WindowsAndMessaging`); `ScreenToClient` (or `MapWindowPoints`) (`windows::Win32::Graphics::Gdi`); `HTCAPTION`/`HTTOP`/`HTCLIENT` (manual fallback only — the strip block goes through `hittest_for_caption_button_kind`); `CaptionButtonKind`, `hittest_for_caption_button_kind`, `PhysicalPoint`, `PhysicalPixels`.)

- [ ] **Step 2: Verify and stage**

Run: `cargo build -p desktop-win32`
Expected: PASS.

```bash
git add native/desktop-win32/src/win32/event_loop.rs native/desktop-win32/src/win32/caption_buttons.rs
```

### Task 6.3: Extend pointer handlers to route to the strip

**Files:** Modify: `native/desktop-win32/src/win32/pointer.rs`, `native/desktop-win32/src/win32/event_loop.rs`.

- [ ] **Step 1: Add a `pointer_id` accessor on `PointerInfo`**

In `pointer.rs`, add to the `impl PointerInfo` block (alongside `get_pointer_state`, `get_timestamp`, etc.):

```rust
    pub(crate) const fn pointer_id(&self) -> u32 {
        self.get_native_pointer_info().pointerId
    }
```

This reads `POINTER_INFO.pointerId`, which is populated by `GetPointerInfo` / `GetPointerTouchInfo` / `GetPointerPenInfo` to the same value `try_from_message` extracted from `LOWORD(wparam.0)`. The accessor matches the existing `get_*` accessor pattern on `PointerInfo` and keeps `wparam`-decoding logic out of the wndproc layer.

- [ ] **Step 2: Add an event-loop helper for pointer device kind**

In `event_loop.rs`, add a helper that maps the existing enum variants:

```rust
fn device_kind_for(pointer_info: &PointerInfo) -> PointerDeviceKind {
    match pointer_info {
        PointerInfo::Touch(_) => PointerDeviceKind::Touch,
        PointerInfo::Pen(_) => PointerDeviceKind::Pen,
        PointerInfo::Common(_) => PointerDeviceKind::Mouse,
    }
}
```

- [ ] **Step 3: In `on_pointerupdate`, after pointer info extraction, route to the strip when over a caption-button hit area**

**Insert after the existing `let pointer_info = PointerInfo::try_from_message(wparam).ok()?;` line and before the `is_pointer_in_window` dispatch.** Caption-button hit-tests return `Some(LRESULT(0))` to suppress the Kotlin-facing pointer events (spec §1, §4.2); `kind = None` (title-bar drag area) falls through to the existing dispatch.

```rust
    if msg == WM_NCPOINTERUPDATE && window.has_custom_title_bar() {
        let kind = caption_button_kind_for_hittest(HIWORD!(wparam.0) as u32);
        if let Some(strip) = window.caption_buttons.borrow_mut().as_mut() {
            let device = device_kind_for(&pointer_info);
            let _ = window.ensure_nc_leave_tracking();
            let _ = strip.on_pointer_update(kind, pointer_info.pointer_id(), device);
        }
        if kind.is_some() {
            // First-entry parity (spec §3.5): mirror `on_pointerupdate`'s
            // else-branch so Kotlin gets a `PointerEntered` even when the
            // pointer's first appearance is over a caption button.
            if !window.is_pointer_in_window() {
                window.set_is_pointer_in_window(true);
                event_loop.handle_event(window, Event::PointerEntered(PointerEnteredEvent {
                    location_in_window: pointer_info.get_location_in_window(),
                    location_on_screen: pointer_info.get_physical_location(),
                    state: pointer_info.get_pointer_state(),
                    timestamp: pointer_info.get_timestamp(),
                }));
            }
            // Strip events are private — suppress the Kotlin dispatch.
            return Some(LRESULT(0));
        }
    }
```

- [ ] **Step 4: Filter primary-button presses, then route `on_pointerdown` / `on_pointerup` to the strip**

Filter primary per spec §4.2. Primary presses over caption-button hit-tests are routed to the strip and suppressed; non-primary falls through. For `WM_NCPOINTERUP`, the gate is `is_primary && strip_owns_press`.

**Insert at the top of `on_pointerdown`, immediately after the existing
`let pointer_info = PointerInfo::try_from_message(wparam).ok()?;` line and
before the existing `pointer_info.get_pointer_button_change()` block. The
`return Some(LRESULT(0))` path must pre-empt the existing Kotlin
`PointerDown` dispatch for caption-button hit-test areas.**

```rust
    if msg == WM_NCPOINTERDOWN && window.has_custom_title_bar() {
        let kind = caption_button_kind_for_hittest(HIWORD!(wparam.0) as u32);
        if let Some(kind) = kind {
            let button_change = pointer_info.get_pointer_button_change();
            let is_primary = button_change.kind() == PointerButtonChangeKind::Pressed
                && button_change.button() == PointerButton::Left;
            if is_primary {
                if let Some(strip) = window.caption_buttons.borrow_mut().as_mut() {
                    let device = device_kind_for(&pointer_info);
                    let _ = strip.on_pointer_down(kind, pointer_info.pointer_id(), device);
                }
                return Some(LRESULT(0));
            }
            // Non-primary falls through.
        }
    }
```

**Insert at the top of `on_pointerup`, immediately after the existing
`let pointer_info = PointerInfo::try_from_message(wparam).ok()?;` line and
before the existing `pointer_info.get_pointer_button_change()` block. The
`return Some(LRESULT(0))` path must pre-empt the existing Kotlin `PointerUp`
dispatch *and* the existing `if is_non_client { None } else { result }`
fallback at the function's tail.**

```rust
    if msg == WM_NCPOINTERUP && window.has_custom_title_bar() {
        let button_change = pointer_info.get_pointer_button_change();
        let is_primary = button_change.kind() == PointerButtonChangeKind::Released
            && button_change.button() == PointerButton::Left;
        let strip_owns_press = window.caption_buttons.borrow().as_ref()
            .map(|s| s.has_active_press_for(pointer_info.pointer_id()))
            .unwrap_or(false);
        if is_primary && strip_owns_press {
            let kind_under_pointer = caption_button_kind_for_hittest(HIWORD!(wparam.0) as u32);
            let action = window.caption_buttons.borrow_mut().as_mut()
                .and_then(|strip| strip.on_pointer_up(kind_under_pointer, pointer_info.pointer_id())
                    .inspect_err(|err| log::warn!("strip on_pointer_up failed: {err}"))
                    .ok()
                    .flatten());
            if let Some(action) = action {
                match action {
                    CaptionButtonAction::Close    => { let _ = window.request_close(); }
                    CaptionButtonAction::Minimize => window.minimize(),
                    CaptionButtonAction::Maximize => window.maximize(),
                    CaptionButtonAction::Restore  => window.restore(),
                }
            }
            return Some(LRESULT(0));
        }
    }
```

(Add `CaptionButtonAction`, `PointerDeviceKind`, `PointerButton`, `PointerButtonChangeKind`, and `caption_button_kind_for_hittest` imports.)

Replace the trailing comment at `event_loop.rs:556` (currently `// WM_NCPOINTERUP should always return None so that the window buttons work`) with:

```rust
    // Strip-claimed primary releases return early above; other NC releases
    // pass through so DefWindowProc handles them.
    if is_non_client { None } else { result }
```

- [ ] **Step 5: Add cleanup-only `WM_POINTERCAPTURECHANGED` handling**

Add `WM_POINTERCAPTURECHANGED` to the `WindowsAndMessaging` imports and add a wndproc dispatch arm:

```rust
            WM_POINTERCAPTURECHANGED => on_pointercapturechanged(window, wparam),
```

Add the handler:

```rust
fn on_pointercapturechanged(window: &Window, wparam: WPARAM) -> Option<LRESULT> {
    if !window.has_custom_title_bar() {
        return None;
    }
    if let Some(strip) = window.caption_buttons.borrow_mut().as_mut() {
        let pointer_id = u32::from(LOWORD!(wparam.0));
        let _ = strip.on_pointer_cancel(pointer_id);
    }
    // Cancellation only. MSDN: mixing selective WM_NCPOINTER* consumption
    // with DefWindowProc fall-through is undefined.
    Some(LRESULT(0))
}
```

`WM_POINTERCAPTURECHANGED` carries the pointer id in `wparam` (no `POINTER_INFO`), so read `LOWORD(wparam.0)` directly. Must not dispatch `CaptionButtonAction`.

- [ ] **Step 6: Verify and stage**

Run: `cargo build -p desktop-win32`
Expected: PASS.

```bash
git add native/desktop-win32/src/win32/pointer.rs native/desktop-win32/src/win32/event_loop.rs
```

### Task 6.4: Extend `on_activate`, rendering-device redraw, `on_ncmouseleave`, `on_dpichanged`, `on_settingchange`, `on_windowposchanged`, `on_nccalcsize`

**Files:** Modify: `native/desktop-win32/src/win32/event_loop.rs`

- [ ] **Step 1: `on_activate`**

In `on_activate` (`event_loop.rs:303-317`), insert the strip dispatch between the existing `extend_content_into_titlebar` / `apply_system_backdrop` block and the `let event = WindowActivatedEvent { ... };` line:

```rust
    if let Some(strip) = window.caption_buttons.borrow_mut().as_mut() {
        let _ = strip.on_activate(is_active)
            .inspect_err(|err| log::warn!("strip on_activate failed: {err}"));
    }
```

- [ ] **Step 2: Add the per-window `RenderingDeviceReplaced` redraw arm**

This step adds the per-window `WM_APP_CAPTION_BUTTONS_RENDERING_DEVICE_REPLACED` wndproc dispatch. The strip registers the `RenderingDeviceReplaced` callback inside `CaptionButtonStrip::new` (Task 5.1); the closure `PostMessageW`'s this message to the owning window's HWND. The wndproc handler invokes `strip.on_rendering_device_replaced` outside the originating call stack (spec §6.2 reentrancy contract).

The constant itself lands in Task 1.2 step 3; only the dispatch arm is added here.

```rust
// inside window_proc's match:
WM_APP_CAPTION_BUTTONS_RENDERING_DEVICE_REPLACED => {
    if let Some(strip) = window.caption_buttons.borrow_mut().as_mut() {
        let _ = strip.on_rendering_device_replaced();
    }
    Some(LRESULT(0))
}
```

- [ ] **Step 3: `on_ncmouseleave`**

At the start of the function:

```rust
    if let Some(strip) = window.caption_buttons.borrow_mut().as_mut() {
        let _ = strip.on_nc_mouse_leave();
    }
    window.nc_leave_tracking_fired();
```

- [ ] **Step 4: `on_dpichanged`**

After the existing scale-update logic:

```rust
    if let Some(strip) = window.caption_buttons.borrow_mut().as_mut() {
        let _ = strip.on_dpi_change(window.get_scale());
    }
```

- [ ] **Step 5: Notify the strip on every appearance / high-contrast event**

The toolkit dispatches three independent system-theme events from `event_loop.rs`
(verified in tree at `event_loop.rs:266-300`):

1. `SystemAppearanceChangeEvent` from `WM_SETTINGCHANGE` `ImmersiveColorSet`.
2. `SystemHighContrastChangeEvent` from `WM_SETTINGCHANGE` `SPI_SETHIGHCONTRAST`.
3. `SystemHighContrastChangeEvent` from `WM_SYSCOLORCHANGE`.

`on_appearance_change` takes both `(appearance, high_contrast)`. Each event arm already has the live value for one axis; pass it through and query the orthogonal axis once (defaulted on failure) so the strip paints from the same tuple Kotlin received.

```rust
fn notify_strip_appearance(window: &Window, appearance: Appearance, high_contrast: HighContrast) {
    if let Some(strip) = window.caption_buttons.borrow_mut().as_mut() {
        let _ = strip.on_appearance_change(appearance, high_contrast)
            .inspect_err(|err| log::warn!("strip on_appearance_change failed: {err}"));
    }
}
```

`unwrap_or(...)` defaults match `PanicDefault` on `Appearance` / `HighContrast` (`appearance.rs:21-25, 60-64`); the strip self-corrects on the next event if the orthogonal-axis read fails.

**Site 1** — `on_settingchange`'s `ImmersiveColorSet` arm at [event_loop.rs:266-274](native/desktop-win32/src/win32/event_loop.rs#L266-L274). Insert immediately after the existing `event_loop.handle_event(window, event);`:

```rust
                let hc = HighContrast::get_current()
                    .inspect_err(|err| log::warn!("strip appearance notify: failed to read high-contrast state: {err}"))
                    .unwrap_or(HighContrast::Off);
                notify_strip_appearance(window, new_appearance, hc);
```

**Site 2** — `on_settingchange`'s `SPI_SETHIGHCONTRAST` arm at [event_loop.rs:277-284](native/desktop-win32/src/win32/event_loop.rs#L277-L284). Same shape, mirrored axes:

```rust
                let appearance = Appearance::get_current()
                    .inspect_err(|err| log::warn!("strip appearance notify: failed to read appearance: {err}"))
                    .unwrap_or(Appearance::Light);
                notify_strip_appearance(window, appearance, new_high_contrast);
```

**Site 3** — `on_syscolorchange` at [event_loop.rs:289-301](native/desktop-win32/src/win32/event_loop.rs#L289-L301). Same as site 2, after the existing `event_loop.handle_event(window, event);`:

```rust
    let appearance = Appearance::get_current()
        .inspect_err(|err| log::warn!("strip appearance notify: failed to read appearance: {err}"))
        .unwrap_or(Appearance::Light);
    notify_strip_appearance(window, appearance, new_high_contrast);
```

- [ ] **Step 6: `on_windowposchanged` — drive `on_max_state_change`**

In `on_windowposchanged`, after the existing `WindowMoveEvent` / `WindowResizeEvent` work and before returning `Some(LRESULT(0))`:

```rust
    if let Some(strip) = window.caption_buttons.borrow_mut().as_mut() {
        let now_maximized = unsafe { IsZoomed(window.hwnd()) }.as_bool();
        let _ = strip
            .on_max_state_change(now_maximized)
            .inspect_err(|err| log::warn!("strip on_max_state_change failed: {err}"));
    }
```

(Add `IsZoomed` to the `WindowsAndMessaging` imports.)

- [ ] **Step 7: `on_nccalcsize` — apply maximized inset and call `strip.on_resize`**

The current handler at `event_loop.rs:319-360` removes the standard frame for non-Custom titlebars and leaves the top inset at 0 for Custom. Per spec §3.6, when a maximized Custom-titlebar window is resizable, add the off-monitor extension on top (`SM_CYSIZEFRAME + SM_CXPADDEDBORDER`, matching Windows Terminal's `_OnNcCalcSize` at commit `e4e3f08efca9d0ffba330eee12edbcb16897ddcb`) and apply the auto-hide-taskbar 2-px claw-back. Both are Custom-titlebar-only.

Modify `on_nccalcsize`:

1. After the existing `AdjustWindowRectEx` + side/bottom subtraction and the existing `if !window.has_custom_title_bar() { ... }` top-inset branch, but BEFORE `let origin = ...`, add the maximized inset and the auto-hide-taskbar claw-back call:

   ```rust
       let is_maximized = unsafe { IsZoomed(hwnd) }.as_bool();
       // The off-monitor overhang only exists when `WS_THICKFRAME` is set
       // (spec §3.6); `WindowStyle::to_system` clears it when `!is_resizable`.
       // `max_chrome_y` stays 0 for non-resizable / non-Custom / non-maximized
       // windows so the strip's `set_strip_position` does not shift buttons
       // down into the title-bar zone.
       let mut max_chrome_y = 0;
       if window.is_resizable() && is_maximized && window.has_custom_title_bar() {
           let dpi = unsafe { GetDpiForWindow(hwnd) };
           max_chrome_y = unsafe {
               GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi)
                   + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi)
           };
           // The Custom-titlebar handler leaves the top inset at 0 so the
           // title-bar area stays in the client rect; add the maximized
           // overhang back here.
           calcsize_params.rgrc[0].top += max_chrome_y;

           // GH#1438 / GH#5209: 2-px claw-back so the cursor can still
           // reveal an auto-hide taskbar. Custom-titlebar only — see
           // spec §3.6 Notes for why standard-frame windows don't need it.
           let _ = apply_autohide_taskbar_inset(hwnd, &mut calcsize_params.rgrc[0])
               .inspect_err(|err| log::warn!("autohide taskbar inset failed: {err}"));
       }
   ```

   Add the helper as a private free function in `event_loop.rs` (alongside the other `on_*` helpers — not on `Window`, since it only needs the HWND and a mutable rect):

   ```rust
   /// Per Windows Terminal `_OnNcCalcSize` GH#1438 / GH#5209: when an
   /// auto-hide taskbar lives on an edge of the window's monitor, reduce
   /// the maximized client rect by 2 px on that edge so the cursor can
   /// reach the screen edge to trigger the taskbar reveal.
   fn apply_autohide_taskbar_inset(hwnd: HWND, rect: &mut RECT) -> anyhow::Result<()> {
       const AUTOHIDE_TASKBAR_SIZE: i32 = 2;

       let mut autohide = APPBARDATA::default();
       autohide.cbSize = size_of::<APPBARDATA>() as u32;
       let state = unsafe { SHAppBarMessage(ABM_GETSTATE, &raw mut autohide) } as u32;
       if state & ABS_AUTOHIDE == 0 {
           return Ok(());
       }

       let hmon = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
       if hmon.is_invalid() {
           anyhow::bail!("MonitorFromWindow returned invalid HMONITOR for HWND {hwnd:?}");
       }
       let mut mon_info = MONITORINFO {
           cbSize: size_of::<MONITORINFO>() as u32,
           ..Default::default()
       };
       if !unsafe { GetMonitorInfoW(hmon, &raw mut mon_info) }.as_bool() {
           anyhow::bail!("GetMonitorInfoW failed for HMONITOR {hmon:?}");
       }

       let has_autohide = |edge: u32| -> bool {
           let mut data = APPBARDATA {
               cbSize: size_of::<APPBARDATA>() as u32,
               uEdge: edge,
               rc: mon_info.rcMonitor,
               ..Default::default()
           };
           let h = unsafe { SHAppBarMessage(ABM_GETAUTOHIDEBAREX, &raw mut data) };
           h != 0
       };
       if has_autohide(ABE_TOP)    { rect.top    += AUTOHIDE_TASKBAR_SIZE; }
       if has_autohide(ABE_BOTTOM) { rect.bottom -= AUTOHIDE_TASKBAR_SIZE; }
       if has_autohide(ABE_LEFT)   { rect.left   += AUTOHIDE_TASKBAR_SIZE; }
       if has_autohide(ABE_RIGHT)  { rect.right  -= AUTOHIDE_TASKBAR_SIZE; }
       Ok(())
   }
   ```

   Imports: `SM_CYSIZEFRAME`, `SM_CXPADDEDBORDER`, `IsZoomed` (`WindowsAndMessaging`); `GetDpiForWindow`, `GetSystemMetricsForDpi` (`HiDpi`); `SHAppBarMessage`, `APPBARDATA`, `ABM_GETSTATE`, `ABM_GETAUTOHIDEBAREX`, `ABS_AUTOHIDE`, `ABE_TOP`/`BOTTOM`/`LEFT`/`RIGHT` (`windows::Win32::UI::Shell`); `MonitorFromWindow`, `GetMonitorInfoW`, `MONITORINFO`, `MONITOR_DEFAULTTONEAREST` (`windows::Win32::Graphics::Gdi`). `SHAppBarMessage` returns `usize`: for `ABM_GETSTATE` it's a state-flags bitmask, for `ABM_GETAUTOHIDEBAREX` it's the auto-hide bar's HWND (or `0`).

2. After the existing `let _ = window.resize_backdrop_tint(size);` line (the last statement before the `Some(LRESULT(0))` return), insert:

   ```rust
       if let Some(strip) = window.caption_buttons.borrow().as_ref() {
           let _ = strip.on_resize(size, max_chrome_y)
               .inspect_err(|err| log::warn!("strip on_resize failed: {err}"));
       }
   ```

   `max_chrome_y` comes from the hoisted value above (it is `0` outside the inset gate). `on_resize` must run *after* `resize_backdrop_tint` and issues the single `Commit()` (spec §5.5). `borrow().as_ref()` is correct since `on_resize` takes `&self`.

- [ ] **Step 8: Drop the caption-button strip at `WM_NCDESTROY`**

Per spec §6.2, the `RenderingDeviceReplacedRegistration` guard must be dropped *before* the HWND is destroyed, otherwise an in-flight WinRT callback can `PostMessageW` to a stale or recycled handle. `Window::Drop` is too late — Kotlin may hold the `Rc<Window>` past the OS destroying the HWND. The strip owns the registration as a field; dropping the strip drops the registration transitively.

In `window.rs`, modify the existing `WM_NCDESTROY` arm in the wndproc (`window.rs:441-446`) to drop `window.caption_buttons` after extracting the weak ref but before returning to the OS — the strip's RDR registration drops with it:

```rust
    if msg == WM_NCDESTROY {
        // Drop the strip (and its RDR registration) before the HWND is recycled.
        if let Ok(raw) = unsafe { RemovePropW(hwnd, WINDOW_PTR_PROP_NAME) } {
            let weak = unsafe { Weak::from_raw(raw.0.cast::<Window>()) };
            if let Some(window) = weak.upgrade() {
                window.caption_buttons.replace(None);
            }
        }
        return LRESULT(0);
    }
```

- [ ] **Step 9: Verify and commit**

Run: `cargo build -p desktop-win32`
Expected: PASS.

```bash
git add native/desktop-win32/src/win32/event_loop.rs native/desktop-win32/src/win32/window.rs
git commit -m "feat(win32): integrate CaptionButtonStrip into wndproc"
```

---

## Phase 7 — Sample app and manual exercise

### Task 7.1: Verify existing Custom-titlebar sample mode

**Files:** Verify: `sample/src/main/kotlin/...` (current `runSkikoSampleWin32` entrypoint).

- [ ] **Step 1: Locate the existing sample entry**

```powershell
Get-ChildItem -Path D:/repos/kotlin-desktop-toolkit/sample -Recurse -File | Select-String -Pattern "WindowTitleBarKind"
```

- [ ] **Step 2: Confirm the sample already creates a window with `WindowTitleBarKind.Custom`**

The current sample already uses `WindowStyle(titleBarKind = WindowTitleBarKind.Custom)` (`sample/src/main/kotlin/org/jetbrains/desktop/sample/win32/SkikoSampleWin32.kt`). Do not treat this as a new implementation task.

- [ ] **Step 3: Run the sample and visually verify the buttons render**

Run: `cd D:/repos/kotlin-desktop-toolkit && ./gradlew :sample:runSkikoSampleWin32`
Expected: a window opens with the toolkit's three caption buttons in the top-right; they respond to mouse hover (light theme: subtle black overlay; close button: red on hover); clicks invoke close / minimise / maximise.

### Task 7.2: Final test run

- [ ] **Step 1: Run the full unit-test suite**

Run: `cd D:/repos/kotlin-desktop-toolkit/native && cargo test -p desktop-win32`
Expected: all caption-button unit tests pass; existing tests unaffected.

- [ ] **Step 2: Run the Skiko sample and exercise the manual checklist, including glyph size/alignment**

Run: `cd D:/repos/kotlin-desktop-toolkit && ./gradlew :sample:runSkikoSampleWin32`

- [ ] **Step 3: Address any issues found during manual exercise; commit fixes**


