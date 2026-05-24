//! Caption-button strip for `WindowTitleBarKind::Custom` windows.
//!
//! See `docs/specs/2026-04-30-win32-caption-buttons-design.md` for the design.
//!
//! State machine driven by typed inputs from the wndproc layer (kind, pointer
//! id, theme, etc.) producing typed outputs (`Option<CaptionButtonAction>`).
//! Win32 coord-transform utilities (`ScreenToClient` / `GetClientRect`) live in
//! `caption_kind_at_screen`; `WM_*` message decoding stays in `event_loop.rs`.
//!
//! Lint allowances: pixel-bounded `i32` geometry cast to `f32` (`Vector2` / `Vector3`)
//! and `SizeInt32`. Button counts (≤ 3) and pixel sizes (≤ a few hundred) never
//! approach lossy cast ranges.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::rc::Rc;

use windows::{
    Foundation::TimeSpan,
    Graphics::SizeInt32,
    UI::Composition::{CompositionColorBrush, CompositionDrawingSurface, Compositor, ContainerVisual, SpriteVisual},
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
        UI::WindowsAndMessaging::{GetClientRect, GetWindowRect, PostMessageW},
    },
};
use windows_core::Interface;
use windows_numerics::{Vector2, Vector3};

use super::{
    appearance::{Appearance, HighContrast},
    composition::{CompositionContext, RenderingDeviceReplacedRegistration},
    geometry::{LogicalSize, PhysicalPixels, PhysicalPoint, PhysicalSize},
    pointer::PointerButton,
    window::Window,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub(crate) enum CaptionButtonKind {
    // Discriminants are load-bearing: `CaptionButtonKinds` uses `1 << kind as u8`
    // as a bitmask. Reordering or inserting a variant silently breaks every consumer.
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

/// Bitset of which caption-button kinds are visible on a window.
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

    /// Yields the visible kinds in left-to-right system order (Minimize → Maximize → Close).
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

/// Free predicate behind `CaptionButtonStrip::consume_swallowed_release` (unit-testable without live resources).
fn is_swallowed_release_match(session: &PressSession, pointer_id: u32, button: PointerButton) -> bool {
    session.pointer_id == pointer_id && matches!(session.mode, PressSessionMode::Suppressed { held_button } if held_button == button)
}

/// True iff `session` is a primary release for `pointer_id` (`Active` or `Suppressed{Left}`).
/// Non-primary `Suppressed` sessions are drained by `consume_swallowed_release`.
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

    // WinUI Fluent palette: microsoft/microsoft-ui-xaml@5f9e851133b.
    // Close reds: microsoft/terminal@e4e3f08efca MinMaxCloseControl.xaml
    // (Opacity 0.9 → α=0xE6; Opacity 0.7 → α=0xB3; source RGB fully opaque, rounded to nearest).
    // Inactive: TitleBarDeactivatedForegroundBrush → TextFillColorTertiary.
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
    /// Strip right edge; strip-left = `reference_width_px - button_count * button_width`.
    reference_width_px: i32,
    /// Vertical offset of the strip. Maximized windows shift the strip down by
    /// their chrome overhang; hit-testing subtracts this before checking button y.
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

/// `WM_NCHITTEST` return code for an enabled button (`HTMINBUTTON` / `HTMAXBUTTON`
/// / `HTCLOSE`); disabled `Minimize` / `Maximize` collapse to `HTCAPTION`.
pub(crate) const fn hittest_for_caption_button_kind(kind: CaptionButtonKind, is_enabled: bool) -> u32 {
    use windows::Win32::UI::WindowsAndMessaging::{HTCAPTION, HTCLOSE, HTMAXBUTTON, HTMINBUTTON};
    match (kind, is_enabled) {
        (CaptionButtonKind::Close, _) => HTCLOSE,
        (CaptionButtonKind::Minimize, true) => HTMINBUTTON,
        (CaptionButtonKind::Maximize, true) => HTMAXBUTTON,
        (CaptionButtonKind::Minimize | CaptionButtonKind::Maximize, false) => HTCAPTION,
    }
}

/// Hit-test the caption-button strip from screen-space coordinates.
/// Returns `None` if no strip exists, the point is outside strip bounds, or
/// the point is inside the top resize-border band on a restored resizable window
/// (that band must reach `DefWindowProc` for the system resize cursor/drag loop).
/// Win32 coord-transform calls (`ScreenToClient`, `GetClientRect`) live here
/// because they're tightly coupled with the strip's hit-test math.
pub(crate) fn caption_kind_at_screen(window: &Window, screen: PhysicalPoint) -> Option<CaptionButtonKind> {
    let strip_ref = window.caption_buttons.borrow();
    let strip = strip_ref.as_ref()?;
    let hwnd = window.hwnd();
    if is_in_top_resize_border(window, screen.y.0) {
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

/// True when `mouse_y` (screen-space) is inside the top resize-border band
/// of a restored resizable window.
fn is_in_top_resize_border(window: &Window, mouse_y: i32) -> bool {
    if !window.is_resizable() || window.is_maximized() {
        return false;
    }
    let mut window_rect = RECT::default();
    if unsafe { GetWindowRect(window.hwnd(), &raw mut window_rect) }
        .inspect_err(|err| log::warn!("GetWindowRect failed during resize-border check: {err}"))
        .is_err()
    {
        return false;
    }
    let m = window.dpi_metrics();
    mouse_y < window_rect.top + m.padded_border + m.size_frame
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
    /// Active press sessions. At most one `Active` (or `Suppressed{Left}`) at a time,
    /// drained by `on_pointer_up`. Non-primary `Suppressed` entries coexist,
    /// drained by `consume_swallowed_release`.
    press_sessions: Vec<PressSession>,
    top_offset_px: i32,
    composition_context: Rc<CompositionContext>,
    // Held for Drop side-effect: removes the RDR subscription.
    #[allow(dead_code)]
    device_replaced_registration: RenderingDeviceReplacedRegistration,
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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        chrome_layer: &ContainerVisual,
        initial_scale: f32,
        style: &crate::win32::window_api::WindowStyle,
        compositor: &Compositor,
        hwnd: HWND,
        initial_is_active: bool,
        initial_is_maximized: bool,
        initial_top_offset_px: i32,
    ) -> anyhow::Result<Self> {
        let composition_context = crate::win32::composition::ensure_composition_context(compositor.clone())?;

        // RDR callback is `Send` and may fire on any thread; post to the HWND so
        // re-rasterisation runs on the UI thread (avoids nested `BeginDraw`).
        let device_replaced_registration = {
            let hwnd_value = hwnd.0 as isize;
            composition_context.add_rendering_device_replaced_callback(move || unsafe {
                let _ = PostMessageW(
                    Some(HWND(hwnd_value as _)),
                    crate::win32::event_loop::WM_APP_CAPTION_BUTTONS_RENDERING_DEVICE_REPLACED,
                    WPARAM(0),
                    LPARAM(0),
                );
            })?
        };

        let composition_root = compositor.CreateContainerVisual()?;

        let visible_kinds = CaptionButtonKinds::from_style(style);
        let metrics = CaptionButtonMetrics::new(initial_scale);

        let mut buttons = Vec::new();
        for kind in visible_kinds.iter_ordered() {
            let availability = availability_from_style(kind, style);
            buttons.push(create_caption_button(
                compositor,
                &composition_root,
                &composition_context,
                kind,
                availability,
                &metrics,
            )?);
        }

        composition_root
            .SetRelativeOffsetAdjustment(Vector3 { X: 1.0, Y: 0.0, Z: 0.0 })
            .inspect_err(|err| log::warn!("composition_root SetRelativeOffsetAdjustment failed: {err}"))?;
        composition_root
            .SetAnchorPoint(Vector2::new(1.0, 0.0))
            .inspect_err(|err| log::warn!("composition_root SetAnchorPoint failed: {err}"))?;

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
            is_active: initial_is_active,
            is_window_maximized: initial_is_maximized,
            appearance,
            high_contrast,
            metrics,
            pointer_over_kind: None,
            pointer_device: None,
            press_sessions: Vec::new(),
            top_offset_px: initial_top_offset_px,
            composition_context,
            device_replaced_registration,
        };
        strip.relayout();
        strip.set_strip_position(initial_top_offset_px);
        strip.apply_visuals_to_all_buttons();
        chrome_layer.Children()?.InsertAtTop(&strip.composition_root)?;
        Ok(strip)
    }

    fn relayout(&mut self) {
        let bw = self.metrics.button_size_px.width.0;
        let bh = self.metrics.button_size_px.height.0;
        let total_width = bw * self.buttons.len() as i32;
        // Right-edge auto-tracks chrome_layer via RelativeOffsetAdjustment(1,0,0) +
        // AnchorPoint(1,0) set in `new`; `set_strip_position` only mutates Offset.Y.
        let _ = self
            .composition_root
            .SetSize(Vector2::new(total_width as f32, bh as f32))
            .inspect_err(|err| log::warn!("composition_root SetSize failed: {err}"));
        let gw = self.metrics.glyph_extent_px.width.0;
        let gh = self.metrics.glyph_extent_px.height.0;
        let gx = (bw - gw) / 2;
        let gy = (bh - gh) / 2;
        for (i, button) in self.buttons.iter_mut().enumerate() {
            let x = (i as i32) * bw;
            let _ = button
                .visuals
                .backplate
                .SetOffset(Vector3 {
                    X: x as f32,
                    Y: 0.0,
                    Z: 0.0,
                })
                .inspect_err(|err| log::warn!("backplate SetOffset failed: {err}"));
            let _ = button
                .visuals
                .backplate
                .SetSize(Vector2::new(bw as f32, bh as f32))
                .inspect_err(|err| log::warn!("backplate SetSize failed: {err}"));
            let _ = button
                .visuals
                .glyph
                .SetOffset(Vector3 {
                    X: gx as f32,
                    Y: gy as f32,
                    Z: 0.0,
                })
                .inspect_err(|err| log::warn!("glyph SetOffset failed: {err}"));
            let _ = button
                .visuals
                .glyph
                .SetSize(Vector2::new(gw as f32, gh as f32))
                .inspect_err(|err| log::warn!("glyph SetSize failed: {err}"));
        }
    }

    fn set_strip_position(&mut self, max_chrome_y: i32) {
        self.top_offset_px = max_chrome_y;
        let _ = self
            .composition_root
            .SetOffset(Vector3 {
                X: 0.0,
                Y: max_chrome_y as f32,
                Z: 0.0,
            })
            .inspect_err(|err| log::warn!("composition_root SetOffset failed: {err}"));
    }

    fn rasterise_all_glyphs(&mut self) {
        for button in &mut self.buttons {
            if !button.glyph_surface_dirty {
                continue;
            }
            let drew = rasterise_glyph(
                &self.composition_context,
                &button.visuals.glyph_surface,
                button.kind,
                self.is_window_maximized,
                self.high_contrast,
                &self.metrics,
            )
            .inspect_err(|err| log::warn!("rasterise_glyph failed for {:?}: {err}", button.kind))
            .unwrap_or(false);
            if drew {
                button.glyph_surface_dirty = false;
            }
        }
    }

    fn apply_visuals_to_all_buttons(&mut self) {
        self.rasterise_all_glyphs();

        let theme = CaptionTheme::resolve(self.appearance, self.high_contrast);
        let pointer_over_kind = self.pointer_over_kind;
        let pointer_device = self.pointer_device;
        let primary_session = self.primary_session().copied();
        let is_active = self.is_active;
        for button in &mut self.buttons {
            let new_interaction = resolve_interaction(
                button.kind,
                button.availability,
                pointer_over_kind,
                pointer_device,
                primary_session.as_ref(),
            );
            apply_button_visuals(button, new_interaction, &theme, is_active);
        }
    }

    /// Session drained by primary-UP (`Active` or `Suppressed{Left}`). At most one exists.
    fn primary_session(&self) -> Option<&PressSession> {
        self.press_sessions.iter().find(|s| {
            matches!(
                s.mode,
                PressSessionMode::Active
                    | PressSessionMode::Suppressed {
                        held_button: PointerButton::Left
                    }
            )
        })
    }

    /// Hit-test a client-space point. `client_width` is the reference right edge.
    /// Returns the caption-button kind under the point, or `None` if outside.
    pub fn hit_test(&self, client_point: PhysicalPoint, client_width: PhysicalPixels) -> Option<CaptionButtonKind> {
        StripGeometry {
            reference_width_px: client_width.0,
            top_offset_px: self.top_offset_px,
            metrics: self.metrics,
            visible_kinds: self.visible_kinds,
        }
        .hit_test(client_point)
    }

    pub fn on_pointer_update(&mut self, kind: Option<CaptionButtonKind>, device: PointerDeviceKind) {
        // Clear device along with kind so a same-device re-entry isn't suppressed by stale state.
        let new_device = kind.map(|_| device);
        if self.pointer_over_kind != kind || self.pointer_device != new_device {
            self.pointer_over_kind = kind;
            self.pointer_device = new_device;
            self.apply_visuals_to_all_buttons();
        }
    }

    pub fn on_pointer_down(&mut self, kind: CaptionButtonKind, pointer_id: u32, device: PointerDeviceKind) {
        if self.primary_session().is_some() {
            return;
        }
        let is_enabled = self.button_for(kind).map(|b| b.availability) == Some(Availability::Enabled);
        // Disabled-primary records Suppressed{Left} so the primary-release branch consumes it silently.
        let mode = if is_enabled {
            PressSessionMode::Active
        } else {
            PressSessionMode::Suppressed {
                held_button: PointerButton::Left,
            }
        };
        self.press_sessions.push(PressSession {
            pointer_id,
            captured_kind: kind,
            mode,
        });
        if mode == PressSessionMode::Active {
            self.pointer_over_kind = Some(kind);
            self.pointer_device = Some(device);
            self.apply_visuals_to_all_buttons();
        }
    }

    pub fn on_pointer_up(&mut self, kind_under_pointer: Option<CaptionButtonKind>, pointer_id: u32) -> Option<CaptionButtonAction> {
        let idx = self.press_sessions.iter().position(|s| is_real_release_match(s, pointer_id))?;
        let session = self.press_sessions.swap_remove(idx);
        match session.mode {
            PressSessionMode::Active => {
                let action = (Some(session.captured_kind) == kind_under_pointer).then(|| self.action_for(session.captured_kind));
                self.pointer_over_kind = kind_under_pointer;
                self.apply_visuals_to_all_buttons();
                action
            }
            // Suppressed{Left}: disabled-primary cycle, no action.
            PressSessionMode::Suppressed { .. } => None,
        }
    }

    pub fn on_pointer_cancel(&mut self, pointer_id: u32) {
        // Visuals only need reverting if an Active session was canceled.
        let had_active = self
            .press_sessions
            .iter()
            .any(|s| s.pointer_id == pointer_id && s.mode == PressSessionMode::Active);
        self.press_sessions.retain(|s| s.pointer_id != pointer_id);
        if had_active {
            self.apply_visuals_to_all_buttons();
        }
    }

    /// Drop every owned press session. Used on `WM_CANCELMODE` and deactivation,
    /// where there is no pointer id to match against. Clears hover state; the next
    /// `WM_NCPOINTERUPDATE` re-establishes Hovered if the cursor is still over a button.
    pub fn cancel_any_press(&mut self) {
        let had_active = self.press_sessions.iter().any(|s| s.mode == PressSessionMode::Active);
        self.press_sessions.clear();
        if had_active {
            self.pointer_over_kind = None;
            self.pointer_device = None;
            self.apply_visuals_to_all_buttons();
        }
    }

    pub fn on_nc_mouse_leave(&mut self) {
        self.pointer_over_kind = None;
        self.pointer_device = None;
        self.apply_visuals_to_all_buttons();
    }

    fn button_for(&self, kind: CaptionButtonKind) -> Option<&CaptionButton> {
        self.buttons.iter().find(|b| b.kind == kind)
    }

    pub fn is_enabled(&self, kind: CaptionButtonKind) -> bool {
        self.button_for(kind).map(|b| b.availability) == Some(Availability::Enabled)
    }

    /// True iff the strip owns a session whose primary (Left) UP the wndproc must consume.
    pub(crate) fn has_active_press_for(&self, pointer_id: u32) -> bool {
        self.press_sessions.iter().any(|s| is_real_release_match(s, pointer_id))
    }

    /// True iff the strip owns any press session for `pointer_id`, regardless of mode.
    pub(crate) fn has_press_for(&self, pointer_id: u32) -> bool {
        self.press_sessions.iter().any(|s| s.pointer_id == pointer_id)
    }

    /// Record a wndproc-consumed non-primary DOWN as a `Suppressed` session keyed by `held_button`.
    /// Idempotent: skips if `(pointer_id, button)` is already tracked.
    /// `kind` fills `captured_kind` for struct uniformity; Suppressed consumers don't read it.
    pub(crate) fn track_swallowed_press(&mut self, kind: CaptionButtonKind, pointer_id: u32, button: PointerButton) {
        debug_assert!(button != PointerButton::None, "track_swallowed_press got None button");
        let already_tracked = self
            .press_sessions
            .iter()
            .any(|s| s.pointer_id == pointer_id && matches!(s.mode, PressSessionMode::Suppressed { held_button } if held_button == button));
        if already_tracked {
            return;
        }
        self.press_sessions.push(PressSession {
            pointer_id,
            captured_kind: kind,
            mode: PressSessionMode::Suppressed { held_button: button },
        });
    }

    /// Drain a `Suppressed` session matching `(pointer_id, button)`.
    /// Returns `true` iff a session was found and removed.
    /// Active sessions are drained by `on_pointer_up`; this only removes `Suppressed` entries.
    pub(crate) fn consume_swallowed_release(&mut self, pointer_id: u32, button: PointerButton) -> bool {
        if let Some(idx) = self
            .press_sessions
            .iter()
            .position(|s| is_swallowed_release_match(s, pointer_id, button))
        {
            self.press_sessions.swap_remove(idx);
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

    pub fn on_activate(&mut self, is_active: bool) {
        if self.is_active != is_active {
            self.is_active = is_active;
            // Activation flips never animate; reset history so the Hovered→Idle
            // predicate doesn't fire across the flip.
            for button in &mut self.buttons {
                button.last_applied_interaction = ButtonInteraction::Idle;
            }
            self.apply_visuals_to_all_buttons();
        }
    }

    pub fn on_dpi_change(&mut self, new_scale: f32, max_chrome_y: i32) -> anyhow::Result<()> {
        let new_metrics = CaptionButtonMetrics::new(new_scale);
        let new_size = SizeInt32 {
            Width: new_metrics.glyph_extent_px.width.0,
            Height: new_metrics.glyph_extent_px.height.0,
        };
        for button in &mut self.buttons {
            button
                .visuals
                .glyph_surface
                .Resize(new_size)
                .inspect_err(|err| log::warn!("glyph_surface Resize failed: {err}"))?;
            button.glyph_surface_dirty = true;
        }
        self.metrics = new_metrics;
        self.relayout();
        self.set_strip_position(max_chrome_y);
        self.apply_visuals_to_all_buttons();
        Ok(())
    }

    pub fn on_appearance_change(&mut self, appearance: Appearance, hc: HighContrast) {
        let glyph_invalidate = self.high_contrast != hc;
        self.appearance = appearance;
        self.high_contrast = hc;
        if glyph_invalidate {
            for button in &mut self.buttons {
                button.glyph_surface_dirty = true;
            }
        }
        self.apply_visuals_to_all_buttons();
    }

    pub fn on_rendering_device_replaced(&mut self) {
        for button in &mut self.buttons {
            button.glyph_surface_dirty = true;
        }
        self.rasterise_all_glyphs();
        self.apply_visuals_to_all_buttons();
    }

    pub fn on_max_state_change(&mut self, is_maximized: bool) {
        if self.is_window_maximized != is_maximized {
            self.is_window_maximized = is_maximized;
            for button in &mut self.buttons {
                if button.kind == CaptionButtonKind::Maximize {
                    button.glyph_surface_dirty = true;
                }
            }
            self.rasterise_all_glyphs();
        }
        // Visual mutations publish via the driver's CommitNeeded fast-path.
    }

    pub fn on_resize(&mut self, max_chrome_y: i32) {
        self.set_strip_position(max_chrome_y);
    }
}

impl Drop for CaptionButtonStrip {
    fn drop(&mut self) {
        // composition_root may be unparented if construction failed before InsertAtTop.
        if let Ok(parent) = self.composition_root.Parent()
            && let Ok(container) = parent.cast::<ContainerVisual>()
            && let Ok(children) = container.Children()
        {
            let _ = children
                .Remove(&self.composition_root)
                .inspect_err(|err| log::warn!("CaptionButtonStrip drop: detach failed: {err}"));
        }
    }
}

fn create_caption_button(
    compositor: &windows::UI::Composition::Compositor,
    parent: &ContainerVisual,
    composition_context: &Rc<CompositionContext>,
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

    // Mask-brush chain is kept alive by `glyph.SetBrush(&mask_brush)`; intermediates aren't stored.
    let glyph_surface = composition_context.create_drawing_surface(SizeInt32 {
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

/// DirectWrite font size (DIPs) that fits the glyph's black-box within `target_extent_px`.
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
    // Cell height = ascent + descent, not advanceHeight (which is the vertical advance).
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
    composition_context: &Rc<CompositionContext>,
    surface: &CompositionDrawingSurface,
    kind: CaptionButtonKind,
    is_maximised: bool,
    hc: HighContrast,
    metrics: &CaptionButtonMetrics,
) -> anyhow::Result<bool> {
    let glyph_char = glyph_for(kind, is_maximised, hc);
    let (font_family, font_face) = composition_context.caption_glyph_font()?;
    let font_size = compute_glyph_font_size(font_face, glyph_char, metrics.glyph_extent_px)?;
    let dwrite = composition_context.dwrite_factory();
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
    let drew = composition_context
        .with_d2d_render_target(surface, |rt, offset| {
            unsafe {
                // 96 DPI so 1 DIP == 1 pixel; compute_glyph_font_size assumes this.
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

fn apply_button_visuals(button: &mut CaptionButton, new_interaction: ButtonInteraction, theme: &CaptionTheme, is_active: bool) {
    let (backplate, foreground) = colours_for(button.kind, button.availability, new_interaction, theme, is_active);
    let prev = button.last_applied_interaction;
    // Only Hovered→Idle on Enabled buttons animates; everything else jumps.
    let is_hover_leave =
        prev == ButtonInteraction::Hovered && new_interaction == ButtonInteraction::Idle && button.availability == Availability::Enabled;

    if is_hover_leave {
        animate_color_or_set(&button.visuals.backplate_brush, backplate, std::time::Duration::from_millis(150));
        animate_color_or_set(&button.visuals.glyph_brush, foreground, std::time::Duration::from_millis(100));
    } else {
        let _ = button
            .visuals
            .backplate_brush
            .SetColor(backplate)
            .inspect_err(|err| log::warn!("backplate_brush SetColor failed: {err}"));
        let _ = button
            .visuals
            .glyph_brush
            .SetColor(foreground)
            .inspect_err(|err| log::warn!("glyph_brush SetColor failed: {err}"));
    }
    button.last_applied_interaction = new_interaction;
}

fn animate_color_or_set(brush: &CompositionColorBrush, target: windows::UI::Color, duration: std::time::Duration) {
    if let Err(err) = animate_color(brush, target, duration) {
        log::warn!("CaptionButtonStrip: color animation failed; applying target colour directly: {err}");
        let _ = brush
            .SetColor(target)
            .inspect_err(|err| log::warn!("animate_color_or_set fallback SetColor failed: {err}"));
    }
}

fn animate_color(brush: &CompositionColorBrush, target: windows::UI::Color, duration: std::time::Duration) -> anyhow::Result<()> {
    let anim = brush.Compositor()?.CreateColorKeyFrameAnimation()?;
    anim.SetDuration(TimeSpan {
        Duration: (duration.as_nanos() / 100) as i64,
    })?;
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

    fn session(kind: CaptionButtonKind) -> PressSession {
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
        let s = session(CaptionButtonKind::Close);
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
        let s = session(CaptionButtonKind::Close);
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
        let s = session(CaptionButtonKind::Minimize);
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
    fn suppressed_session_does_not_drive_pressed_state_on_captured() {
        // Suppressed session must not render the captured button as Pressed (verifies `mode == Active` gate).
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
        // Suppressed session must not force Maximize to Idle when cursor is over it
        // (verifies the `mode == Active` second-arm gate; primary presses do suppress).
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
        // Primary press over X must suppress hover on every other button.
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
        assert!(!is_swallowed_release_match(&s, 8, PointerButton::Right)); // wrong pointer id
        assert!(is_swallowed_release_match(&s, 7, PointerButton::Right)); // match
    }

    #[test]
    fn swallowed_release_skips_active_sessions() {
        let s = PressSession {
            pointer_id: 7,
            captured_kind: CaptionButtonKind::Close,
            mode: PressSessionMode::Active,
        };
        // Active sessions must not match; primary-action path drains them.
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

    // --- press_sessions Vec: Active + Suppressed coexistence ---

    fn active(pointer_id: u32, kind: CaptionButtonKind) -> PressSession {
        PressSession {
            pointer_id,
            captured_kind: kind,
            mode: PressSessionMode::Active,
        }
    }

    fn suppressed(pointer_id: u32, kind: CaptionButtonKind, button: PointerButton) -> PressSession {
        PressSession {
            pointer_id,
            captured_kind: kind,
            mode: PressSessionMode::Suppressed { held_button: button },
        }
    }

    #[test]
    fn non_primary_press_during_active_session_coexists() {
        let mut sessions = vec![active(1, CaptionButtonKind::Minimize)];
        sessions.push(suppressed(1, CaptionButtonKind::Close, PointerButton::Right));
        assert_eq!(sessions.len(), 2);

        // consume_swallowed_release(p=1, Right) removes only the Suppressed entry.
        let idx = sessions.iter().position(|s| is_swallowed_release_match(s, 1, PointerButton::Right));
        sessions.swap_remove(idx.expect("Suppressed{Right} not found"));
        assert_eq!(sessions.len(), 1);
        assert!(matches!(sessions[0].mode, PressSessionMode::Active));
    }

    #[test]
    fn active_release_does_not_drain_concurrent_suppressed() {
        let mut sessions = vec![
            active(1, CaptionButtonKind::Minimize),
            suppressed(1, CaptionButtonKind::Close, PointerButton::Right),
        ];

        // Primary-UP drain removes only the Active entry.
        let idx = sessions.iter().position(|s| is_real_release_match(s, 1));
        sessions.swap_remove(idx.expect("Active not found"));

        assert_eq!(sessions.len(), 1);
        assert!(is_swallowed_release_match(&sessions[0], 1, PointerButton::Right));
    }

    #[test]
    fn cancel_any_press_drops_active_and_suppressed_together() {
        let mut sessions = vec![
            active(1, CaptionButtonKind::Minimize),
            suppressed(1, CaptionButtonKind::Close, PointerButton::Right),
        ];
        let had_active = sessions.iter().any(|s| s.mode == PressSessionMode::Active);
        sessions.clear();
        assert!(had_active);
        assert!(sessions.is_empty());
    }

    #[test]
    fn on_pointer_cancel_drains_only_matching_pointer() {
        let mut sessions = vec![
            active(1, CaptionButtonKind::Minimize),
            suppressed(1, CaptionButtonKind::Close, PointerButton::Right),
            suppressed(2, CaptionButtonKind::Close, PointerButton::Right),
        ];
        sessions.retain(|s| s.pointer_id != 1);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].pointer_id, 2);
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
        // Opacity 0.9 → α=0xE6 (0.9 × 255 = 229.5, rounded to nearest).
        let light = CaptionTheme::resolve(Appearance::Light, HighContrast::Off);
        let dark = CaptionTheme::resolve(Appearance::Dark, HighContrast::Off);
        assert_eq!(light.close_backplate_pressed.A, 0xE6);
        assert_eq!(dark.close_backplate_pressed.A, 0xE6);
    }

    #[test]
    fn pressed_backplate_is_more_subtle_than_hover_in_off_branch() {
        // Fluent: SubtleFillColorTertiary (pressed) has lower alpha than SubtleFillColorSecondary (hover).
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
        // System colours vary by HC theme; hover == pressed holds across all four shipped themes.
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
        // LogicalSize::to_physical rounding: floor(v.mul_add(scale, 0.5)).
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
        // Maximize absent → layout shifts left; x=753 is outside Close's range.
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
        // Disabled Max must not trigger Snap Layouts; HTCAPTION keeps caption-drag semantics.
        use windows::Win32::UI::WindowsAndMessaging::{HTCAPTION, HTCLOSE};
        assert_eq!(hittest_for_caption_button_kind(CaptionButtonKind::Minimize, false), HTCAPTION);
        assert_eq!(hittest_for_caption_button_kind(CaptionButtonKind::Maximize, false), HTCAPTION);
        assert_eq!(hittest_for_caption_button_kind(CaptionButtonKind::Close, false), HTCLOSE);
    }

    // --- caption_kind_at_screen exercise via StripGeometry ---
    // Win32 calls require a real HWND; the math they feed is the same StripGeometry
    // path tested below. These cases pin the client-space contract
    // (origin = client top-left, strip anchored at right edge).

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
        // Three 46-px buttons: Min [662,708), Max [708,754), Close [754,800).
        assert_eq!(g.hit_test(pt(799, 0)), Some(CaptionButtonKind::Close));
        assert_eq!(g.hit_test(pt(708, 0)), Some(CaptionButtonKind::Maximize));
        assert_eq!(g.hit_test(pt(662, 0)), Some(CaptionButtonKind::Minimize));
        assert_eq!(g.hit_test(pt(661, 0)), None); // just left of strip
        assert_eq!(g.hit_test(pt(799, 32)), None); // outside button height (32 px)
    }

    #[test]
    fn client_space_hit_test_handles_negative_y_above_client() {
        // Negative y from NC pointer events near the top edge after overhang inset.
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
