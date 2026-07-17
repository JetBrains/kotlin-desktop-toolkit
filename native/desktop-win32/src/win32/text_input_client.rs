use desktop_common::ffi_utils::{BorrowedArray, BorrowedUtf8};

use windows::Win32::{
    Foundation::RECT,
    UI::Input::Ime::{
        ATTR_CONVERTED, ATTR_FIXEDCONVERTED, ATTR_INPUT, ATTR_TARGET_CONVERTED, ATTR_TARGET_NOTCONVERTED, GCS_COMPATTR, GCS_COMPCLAUSE,
        GCS_COMPREADATTR, GCS_COMPREADCLAUSE, GCS_COMPREADSTR, GCS_COMPSTR, GCS_CURSORPOS, GCS_DELTASTART, GCS_RESULTCLAUSE,
        GCS_RESULTREADCLAUSE, GCS_RESULTREADSTR, GCS_RESULTSTR, IME_COMPOSITION_STRING, ImmGetCompositionStringW,
    },
};

use super::{
    geometry::{LogicalPoint, LogicalRect, LogicalSize},
    ime::ImmContext,
    window::Window,
};

/// cbindgen:ignore
const NOT_FOUND: usize = usize::MAX;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRange {
    pub location: usize,
    pub length: usize,
}

impl TextRange {
    pub(crate) const fn none() -> Self {
        Self {
            location: NOT_FOUND,
            length: 0,
        }
    }

    pub(crate) const fn into_option(self) -> Option<Self> {
        if self.location == NOT_FOUND { None } else { Some(self) }
    }
}

#[repr(C)]
pub struct InsertTextArgs<'a> {
    pub text: BorrowedUtf8<'a>,
}

#[repr(C)]
pub struct SetMarkedTextArgs<'a> {
    pub text: BorrowedUtf8<'a>,
    pub selected_range: TextRange,
    pub underlines: BorrowedArray<'a, UnderlineSegment>,
}

#[repr(C)]
pub struct CaretRectArgs {
    pub range_in: TextRange,
    pub rect_out: LogicalRect,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnderlineSegment {
    pub range: TextRange,
    pub style: UnderlineStyle,
    pub target_clause: bool,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnderlineStyle {
    Solid,
    Dotted,
    Thick,
}

pub type SelectedRangeCallback = extern "C" fn(range_out: &mut TextRange);
pub type CaretRectCallback = extern "C" fn(args: &mut CaretRectArgs);
pub type InsertTextCallback = extern "C" fn(args: InsertTextArgs);
pub type SetMarkedTextCallback = extern "C" fn(args: SetMarkedTextArgs);
pub type UnmarkTextCallback = extern "C" fn();
pub type DiscardMarkedTextCallback = extern "C" fn();

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TextInputClient {
    pub selected_range: SelectedRangeCallback,
    pub caret_rect: CaretRectCallback,
    pub insert_text: InsertTextCallback,
    pub set_marked_text: SetMarkedTextCallback,
    pub unmark_text: UnmarkTextCallback,
    pub discard_marked_text: DiscardMarkedTextCallback,
}

impl TextInputClient {
    pub(crate) fn selected_range(self) -> Option<TextRange> {
        let mut out = TextRange::none();
        (self.selected_range)(&mut out);
        out.into_option()
    }

    pub(crate) fn caret_rect(self, range: TextRange) -> LogicalRect {
        let mut args = CaretRectArgs {
            range_in: range,
            rect_out: LogicalRect {
                origin: LogicalPoint::new(0.0, 0.0),
                size: LogicalSize::new(0.0, 0.0),
            },
        };
        (self.caret_rect)(&mut args);
        args.rect_out
    }

    pub(crate) fn insert_text(self, text: &str) {
        (self.insert_text)(InsertTextArgs {
            text: BorrowedUtf8::new(text),
        });
    }

    pub(crate) fn set_marked_text(self, text: &str, selected_range: Option<TextRange>, underlines: &[UnderlineSegment]) {
        (self.set_marked_text)(SetMarkedTextArgs {
            text: BorrowedUtf8::new(text),
            selected_range: selected_range.unwrap_or_else(TextRange::none),
            underlines: BorrowedArray::from_slice(underlines),
        });
    }

    pub(crate) fn unmark_text(self) {
        (self.unmark_text)();
    }

    pub(crate) fn discard_marked_text(self) {
        (self.discard_marked_text)();
    }
}

pub(crate) fn client_logical_to_physical_rect(rect: LogicalRect, scale: f32) -> RECT {
    let top_left = rect.origin.to_physical(scale);
    let bottom_right = LogicalPoint::new(rect.origin.x.0 + rect.size.width.0, rect.origin.y.0 + rect.size.height.0).to_physical(scale);
    RECT {
        left: top_left.x.0,
        top: top_left.y.0,
        right: bottom_right.x.0,
        bottom: bottom_right.y.0,
    }
}

pub(crate) trait CompositionSource {
    fn bytes(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<Vec<u8>>;
    /// `None` when the IME shows no composition cursor.
    fn cursor(&self) -> Option<usize>;
}

impl CompositionSource for ImmContext {
    fn bytes(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<Vec<u8>> {
        // SAFETY: this guard owns a valid HIMC; null buffer and zero size is the documented probe.
        let required = unsafe { ImmGetCompositionStringW(self.himc(), which, None, 0) };
        anyhow::ensure!(required >= 0, "ImmGetCompositionStringW({which:?}) probe failed: {required}");
        if required == 0 {
            return Ok(Vec::new());
        }
        let capacity = usize::try_from(required)?;
        let capacity_u32 = u32::try_from(capacity)?;
        let mut bytes = vec![0u8; capacity];
        // SAFETY: `bytes` is writable for exactly `capacity_u32` bytes and this guard owns the HIMC.
        let written = unsafe { ImmGetCompositionStringW(self.himc(), which, Some(bytes.as_mut_ptr().cast()), capacity_u32) };
        anyhow::ensure!(written >= 0, "ImmGetCompositionStringW({which:?}) fill failed: {written}");
        let written = usize::try_from(written)?;
        anyhow::ensure!(
            written <= capacity,
            "ImmGetCompositionStringW({which:?}) returned {written} > {capacity}"
        );
        bytes.truncate(written);
        Ok(bytes)
    }

    fn cursor(&self) -> Option<usize> {
        // SAFETY: this guard owns a valid HIMC; `GCS_CURSORPOS` returns the scalar as the result.
        let cursor = unsafe { ImmGetCompositionStringW(self.himc(), GCS_CURSORPOS, None, 0) };
        // A negative value is the documented "no visible cursor" state, not an IMM error.
        usize::try_from(cursor).ok()
    }
}

fn decode_utf16_bytes(bytes: &[u8]) -> anyhow::Result<String> {
    anyhow::ensure!(
        bytes.len().is_multiple_of(size_of::<u16>()),
        "odd UTF-16 byte count: {}",
        bytes.len()
    );
    let units = bytes
        .chunks_exact(size_of::<u16>())
        .map(|pair| u16::from_ne_bytes([pair[0], pair[1]]))
        .collect::<Vec<_>>();
    Ok(String::from_utf16_lossy(&units))
}

fn decode_u32_bytes(bytes: &[u8]) -> anyhow::Result<Vec<u32>> {
    anyhow::ensure!(
        bytes.len().is_multiple_of(size_of::<u32>()),
        "unaligned u32 byte count: {}",
        bytes.len()
    );
    Ok(bytes
        .chunks_exact(size_of::<u32>())
        .map(|part| u32::from_ne_bytes([part[0], part[1], part[2], part[3]]))
        .collect())
}

fn underlines_from_parts(attrs: &[u8], clauses: &[u32], preedit_len: usize) -> Vec<UnderlineSegment> {
    let bounds = clauses.iter().map(|value| usize::try_from(*value)).collect::<Result<Vec<_>, _>>();
    let Ok(bounds) = bounds else {
        return fallback_underlines(preedit_len);
    };
    if bounds.len() < 2
        || bounds.first() != Some(&0)
        || bounds.last() != Some(&preedit_len)
        || bounds.iter().any(|value| *value > preedit_len)
        || bounds.windows(2).any(|pair| pair[0] > pair[1])
    {
        return fallback_underlines(preedit_len);
    }

    bounds
        .windows(2)
        .filter_map(|pair| {
            let (start, end) = (pair[0], pair[1]);
            if start >= end {
                return None;
            }
            let attribute = attrs.get(start).copied().map_or(ATTR_INPUT, u32::from);
            let (style, target_clause) = match attribute {
                ATTR_TARGET_CONVERTED => (UnderlineStyle::Thick, true),
                ATTR_TARGET_NOTCONVERTED => (UnderlineStyle::Dotted, true),
                ATTR_CONVERTED | ATTR_FIXEDCONVERTED => (UnderlineStyle::Solid, false),
                _ => (UnderlineStyle::Dotted, false),
            };
            Some(UnderlineSegment {
                range: TextRange {
                    location: start,
                    length: end - start,
                },
                style,
                target_clause,
            })
        })
        .collect()
}

fn fallback_underlines(preedit_len: usize) -> Vec<UnderlineSegment> {
    (preedit_len != 0)
        .then_some(UnderlineSegment {
            range: TextRange {
                location: 0,
                length: preedit_len,
            },
            style: UnderlineStyle::Dotted,
            target_clause: false,
        })
        .into_iter()
        .collect()
}

/// cbindgen:ignore
const GCS_ANY: u32 = GCS_COMPREADSTR.0
    | GCS_COMPREADATTR.0
    | GCS_COMPREADCLAUSE.0
    | GCS_COMPSTR.0
    | GCS_COMPATTR.0
    | GCS_COMPCLAUSE.0
    | GCS_CURSORPOS.0
    | GCS_DELTASTART.0
    | GCS_RESULTREADSTR.0
    | GCS_RESULTREADCLAUSE.0
    | GCS_RESULTSTR.0
    | GCS_RESULTCLAUSE.0;

/// cbindgen:ignore
const GCS_PREEDIT_UPDATE: u32 = GCS_COMPSTR.0 | GCS_COMPATTR.0 | GCS_COMPCLAUSE.0 | GCS_CURSORPOS.0 | GCS_DELTASTART.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreeditSnapshot {
    text: String,
    selected: TextRange,
    underlines: Vec<UnderlineSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompositionSnapshot {
    result: Option<String>,
    preedit: Option<PreeditSnapshot>,
    cancelled: bool,
}

impl CompositionSnapshot {
    fn read(source: &impl CompositionSource, gcs: u32) -> anyhow::Result<Self> {
        let result = (gcs & GCS_RESULTSTR.0 != 0)
            .then(|| source.bytes(GCS_RESULTSTR).and_then(|bytes| decode_utf16_bytes(&bytes)))
            .transpose()?;
        let preedit = if gcs & GCS_PREEDIT_UPDATE != 0 {
            let text = decode_utf16_bytes(&source.bytes(GCS_COMPSTR)?)?;
            let length = text.encode_utf16().count();
            let selected = source.cursor().map_or_else(TextRange::none, |cursor| TextRange {
                location: cursor.min(length),
                length: 0,
            });
            let underlines = match (
                source.bytes(GCS_COMPATTR),
                source.bytes(GCS_COMPCLAUSE).and_then(|bytes| decode_u32_bytes(&bytes)),
            ) {
                (Ok(attrs), Ok(clauses)) => underlines_from_parts(&attrs, &clauses, length),
                (Err(err), _) | (_, Err(err)) => {
                    log::warn!("reading IME underline data failed: {err:#}");
                    fallback_underlines(length)
                }
            };
            Some(PreeditSnapshot {
                text,
                selected,
                underlines,
            })
        } else {
            None
        };
        Ok(Self {
            result,
            preedit,
            cancelled: gcs & GCS_ANY == 0,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CompositionAction {
    Insert(String),
    SetMarked(PreeditSnapshot),
    DiscardMarked,
    SetAppMarked(bool),
    ClearComposition,
    UpdateWindows,
}

fn reduce_composition(snapshot: CompositionSnapshot) -> Vec<CompositionAction> {
    let mut actions = Vec::new();
    if let Some(result) = snapshot.result.filter(|text| !text.is_empty()) {
        actions.push(CompositionAction::Insert(result));
        actions.push(CompositionAction::SetAppMarked(false));
    }
    if let Some(preedit) = snapshot.preedit {
        if preedit.text.is_empty() {
            actions.push(CompositionAction::DiscardMarked);
            actions.push(CompositionAction::SetAppMarked(false));
        } else {
            actions.push(CompositionAction::SetAppMarked(true));
            actions.push(CompositionAction::SetMarked(preedit));
        }
        actions.push(CompositionAction::UpdateWindows);
    } else if snapshot.cancelled {
        actions.push(CompositionAction::ClearComposition);
        actions.push(CompositionAction::DiscardMarked);
    }
    actions
}

pub(crate) trait CompositionSink {
    fn revision(&self) -> u64;
    fn set_app_marked(&self, value: bool) -> u64;
    fn clear_composition(&self) -> u64;
    fn insert_text(&self, text: &str);
    fn set_marked_text(&self, preedit: &PreeditSnapshot);
    fn discard_marked_text(&self);
    fn update_windows(&self);
}

impl CompositionSink for Window {
    fn revision(&self) -> u64 {
        self.ime_revision()
    }
    fn set_app_marked(&self, value: bool) -> u64 {
        self.ime_set_app_marked(value)
    }
    fn clear_composition(&self) -> u64 {
        self.clear_composition_state()
    }
    fn insert_text(&self, text: &str) {
        let _ = self.with_enabled_client(|client| client.insert_text(text));
    }
    fn set_marked_text(&self, preedit: &PreeditSnapshot) {
        let _ = self.with_enabled_client(|client| {
            client.set_marked_text(&preedit.text, Some(preedit.selected), &preedit.underlines);
        });
    }
    fn discard_marked_text(&self) {
        let _ = self.with_enabled_client(TextInputClient::discard_marked_text);
    }
    fn update_windows(&self) {
        self.update_ime_windows();
    }
}

fn apply_composition_actions(sink: &impl CompositionSink, actions: Vec<CompositionAction>) {
    let mut expected_revision = sink.revision();
    for action in actions {
        let called_client = match action {
            CompositionAction::Insert(text) => {
                sink.insert_text(&text);
                true
            }
            CompositionAction::SetMarked(preedit) => {
                sink.set_marked_text(&preedit);
                true
            }
            CompositionAction::DiscardMarked => {
                sink.discard_marked_text();
                true
            }
            CompositionAction::SetAppMarked(value) => {
                expected_revision = sink.set_app_marked(value);
                false
            }
            CompositionAction::ClearComposition => {
                expected_revision = sink.clear_composition();
                false
            }
            CompositionAction::UpdateWindows => {
                sink.update_windows();
                true
            }
        };
        if called_client && sink.revision() != expected_revision {
            return;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnedCompositionResult {
    Applied,
    ReadFailed,
}

pub(crate) fn apply_owned_composition(sink: &impl CompositionSink, source: &impl CompositionSource, gcs: u32) -> OwnedCompositionResult {
    let snapshot = match CompositionSnapshot::read(source, gcs) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            log::warn!("reading IME composition failed; keeping ownership until next update or END: {err:#}");
            return OwnedCompositionResult::ReadFailed;
        }
    };
    apply_composition_actions(sink, reduce_composition(snapshot));
    OwnedCompositionResult::Applied
}

/// Deliver the result of this window's own `CPS_COMPLETE` finalization. The reentrant
/// `WM_IME_COMPOSITION` arrives while composition state is being torn down, so only
/// `GCS_RESULTSTR` matters — preedit flags describe a composition that no longer exists.
pub(crate) fn apply_finalizing_composition(sink: &impl CompositionSink, source: &impl CompositionSource, gcs: u32) {
    if gcs & GCS_RESULTSTR.0 == 0 {
        return;
    }
    match source.bytes(GCS_RESULTSTR).and_then(|bytes| decode_utf16_bytes(&bytes)) {
        Ok(text) if !text.is_empty() => sink.insert_text(&text),
        Ok(_) => {}
        Err(err) => log::warn!("reading the finalized IME result failed: {err:#}"),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;

    use windows::Win32::UI::Input::Ime::{
        ATTR_INPUT, ATTR_TARGET_CONVERTED, CS_INSERTCHAR, CS_NOMOVECARET, GCS_COMPATTR, GCS_COMPCLAUSE, GCS_COMPSTR, GCS_CURSORPOS,
        GCS_DELTASTART, GCS_RESULTSTR, IME_COMPOSITION_STRING,
    };

    #[test]
    fn none_range_round_trips() {
        assert_eq!(TextRange::none().into_option(), None);
        let range = TextRange { location: 4, length: 2 };
        assert_eq!(range.into_option(), Some(range));
    }

    #[test]
    fn logical_caret_rect_scales_both_corners() {
        let rect = LogicalRect {
            origin: LogicalPoint::new(10.25, 5.25),
            size: LogicalSize::new(3.5, 4.5),
        };
        let physical = client_logical_to_physical_rect(rect, 1.5);
        assert_eq!((physical.left, physical.top, physical.right, physical.bottom), (15, 8, 21, 15));
    }

    fn utf16_bytes(value: &str) -> Vec<u8> {
        value.encode_utf16().flat_map(u16::to_ne_bytes).collect()
    }

    fn u32_bytes(values: &[u32]) -> Vec<u8> {
        values.iter().flat_map(|value| value.to_ne_bytes()).collect()
    }

    #[derive(Default)]
    struct FakeSource {
        result: Vec<u8>,
        composition: Vec<u8>,
        attributes: Vec<u8>,
        clauses: Vec<u8>,
        cursor: Option<usize>,
        fail_on: Option<IME_COMPOSITION_STRING>,
    }

    impl CompositionSource for FakeSource {
        fn bytes(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<Vec<u8>> {
            if self.fail_on == Some(which) {
                anyhow::bail!("injected {which:?} failure");
            }
            Ok(match which {
                GCS_RESULTSTR => self.result.clone(),
                GCS_COMPSTR => self.composition.clone(),
                GCS_COMPATTR => self.attributes.clone(),
                GCS_COMPCLAUSE => self.clauses.clone(),
                _ => Vec::new(),
            })
        }

        fn cursor(&self) -> Option<usize> {
            self.cursor
        }
    }

    #[test]
    fn byte_decoders_reject_misaligned_data() {
        assert!(decode_utf16_bytes(&[1]).is_err());
        assert!(decode_u32_bytes(&[0, 0, 0]).is_err());
    }

    #[test]
    fn underline_conversion_maps_clauses_and_targets() {
        let underlines = underlines_from_parts(
            &[
                u8::try_from(ATTR_INPUT).unwrap(),
                u8::try_from(ATTR_INPUT).unwrap(),
                u8::try_from(ATTR_TARGET_CONVERTED).unwrap(),
                u8::try_from(ATTR_TARGET_CONVERTED).unwrap(),
            ],
            &[0, 2, 4],
            4,
        );
        assert_eq!(
            underlines,
            vec![
                UnderlineSegment {
                    range: TextRange { location: 0, length: 2 },
                    style: UnderlineStyle::Dotted,
                    target_clause: false,
                },
                UnderlineSegment {
                    range: TextRange { location: 2, length: 2 },
                    style: UnderlineStyle::Thick,
                    target_clause: true,
                },
            ],
        );
    }

    #[test]
    fn malformed_clauses_fall_back_to_whole_preedit() {
        assert_eq!(underlines_from_parts(&[], &[1, 3], 3), fallback_underlines(3),);
    }

    #[test]
    fn partial_preedit_flags_refetch_complete_preedit() {
        let source = FakeSource {
            composition: utf16_bytes("かな"),
            attributes: vec![u8::try_from(ATTR_INPUT).unwrap(), u8::try_from(ATTR_INPUT).unwrap()],
            clauses: u32_bytes(&[0, 2]),
            cursor: Some(1),
            ..Default::default()
        };
        for flag in [GCS_COMPATTR, GCS_COMPCLAUSE, GCS_CURSORPOS, GCS_DELTASTART] {
            let snapshot = CompositionSnapshot::read(&source, flag.0).unwrap();
            let preedit = snapshot.preedit.unwrap();
            assert_eq!(preedit.text, "かな");
            assert_eq!(preedit.selected, TextRange { location: 1, length: 0 });
        }
    }

    #[test]
    fn snapshot_reads_result_and_new_preedit_together() {
        let source = FakeSource {
            result: utf16_bytes("確定"),
            composition: utf16_bytes("つぎ"),
            cursor: Some(1),
            ..Default::default()
        };
        let snapshot = CompositionSnapshot::read(&source, GCS_RESULTSTR.0 | GCS_COMPSTR.0).unwrap();
        assert_eq!(snapshot.result.as_deref(), Some("確定"));
        assert_eq!(snapshot.preedit.unwrap().text, "つぎ");
    }

    #[test]
    fn empty_preedit_is_present_and_cursor_is_clamped() {
        let empty = CompositionSnapshot::read(&FakeSource::default(), GCS_COMPSTR.0).unwrap();
        assert_eq!(empty.preedit.unwrap().text, "");
        assert!(!empty.cancelled);

        let source = FakeSource {
            composition: utf16_bytes("かな"),
            cursor: Some(99),
            ..Default::default()
        };
        let clamped = CompositionSnapshot::read(&source, GCS_COMPSTR.0).unwrap();
        assert_eq!(clamped.preedit.unwrap().selected, TextRange { location: 2, length: 0 });
    }

    #[test]
    fn hidden_cursor_renders_preedit_without_selection() {
        let source = FakeSource {
            composition: utf16_bytes("かな"),
            cursor: None,
            ..Default::default()
        };
        let snapshot = CompositionSnapshot::read(&source, GCS_COMPSTR.0).unwrap();
        assert_eq!(snapshot.preedit.unwrap().selected, TextRange::none());
    }

    #[test]
    fn status_only_mask_is_cancellation() {
        let snapshot = CompositionSnapshot::read(&FakeSource::default(), CS_INSERTCHAR | CS_NOMOVECARET).unwrap();
        assert!(snapshot.cancelled);
        assert!(snapshot.result.is_none());
        assert!(snapshot.preedit.is_none());
    }

    #[test]
    fn optional_decoration_failure_uses_fallback() {
        let source = FakeSource {
            composition: utf16_bytes("かな"),
            fail_on: Some(GCS_COMPATTR),
            ..Default::default()
        };
        let snapshot = CompositionSnapshot::read(&source, GCS_COMPSTR.0).unwrap();
        assert_eq!(snapshot.preedit.unwrap().underlines, fallback_underlines(2));
    }

    #[test]
    fn core_read_failure_is_returned_before_any_action() {
        let source = FakeSource {
            fail_on: Some(GCS_COMPSTR),
            ..Default::default()
        };
        assert!(CompositionSnapshot::read(&source, GCS_COMPSTR.0).is_err());
    }

    #[test]
    fn reducer_commits_before_starting_next_preedit() {
        let preedit = PreeditSnapshot {
            text: "つぎ".to_owned(),
            selected: TextRange { location: 1, length: 0 },
            underlines: fallback_underlines(2),
        };
        let actions = reduce_composition(CompositionSnapshot {
            result: Some("確定".to_owned()),
            preedit: Some(preedit.clone()),
            cancelled: false,
        });
        assert_eq!(
            actions,
            vec![
                CompositionAction::Insert("確定".to_owned()),
                CompositionAction::SetAppMarked(false),
                CompositionAction::SetAppMarked(true),
                CompositionAction::SetMarked(preedit),
                CompositionAction::UpdateWindows,
            ],
        );
    }

    #[test]
    fn reducer_distinguishes_empty_preedit_and_cancel() {
        let empty = PreeditSnapshot {
            text: String::new(),
            selected: TextRange { location: 0, length: 0 },
            underlines: Vec::new(),
        };
        assert_eq!(
            reduce_composition(CompositionSnapshot {
                result: None,
                preedit: Some(empty),
                cancelled: false
            }),
            vec![
                CompositionAction::DiscardMarked,
                CompositionAction::SetAppMarked(false),
                CompositionAction::UpdateWindows
            ],
        );
        assert_eq!(
            reduce_composition(CompositionSnapshot {
                result: None,
                preedit: None,
                cancelled: true
            }),
            vec![CompositionAction::ClearComposition, CompositionAction::DiscardMarked],
        );
    }

    #[test]
    fn reducer_accepts_post_end_result_without_active_composition() {
        assert_eq!(
            reduce_composition(CompositionSnapshot {
                result: Some("한".to_owned()),
                preedit: None,
                cancelled: false,
            }),
            vec![CompositionAction::Insert("한".to_owned()), CompositionAction::SetAppMarked(false)],
        );
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ReenterOn {
        Insert,
        SetMarked,
    }

    #[derive(Default)]
    struct FakeSink {
        revision: Cell<u64>,
        composition_active: Cell<bool>,
        app_marked: Cell<bool>,
        callbacks: RefCell<Vec<&'static str>>,
        inserted: RefCell<Vec<String>>,
        reenter_on: Option<ReenterOn>,
    }

    impl FakeSink {
        fn advance_revision(&self) -> u64 {
            let next = self.revision.get().checked_add(1).expect("fake revision overflow");
            self.revision.set(next);
            next
        }

        fn callback(&self, name: &'static str, kind: ReenterOn) {
            self.callbacks.borrow_mut().push(name);
            if self.reenter_on == Some(kind) {
                // Model a nested END/focus-loss finalization before returning to the outer callback.
                self.composition_active.set(false);
                self.app_marked.set(false);
                self.advance_revision();
            }
        }
    }

    impl CompositionSink for FakeSink {
        fn revision(&self) -> u64 {
            self.revision.get()
        }
        fn set_app_marked(&self, value: bool) -> u64 {
            self.app_marked.set(value);
            self.advance_revision()
        }
        fn clear_composition(&self) -> u64 {
            self.composition_active.set(false);
            self.app_marked.set(false);
            self.advance_revision()
        }
        fn insert_text(&self, text: &str) {
            self.inserted.borrow_mut().push(text.to_owned());
            self.callback("insert", ReenterOn::Insert);
        }
        fn set_marked_text(&self, _preedit: &PreeditSnapshot) {
            self.callback("set_marked", ReenterOn::SetMarked);
        }
        fn discard_marked_text(&self) {
            self.callbacks.borrow_mut().push("discard");
        }
        fn update_windows(&self) {
            self.callbacks.borrow_mut().push("update_windows");
        }
    }

    #[test]
    fn action_driver_stops_after_reentrant_end_during_insert() {
        let sink = FakeSink {
            reenter_on: Some(ReenterOn::Insert),
            ..Default::default()
        };
        sink.app_marked.set(true);
        apply_composition_actions(
            &sink,
            vec![
                CompositionAction::Insert("commit".to_owned()),
                CompositionAction::SetAppMarked(false),
                CompositionAction::SetAppMarked(true),
                CompositionAction::SetMarked(PreeditSnapshot {
                    text: "stale".to_owned(),
                    selected: TextRange { location: 0, length: 0 },
                    underlines: Vec::new(),
                }),
            ],
        );
        assert_eq!(&*sink.callbacks.borrow(), &["insert"]);
        assert!(!sink.app_marked.get());
    }

    #[test]
    fn action_driver_stops_before_positioning_after_reentrant_focus_loss() {
        let sink = FakeSink {
            reenter_on: Some(ReenterOn::SetMarked),
            ..Default::default()
        };
        apply_composition_actions(
            &sink,
            vec![
                CompositionAction::SetAppMarked(true),
                CompositionAction::SetMarked(PreeditSnapshot {
                    text: "preedit".to_owned(),
                    selected: TextRange { location: 0, length: 0 },
                    underlines: Vec::new(),
                }),
                CompositionAction::UpdateWindows,
            ],
        );
        assert_eq!(&*sink.callbacks.borrow(), &["set_marked"]);
        assert!(!sink.app_marked.get());
    }

    #[test]
    fn owned_core_read_failure_preserves_sink_state() {
        let sink = FakeSink::default();
        sink.revision.set(7);
        sink.composition_active.set(true);
        sink.app_marked.set(true);
        let source = FakeSource {
            fail_on: Some(GCS_COMPSTR),
            ..Default::default()
        };
        assert_eq!(
            apply_owned_composition(&sink, &source, GCS_COMPSTR.0),
            OwnedCompositionResult::ReadFailed,
        );
        assert_eq!(sink.revision.get(), 7);
        assert!(sink.composition_active.get());
        assert!(sink.app_marked.get());
        assert!(sink.callbacks.borrow().is_empty());
    }

    #[test]
    fn owned_post_end_result_is_still_applied() {
        let sink = FakeSink::default();
        let source = FakeSource {
            result: utf16_bytes("한"),
            ..Default::default()
        };
        assert_eq!(
            apply_owned_composition(&sink, &source, GCS_RESULTSTR.0),
            OwnedCompositionResult::Applied,
        );
        assert_eq!(&*sink.callbacks.borrow(), &["insert"]);
    }

    #[test]
    fn finalizing_result_routes_to_insert_only() {
        let sink = FakeSink::default();
        let source = FakeSource {
            result: utf16_bytes("確定"),
            composition: utf16_bytes("つぎ"),
            cursor: Some(1),
            ..Default::default()
        };
        apply_finalizing_composition(&sink, &source, GCS_RESULTSTR.0 | GCS_COMPSTR.0);
        assert_eq!(&*sink.callbacks.borrow(), &["insert"]);
        assert_eq!(&*sink.inserted.borrow(), &["確定"]);
    }

    #[test]
    fn finalizing_without_result_flag_makes_no_callbacks() {
        let sink = FakeSink::default();
        let source = FakeSource {
            composition: utf16_bytes("つぎ"),
            ..Default::default()
        };
        apply_finalizing_composition(&sink, &source, GCS_COMPSTR.0);
        assert!(sink.callbacks.borrow().is_empty());
    }

    #[test]
    fn finalizing_empty_result_makes_no_callbacks() {
        let sink = FakeSink::default();
        apply_finalizing_composition(&sink, &FakeSource::default(), GCS_RESULTSTR.0);
        assert!(sink.callbacks.borrow().is_empty());
    }

    #[test]
    fn finalizing_result_read_failure_is_tolerated() {
        let sink = FakeSink::default();
        let source = FakeSource {
            fail_on: Some(GCS_RESULTSTR),
            ..Default::default()
        };
        apply_finalizing_composition(&sink, &source, GCS_RESULTSTR.0);
        assert!(sink.callbacks.borrow().is_empty());
    }
}
