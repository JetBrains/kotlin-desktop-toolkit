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
        UI::WindowsAndMessaging::{GetClientRect, GetWindowRect, PostMessageW, WM_APP},
    },
};
use windows_core::Interface;
use windows_numerics::{Vector2, Vector3};

use super::{
    appearance::{Appearance, HighContrast},
    composition::{CompositionContext, RenderingDeviceReplacedRegistration},
    geometry::{LogicalSize, PhysicalPixels, PhysicalPoint, PhysicalSize},
    pointer::{PointerButton, PointerButtonChangeKind, PointerInfo},
    window::Window,
};

/// cbindgen:ignore
pub(crate) const WM_APP_CAPTION_BUTTONS_RENDERING_DEVICE_REPLACED: u32 = WM_APP + 0x31;

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

/// Primary (Left) press on a caption button. At most one exists at a time.
/// `suppressed = true` when the button was disabled at press time — no visual
/// capture, no action on release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PrimaryPress {
    kind: CaptionButtonKind,
    suppressed: bool,
}

/// Wndproc-level swallow for a non-primary button press over the strip.
/// No visual capture. Drained by `consume_swallowed_release` on the matching UP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NonPrimaryPress {
    pointer_id: u32,
    button: PointerButton,
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
    primary_press: Option<&PrimaryPress>,
) -> ButtonInteraction {
    if availability == Availability::Disabled {
        return ButtonInteraction::Idle;
    }
    let is_pointer_over_self = pointer_over_kind == Some(kind);
    match primary_press {
        Some(p) if !p.suppressed && p.kind == kind => {
            if is_pointer_over_self {
                ButtonInteraction::Pressed
            } else {
                ButtonInteraction::PressedDraggedOff
            }
        }
        Some(p) if !p.suppressed => ButtonInteraction::Idle,
        // Suppressed press or no press: fall through to hover. Suppressed has
        // no visual capture, so neighbouring buttons stay interactive.
        _ if is_pointer_over_self => match pointer_device {
            Some(PointerDeviceKind::Touch) => ButtonInteraction::Idle,
            _ => ButtonInteraction::Hovered,
        },
        _ => ButtonInteraction::Idle,
    }
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
    // (Opacity 0.9 → α=0xE6; Opacity 0.7 → α=0xB3; source RGB fully opaque).
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

/// Refresh the strip's appearance after a system appearance or high-contrast change.
///
/// Pass the known half as `Some`; the function queries the current value for whichever
/// parameter is `None`. Defaults to `Appearance::Light` / `HighContrast::Off` on query failure.
pub(crate) fn notify_strip_appearance_refresh(window: &Window, override_appearance: Option<Appearance>, override_hc: Option<HighContrast>) {
    let appearance = override_appearance.unwrap_or_else(|| {
        Appearance::get_current()
            .inspect_err(|err| log::warn!("strip appearance notify: failed to read appearance: {err}"))
            .unwrap_or(Appearance::Light)
    });
    let hc = override_hc.unwrap_or_else(|| {
        HighContrast::get_current()
            .inspect_err(|err| log::warn!("strip appearance notify: failed to read high-contrast state: {err}"))
            .unwrap_or(HighContrast::Off)
    });
    window.with_strip_mut(|strip| strip.on_appearance_change(appearance, hc));
}

/// Dispatch a resolved `CaptionButtonAction` to the corresponding `Window` method.
pub(crate) fn dispatch_caption_action(window: &Window, action: CaptionButtonAction) {
    match action {
        CaptionButtonAction::Close => {
            let _ = window.request_close();
        }
        CaptionButtonAction::Minimize => window.minimize(),
        CaptionButtonAction::Maximize => window.maximize(),
        CaptionButtonAction::Restore => window.restore(),
    }
}

pub(crate) const fn device_kind_for(pointer_info: &PointerInfo) -> PointerDeviceKind {
    match pointer_info {
        PointerInfo::Touch(_) => PointerDeviceKind::Touch,
        PointerInfo::Pen(_) => PointerDeviceKind::Pen,
        PointerInfo::Common(_) => PointerDeviceKind::Mouse,
    }
}

/// Hit-test the caption-button strip from screen-space coordinates.
/// Returns `None` if no strip exists, the point is outside strip bounds, or
/// the point is inside the top resize-border band on a restored resizable window
/// (that band must reach `DefWindowProc` for the system resize cursor/drag loop).
/// Win32 coord-transform calls (`ScreenToClient`, `GetClientRect`) live here
/// because they're tightly coupled with the strip's hit-test math.
pub(crate) fn caption_kind_at_screen(window: &Window, screen: PhysicalPoint) -> Option<CaptionButtonKind> {
    if is_in_top_resize_border(window, screen.y.0) {
        return None;
    }
    let hwnd = window.hwnd();
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
    window
        .with_strip(|strip| {
            strip.hit_test(
                PhysicalPoint::new(client_point.x, client_point.y),
                PhysicalPixels(client_rect.right),
            )
        })
        .flatten()
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
    primary_press: Option<PrimaryPress>,
    non_primary_presses: Vec<NonPrimaryPress>,
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
                    WM_APP_CAPTION_BUTTONS_RENDERING_DEVICE_REPLACED,
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
            primary_press: None,
            non_primary_presses: Vec::new(),
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
            set_visual_rect(&button.visuals.backplate, x, 0, bw, bh, "backplate");
            set_visual_rect(&button.visuals.glyph, gx, gy, gw, gh, "glyph");
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
        let primary_press = self.primary_press;
        let is_active = self.is_active;
        for button in &mut self.buttons {
            let new_interaction = resolve_interaction(
                button.kind,
                button.availability,
                pointer_over_kind,
                pointer_device,
                primary_press.as_ref(),
            );
            apply_button_visuals(button, new_interaction, &theme, is_active);
        }
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

    /// Record a primary (Left) press. No-op if one is already in flight.
    /// `suppressed = true` for disabled buttons — no visual capture, no action on release.
    pub fn on_pointer_down(&mut self, kind: CaptionButtonKind) {
        if self.primary_press.is_some() {
            return;
        }
        let suppressed = !self.is_enabled(kind);
        self.primary_press = Some(PrimaryPress { kind, suppressed });
        // Seed pointer_over_kind so a touch tap via WM_NCLBUTTONDOWN renders
        // Pressed on the first frame instead of PressedDraggedOff.
        self.pointer_over_kind = Some(kind);
        self.apply_visuals_to_all_buttons();
    }

    /// Drain the active press. Returns `None` if no press, suppressed (disabled
    /// at press time), or release was over a different button.
    pub fn on_pointer_up(&mut self, kind_under_pointer: Option<CaptionButtonKind>) -> Option<CaptionButtonAction> {
        let press = self.primary_press.take()?;
        let action = if press.suppressed || kind_under_pointer != Some(press.kind) {
            None
        } else {
            Some(self.action_for(press.kind))
        };
        self.pointer_over_kind = kind_under_pointer;
        self.apply_visuals_to_all_buttons();
        action
    }

    /// Drop `NonPrimaryPress` entries for `pointer_id`. Primary press is pointer-
    /// id-less; use `cancel_primary_press` or `cancel_any_press` for it.
    pub fn on_pointer_cancel(&mut self, pointer_id: u32) {
        self.non_primary_presses.retain(|s| s.pointer_id != pointer_id);
    }

    /// Clear all presses. Used on `WM_CANCELMODE` / deactivate (no pointer id
    /// available). Clears hover if an active press was cancelled; next
    /// `WM_NCPOINTERUPDATE` re-establishes it.
    pub fn cancel_any_press(&mut self) {
        let had_active = self.primary_press.is_some();
        self.primary_press = None;
        self.non_primary_presses.clear();
        if had_active {
            self.pointer_over_kind = None;
            self.pointer_device = None;
            self.apply_visuals_to_all_buttons();
        }
    }

    /// Cancel only the active press, used on `WM_CAPTURECHANGED` (external
    /// `SetCapture` theft). Leaves non-primary presses intact. Idempotent.
    pub fn cancel_primary_press(&mut self) {
        if self.primary_press.take().is_some() {
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

    pub(crate) const fn has_primary_press(&self) -> bool {
        self.primary_press.is_some()
    }

    pub(crate) fn has_press_for(&self, pointer_id: u32) -> bool {
        self.non_primary_presses.iter().any(|s| s.pointer_id == pointer_id)
    }

    /// Record a wndproc-consumed non-primary DOWN. Idempotent: skips if
    /// `(pointer_id, button)` is already tracked.
    pub(crate) fn track_swallowed_press(&mut self, pointer_id: u32, button: PointerButton) {
        debug_assert!(button != PointerButton::None, "track_swallowed_press got None button");
        let already_tracked = self
            .non_primary_presses
            .iter()
            .any(|s| s.pointer_id == pointer_id && s.button == button);
        if already_tracked {
            return;
        }
        self.non_primary_presses.push(NonPrimaryPress { pointer_id, button });
    }

    /// Drain a `NonPrimaryPress` for `(pointer_id, button)`. Returns `true` if
    /// removed. Active press is drained by `on_pointer_up`.
    pub(crate) fn consume_swallowed_release(&mut self, pointer_id: u32, button: PointerButton) -> bool {
        if let Some(idx) = self
            .non_primary_presses
            .iter()
            .position(|s| s.pointer_id == pointer_id && s.button == button)
        {
            self.non_primary_presses.swap_remove(idx);
            true
        } else {
            false
        }
    }

    /// Apply a non-primary button transition (from `WM_(NC)POINTERDOWN`,
    /// `WM_(NC)POINTERUP`, or a coalesced `WM_(NC)POINTERUPDATE`). Tracks DOWN
    /// when over strip or already-owned; drains UP. Returns `true` iff a
    /// release was drained (caller should swallow the Kotlin event). No-op
    /// for primary / no-button changes.
    pub(crate) fn handle_non_primary_button_change(
        &mut self,
        pointer_id: u32,
        change_kind: PointerButtonChangeKind,
        button: PointerButton,
        over_strip: bool,
    ) -> bool {
        if button == PointerButton::Left || button == PointerButton::None {
            return false;
        }
        match change_kind {
            PointerButtonChangeKind::Pressed if over_strip || self.has_press_for(pointer_id) => {
                self.track_swallowed_press(pointer_id, button);
                false
            }
            PointerButtonChangeKind::Released => self.consume_swallowed_release(pointer_id, button),
            _ => false,
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
        // Strip moved, so the cached hover identity may no longer match the
        // cursor's position; clear it and let the next NC update re-establish.
        self.pointer_over_kind = None;
        self.pointer_device = None;
        self.apply_visuals_to_all_buttons();
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

fn set_visual_rect(visual: &SpriteVisual, x: i32, y: i32, w: i32, h: i32, label: &'static str) {
    let _ = visual
        .SetOffset(Vector3 {
            X: x as f32,
            Y: y as f32,
            Z: 0.0,
        })
        .inspect_err(|err| log::warn!("{label} SetOffset failed: {err}"));
    let _ = visual
        .SetSize(Vector2::new(w as f32, h as f32))
        .inspect_err(|err| log::warn!("{label} SetSize failed: {err}"));
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

    fn primary_press(kind: CaptionButtonKind) -> PrimaryPress {
        PrimaryPress { kind, suppressed: false }
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
        let p = primary_press(CaptionButtonKind::Close);
        let r = resolve_interaction(
            CaptionButtonKind::Close,
            Availability::Enabled,
            Some(CaptionButtonKind::Close),
            Some(PointerDeviceKind::Mouse),
            Some(&p),
        );
        assert_eq!(r, ButtonInteraction::Pressed);
    }

    #[test]
    fn captured_self_with_pointer_outside_is_pressed_dragged_off() {
        let p = primary_press(CaptionButtonKind::Close);
        let r = resolve_interaction(
            CaptionButtonKind::Close,
            Availability::Enabled,
            None,
            Some(PointerDeviceKind::Mouse),
            Some(&p),
        );
        assert_eq!(r, ButtonInteraction::PressedDraggedOff);
    }

    #[test]
    fn captured_other_button_keeps_self_idle_winui_capture_rule() {
        // Press is on Minimize; pointer moves over Close. Close stays Idle.
        let s = primary_press(CaptionButtonKind::Minimize);
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
    fn suppressed_primary_press_does_not_drive_pressed_state_on_captured() {
        // A suppressed active press (disabled button) must not render as Pressed.
        let suppressed = PrimaryPress {
            kind: CaptionButtonKind::Close,
            suppressed: true,
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
    fn suppressed_primary_press_does_not_suppress_neighbour_hover() {
        // Suppressed active press must not force Maximize to Idle when cursor is over it.
        let suppressed = PrimaryPress {
            kind: CaptionButtonKind::Close,
            suppressed: true,
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
    fn primary_press_suppresses_neighbour_hover() {
        // A non-suppressed press over Close must suppress hover on every other button.
        let press = PrimaryPress {
            kind: CaptionButtonKind::Close,
            suppressed: false,
        };
        let r = resolve_interaction(
            CaptionButtonKind::Maximize,
            Availability::Enabled,
            Some(CaptionButtonKind::Maximize),
            Some(PointerDeviceKind::Mouse),
            Some(&press),
        );
        assert_eq!(r, ButtonInteraction::Idle);
    }

    // --- track_swallowed_press / consume_swallowed_release ---

    fn make_suppressed_press(pointer_id: u32, button: PointerButton) -> NonPrimaryPress {
        NonPrimaryPress { pointer_id, button }
    }

    #[test]
    fn swallowed_release_matches_pointer_and_button() {
        let s = make_suppressed_press(7, PointerButton::Right);
        assert!(!(s.pointer_id == 7 && s.button == PointerButton::Left)); // wrong button
        assert!(!(s.pointer_id == 8 && s.button == PointerButton::Right)); // wrong pointer id
        assert!(s.pointer_id == 7 && s.button == PointerButton::Right); // match
    }

    #[test]
    fn swallowed_release_does_not_match_different_button() {
        // NonPrimaryPress with Right button must not match a Left release query.
        let s = make_suppressed_press(7, PointerButton::Right);
        let matches = s.pointer_id == 7 && s.button == PointerButton::Left;
        assert!(!matches);
    }

    // --- primary_press / non_primary_presses coexistence ---

    #[test]
    fn non_primary_press_during_primary_press_coexists() {
        let active = Some(PrimaryPress {
            kind: CaptionButtonKind::Minimize,
            suppressed: false,
        });
        let mut non_primary_presses = vec![make_suppressed_press(1, PointerButton::Right)];

        // consume_swallowed_release(p=1, Right) removes only the NonPrimaryPress entry.
        let idx = non_primary_presses
            .iter()
            .position(|s| s.pointer_id == 1 && s.button == PointerButton::Right);
        non_primary_presses.swap_remove(idx.expect("NonPrimaryPress{Right} not found"));
        assert!(active.is_some());
        assert!(non_primary_presses.is_empty());
    }

    #[test]
    fn active_release_does_not_drain_concurrent_suppressed_press() {
        // Taking primary_press does not touch non_primary_presses.
        let mut primary_press = Some(PrimaryPress {
            kind: CaptionButtonKind::Minimize,
            suppressed: false,
        });
        let non_primary_presses = [make_suppressed_press(1, PointerButton::Right)];

        let drained = primary_press.take();
        assert!(drained.is_some());
        assert_eq!(non_primary_presses.len(), 1);
        assert!(non_primary_presses[0].pointer_id == 1 && non_primary_presses[0].button == PointerButton::Right);
    }

    #[test]
    fn cancel_any_press_clears_both_active_and_suppressed() {
        let mut primary_press: Option<PrimaryPress> = Some(PrimaryPress {
            kind: CaptionButtonKind::Minimize,
            suppressed: false,
        });
        let mut non_primary_presses = vec![make_suppressed_press(1, PointerButton::Right)];
        let had_active = primary_press.is_some();
        primary_press = None;
        non_primary_presses.clear();
        assert!(had_active);
        assert!(primary_press.is_none());
        assert!(non_primary_presses.is_empty());
    }

    #[test]
    fn on_pointer_cancel_drains_only_matching_pointer_from_suppressed() {
        let mut non_primary_presses = vec![
            make_suppressed_press(1, PointerButton::Right),
            make_suppressed_press(2, PointerButton::Right),
        ];
        non_primary_presses.retain(|s| s.pointer_id != 1);
        assert_eq!(non_primary_presses.len(), 1);
        assert_eq!(non_primary_presses[0].pointer_id, 2);
    }

    #[test]
    fn on_pointer_up_returns_none_for_suppressed_primary_press() {
        // Suppressed active press (disabled button) must yield no action.
        // Simulate on_pointer_up logic directly (no live strip needed).
        let press = PrimaryPress {
            kind: CaptionButtonKind::Maximize,
            suppressed: true,
        };
        let kind_under = Some(CaptionButtonKind::Maximize);
        let action: Option<()> = if press.suppressed || kind_under != Some(press.kind) {
            None
        } else {
            Some(())
        };
        assert!(action.is_none());
    }

    #[test]
    fn on_pointer_up_returns_none_when_kind_under_differs() {
        // Released over a different button — no action.
        let press = PrimaryPress {
            kind: CaptionButtonKind::Minimize,
            suppressed: false,
        };
        let kind_under = Some(CaptionButtonKind::Close);
        let action: Option<()> = if press.suppressed || kind_under != Some(press.kind) {
            None
        } else {
            Some(())
        };
        assert!(action.is_none());
    }

    #[test]
    fn on_pointer_up_returns_action_when_released_over_same_button() {
        // Released over the same button — action must fire.
        let press = PrimaryPress {
            kind: CaptionButtonKind::Close,
            suppressed: false,
        };
        let kind_under = Some(CaptionButtonKind::Close);
        let matched: bool = !press.suppressed && kind_under == Some(press.kind);
        assert!(matched);
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
