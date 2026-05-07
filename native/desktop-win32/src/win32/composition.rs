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
        Foundation::{D2DERR_RECREATE_TARGET, HMODULE, POINT},
        Graphics::{
            Direct2D::{
                D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1CreateFactory, ID2D1Device, ID2D1DeviceContext, ID2D1Factory1, ID2D1RenderTarget,
            },
            Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1},
            Direct3D11::{D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device},
            DirectWrite::{
                DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_REGULAR,
                DWriteCreateFactory, IDWriteFactory, IDWriteFontFace,
            },
            Dxgi::{DXGI_ERROR_DEVICE_REMOVED, DXGI_ERROR_DEVICE_RESET, IDXGIAdapter, IDXGIDevice},
        },
        System::WinRT::Composition::{ICompositionDrawingSurfaceInterop, ICompositorInterop},
    },
};
use windows_core::{HRESULT, HSTRING, Interface};

pub(crate) struct D2dContext {
    dwrite_factory: IDWriteFactory,
    composition_graphics_device: CompositionGraphicsDevice,
    caption_glyph_font: OnceCell<(&'static HSTRING, IDWriteFontFace)>,
}

impl D2dContext {
    /// Eagerly constructs the D3D11 / D2D devices, the DirectWrite factory,
    /// and the `CompositionGraphicsDevice`. The `Rc<D2dContext>` singleton
    /// wrapping happens once at `composition::ensure_d2d_context`.
    // Takes `Compositor` by value to mirror the singleton-accessor signature
    // (`ensure_d2d_context(compositor: Compositor)`); the WinRT smart pointer
    // is cheap to clone, so we accept the shadow rather than thread `&` through
    // the public surface.
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(compositor: Compositor) -> anyhow::Result<Self> {
        let d2d_device = build_d2d_device()?;
        let dwrite_factory: IDWriteFactory = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };
        let compositor_interop: ICompositorInterop = compositor.cast()?;
        let composition_graphics_device = unsafe { compositor_interop.CreateGraphicsDevice(&d2d_device)? };
        Ok(Self {
            dwrite_factory,
            composition_graphics_device,
            caption_glyph_font: OnceCell::new(),
        })
    }

    pub fn dwrite_factory(&self) -> IDWriteFactory {
        self.dwrite_factory.clone()
    }

    /// Resolve the system font collection's first available caption-glyph
    /// family (Segoe Fluent Icons, falling back to Segoe MDL2 Assets) and
    /// produce a concrete `IDWriteFontFace` for it. Cached on first call;
    /// subsequent calls return the cached face infallibly. The face
    /// survives device-replaced events because the font collection is
    /// independent of the D3D rendering device.
    pub fn caption_glyph_font(&self) -> anyhow::Result<&(&'static HSTRING, IDWriteFontFace)> {
        if let Some(cached) = self.caption_glyph_font.get() {
            return Ok(cached);
        }
        let mut collection = None;
        unsafe { self.dwrite_factory.GetSystemFontCollection(&raw mut collection, false)? };
        let collection = collection.ok_or_else(|| anyhow::anyhow!("DirectWrite returned no system font collection"))?;
        for family_name in [windows_core::h!("Segoe Fluent Icons"), windows_core::h!("Segoe MDL2 Assets")] {
            let mut index = 0u32;
            let mut exists = windows_core::BOOL(0);
            unsafe { collection.FindFamilyName(family_name, &raw mut index, &raw mut exists)? };
            if exists.as_bool() {
                let family = unsafe { collection.GetFontFamily(index)? };
                let font = unsafe {
                    family.GetFirstMatchingFont(DWRITE_FONT_WEIGHT_REGULAR, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL)?
                };
                let face = unsafe { font.CreateFontFace()? };
                return Ok(self.caption_glyph_font.get_or_init(|| (family_name, face)));
            }
        }
        anyhow::bail!("neither Segoe Fluent Icons nor Segoe MDL2 Assets is present in the system font collection")
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
    /// invocation re-rasterises. Device-loss is trapped on both `BeginDraw`
    /// and `EndDraw` (see [`is_device_lost_hresult`]) and dispatches to
    /// the `rebuild_d2d_device` path in this module. Other errors propagate.
    pub fn with_d2d_render_target<R>(
        &self,
        surface: &CompositionDrawingSurface,
        body: impl FnOnce(&ID2D1RenderTarget, POINT) -> anyhow::Result<R>,
    ) -> anyhow::Result<Option<R>> {
        let surface_interop: ICompositionDrawingSurfaceInterop = surface.cast()?;
        let mut offset = POINT::default();
        let context = match unsafe { surface_interop.BeginDraw::<ID2D1DeviceContext>(None, &raw mut offset) } {
            Ok(context) => context,
            Err(err) if is_device_lost_hresult(err.code()) => {
                self.rebuild_d2d_device()?;
                return Ok(None);
            }
            Err(err) => return Err(err.into()),
        };
        let rt: &ID2D1RenderTarget = (&context).into();
        // EndDraw must run even on body failure — an open BeginDraw breaks
        // future rasterisations on this surface.
        let body_result = body(rt, offset);
        let end_draw_result = unsafe { surface_interop.EndDraw() };
        // Device loss reported on EndDraw bypasses the normal precedence:
        // skip this frame, rebuild, and let the caller leave dirty flags
        // set. The body error (if any) is moot — the device is gone, so
        // the closure's failure was almost certainly a consequence.
        if let Err(ref err) = end_draw_result
            && is_device_lost_hresult(err.code())
        {
            self.rebuild_d2d_device()?;
            return Ok(None);
        }
        // Normal precedence: body error wins, EndDraw error attaches as context.
        match (body_result, end_draw_result) {
            (Ok(value), Ok(())) => Ok(Some(value)),
            (Err(body_err), Ok(())) => Err(body_err),
            (Ok(_), Err(end_draw_err)) => Err(end_draw_err.into()),
            (Err(body_err), Err(end_draw_err)) => Err(body_err.context(format!("EndDraw also failed: {end_draw_err}"))),
        }
    }

    /// Rebuild the D3D/D2D devices after device loss is detected.
    ///
    /// Called from device-loss paths inside `with_d2d_render_target`. Do
    /// not call from `RenderingDeviceReplaced`; that event is only the
    /// redraw notification after `SetRenderingDevice`.
    ///
    /// `build_d2d_device` is retried in a bounded loop so a transient
    /// driver hiccup mid-rebuild doesn't leave the `CompositionGraphicsDevice`
    /// stuck on the dead device. Per-frame device-loss storms are still
    /// possible (each frame's call exhausts its own budget) but each call is
    /// fast and the strip's caller logs via `inspect_err`.
    pub(crate) fn rebuild_d2d_device(&self) -> anyhow::Result<()> {
        const MAX_REBUILD_ATTEMPTS: u32 = 3;
        let cgd_interop: windows::Win32::System::WinRT::Composition::ICompositionGraphicsDeviceInterop =
            self.composition_graphics_device.cast()?;
        let mut last_err: Option<anyhow::Error> = None;
        for attempt in 1..=MAX_REBUILD_ATTEMPTS {
            match build_d2d_device() {
                Ok(d2d_device) => {
                    unsafe { cgd_interop.SetRenderingDevice(&d2d_device)? };
                    return Ok(());
                }
                Err(err) => {
                    log::warn!("D2D device rebuild attempt {attempt}/{MAX_REBUILD_ATTEMPTS} failed: {err}");
                    last_err = Some(err);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("D2D device rebuild exhausted attempts")))
    }

    /// Subscribe to `RenderingDeviceReplaced` (the redraw notification fired
    /// synchronously inside `SetRenderingDevice`). Callbacks must post a
    /// `WM_APP_*` message rather than call the strip directly — the event
    /// fires nested inside `with_d2d_render_target → rebuild_d2d_device`, so a
    /// direct re-entry would nest `BeginDraw` on the active surface.
    pub(crate) fn add_rendering_device_replaced_callback<F>(&self, callback: F) -> anyhow::Result<RenderingDeviceReplacedRegistration>
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
        // Spec §3.4: RDR callback runs on UI thread; no concurrent
        // delivery against this Drop. See TODO.md for the affinity probe.
        if let Err(err) = self.composition_graphics_device.RemoveRenderingDeviceReplaced(self.token) {
            log::warn!("RemoveRenderingDeviceReplaced failed on Drop: {err}");
        }
    }
}

/// HRESULTs Microsoft documents as signalling D3D/D2D device loss. Trap on
/// both [`ID2D1RenderTarget::BeginDraw`] and [`ID2D1RenderTarget::EndDraw`]
/// — `EndDraw` doc explicitly returns `D2DERR_RECREATE_TARGET` when the
/// device is gone, while `BeginDraw` typically reports the underlying
/// DXGI HRESULT.
const fn is_device_lost_hresult(code: HRESULT) -> bool {
    matches!(code, DXGI_ERROR_DEVICE_REMOVED | DXGI_ERROR_DEVICE_RESET | D2DERR_RECREATE_TARGET)
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
