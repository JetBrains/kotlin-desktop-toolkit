//! `D2dContext` — caption-button D2D / DirectWrite gateway.
//!
//! Holds the `IDWriteFactory` and `CompositionGraphicsDevice`. The CGD
//! retains the D3D11 / D2D rendering device, swapped on device loss via
//! `SetRenderingDevice`. Hides `BeginDraw`/`EndDraw`/device-loss behind a
//! closure-shaped `with_d2d_render_target` chokepoint.
//!
//! See `docs/specs/2026-04-30-win32-caption-buttons-design.md` § 4.1 for the
//! full design rationale.

use std::cell::OnceCell;
use std::rc::Rc;

use windows::{
    Graphics::{
        DirectX::{DirectXAlphaMode, DirectXPixelFormat},
        SizeInt32,
    },
    UI::Composition::{CompositionDrawingSurface, CompositionGraphicsDevice, Compositor},
    Win32::{
        Foundation::{HMODULE, POINT},
        Graphics::{
            Direct2D::{
                D2D1CreateFactory, D2D1_FACTORY_TYPE_SINGLE_THREADED, ID2D1Device, ID2D1DeviceContext, ID2D1Factory1,
                ID2D1RenderTarget,
            },
            Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1},
            Direct3D11::{D3D11CreateDevice, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, ID3D11Device},
            DirectWrite::{DWRITE_FACTORY_TYPE_SHARED, DWriteCreateFactory, IDWriteFactory},
            Dxgi::{DXGI_ERROR_DEVICE_REMOVED, IDXGIAdapter, IDXGIDevice},
        },
        System::WinRT::Composition::{ICompositionDrawingSurfaceInterop, ICompositorInterop},
    },
};
use windows_core::Interface;

pub(crate) struct D2dContext {
    dwrite_factory: IDWriteFactory,
    composition_graphics_device: CompositionGraphicsDevice,
}

impl D2dContext {
    /// Eagerly constructs the D3D11 / D2D devices, the DirectWrite factory,
    /// and the `CompositionGraphicsDevice`. The `Rc<D2dContext>` singleton
    /// wrapping happens once at `composition::ensure_d2d_context` (Task 1.3).
    pub fn new(compositor: Compositor) -> anyhow::Result<Self> {
        let d2d_device = build_d2d_device()?;
        let dwrite_factory: IDWriteFactory = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };
        let compositor_interop: ICompositorInterop = compositor.cast()?;
        let composition_graphics_device = unsafe { compositor_interop.CreateGraphicsDevice(&d2d_device)? };
        Ok(Self {
            dwrite_factory,
            composition_graphics_device,
        })
    }

    pub fn dwrite_factory(&self) -> IDWriteFactory {
        self.dwrite_factory.clone()
    }

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
            Err(err) if err.code() == DXGI_ERROR_DEVICE_REMOVED => {
                self.rebuild_d2d_device()?;
                return Ok(None);
            }
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
            (Err(body_err), Err(end_draw_err)) => {
                Err(body_err.context(format!("EndDraw also failed: {end_draw_err}")))
            }
            (Err(body_err), Ok(())) => Err(body_err),
            (Ok(_), Err(end_draw_err)) => Err(end_draw_err.into()),
        }
    }

    /// Rebuild the D3D/D2D devices after device loss is detected.
    ///
    /// Called from the `BeginDraw` `DXGI_ERROR_DEVICE_REMOVED` path inside
    /// `with_d2d_render_target`. Do not call this from `RenderingDeviceReplaced`;
    /// that event is only the redraw notification after `SetRenderingDevice`.
    pub(crate) fn rebuild_d2d_device(&self) -> anyhow::Result<()> {
        let d2d_device = build_d2d_device()?;
        let cgd_interop: windows::Win32::System::WinRT::Composition::ICompositionGraphicsDeviceInterop =
            self.composition_graphics_device.cast()?;
        unsafe { cgd_interop.SetRenderingDevice(&d2d_device)? };
        Ok(())
    }

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
}

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
    let d2d_factory: ID2D1Factory1 = unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)? };
    let dxgi_device: IDXGIDevice = d3d_device.cast()?;
    Ok(unsafe { d2d_factory.CreateDevice(&dxgi_device)? })
}

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
