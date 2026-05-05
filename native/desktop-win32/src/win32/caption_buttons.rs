//! Caption-button strip for `WindowTitleBarKind::Custom` windows.
//!
//! See `docs/specs/2026-04-30-win32-caption-buttons-design.md` for the design.
//!
//! This module is a pure state machine over typed inputs. It does not call
//! Win32 APIs; the wndproc layer in `event_loop.rs` is the only place that
//! bridges Win32 messages and this module.

use windows::Win32::Graphics::Gdi::{GetSysColor, SYS_COLOR_INDEX};

use super::appearance::{Appearance, HighContrast};
use super::geometry::{LogicalSize, PhysicalPoint, PhysicalSize};

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
    pub fn empty() -> Self {
        Self(0)
    }

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

    pub fn from_style(style: &crate::win32::window_api::WindowStyle) -> Self {
        let mut kinds = Self::empty().with(CaptionButtonKind::Close);
        if style.is_minimizable || style.is_maximizable {
            kinds = kinds.with(CaptionButtonKind::Minimize);
            kinds = kinds.with(CaptionButtonKind::Maximize);
        }
        kinds
    }
}

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
            if is_pointer_over_self {
                ButtonInteraction::Pressed
            } else {
                ButtonInteraction::PressedDraggedOff
            }
        }
        Some(_) => ButtonInteraction::Idle,
        None if is_pointer_over_self => match pointer_device {
            Some(PointerDeviceKind::Touch) => ButtonInteraction::Idle,
            _ => ButtonInteraction::Hovered,
        },
        None => ButtonInteraction::Idle,
    }
}

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
            (HighContrast::Off, Appearance::Dark) => Self::dark(),
        }
    }

    // WinUI Fluent palette: `microsoft/microsoft-ui-xaml@5f9e851133b…`.
    // Close-specific reds: `microsoft/terminal@e4e3f08efca…` MinMaxCloseControl.xaml
    // (`Opacity 0.9` → α=0xE6; `Opacity 0.7` → α=0xB3 — valid only because the
    // source RGB is fully opaque; both rounded to nearest).
    fn light() -> Self {
        Self {
            backplate_rest: rgba(0, 0, 0, 0),
            backplate_hover: rgba(0, 0, 0, 0x09),    // SubtleFillColorSecondary
            backplate_pressed: rgba(0, 0, 0, 0x06),  // SubtleFillColorTertiary
            backplate_inactive: rgba(0, 0, 0, 0),
            foreground_rest: rgba(0, 0, 0, 0xE4),    // TextFillColorPrimary
            foreground_hover: rgba(0, 0, 0, 0xE4),
            foreground_pressed: rgba(0, 0, 0, 0x9E), // TextFillColorSecondary
            foreground_disabled: rgba(0, 0, 0, 0x5C),// TextFillColorDisabled
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
            backplate_hover: rgba(0xFF, 0xFF, 0xFF, 0x0F),    // SubtleFillColorSecondary
            backplate_pressed: rgba(0xFF, 0xFF, 0xFF, 0x0A),  // SubtleFillColorTertiary
            backplate_inactive: rgba(0, 0, 0, 0),
            foreground_rest: rgba(0xFF, 0xFF, 0xFF, 0xFF),    // TextFillColorPrimary
            foreground_hover: rgba(0xFF, 0xFF, 0xFF, 0xFF),
            foreground_pressed: rgba(0xFF, 0xFF, 0xFF, 0xC5), // TextFillColorSecondary
            foreground_disabled: rgba(0xFF, 0xFF, 0xFF, 0x5D),// TextFillColorDisabled
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
    pub fn new(scale: f32) -> Self {
        Self {
            button_size_px: LogicalSize::new(46.0, 32.0).to_physical(scale),
            glyph_extent_px: LogicalSize::new(10.0, 10.0).to_physical(scale),
        }
    }
}

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
        if point.y.0 < 0 || point.y.0 >= bh {
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

/// Map a caption-button hit to the WM_NCHITTEST return code per spec §4.2:
/// enabled Min/Max/Close → HTMINBUTTON / HTMAXBUTTON / HTCLOSE; visible
/// disabled Min/Max → HTCAPTION (drag region) so the rectangle stays in the
/// title-bar surface and Snap Layouts is *not* advertised on a disabled
/// Maximize.
pub(crate) fn hittest_for_caption_button_kind(kind: CaptionButtonKind, is_enabled: bool) -> u32 {
    use windows::Win32::UI::WindowsAndMessaging::{HTCAPTION, HTCLOSE, HTMAXBUTTON, HTMINBUTTON};
    match (kind, is_enabled) {
        (CaptionButtonKind::Close, _) => HTCLOSE,
        (CaptionButtonKind::Minimize, true) => HTMINBUTTON,
        (CaptionButtonKind::Maximize, true) => HTMAXBUTTON,
        (CaptionButtonKind::Minimize | CaptionButtonKind::Maximize, false) => HTCAPTION,
    }
}

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

// CaptionButtonStrip definition lives here once Phase 5 adds it.
pub(crate) struct CaptionButtonStrip {
    _placeholder: (),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::win32::geometry::PhysicalPixels;
    use crate::win32::window_api::{WindowStyle, WindowSystemBackdropType, WindowTitleBarKind};

    fn session(kind: CaptionButtonKind, device: PointerDeviceKind) -> PressSession {
        PressSession {
            pointer_id: 1,
            captured_kind: kind,
            device,
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
    fn non_resizable_disables_maximize_even_when_maximizable_bit_is_set() {
        // Spec §4.2 Maximize policy: requires is_resizable && is_maximizable.
        let style = style_with(true, true, false);
        assert_eq!(availability_from_style(CaptionButtonKind::Minimize, &style), Availability::Enabled);
        assert_eq!(availability_from_style(CaptionButtonKind::Maximize, &style), Availability::Disabled);
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

    // --- hittest_for_caption_button_kind / caption_button_kind_for_hittest ---

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
        // rectangle stays in the title-bar drag region.
        use windows::Win32::UI::WindowsAndMessaging::{HTCAPTION, HTCLOSE};
        assert_eq!(hittest_for_caption_button_kind(CaptionButtonKind::Minimize, false), HTCAPTION);
        assert_eq!(hittest_for_caption_button_kind(CaptionButtonKind::Maximize, false), HTCAPTION);
        // Close is always enabled, but the policy still returns HTCLOSE for completeness.
        assert_eq!(hittest_for_caption_button_kind(CaptionButtonKind::Close, false), HTCLOSE);
    }

    #[test]
    fn hittest_to_kind_matches_three_caption_codes() {
        use windows::Win32::UI::WindowsAndMessaging::{HTCLIENT, HTCLOSE, HTMAXBUTTON, HTMINBUTTON};
        assert_eq!(caption_button_kind_for_hittest(HTCLOSE), Some(CaptionButtonKind::Close));
        assert_eq!(caption_button_kind_for_hittest(HTMAXBUTTON), Some(CaptionButtonKind::Maximize));
        assert_eq!(caption_button_kind_for_hittest(HTMINBUTTON), Some(CaptionButtonKind::Minimize));
        assert_eq!(caption_button_kind_for_hittest(HTCLIENT), None);
    }
}
