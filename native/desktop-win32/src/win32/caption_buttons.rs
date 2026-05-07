//! Caption-button strip for `WindowTitleBarKind::Custom` windows.
//!
//! See `docs/specs/2026-04-30-win32-caption-buttons-design.md` for the design.
//!
//! The strip itself is a state machine driven by typed inputs from the
//! wndproc layer (`kind`, pointer id, theme, etc.) and produces typed
//! outputs (`Option<CaptionButtonAction>`). It does call `WinRT` Composition
//! APIs to manage its visuals, plus a small set of Win32 coord-transform
//! utilities (`ScreenToClient` / `GetClientRect`) inside `caption_kind_at_screen`
//! so that the strip-anchoring math has a single home — wndproc-level
//! `WM_*` message decoding stays in `event_loop.rs`.
//!
//! Module-level lint allowances: this is graphics-math code that bridges
//! pixel-bounded `i32` geometry into `WinRT` `Vector2`/`Vector3` (`f32`) and
//! into surface sizes (`SizeInt32` / `i32`). Caption-button counts (≤ 3) and
//! pixel sizes (≤ a few hundred) never approach the lossy ranges of these
//! casts, so we silence them at the module level rather than peppering each
//! site with `#[allow]`.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::rc::Rc;

use windows::{
    Foundation::TimeSpan,
    Graphics::SizeInt32,
    UI::Composition::{CompositionColorBrush, CompositionDrawingSurface, ContainerVisual, Core::CompositorController, SpriteVisual},
    Win32::{
        Foundation::{HWND, LPARAM, POINT, RECT, WPARAM},
        Graphics::{
            Direct2D::{
                Common::{D2D_RECT_F, D2D1_COLOR_F},
                D2D1_DRAW_TEXT_OPTIONS_NONE, ID2D1Brush,
            },
            DirectWrite::{
                DWRITE_FONT_METRICS, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_REGULAR,
                DWRITE_GLYPH_METRICS, DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_CENTER,
                IDWriteFontFace,
            },
            Gdi::{GetSysColor, SYS_COLOR_INDEX, ScreenToClient},
        },
        UI::{
            HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi},
            WindowsAndMessaging::{GetClientRect, GetWindowRect, PostMessageW, SM_CXPADDEDBORDER, SM_CYSIZEFRAME},
        },
    },
};
use windows_numerics::{Vector2, Vector3};

use super::{
    appearance::{Appearance, HighContrast},
    composition::{D2dContext, RenderingDeviceReplacedRegistration},
    geometry::{LogicalSize, PhysicalPixels, PhysicalPoint, PhysicalSize},
    pointer::PointerButton,
    window::Window,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PressSessionMode {
    /// Primary press on an enabled button. Visual capture engaged; matched
    /// primary UP dispatches a `CaptionButtonAction` if released over the
    /// same button. Held button is implicitly `Left`.
    Active,
    /// Wndproc-level swallow; no visual capture. `held_button = Left` for
    /// primary-on-disabled, `Right` / `Middle` / `XButton1` / `XButton2` for non-primary.
    Suppressed { held_button: PointerButton },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PressSession {
    pointer_id: u32,
    captured_kind: CaptionButtonKind,
    mode: PressSessionMode,
}

/// Bitset of which caption-button kinds are visible on a window. Derived from
/// `WindowStyle` flags at strip-construction time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct CaptionButtonKinds(u8);

impl CaptionButtonKinds {
    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn with(self, kind: CaptionButtonKind) -> Self {
        Self(self.0 | (1 << kind as u8))
    }

    pub const fn contains(self, kind: CaptionButtonKind) -> bool {
        (self.0 & (1 << kind as u8)) != 0
    }

    /// Yields the visible kinds in left-to-right system order
    /// (Minimize → Maximize → Close). Single source of truth used by
    /// both hit-testing (`StripGeometry::hit_test`) and layout
    /// (`CaptionButtonStrip::relayout`, `CaptionButtonStrip::new`).
    pub fn iter_ordered(self) -> impl Iterator<Item = CaptionButtonKind> {
        [CaptionButtonKind::Minimize, CaptionButtonKind::Maximize, CaptionButtonKind::Close]
            .into_iter()
            .filter(move |kind| self.contains(*kind))
    }

    pub const fn from_style(style: &crate::win32::window_api::WindowStyle) -> Self {
        let mut kinds = Self::empty().with(CaptionButtonKind::Close);
        if style.is_minimizable || style.is_maximizable {
            kinds = kinds.with(CaptionButtonKind::Minimize);
            kinds = kinds.with(CaptionButtonKind::Maximize);
        }
        kinds
    }
}

const fn availability_from_style(kind: CaptionButtonKind, style: &crate::win32::window_api::WindowStyle) -> Availability {
    match kind {
        CaptionButtonKind::Minimize if !style.is_minimizable => Availability::Disabled,
        CaptionButtonKind::Maximize if !style.is_maximizable => Availability::Disabled,
        _ => Availability::Enabled,
    }
}

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
        Some(s) if s.mode == PressSessionMode::Active && s.captured_kind == kind => {
            if is_pointer_over_self {
                ButtonInteraction::Pressed
            } else {
                ButtonInteraction::PressedDraggedOff
            }
        }
        Some(s) if s.mode == PressSessionMode::Active => ButtonInteraction::Idle,
        // Suppressed session OR no session: fall through to hover resolution.
        // A non-primary press / disabled-primary press is consumed silently
        // by the wndproc and must not suppress hover on neighbouring
        // buttons — Suppressed sessions have no visual capture.
        _ if is_pointer_over_self => match pointer_device {
            Some(PointerDeviceKind::Touch) => ButtonInteraction::Idle,
            _ => ButtonInteraction::Hovered,
        },
        _ => ButtonInteraction::Idle,
    }
}

/// Predicate behind `CaptionButtonStrip::consume_swallowed_release`. Free
/// so unit tests can exercise it without constructing a full
/// `CaptionButtonStrip` (which requires live Composition / D2D resources).
fn is_swallowed_release_match(session: &PressSession, pointer_id: u32, button: PointerButton) -> bool {
    session.pointer_id == pointer_id && matches!(session.mode, PressSessionMode::Suppressed { held_button } if held_button == button)
}

/// True iff `session` is a real primary release for `pointer_id`: `Active`
/// or `Suppressed { held_button: Left }`. Non-primary `Suppressed` is
/// drained via `consume_swallowed_release` instead.
const fn is_real_release_match(session: &PressSession, pointer_id: u32) -> bool {
    session.pointer_id == pointer_id
        && matches!(
            session.mode,
            PressSessionMode::Active
                | PressSessionMode::Suppressed {
                    held_button: PointerButton::Left
                }
        )
}

/// Clears `*session` if it matches `pointer_id` and returns `true` iff the
/// cleared session was `Active` (i.e. visuals must be refreshed).
fn cancel_press_session(session: &mut Option<PressSession>, pointer_id: u32) -> bool {
    let Some(s) = *session else { return false };
    if s.pointer_id != pointer_id {
        return false;
    }
    *session = None;
    s.mode == PressSessionMode::Active
}

/// `WM_CANCELMODE` and `WM_ACTIVATE` deactivation deliver no pointer id, so
/// cancellation has to be unconditional — a sibling of `cancel_press_session`
/// minus the pointer-id gate.
fn cancel_any_press_session(session: &mut Option<PressSession>) -> bool {
    let Some(s) = *session else { return false };
    *session = None;
    s.mode == PressSessionMode::Active
}

struct CaptionTheme {
    backplate_rest: windows::UI::Color,
    backplate_hover: windows::UI::Color,
    backplate_pressed: windows::UI::Color,
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
            (HighContrast::Off, Appearance::Dark) => Self::dark(),
        }
    }

    // WinUI Fluent palette: `microsoft/microsoft-ui-xaml@5f9e851133b…`.
    // Close-specific reds: `microsoft/terminal@e4e3f08efca…` MinMaxCloseControl.xaml
    // (`Opacity 0.9` → α=0xE6; `Opacity 0.7` → α=0xB3 — valid only because the
    // source RGB is fully opaque; both rounded to nearest).
    // Inactive foreground: `TitleBarDeactivatedForegroundBrush` →
    // `TextFillColorTertiary` (microsoft-ui-xaml `TitleBar` resources).
    const fn light() -> Self {
        Self {
            backplate_rest: rgba(0, 0, 0, 0),
            backplate_hover: rgba(0, 0, 0, 0x09),   // SubtleFillColorSecondary
            backplate_pressed: rgba(0, 0, 0, 0x06), // SubtleFillColorTertiary
            foreground_rest: rgba(0, 0, 0, 0xE4),   // TextFillColorPrimary
            foreground_hover: rgba(0, 0, 0, 0xE4),
            foreground_pressed: rgba(0, 0, 0, 0x9E),  // TextFillColorSecondary
            foreground_disabled: rgba(0, 0, 0, 0x5C), // TextFillColorDisabled
            foreground_inactive: rgba(0, 0, 0, 0x72),
            close_backplate_hover: rgba(0xC4, 0x2B, 0x1C, 0xFF),
            close_backplate_pressed: rgba(0xC4, 0x2B, 0x1C, 0xE6),
            close_foreground_hover: rgba(0xFF, 0xFF, 0xFF, 0xFF),
            close_foreground_pressed: rgba(0xFF, 0xFF, 0xFF, 0xB3),
        }
    }

    const fn dark() -> Self {
        Self {
            backplate_rest: rgba(0, 0, 0, 0),
            backplate_hover: rgba(0xFF, 0xFF, 0xFF, 0x0F),   // SubtleFillColorSecondary
            backplate_pressed: rgba(0xFF, 0xFF, 0xFF, 0x0A), // SubtleFillColorTertiary
            foreground_rest: rgba(0xFF, 0xFF, 0xFF, 0xFF),   // TextFillColorPrimary
            foreground_hover: rgba(0xFF, 0xFF, 0xFF, 0xFF),
            foreground_pressed: rgba(0xFF, 0xFF, 0xFF, 0xC5),  // TextFillColorSecondary
            foreground_disabled: rgba(0xFF, 0xFF, 0xFF, 0x5D), // TextFillColorDisabled
            foreground_inactive: rgba(0xFF, 0xFF, 0xFF, 0x87),
            close_backplate_hover: rgba(0xC4, 0x2B, 0x1C, 0xFF),
            close_backplate_pressed: rgba(0xC4, 0x2B, 0x1C, 0xE6),
            close_foreground_hover: rgba(0xFF, 0xFF, 0xFF, 0xFF),
            close_foreground_pressed: rgba(0xFF, 0xFF, 0xFF, 0xB3),
        }
    }

    fn high_contrast() -> Self {
        use windows::Win32::Graphics::Gdi::{COLOR_BTNFACE, COLOR_BTNTEXT, COLOR_GRAYTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT};
        let face = sys_color(COLOR_BTNFACE);
        let text = sys_color(COLOR_BTNTEXT);
        let highlight = sys_color(COLOR_HIGHLIGHT);
        let highlight_text = sys_color(COLOR_HIGHLIGHTTEXT);
        let grayed = sys_color(COLOR_GRAYTEXT);
        Self {
            backplate_rest: face,
            backplate_hover: highlight,
            backplate_pressed: highlight,
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
    let colorref = unsafe { GetSysColor(index) };
    windows::UI::Color {
        A: 0xFF,
        R: (colorref & 0xFF) as u8,
        G: ((colorref >> 8) & 0xFF) as u8,
        B: ((colorref >> 16) & 0xFF) as u8,
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CaptionButtonMetrics {
    pub button_size_px: PhysicalSize,
    pub glyph_extent_px: PhysicalSize,
}

impl CaptionButtonMetrics {
    pub const fn new(scale: f32) -> Self {
        Self {
            button_size_px: LogicalSize::new(46.0, 32.0).to_physical(scale),
            glyph_extent_px: LogicalSize::new(10.0, 10.0).to_physical(scale),
        }
    }
}

struct StripGeometry {
    /// Width of the coordinate space `point` is in. The strip's right edge sits
    /// at `reference_width_px`; the strip-left is computed by subtracting
    /// `button_count * button_width`. For client-space points, pass the full
    /// client width.
    reference_width_px: i32,
    /// Vertical offset of the strip within the same coordinate space as `point`.
    /// Maximized custom-titlebar windows shift the strip down by their chrome
    /// overhang, so hit-testing must subtract this before checking button y.
    top_offset_px: i32,
    metrics: CaptionButtonMetrics,
    visible_kinds: CaptionButtonKinds,
}

impl StripGeometry {
    fn hit_test(&self, point: PhysicalPoint) -> Option<CaptionButtonKind> {
        let bw = self.metrics.button_size_px.width.0;
        let bh = self.metrics.button_size_px.height.0;
        let y = point.y.0 - self.top_offset_px;
        if y < 0 || y >= bh {
            return None;
        }
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

/// Map a caption-button hit to the `WM_NCHITTEST` return code per spec §4.2:
/// enabled Min/Max/Close → HTMINBUTTON / HTMAXBUTTON / HTCLOSE; visible
/// disabled Min/Max → HTCAPTION for caption-like hit-test semantics, while
/// DOWN/UP remain swallowed by the strip with no hover, press, or action.
pub(crate) const fn hittest_for_caption_button_kind(kind: CaptionButtonKind, is_enabled: bool) -> u32 {
    use windows::Win32::UI::WindowsAndMessaging::{HTCAPTION, HTCLOSE, HTMAXBUTTON, HTMINBUTTON};
    match (kind, is_enabled) {
        (CaptionButtonKind::Close, _) => HTCLOSE,
        (CaptionButtonKind::Minimize, true) => HTMINBUTTON,
        (CaptionButtonKind::Maximize, true) => HTMAXBUTTON,
        (CaptionButtonKind::Minimize | CaptionButtonKind::Maximize, false) => HTCAPTION,
    }
}

/// Hit-test the caption-button strip from the pointer's screen-space
/// coordinates. Returns the caption-button kind under the point, or `None`
/// if no strip exists, the point falls outside the strip's bounds, or the
/// point is inside the top resize-border band on a restored, resizable
/// window — that band must reach `DefWindowProc` so the system resize
/// cursor and drag loop fire instead of the strip claiming the hit.
///
/// This is the single source of truth for "screen coords → caption-button
/// kind" — `WM_NCHITTEST` and the caption-button pointer handlers go through
/// it so the geometry stays in one place. The Win32 coordinate-transform
/// calls (`ScreenToClient`, `GetClientRect`) live here rather than in the
/// wndproc layer because they're a tightly-coupled pair with the strip's
/// own hit-test math.
pub(crate) fn caption_kind_at_screen(window: &Window, screen: PhysicalPoint) -> Option<CaptionButtonKind> {
    let strip_ref = window.caption_buttons.borrow();
    let strip = strip_ref.as_ref()?;
    let hwnd = window.hwnd();
    if is_in_top_resize_border(window, hwnd, screen.y.0) {
        return None;
    }
    let mut client_point = POINT {
        x: screen.x.0,
        y: screen.y.0,
    };
    unsafe { ScreenToClient(hwnd, &raw mut client_point) }
        .ok()
        .inspect_err(|err| log::warn!("ScreenToClient failed: {err}"))
        .ok()?;
    let mut client_rect = RECT::default();
    unsafe { GetClientRect(hwnd, &raw mut client_rect) }
        .inspect_err(|err| log::warn!("GetClientRect failed: {err}"))
        .ok()?;
    strip.hit_test(
        PhysicalPoint::new(client_point.x, client_point.y),
        PhysicalPixels(client_rect.right),
    )
}

/// True when `mouse_y` (screen-space) lands inside the top resize-border band
/// of a restored, resizable window. Maximized and non-resizable windows have
/// no resize band so the strip wins by default.
fn is_in_top_resize_border(window: &Window, hwnd: HWND, mouse_y: i32) -> bool {
    if !window.is_resizable() || window.is_maximized() {
        return false;
    }
    let mut window_rect = RECT::default();
    if unsafe { GetWindowRect(hwnd, &raw mut window_rect) }
        .inspect_err(|err| log::warn!("GetWindowRect failed during resize-border check: {err}"))
        .is_err()
    {
        return false;
    }
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    let resize_handle_height = unsafe { GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi) + GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi) };
    mouse_y < window_rect.top + resize_handle_height
}

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
    top_offset_px: i32,
    d2d_context: Rc<D2dContext>,
    // Dropping the registration removes the RDR subscription (spec §6.2). Field is held
    // for its `Drop` side-effect even though we never read it after construction.
    #[allow(dead_code)]
    device_replaced_registration: RenderingDeviceReplacedRegistration,
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
    glyph_brush: CompositionColorBrush,
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

        // Subscribe to RenderingDeviceReplaced; see spec §6.2 for the reentrancy contract.
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
            buttons.push(create_caption_button(
                &compositor,
                &composition_root,
                &d2d_context,
                kind,
                availability,
                &metrics,
            )?);
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
            top_offset_px: 0,
            d2d_context,
            device_replaced_registration,
            compositor_controller,
        };
        strip.relayout()?;
        strip.apply_visuals_to_all_buttons()?;
        strip.compositor_controller.Commit()?;
        Ok(strip)
    }

    fn relayout(&mut self) -> anyhow::Result<()> {
        let bw = self.metrics.button_size_px.width.0;
        let bh = self.metrics.button_size_px.height.0;
        let total_width = bw * self.buttons.len() as i32;
        // Buttons line up at increasing x within the strip's parent;
        // `set_strip_position` places the parent at top-right of `chrome_layer`.
        self.composition_root.SetSize(Vector2::new(total_width as f32, bh as f32))?;
        let gw = self.metrics.glyph_extent_px.width.0;
        let gh = self.metrics.glyph_extent_px.height.0;
        let gx = (bw - gw) / 2;
        let gy = (bh - gh) / 2;
        for (i, button) in self.buttons.iter_mut().enumerate() {
            let x = (i as i32) * bw;
            button.visuals.backplate.SetOffset(Vector3 {
                X: x as f32,
                Y: 0.0,
                Z: 0.0,
            })?;
            button.visuals.backplate.SetSize(Vector2::new(bw as f32, bh as f32))?;
            button.visuals.glyph.SetOffset(Vector3 {
                X: gx as f32,
                Y: gy as f32,
                Z: 0.0,
            })?;
            button.visuals.glyph.SetSize(Vector2::new(gw as f32, gh as f32))?;
        }
        Ok(())
    }

    pub(crate) fn set_strip_position(&mut self, client_size: PhysicalSize, max_chrome_y: i32) -> anyhow::Result<()> {
        let x = strip_offset_x(client_size.width.0, self.metrics.button_size_px.width.0, self.buttons.len());
        self.top_offset_px = max_chrome_y;
        self.composition_root.SetOffset(Vector3 {
            X: x as f32,
            Y: max_chrome_y as f32,
            Z: 0.0,
        })?;
        Ok(())
    }

    fn rasterise_all_glyphs(&mut self) -> anyhow::Result<()> {
        for button in &mut self.buttons {
            if button.glyph_surface_dirty
                && rasterise_glyph(
                    &self.d2d_context,
                    &button.visuals.glyph_surface,
                    button.kind,
                    self.is_window_maximized,
                    self.high_contrast,
                    &self.metrics,
                )?
            {
                button.glyph_surface_dirty = false;
            }
        }
        Ok(())
    }

    fn apply_visuals_to_all_buttons(&mut self) -> anyhow::Result<()> {
        // Re-rasterise dirty glyphs (spec §6.2 reactive device-loss heal).
        self.rasterise_all_glyphs()?;

        let theme = CaptionTheme::resolve(self.appearance, self.high_contrast);
        let pointer_over_kind = self.pointer_over_kind;
        let pointer_device = self.pointer_device;
        let press_session = self.press_session;
        let is_active = self.is_active;
        for button in &mut self.buttons {
            let new_interaction = resolve_interaction(
                button.kind,
                button.availability,
                pointer_over_kind,
                pointer_device,
                press_session.as_ref(),
            );
            apply_button_visuals(button, new_interaction, &theme, is_active)?;
        }
        Ok(())
    }

    /// Hit-test a point given in **client-space** coordinates (origin =
    /// client top-left). The strip is anchored at the right edge, so the
    /// caller passes the client area's full width as the reference. Returns
    /// the visible caption-button kind under the point, or `None` if the
    /// point is outside the strip's bounds.
    pub fn hit_test(&self, client_point: PhysicalPoint, client_width: PhysicalPixels) -> Option<CaptionButtonKind> {
        StripGeometry {
            reference_width_px: client_width.0,
            top_offset_px: self.top_offset_px,
            metrics: self.metrics,
            visible_kinds: self.visible_kinds,
        }
        .hit_test(client_point)
    }

    pub fn on_pointer_update(
        &mut self,
        kind: Option<CaptionButtonKind>,
        _pointer_id: u32,
        device: PointerDeviceKind,
    ) -> anyhow::Result<()> {
        if self.pointer_over_kind != kind || self.pointer_device != Some(device) {
            self.pointer_over_kind = kind;
            self.pointer_device = Some(device);
            self.apply_visuals_to_all_buttons()?;
            self.compositor_controller.Commit()?;
        }
        Ok(())
    }

    pub fn on_pointer_down(&mut self, kind: CaptionButtonKind, pointer_id: u32, device: PointerDeviceKind) -> anyhow::Result<()> {
        if self.press_session.is_some() {
            return Ok(());
        }
        let is_enabled = self.button_for(kind).map(|b| b.availability) == Some(Availability::Enabled);
        // Disabled-primary records Suppressed{Left} so has_active_press_for
        // matches and the existing primary-release branch consumes silently.
        let mode = if is_enabled {
            PressSessionMode::Active
        } else {
            PressSessionMode::Suppressed {
                held_button: PointerButton::Left,
            }
        };
        self.press_session = Some(PressSession {
            pointer_id,
            captured_kind: kind,
            mode,
        });
        if mode == PressSessionMode::Active {
            self.pointer_over_kind = Some(kind);
            self.pointer_device = Some(device);
            self.apply_visuals_to_all_buttons()?;
            self.compositor_controller.Commit()?;
        }
        // Suppressed: no visual side effects (spec §4.3 — disabled Min/Max
        // never enter Hovered / Pressed).
        Ok(())
    }

    pub fn on_pointer_up(
        &mut self,
        kind_under_pointer: Option<CaptionButtonKind>,
        pointer_id: u32,
    ) -> anyhow::Result<Option<CaptionButtonAction>> {
        let session = match self.press_session {
            Some(s) if is_real_release_match(&s, pointer_id) => s,
            _ => return Ok(None),
        };
        self.press_session = None;
        match session.mode {
            PressSessionMode::Active => {
                let action = (Some(session.captured_kind) == kind_under_pointer).then(|| self.action_for(session.captured_kind));
                self.pointer_over_kind = kind_under_pointer;
                self.apply_visuals_to_all_buttons()?;
                self.compositor_controller.Commit()?;
                Ok(action)
            }
            // Disabled-primary cycle (Suppressed{Left}); fire no action.
            PressSessionMode::Suppressed { .. } => Ok(None),
        }
    }

    pub fn on_pointer_cancel(&mut self, pointer_id: u32) -> anyhow::Result<()> {
        // Visuals only need reverting if Active had been engaged.
        if cancel_press_session(&mut self.press_session, pointer_id) {
            self.apply_visuals_to_all_buttons()?;
            self.compositor_controller.Commit()?;
        }
        Ok(())
    }

    /// Drop any owned press session. Used by the wndproc on `WM_CANCELMODE`
    /// and on deactivation, where there is no pointer id to match against.
    pub fn cancel_any_press(&mut self) -> anyhow::Result<()> {
        if cancel_any_press_session(&mut self.press_session) {
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

    /// Release-gate predicate: true iff the strip owns a session whose
    /// matching primary (Left) UP must be consumed by the wndproc. Pair
    /// with `has_press_for` for the broader cleanup gate.
    pub(crate) const fn has_active_press_for(&self, pointer_id: u32) -> bool {
        matches!(self.press_session, Some(s) if is_real_release_match(&s, pointer_id))
    }

    /// Cleanup-gate predicate: true iff the strip owns any press session
    /// for `pointer_id`, regardless of mode (capture loss, deactivation).
    pub(crate) const fn has_press_for(&self, pointer_id: u32) -> bool {
        matches!(self.press_session, Some(s) if s.pointer_id == pointer_id)
    }

    /// Record a wndproc-consumed non-primary DOWN as a `Suppressed` session
    /// keyed by `held_button`; the matching UP is drained by
    /// `consume_swallowed_release`. No-op if a session already exists
    /// (concurrent multi-button presses drop the second DOWN — its UP will
    /// leak; consistent with `on_pointer_down`'s single-press limitation).
    /// `kind` populates `captured_kind` for struct-shape uniformity with
    /// `Active`; Suppressed-mode consumers don't read it.
    pub(crate) const fn track_swallowed_press(&mut self, kind: CaptionButtonKind, pointer_id: u32, button: PointerButton) {
        if self.press_session.is_some() {
            return;
        }
        self.press_session = Some(PressSession {
            pointer_id,
            captured_kind: kind,
            mode: PressSessionMode::Suppressed { held_button: button },
        });
    }

    /// Drain a `Suppressed` session whose pointer-id matches and whose
    /// held button is `button`. Returns `true` iff the session was found
    /// and dropped — callers swallowing the matching UP should treat
    /// `true` as the signal to suppress further dispatch.
    ///
    /// Only `Suppressed` sessions are drained here; `Active` sessions
    /// continue to flow through `on_pointer_up`'s primary-action path.
    pub(crate) fn consume_swallowed_release(&mut self, pointer_id: u32, button: PointerButton) -> bool {
        let matches_session = self
            .press_session
            .as_ref()
            .is_some_and(|s| is_swallowed_release_match(s, pointer_id, button));
        if matches_session {
            self.press_session = None;
            true
        } else {
            false
        }
    }

    const fn action_for(&self, kind: CaptionButtonKind) -> CaptionButtonAction {
        match kind {
            CaptionButtonKind::Close => CaptionButtonAction::Close,
            CaptionButtonKind::Minimize => CaptionButtonAction::Minimize,
            CaptionButtonKind::Maximize => {
                if self.is_window_maximized {
                    CaptionButtonAction::Restore
                } else {
                    CaptionButtonAction::Maximize
                }
            }
        }
    }

    pub fn on_activate(&mut self, is_active: bool) -> anyhow::Result<()> {
        if self.is_active != is_active {
            self.is_active = is_active;
            // Spec §5.2: `is_active` flips never animate. Resetting per-button
            // history keeps the `Hovered → Idle` predicate false across the flip.
            for button in &mut self.buttons {
                button.last_applied_interaction = ButtonInteraction::Idle;
            }
            self.apply_visuals_to_all_buttons()?;
            self.compositor_controller.Commit()?;
        }
        Ok(())
    }

    pub fn on_dpi_change(&mut self, new_scale: f32) -> anyhow::Result<()> {
        self.metrics = CaptionButtonMetrics::new(new_scale);
        for button in &mut self.buttons {
            let new_size = SizeInt32 {
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
            for button in &mut self.buttons {
                button.glyph_surface_dirty = true;
            }
        }
        self.apply_visuals_to_all_buttons()?;
        self.compositor_controller.Commit()?;
        Ok(())
    }

    pub fn on_rendering_device_replaced(&mut self) -> anyhow::Result<()> {
        for button in &mut self.buttons {
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
            for button in &mut self.buttons {
                if button.kind == CaptionButtonKind::Maximize {
                    button.glyph_surface_dirty = true;
                }
            }
            self.rasterise_all_glyphs()?;
        }
        // Always commit: WM_WINDOWPOSCHANGED queues companion updates
        // (e.g. `Window::set_content_top_offset`) on this controller and
        // relies on this commit to publish them.
        self.compositor_controller.Commit()?;
        Ok(())
    }

    pub fn on_resize(&mut self, client_size: PhysicalSize, max_chrome_y: i32) -> anyhow::Result<()> {
        self.set_strip_position(client_size, max_chrome_y)?;
        self.compositor_controller.Commit()?;
        Ok(())
    }
}

/// X offset that anchors the strip's right edge to the client area's right
/// edge. Pure arithmetic; extracted so the layout invariant lives in one
/// readable place.
const fn strip_offset_x(client_width_px: i32, button_width_px: i32, button_count: usize) -> i32 {
    client_width_px - button_width_px * button_count as i32
}

fn create_caption_button(
    compositor: &windows::UI::Composition::Compositor,
    parent: &ContainerVisual,
    d2d_context: &Rc<D2dContext>,
    kind: CaptionButtonKind,
    availability: Availability,
    metrics: &CaptionButtonMetrics,
) -> anyhow::Result<CaptionButton> {
    let backplate = compositor.CreateSpriteVisual()?;
    let backplate_brush = compositor.CreateColorBrushWithColor(rgba(0, 0, 0, 0))?;
    backplate.SetBrush(&backplate_brush)?;
    backplate.SetSize(Vector2::new(
        metrics.button_size_px.width.0 as f32,
        metrics.button_size_px.height.0 as f32,
    ))?;
    parent.Children()?.InsertAtTop(&backplate)?;

    // Mask-brush topology: see spec §4.3. The `CompositionMaskBrush` and the
    // wrapping `CompositionSurfaceBrush` for the glyph surface aren't stored
    // on the visuals — the `glyph` SpriteVisual's `SetBrush(&mask_brush)`
    // keeps the chain alive.
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
    glyph.SetSize(Vector2::new(
        metrics.glyph_extent_px.width.0 as f32,
        metrics.glyph_extent_px.height.0 as f32,
    ))?;
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

const fn glyph_for(kind: CaptionButtonKind, is_maximised: bool, hc: HighContrast) -> char {
    match (kind, is_maximised, hc) {
        (CaptionButtonKind::Minimize, _, HighContrast::Off) => '\u{E921}',
        (CaptionButtonKind::Maximize, false, HighContrast::Off) => '\u{E922}',
        (CaptionButtonKind::Maximize, true, HighContrast::Off) => '\u{E923}',
        (CaptionButtonKind::Close, _, HighContrast::Off) => '\u{E8BB}',
        (CaptionButtonKind::Minimize, _, HighContrast::On) => '\u{EF2D}',
        (CaptionButtonKind::Maximize, false, HighContrast::On) => '\u{EF2E}',
        (CaptionButtonKind::Maximize, true, HighContrast::On) => '\u{EF2F}',
        (CaptionButtonKind::Close, _, HighContrast::On) => '\u{EF2C}',
    }
}

/// Compute the DirectWrite font size (DIPs) at which the glyph's visible
/// black-box fits within `target_extent_px`. Algorithm per spec §4.5.
fn compute_glyph_font_size(face: &IDWriteFontFace, glyph_char: char, target_extent_px: PhysicalSize) -> anyhow::Result<f32> {
    let codepoint = glyph_char as u32;
    let mut glyph_index: u16 = 0;
    unsafe { face.GetGlyphIndices(&raw const codepoint, 1, &raw mut glyph_index)? };
    if glyph_index == 0 {
        anyhow::bail!("caption glyph U+{codepoint:04X} maps to .notdef in selected font");
    }

    let mut glyph_metrics = DWRITE_GLYPH_METRICS::default();
    unsafe { face.GetDesignGlyphMetrics(&raw const glyph_index, 1, &raw mut glyph_metrics, false)? };
    let mut font_metrics = DWRITE_FONT_METRICS::default();
    unsafe { face.GetMetrics(&raw mut font_metrics) };

    let design_units_per_em = i32::from(font_metrics.designUnitsPerEm);
    if design_units_per_em <= 0 {
        anyhow::bail!("DirectWrite returned designUnitsPerEm = {design_units_per_em}");
    }

    let bbox_w = (glyph_metrics.advanceWidth as i32) - glyph_metrics.leftSideBearing - glyph_metrics.rightSideBearing;
    // Horizontal-layout cell height per DWRITE_FONT_METRICS — `ascent + descent`,
    // not `glyph_metrics.advanceHeight` (which is the *vertical* advance).
    let cell_height_du = i32::from(font_metrics.ascent) + i32::from(font_metrics.descent);
    let bbox_h = cell_height_du - glyph_metrics.topSideBearing - glyph_metrics.bottomSideBearing;
    if bbox_w <= 0 || bbox_h <= 0 {
        anyhow::bail!("DirectWrite returned non-positive glyph bbox: {bbox_w}x{bbox_h}");
    }

    let dpem = design_units_per_em as f32;
    let font_size_x = (target_extent_px.width.0 as f32) * dpem / (bbox_w as f32);
    let font_size_y = (target_extent_px.height.0 as f32) * dpem / (bbox_h as f32);
    Ok(font_size_x.min(font_size_y))
}

fn rasterise_glyph(
    d2d_context: &Rc<D2dContext>,
    surface: &CompositionDrawingSurface,
    kind: CaptionButtonKind,
    is_maximised: bool,
    hc: HighContrast,
    metrics: &CaptionButtonMetrics,
) -> anyhow::Result<bool> {
    let glyph_char = glyph_for(kind, is_maximised, hc);
    let (font_family, font_face) = d2d_context.caption_glyph_font()?;
    let font_size = compute_glyph_font_size(font_face, glyph_char, metrics.glyph_extent_px)?;
    let dwrite = d2d_context.dwrite_factory();
    let format = unsafe {
        dwrite.CreateTextFormat(
            *font_family,
            None::<&windows::Win32::Graphics::DirectWrite::IDWriteFontCollection>,
            DWRITE_FONT_WEIGHT_REGULAR,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            font_size,
            windows_core::h!("en-US"),
        )?
    };
    unsafe {
        format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
    }
    let mut text_buf = [0u16; 2];
    let text: &[u16] = glyph_char.encode_utf16(&mut text_buf);
    let drew = d2d_context
        .with_d2d_render_target(surface, |rt, offset| {
            unsafe {
                // Pin to 96 DPI so 1 DIP == 1 pixel; `compute_glyph_font_size`
                // assumes that mapping.
                rt.SetDpi(96.0, 96.0);
                let clear_color = D2D1_COLOR_F {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                };
                rt.Clear(Some(&raw const clear_color));
                let brush_color = D2D1_COLOR_F {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                };
                let brush = rt.CreateSolidColorBrush(&raw const brush_color, None)?;
                let rect = D2D_RECT_F {
                    left: offset.x as f32,
                    top: offset.y as f32,
                    right: offset.x as f32 + metrics.glyph_extent_px.width.0 as f32,
                    bottom: offset.y as f32 + metrics.glyph_extent_px.height.0 as f32,
                };
                let brush: ID2D1Brush = brush.into();
                rt.DrawText(
                    text,
                    &format,
                    &raw const rect,
                    &brush,
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                    DWRITE_MEASURING_MODE_NATURAL,
                );
            }
            Ok(())
        })?
        .is_some();
    Ok(drew)
}

fn apply_button_visuals(
    button: &mut CaptionButton,
    new_interaction: ButtonInteraction,
    theme: &CaptionTheme,
    is_active: bool,
) -> anyhow::Result<()> {
    let (backplate, foreground) = colours_for(button.kind, button.availability, new_interaction, theme, is_active);
    let prev = button.last_applied_interaction;
    // Spec §5.2: animate only `Hovered → Idle` on Enabled buttons; everything else jumps.
    let is_hover_leave =
        prev == ButtonInteraction::Hovered && new_interaction == ButtonInteraction::Idle && button.availability == Availability::Enabled;

    if is_hover_leave {
        animate_color_or_set(&button.visuals.backplate_brush, backplate, std::time::Duration::from_millis(150))?;
        animate_color_or_set(&button.visuals.glyph_brush, foreground, std::time::Duration::from_millis(100))?;
    } else {
        button.visuals.backplate_brush.SetColor(backplate)?;
        button.visuals.glyph_brush.SetColor(foreground)?;
    }
    button.last_applied_interaction = new_interaction;
    Ok(())
}

/// Spec §6.3: `StartAnimation` failure logs at warning level and falls
/// back to an instant `SetColor`. The visual jumps instead of fading.
fn animate_color_or_set(brush: &CompositionColorBrush, target: windows::UI::Color, duration: std::time::Duration) -> anyhow::Result<()> {
    if let Err(err) = animate_color(brush, target, duration) {
        log::warn!("CaptionButtonStrip: color animation failed; applying target colour directly: {err}");
        brush.SetColor(target)?;
    }
    Ok(())
}

fn animate_color(brush: &CompositionColorBrush, target: windows::UI::Color, duration: std::time::Duration) -> anyhow::Result<()> {
    let anim = brush.Compositor()?.CreateColorKeyFrameAnimation()?;
    anim.SetDuration(TimeSpan {
        Duration: (duration.as_nanos() / 100) as i64,
    })?;
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
    if kind == CaptionButtonKind::Close {
        match interaction {
            ButtonInteraction::Hovered => return (theme.close_backplate_hover, theme.close_foreground_hover),
            ButtonInteraction::Pressed => return (theme.close_backplate_pressed, theme.close_foreground_pressed),
            _ => {}
        }
    }
    match interaction {
        ButtonInteraction::Idle | ButtonInteraction::PressedDraggedOff => {
            let foreground = if is_active {
                theme.foreground_rest
            } else {
                theme.foreground_inactive
            };
            (theme.backplate_rest, foreground)
        }
        ButtonInteraction::Hovered => (theme.backplate_hover, theme.foreground_hover),
        ButtonInteraction::Pressed => (theme.backplate_pressed, theme.foreground_pressed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::win32::geometry::PhysicalPixels;
    use crate::win32::window_api::{WindowStyle, WindowSystemBackdropType, WindowTitleBarKind};

    fn session(kind: CaptionButtonKind, _device: PointerDeviceKind) -> PressSession {
        PressSession {
            pointer_id: 1,
            captured_kind: kind,
            mode: PressSessionMode::Active,
        }
    }

    fn style_with(is_min: bool, is_max: bool, is_resize: bool) -> WindowStyle {
        WindowStyle {
            title_bar_kind: WindowTitleBarKind::Custom,
            is_resizable: is_resize,
            is_minimizable: is_min,
            is_maximizable: is_max,
            system_backdrop_type: WindowSystemBackdropType::Auto,
        }
    }

    fn pt(x: i32, y: i32) -> PhysicalPoint {
        PhysicalPoint {
            x: PhysicalPixels(x),
            y: PhysicalPixels(y),
        }
    }

    fn ltr_geometry(width: i32, kinds: CaptionButtonKinds) -> StripGeometry {
        StripGeometry {
            reference_width_px: width,
            top_offset_px: 0,
            metrics: CaptionButtonMetrics::new(1.0),
            visible_kinds: kinds,
        }
    }

    fn ltr_geometry_with_top_offset(width: i32, top_offset_px: i32, kinds: CaptionButtonKinds) -> StripGeometry {
        StripGeometry {
            reference_width_px: width,
            top_offset_px,
            metrics: CaptionButtonMetrics::new(1.0),
            visible_kinds: kinds,
        }
    }

    // --- resolve_interaction ---

    #[test]
    fn disabled_button_is_idle_regardless_of_input() {
        let r = resolve_interaction(
            CaptionButtonKind::Maximize,
            Availability::Disabled,
            Some(CaptionButtonKind::Maximize),
            Some(PointerDeviceKind::Mouse),
            None,
        );
        assert_eq!(r, ButtonInteraction::Idle);
    }

    #[test]
    fn no_pointer_no_press_is_idle() {
        let r = resolve_interaction(CaptionButtonKind::Close, Availability::Enabled, None, None, None);
        assert_eq!(r, ButtonInteraction::Idle);
    }

    #[test]
    fn pointer_over_self_with_mouse_is_hovered() {
        let r = resolve_interaction(
            CaptionButtonKind::Close,
            Availability::Enabled,
            Some(CaptionButtonKind::Close),
            Some(PointerDeviceKind::Mouse),
            None,
        );
        assert_eq!(r, ButtonInteraction::Hovered);
    }

    #[test]
    fn pointer_over_self_with_pen_is_hovered() {
        let r = resolve_interaction(
            CaptionButtonKind::Close,
            Availability::Enabled,
            Some(CaptionButtonKind::Close),
            Some(PointerDeviceKind::Pen),
            None,
        );
        assert_eq!(r, ButtonInteraction::Hovered);
    }

    #[test]
    fn pointer_over_self_with_touch_skips_hover() {
        let r = resolve_interaction(
            CaptionButtonKind::Close,
            Availability::Enabled,
            Some(CaptionButtonKind::Close),
            Some(PointerDeviceKind::Touch),
            None,
        );
        assert_eq!(r, ButtonInteraction::Idle);
    }

    #[test]
    fn captured_self_with_pointer_inside_is_pressed() {
        let s = session(CaptionButtonKind::Close, PointerDeviceKind::Mouse);
        let r = resolve_interaction(
            CaptionButtonKind::Close,
            Availability::Enabled,
            Some(CaptionButtonKind::Close),
            Some(PointerDeviceKind::Mouse),
            Some(&s),
        );
        assert_eq!(r, ButtonInteraction::Pressed);
    }

    #[test]
    fn captured_self_with_pointer_outside_is_pressed_dragged_off() {
        let s = session(CaptionButtonKind::Close, PointerDeviceKind::Mouse);
        let r = resolve_interaction(
            CaptionButtonKind::Close,
            Availability::Enabled,
            None,
            Some(PointerDeviceKind::Mouse),
            Some(&s),
        );
        assert_eq!(r, ButtonInteraction::PressedDraggedOff);
    }

    #[test]
    fn captured_other_button_keeps_self_idle_winui_capture_rule() {
        // Press is on Minimize; pointer moves over Close. Close stays Idle.
        let s = session(CaptionButtonKind::Minimize, PointerDeviceKind::Mouse);
        let r = resolve_interaction(
            CaptionButtonKind::Close,
            Availability::Enabled,
            Some(CaptionButtonKind::Close),
            Some(PointerDeviceKind::Mouse),
            Some(&s),
        );
        assert_eq!(r, ButtonInteraction::Idle);
    }

    #[test]
    fn captured_other_with_touch_keeps_self_idle() {
        let s = session(CaptionButtonKind::Minimize, PointerDeviceKind::Touch);
        let r = resolve_interaction(
            CaptionButtonKind::Close,
            Availability::Enabled,
            Some(CaptionButtonKind::Close),
            Some(PointerDeviceKind::Touch),
            Some(&s),
        );
        assert_eq!(r, ButtonInteraction::Idle);
    }

    #[test]
    fn suppressed_session_does_not_drive_pressed_state_on_captured() {
        // A Suppressed session — e.g. non-primary press over Close —
        // must not render Close as Pressed even though the cursor is
        // over the captured button. Verifies the `mode == Active` gate
        // on the first match arm.
        let suppressed = PressSession {
            pointer_id: 1,
            captured_kind: CaptionButtonKind::Close,
            mode: PressSessionMode::Suppressed {
                held_button: PointerButton::Right,
            },
        };
        let r = resolve_interaction(
            CaptionButtonKind::Close,
            Availability::Enabled,
            Some(CaptionButtonKind::Close),
            Some(PointerDeviceKind::Mouse),
            Some(&suppressed),
        );
        assert_eq!(r, ButtonInteraction::Hovered);
    }

    #[test]
    fn suppressed_session_does_not_suppress_neighbour_hover() {
        // A Suppressed session over Close must not force Maximize to
        // Idle when the cursor moves over Maximize. Primary presses
        // suppress neighbour hover (existing rule, preserved by the
        // `mode == Active` second-arm gate); non-primary swallows do not.
        let suppressed = PressSession {
            pointer_id: 1,
            captured_kind: CaptionButtonKind::Close,
            mode: PressSessionMode::Suppressed {
                held_button: PointerButton::Right,
            },
        };
        let r = resolve_interaction(
            CaptionButtonKind::Maximize,
            Availability::Enabled,
            Some(CaptionButtonKind::Maximize),
            Some(PointerDeviceKind::Mouse),
            Some(&suppressed),
        );
        assert_eq!(r, ButtonInteraction::Hovered);
    }

    #[test]
    fn active_session_still_suppresses_neighbour_hover() {
        // Regression check: the existing rule ("primary press over X
        // suppresses hover on every other button") must still hold.
        let active = PressSession {
            pointer_id: 1,
            captured_kind: CaptionButtonKind::Close,
            mode: PressSessionMode::Active,
        };
        let r = resolve_interaction(
            CaptionButtonKind::Maximize,
            Availability::Enabled,
            Some(CaptionButtonKind::Maximize),
            Some(PointerDeviceKind::Mouse),
            Some(&active),
        );
        assert_eq!(r, ButtonInteraction::Idle);
    }

    // --- track_swallowed_press / consume_swallowed_release ---

    #[test]
    fn swallowed_release_matches_pointer_and_button() {
        let s = PressSession {
            pointer_id: 7,
            captured_kind: CaptionButtonKind::Close,
            mode: PressSessionMode::Suppressed {
                held_button: PointerButton::Right,
            },
        };
        assert!(!is_swallowed_release_match(&s, 7, PointerButton::Left)); // wrong button
        assert!(!is_swallowed_release_match(&s, 8, PointerButton::Right)); // wrong pointer
        assert!(is_swallowed_release_match(&s, 7, PointerButton::Right)); // match
    }

    #[test]
    fn swallowed_release_skips_active_sessions() {
        let s = PressSession {
            pointer_id: 7,
            captured_kind: CaptionButtonKind::Close,
            mode: PressSessionMode::Active,
        };
        // Active sessions must NOT match — the primary-action path owns
        // their drainage.
        assert!(!is_swallowed_release_match(&s, 7, PointerButton::Left));
    }

    // --- is_real_release_match (on_pointer_up local guard) ---

    #[test]
    fn real_release_matches_active_session_with_same_pointer_id() {
        let session = PressSession {
            pointer_id: 7,
            captured_kind: CaptionButtonKind::Close,
            mode: PressSessionMode::Active,
        };
        assert!(is_real_release_match(&session, 7));
    }

    #[test]
    fn real_release_matches_suppressed_left_disabled_primary_cycle() {
        let session = PressSession {
            pointer_id: 7,
            captured_kind: CaptionButtonKind::Maximize,
            mode: PressSessionMode::Suppressed {
                held_button: PointerButton::Left,
            },
        };
        assert!(is_real_release_match(&session, 7));
    }

    #[test]
    fn real_release_rejects_suppressed_right_so_caller_preserves_session_for_drain() {
        let session = PressSession {
            pointer_id: 7,
            captured_kind: CaptionButtonKind::Close,
            mode: PressSessionMode::Suppressed {
                held_button: PointerButton::Right,
            },
        };
        assert!(!is_real_release_match(&session, 7));
    }

    #[test]
    fn real_release_rejects_mismatched_pointer_id() {
        let session = PressSession {
            pointer_id: 7,
            captured_kind: CaptionButtonKind::Close,
            mode: PressSessionMode::Active,
        };
        assert!(!is_real_release_match(&session, 99));
    }

    #[test]
    fn real_release_in_outer_match_preserves_suppressed_right_session() {
        let mut press_session = Some(PressSession {
            pointer_id: 7,
            captured_kind: CaptionButtonKind::Close,
            mode: PressSessionMode::Suppressed {
                held_button: PointerButton::Right,
            },
        });
        let pointer_id = 7;
        let cleared = match press_session {
            Some(s) if is_real_release_match(&s, pointer_id) => {
                press_session = None;
                Some(s)
            }
            _ => None,
        };
        assert!(cleared.is_none());
        assert!(press_session.is_some());
    }

    // --- cancel_press_session (capture-loss / deactivation cleanup) ---

    #[test]
    fn cancel_clears_active_session_and_signals_visual_refresh() {
        let mut session = Some(PressSession {
            pointer_id: 7,
            captured_kind: CaptionButtonKind::Close,
            mode: PressSessionMode::Active,
        });
        let needs_visual_refresh = cancel_press_session(&mut session, 7);
        assert!(session.is_none());
        assert!(needs_visual_refresh);
    }

    #[test]
    fn cancel_clears_suppressed_left_session_without_visual_refresh() {
        let mut session = Some(PressSession {
            pointer_id: 7,
            captured_kind: CaptionButtonKind::Close,
            mode: PressSessionMode::Suppressed {
                held_button: PointerButton::Left,
            },
        });
        let needs_visual_refresh = cancel_press_session(&mut session, 7);
        assert!(session.is_none());
        assert!(!needs_visual_refresh);
    }

    #[test]
    fn cancel_clears_suppressed_right_session_regression_for_capture_loss_leak() {
        let mut session = Some(PressSession {
            pointer_id: 7,
            captured_kind: CaptionButtonKind::Close,
            mode: PressSessionMode::Suppressed {
                held_button: PointerButton::Right,
            },
        });
        let needs_visual_refresh = cancel_press_session(&mut session, 7);
        assert!(session.is_none());
        assert!(!needs_visual_refresh);
    }

    #[test]
    fn cancel_with_mismatched_pointer_id_is_a_noop() {
        let original = PressSession {
            pointer_id: 7,
            captured_kind: CaptionButtonKind::Close,
            mode: PressSessionMode::Active,
        };
        let mut session = Some(original);
        let needs_visual_refresh = cancel_press_session(&mut session, 99);
        assert_eq!(session, Some(original));
        assert!(!needs_visual_refresh);
    }

    #[test]
    fn cancel_press_session_with_no_session_is_a_noop() {
        let mut session: Option<PressSession> = None;
        let needs_visual_refresh = cancel_press_session(&mut session, 0);
        assert!(session.is_none());
        assert!(!needs_visual_refresh);
    }

    #[test]
    fn cancel_any_press_session_clears_active_regardless_of_pointer_id() {
        // `WM_CANCELMODE` does not carry a pointer id, so the strip must
        // drop the session unconditionally.
        let mut session = Some(PressSession {
            pointer_id: 7,
            captured_kind: CaptionButtonKind::Maximize,
            mode: PressSessionMode::Active,
        });
        let needs_visual_refresh = cancel_any_press_session(&mut session);
        assert!(session.is_none());
        assert!(needs_visual_refresh);
    }

    #[test]
    fn cancel_any_press_session_clears_suppressed_without_visual_refresh() {
        let mut session = Some(PressSession {
            pointer_id: 7,
            captured_kind: CaptionButtonKind::Close,
            mode: PressSessionMode::Suppressed {
                held_button: PointerButton::Left,
            },
        });
        let needs_visual_refresh = cancel_any_press_session(&mut session);
        assert!(session.is_none());
        assert!(!needs_visual_refresh);
    }

    #[test]
    fn cancel_any_press_session_with_no_session_is_a_noop() {
        let mut session: Option<PressSession> = None;
        let needs_visual_refresh = cancel_any_press_session(&mut session);
        assert!(session.is_none());
        assert!(!needs_visual_refresh);
    }

    // --- visible-button derivation ---

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
    fn non_resizable_keeps_maximize_enabled_when_maximizable_bit_is_set() {
        let style = style_with(true, true, false);
        assert_eq!(availability_from_style(CaptionButtonKind::Minimize, &style), Availability::Enabled);
        assert_eq!(availability_from_style(CaptionButtonKind::Maximize, &style), Availability::Enabled);
    }

    // --- CaptionTheme palette ---

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
        let dark = CaptionTheme::resolve(Appearance::Dark, HighContrast::Off);
        assert_eq!(light.close_backplate_hover, dark.close_backplate_hover);
        assert_eq!(light.close_backplate_hover.R, 0xC4);
    }

    #[test]
    fn close_pressed_alpha_is_e6_in_both_themes() {
        // Brush.Opacity 0.9 → alpha 0xE6 (0.9 × 255 = 229.5, rounded to nearest).
        let light = CaptionTheme::resolve(Appearance::Light, HighContrast::Off);
        let dark = CaptionTheme::resolve(Appearance::Dark, HighContrast::Off);
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

    // --- CaptionButtonMetrics ---

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

    // --- StripGeometry::hit_test ---

    #[test]
    fn hits_close_in_rightmost_46px() {
        let g = ltr_geometry(
            800,
            CaptionButtonKinds::empty()
                .with(CaptionButtonKind::Minimize)
                .with(CaptionButtonKind::Maximize)
                .with(CaptionButtonKind::Close),
        );
        assert_eq!(g.hit_test(pt(799, 16)), Some(CaptionButtonKind::Close));
        assert_eq!(g.hit_test(pt(754, 16)), Some(CaptionButtonKind::Close));
    }

    #[test]
    fn hits_maximize_left_of_close() {
        let g = ltr_geometry(
            800,
            CaptionButtonKinds::empty()
                .with(CaptionButtonKind::Minimize)
                .with(CaptionButtonKind::Maximize)
                .with(CaptionButtonKind::Close),
        );
        assert_eq!(g.hit_test(pt(753, 16)), Some(CaptionButtonKind::Maximize));
        assert_eq!(g.hit_test(pt(708, 16)), Some(CaptionButtonKind::Maximize));
    }

    #[test]
    fn hits_minimize_left_of_maximize() {
        let g = ltr_geometry(
            800,
            CaptionButtonKinds::empty()
                .with(CaptionButtonKind::Minimize)
                .with(CaptionButtonKind::Maximize)
                .with(CaptionButtonKind::Close),
        );
        assert_eq!(g.hit_test(pt(707, 16)), Some(CaptionButtonKind::Minimize));
        assert_eq!(g.hit_test(pt(662, 16)), Some(CaptionButtonKind::Minimize));
    }

    #[test]
    fn no_hit_outside_strip_height() {
        let g = ltr_geometry(800, CaptionButtonKinds::empty().with(CaptionButtonKind::Close));
        assert_eq!(g.hit_test(pt(799, 32)), None);
        assert_eq!(g.hit_test(pt(799, -1)), None);
    }

    #[test]
    fn no_hit_for_hidden_button_kinds_not_in_visible_set() {
        let g = ltr_geometry(800, CaptionButtonKinds::empty().with(CaptionButtonKind::Close));
        // Maximize would have lived at x in [708, 753] if visible. With Maximize
        // hidden the layout shifts left; (753) is now outside Close.
        assert_eq!(g.hit_test(pt(753, 16)), None);
    }

    // --- hittest_for_caption_button_kind ---

    #[test]
    fn enabled_min_max_close_return_their_dedicated_codes() {
        use windows::Win32::UI::WindowsAndMessaging::{HTCLOSE, HTMAXBUTTON, HTMINBUTTON};
        assert_eq!(hittest_for_caption_button_kind(CaptionButtonKind::Minimize, true), HTMINBUTTON);
        assert_eq!(hittest_for_caption_button_kind(CaptionButtonKind::Maximize, true), HTMAXBUTTON);
        assert_eq!(hittest_for_caption_button_kind(CaptionButtonKind::Close, true), HTCLOSE);
    }

    #[test]
    fn disabled_min_max_collapse_to_htcaption_not_their_codes() {
        // Snap Layouts must not appear on a disabled Maximize button; the
        // rectangle keeps caption-like hit-test semantics.
        use windows::Win32::UI::WindowsAndMessaging::{HTCAPTION, HTCLOSE};
        assert_eq!(hittest_for_caption_button_kind(CaptionButtonKind::Minimize, false), HTCAPTION);
        assert_eq!(hittest_for_caption_button_kind(CaptionButtonKind::Maximize, false), HTCAPTION);
        // Close is always enabled, but the policy still returns HTCLOSE for completeness.
        assert_eq!(hittest_for_caption_button_kind(CaptionButtonKind::Close, false), HTCLOSE);
    }

    // --- caption_kind_at_screen exercise via StripGeometry ---
    //
    // `caption_kind_at_screen` is the wndproc-facing wrapper. Its body is:
    //
    //   ScreenToClient(hwnd, &mut p);
    //   GetClientRect(hwnd, &mut r);
    //   strip.hit_test(client_pt, client_width)
    //
    // `strip.hit_test` constructs a `StripGeometry` with `reference_width_px =
    // client_width` and forwards to `StripGeometry::hit_test`. The Win32
    // calls themselves can't be exercised without a real HWND, but the math
    // they feed into is the same code path the tests below already cover —
    // a client-space hit-test against an 800-px-wide reference. The cases
    // here pin the client-space-coordinates contract (origin = client
    // top-left, strip anchored at the right edge).

    #[test]
    fn client_space_hit_test_anchors_strip_at_right_edge() {
        let kinds = CaptionButtonKinds::empty()
            .with(CaptionButtonKind::Minimize)
            .with(CaptionButtonKind::Maximize)
            .with(CaptionButtonKind::Close);
        let g = StripGeometry {
            reference_width_px: 800,
            top_offset_px: 0,
            metrics: CaptionButtonMetrics::new(1.0),
            visible_kinds: kinds,
        };
        // Strip = three 46-px-wide buttons anchored at x=800.
        // Min: [662, 708), Max: [708, 754), Close: [754, 800).
        assert_eq!(g.hit_test(pt(799, 0)), Some(CaptionButtonKind::Close));
        assert_eq!(g.hit_test(pt(708, 0)), Some(CaptionButtonKind::Maximize));
        assert_eq!(g.hit_test(pt(662, 0)), Some(CaptionButtonKind::Minimize));
        // x just left of the strip → no hit.
        assert_eq!(g.hit_test(pt(661, 0)), None);
        // y outside the button height (32 px) → no hit, even within x range.
        assert_eq!(g.hit_test(pt(799, 32)), None);
    }

    #[test]
    fn client_space_hit_test_handles_negative_y_above_client() {
        // Negative y can occur if the cursor is above the client area
        // (e.g., NC pointer events near the top edge after the overhang
        // inset). Should always return `None`.
        let kinds = CaptionButtonKinds::empty().with(CaptionButtonKind::Close);
        let g = StripGeometry {
            reference_width_px: 800,
            top_offset_px: 0,
            metrics: CaptionButtonMetrics::new(1.0),
            visible_kinds: kinds,
        };
        assert_eq!(g.hit_test(pt(799, -1)), None);
        assert_eq!(g.hit_test(pt(799, -100)), None);
    }

    #[test]
    fn hit_test_respects_maximized_strip_top_offset() {
        let g = ltr_geometry_with_top_offset(800, 8, CaptionButtonKinds::empty().with(CaptionButtonKind::Close));
        assert_eq!(g.hit_test(pt(799, 7)), None);
        assert_eq!(g.hit_test(pt(799, 8)), Some(CaptionButtonKind::Close));
        assert_eq!(g.hit_test(pt(799, 39)), Some(CaptionButtonKind::Close));
        assert_eq!(g.hit_test(pt(799, 40)), None);
    }

    #[test]
    fn inactive_idle_uses_tertiary_foreground_with_transparent_backplate_light() {
        let theme = CaptionTheme::light();
        let (bg, fg) = colours_for(
            CaptionButtonKind::Minimize,
            Availability::Enabled,
            ButtonInteraction::Idle,
            &theme,
            false,
        );
        assert_eq!(bg, theme.backplate_rest);
        assert_eq!(fg, rgba(0, 0, 0, 0x72));
    }

    #[test]
    fn inactive_idle_uses_tertiary_foreground_dark() {
        let theme = CaptionTheme::dark();
        let (_, fg) = colours_for(
            CaptionButtonKind::Minimize,
            Availability::Enabled,
            ButtonInteraction::Idle,
            &theme,
            false,
        );
        assert_eq!(fg, rgba(0xFF, 0xFF, 0xFF, 0x87));
    }

    #[test]
    fn inactive_hovered_matches_active_hovered() {
        let theme = CaptionTheme::dark();
        let inactive = colours_for(
            CaptionButtonKind::Minimize,
            Availability::Enabled,
            ButtonInteraction::Hovered,
            &theme,
            false,
        );
        let active = colours_for(
            CaptionButtonKind::Minimize,
            Availability::Enabled,
            ButtonInteraction::Hovered,
            &theme,
            true,
        );
        assert_eq!(inactive, active);
    }

    #[test]
    fn inactive_pressed_matches_active_pressed() {
        let theme = CaptionTheme::light();
        let inactive = colours_for(
            CaptionButtonKind::Maximize,
            Availability::Enabled,
            ButtonInteraction::Pressed,
            &theme,
            false,
        );
        let active = colours_for(
            CaptionButtonKind::Maximize,
            Availability::Enabled,
            ButtonInteraction::Pressed,
            &theme,
            true,
        );
        assert_eq!(inactive, active);
    }

    #[test]
    fn inactive_close_hover_keeps_close_red() {
        let theme = CaptionTheme::light();
        let (bg, fg) = colours_for(
            CaptionButtonKind::Close,
            Availability::Enabled,
            ButtonInteraction::Hovered,
            &theme,
            false,
        );
        assert_eq!(bg, theme.close_backplate_hover);
        assert_eq!(fg, theme.close_foreground_hover);
    }
}
