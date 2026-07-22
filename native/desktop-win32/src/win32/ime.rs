use std::cell::Cell;

use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    UI::Input::Ime::{
        ATTR_CONVERTED, ATTR_FIXEDCONVERTED, ATTR_INPUT, ATTR_INPUT_ERROR, ATTR_TARGET_CONVERTED, ATTR_TARGET_NOTCONVERTED, CANDIDATEFORM,
        CFS_EXCLUDE, CFS_POINT, COMPOSITIONFORM, GCS_COMPATTR, GCS_COMPCLAUSE, GCS_COMPREADATTR, GCS_COMPREADCLAUSE, GCS_COMPREADSTR,
        GCS_COMPSTR, GCS_CURSORPOS, GCS_DELTASTART, GCS_RESULTCLAUSE, GCS_RESULTREADCLAUSE, GCS_RESULTREADSTR, GCS_RESULTSTR, HIMC,
        IME_COMPOSITION_STRING, ImmGetCompositionStringW, ImmGetContext, ImmNotifyIME, ImmReleaseContext, ImmSetCandidateWindow,
        ImmSetCompositionWindow, NI_COMPOSITIONSTR, NOTIFY_IME_INDEX,
    },
};

use super::text_input_client::{TextCompositionAttribute, TextCompositionSegment, TextInputClient, TextRange};

pub(crate) struct ImmContext {
    hwnd: HWND,
    himc: HIMC,
}

impl ImmContext {
    pub(crate) fn get(hwnd: HWND) -> Option<Self> {
        // SAFETY: callers pass the live HWND owned by `Window`; the guard releases the acquired
        // context against the same handle.
        let himc = unsafe { ImmGetContext(hwnd) };
        (!himc.is_invalid()).then_some(Self { hwnd, himc })
    }

    /// Two-call `ImmGetCompositionStringW` transport: probe for the byte size, then fill.
    fn composition_payload<T: Copy + Default>(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<Vec<T>> {
        // SAFETY: this guard owns a valid HIMC; null buffer and zero size is the documented probe.
        let required = unsafe { ImmGetCompositionStringW(self.himc, which, None, 0) };
        anyhow::ensure!(required >= 0, "ImmGetCompositionStringW({which:?}) probe failed: {required}");
        let byte_count = usize::try_from(required)?;
        anyhow::ensure!(
            byte_count.is_multiple_of(size_of::<T>()),
            "ImmGetCompositionStringW({which:?}) returned a misaligned byte count: {byte_count}"
        );
        if byte_count == 0 {
            return Ok(Vec::new());
        }
        let mut payload = vec![T::default(); byte_count / size_of::<T>()];
        // SAFETY: `payload` is writable for exactly `byte_count` bytes and this guard owns the HIMC.
        let written = unsafe { ImmGetCompositionStringW(self.himc, which, Some(payload.as_mut_ptr().cast()), u32::try_from(byte_count)?) };
        anyhow::ensure!(written >= 0, "ImmGetCompositionStringW({which:?}) fill failed: {written}");
        let written = usize::try_from(written)?;
        anyhow::ensure!(
            written <= byte_count,
            "ImmGetCompositionStringW({which:?}) returned {written} > {byte_count}"
        );
        payload.truncate(written / size_of::<T>());
        Ok(payload)
    }

    pub(crate) fn set_composition_window(&self, origin: POINT) {
        let composition = COMPOSITIONFORM {
            dwStyle: CFS_POINT,
            ptCurrentPos: origin,
            ..Default::default()
        };
        // SAFETY: this guard owns a valid HIMC and `composition` is live for the synchronous call.
        if !unsafe { ImmSetCompositionWindow(self.himc, &raw const composition) }.as_bool() {
            log::warn!("ImmSetCompositionWindow failed");
        }
    }

    pub(crate) fn set_candidate_window(&self, origin: POINT, exclude: RECT) {
        let candidate = CANDIDATEFORM {
            dwIndex: 0,
            dwStyle: CFS_EXCLUDE,
            ptCurrentPos: origin,
            rcArea: exclude,
        };
        // SAFETY: this guard owns a valid HIMC and `candidate` is live for the synchronous call.
        if !unsafe { ImmSetCandidateWindow(self.himc, &raw const candidate) }.as_bool() {
            log::warn!("ImmSetCandidateWindow failed");
        }
    }

    /// Ask the IME to finalize the composition string (`CPS_COMPLETE` / `CPS_CANCEL`).
    pub(crate) fn notify_composition(&self, action: NOTIFY_IME_INDEX) -> bool {
        // SAFETY: this guard owns a valid HIMC.
        unsafe { ImmNotifyIME(self.himc, NI_COMPOSITIONSTR, action, 0) }.as_bool()
    }
}

impl Drop for ImmContext {
    fn drop(&mut self) {
        // SAFETY: this guard owns exactly one successful `ImmGetContext` result for `self.hwnd`.
        if !unsafe { ImmReleaseContext(self.hwnd, self.himc) }.as_bool() {
            log::warn!("ImmReleaseContext failed");
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ImeState {
    client: Option<TextInputClient>,
    enabled: bool,
    focused: bool,
    composition_active: bool,
    app_has_marked_text: bool,
    finalizing: bool,
    composition_revision: u64,
    callback_depth: u32,
    pending_high_surrogate: Option<u16>,
}

impl ImeState {
    pub(crate) const fn new() -> Self {
        Self {
            client: None,
            enabled: true,
            focused: false,
            composition_active: false,
            app_has_marked_text: false,
            finalizing: false,
            composition_revision: 0,
            callback_depth: 0,
            pending_high_surrogate: None,
        }
    }

    pub(crate) const fn enabled_client(self) -> Option<TextInputClient> {
        if self.enabled { self.client } else { None }
    }

    pub(crate) const fn active_client(self) -> Option<TextInputClient> {
        if self.focused && self.enabled { self.client } else { None }
    }

    pub(crate) const fn is_active(self) -> bool {
        self.focused && self.enabled && self.client.is_some()
    }

    pub(crate) const fn is_enabled(self) -> bool {
        self.enabled
    }

    pub(crate) const fn is_composition_active(self) -> bool {
        self.composition_active
    }

    pub(crate) const fn app_has_marked_text(self) -> bool {
        self.app_has_marked_text
    }

    pub(crate) const fn is_finalizing(self) -> bool {
        self.finalizing
    }

    pub(crate) const fn revision(self) -> u64 {
        self.composition_revision
    }

    const fn advance_composition_revision(&mut self) -> u64 {
        self.composition_revision += 1;
        self.composition_revision
    }

    pub(crate) fn join_surrogate(&mut self, unit: u16) -> Option<String> {
        let pending = self.pending_high_surrogate.take();
        if (0xD800..=0xDBFF).contains(&unit) {
            self.pending_high_surrogate = Some(unit);
            return None;
        }
        if (0xDC00..=0xDFFF).contains(&unit) {
            return pending.map(|high| String::from_utf16_lossy(&[high, unit]));
        }
        Some(String::from_utf16_lossy(&[unit]))
    }

    pub(crate) const fn reset_pending_surrogate(&mut self) {
        self.pending_high_surrogate = None;
    }

    pub(crate) const fn replace_client(&mut self, client: Option<TextInputClient>) {
        self.client = client;
        self.reset_pending_surrogate();
    }

    pub(crate) const fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.reset_pending_surrogate();
    }

    pub(crate) const fn set_focused(&mut self, focused: bool) {
        let changed = self.focused != focused;
        self.focused = focused;
        self.reset_pending_surrogate();
        if changed {
            self.advance_composition_revision();
        }
    }

    pub(crate) const fn start_composition(&mut self) {
        self.composition_active = true;
        self.app_has_marked_text = false;
        self.reset_pending_surrogate();
        self.advance_composition_revision();
    }

    pub(crate) const fn set_app_marked(&mut self, value: bool) -> u64 {
        self.app_has_marked_text = value;
        self.advance_composition_revision()
    }

    pub(crate) const fn begin_finalizing(&mut self) {
        self.finalizing = true;
        self.advance_composition_revision();
    }

    pub(crate) const fn clear_composition_state(&mut self) -> u64 {
        self.composition_active = false;
        self.app_has_marked_text = false;
        self.finalizing = false;
        self.reset_pending_surrogate();
        self.advance_composition_revision()
    }

    pub(crate) fn ensure_mutation_allowed(self, operation: &str) -> anyhow::Result<()> {
        anyhow::ensure!(self.callback_depth == 0, "{operation} is not allowed during a text input callback");
        Ok(())
    }
}

pub(crate) struct ClientCallbackGuard<'a>(&'a Cell<ImeState>);

impl<'a> ClientCallbackGuard<'a> {
    pub(crate) fn enter(state: &'a Cell<ImeState>) -> Self {
        let mut ime = state.get();
        ime.callback_depth += 1;
        state.set(ime);
        Self(state)
    }
}

impl Drop for ClientCallbackGuard<'_> {
    fn drop(&mut self) {
        let mut ime = self.0.get();
        ime.callback_depth = ime.callback_depth.checked_sub(1).expect("text input callback depth underflow");
        self.0.set(ime);
    }
}

pub(crate) trait CompositionSource {
    /// Raw byte payloads (`GCS_COMPATTR`, `GCS_COMPCLAUSE`).
    fn bytes(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<Vec<u8>>;
    /// UTF-16 string payloads (`GCS_COMPSTR`, `GCS_RESULTSTR`).
    fn utf16(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<Vec<u16>>;
    /// `None` when the IME shows no composition cursor.
    fn cursor(&self) -> Option<usize>;
}

impl CompositionSource for ImmContext {
    fn bytes(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<Vec<u8>> {
        self.composition_payload(which)
    }

    fn utf16(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<Vec<u16>> {
        self.composition_payload(which)
    }

    fn cursor(&self) -> Option<usize> {
        // SAFETY: this guard owns a valid HIMC; `GCS_CURSORPOS` returns the scalar as the result.
        let cursor = unsafe { ImmGetCompositionStringW(self.himc, GCS_CURSORPOS, None, 0) };
        // A negative value is the documented "no visible cursor" state, not an IMM error.
        usize::try_from(cursor).ok()
    }
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

fn composition_attribute_from_raw(value: u8) -> TextCompositionAttribute {
    match u32::from(value) {
        ATTR_INPUT => TextCompositionAttribute::Input,
        ATTR_TARGET_CONVERTED => TextCompositionAttribute::TargetConverted,
        ATTR_CONVERTED => TextCompositionAttribute::Converted,
        ATTR_TARGET_NOTCONVERTED => TextCompositionAttribute::TargetNotConverted,
        ATTR_INPUT_ERROR => TextCompositionAttribute::InputError,
        ATTR_FIXEDCONVERTED => TextCompositionAttribute::FixedConverted,
        _ => TextCompositionAttribute::Unspecified,
    }
}

fn segments_from_parts(attrs: &[u8], clauses: &[u32], preedit_len: usize) -> Vec<TextCompositionSegment> {
    let bounds = clauses.iter().map(|value| usize::try_from(*value)).collect::<Result<Vec<_>, _>>();
    let Ok(bounds) = bounds else {
        return fallback_segments(preedit_len);
    };
    if attrs.len() != preedit_len
        || bounds.len() < 2
        || bounds.first() != Some(&0)
        || bounds.last() != Some(&preedit_len)
        || bounds.iter().any(|value| *value > preedit_len)
        || bounds.windows(2).any(|pair| pair[0] > pair[1])
    {
        return fallback_segments(preedit_len);
    }

    bounds
        .windows(2)
        .filter_map(|pair| {
            let (start, end) = (pair[0], pair[1]);
            if start >= end {
                return None;
            }
            Some(TextCompositionSegment {
                range: TextRange {
                    location: start,
                    length: end - start,
                },
                attribute: composition_attribute_from_raw(attrs[start]),
            })
        })
        .collect()
}

fn fallback_segments(preedit_len: usize) -> Vec<TextCompositionSegment> {
    (preedit_len != 0)
        .then_some(TextCompositionSegment {
            range: TextRange {
                location: 0,
                length: preedit_len,
            },
            attribute: TextCompositionAttribute::Unspecified,
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
    pub(crate) text: String,
    /// `TextRange::none()` when the IME shows no composition cursor.
    pub(crate) selected: TextRange,
    pub(crate) segments: Vec<TextCompositionSegment>,
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
            .then(|| source.utf16(GCS_RESULTSTR).map(|units| String::from_utf16_lossy(&units)))
            .transpose()?;
        let preedit = if gcs & GCS_PREEDIT_UPDATE != 0 {
            let units = source.utf16(GCS_COMPSTR)?;
            let length = units.len();
            let text = String::from_utf16_lossy(&units);
            let selected = source.cursor().map_or_else(TextRange::none, |cursor| TextRange {
                location: cursor.min(length),
                length: 0,
            });
            let segments = match (
                source.bytes(GCS_COMPATTR),
                source.bytes(GCS_COMPCLAUSE).and_then(|bytes| decode_u32_bytes(&bytes)),
            ) {
                (Ok(attrs), Ok(clauses)) => segments_from_parts(&attrs, &clauses, length),
                (Err(err), _) | (_, Err(err)) => {
                    log::warn!("reading IME composition metadata failed: {err:#}");
                    fallback_segments(length)
                }
            };
            Some(PreeditSnapshot { text, selected, segments })
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

pub(crate) trait CompositionSink {
    fn revision(&self) -> u64;
    fn set_app_marked(&self, value: bool) -> u64;
    fn clear_composition(&self) -> u64;
    fn insert_text(&self, text: &str);
    fn set_marked_text(&self, preedit: &PreeditSnapshot);
    fn discard_marked_text(&self);
    fn update_windows(&self);
}

/// Deliver one composition snapshot to the sink. Every client callback can synchronously reenter
/// composition teardown (a nested END or focus loss); comparing the revision after each callback
/// against the last state transition this function made detects that and abandons the remaining,
/// now-stale steps.
fn apply_composition(sink: &impl CompositionSink, snapshot: CompositionSnapshot) {
    let mut expected_revision = sink.revision();
    if let Some(result) = snapshot.result.filter(|text| !text.is_empty()) {
        sink.insert_text(&result);
        if sink.revision() != expected_revision {
            return;
        }
        expected_revision = sink.set_app_marked(false);
    }
    if let Some(preedit) = snapshot.preedit {
        if preedit.text.is_empty() {
            sink.discard_marked_text();
            if sink.revision() != expected_revision {
                return;
            }
            sink.set_app_marked(false);
        } else {
            expected_revision = sink.set_app_marked(true);
            sink.set_marked_text(&preedit);
            if sink.revision() != expected_revision {
                return;
            }
        }
        sink.update_windows();
    } else if snapshot.cancelled {
        sink.clear_composition();
        sink.discard_marked_text();
    }
}

pub(crate) fn apply_owned_composition(sink: &impl CompositionSink, source: &impl CompositionSource, gcs: u32) {
    match CompositionSnapshot::read(source, gcs) {
        Ok(snapshot) => apply_composition(sink, snapshot),
        Err(err) => log::warn!("reading IME composition failed; keeping ownership until next update or END: {err:#}"),
    }
}

/// Deliver the result of this window's own `CPS_COMPLETE` finalization. The reentrant
/// `WM_IME_COMPOSITION` arrives while composition state is being torn down, so only
/// `GCS_RESULTSTR` matters — preedit flags describe a composition that no longer exists.
pub(crate) fn apply_finalizing_composition(sink: &impl CompositionSink, source: &impl CompositionSource, gcs: u32) {
    if gcs & GCS_RESULTSTR.0 == 0 {
        return;
    }
    match source.utf16(GCS_RESULTSTR) {
        Ok(units) if !units.is_empty() => sink.insert_text(&String::from_utf16_lossy(&units)),
        Ok(_) => {}
        Err(err) => log::warn!("reading the finalized IME result failed: {err:#}"),
    }
}

#[cfg(test)]
mod ime_state_tests {
    use super::*;

    #[test]
    fn ime_starts_enabled_without_a_client_or_focus() {
        let state = ImeState::new();
        assert!(state.enabled);
        assert!(!state.focused);
        assert!(state.client.is_none());
        assert!(!state.is_active());
    }

    #[test]
    fn callback_guard_nests_and_restores_depth() {
        let state = Cell::new(ImeState::new());
        {
            let _outer = ClientCallbackGuard::enter(&state);
            assert_eq!(state.get().callback_depth, 1);
            {
                let _inner = ClientCallbackGuard::enter(&state);
                assert_eq!(state.get().callback_depth, 2);
            }
            assert_eq!(state.get().callback_depth, 1);
        }
        assert_eq!(state.get().callback_depth, 0);
    }

    #[test]
    fn surrogate_joiner_handles_bmp_pair_and_lone_low() {
        let mut ime = ImeState::new();
        assert_eq!(ime.join_surrogate('A' as u16), Some("A".to_owned()));
        assert_eq!(ime.join_surrogate(0xD83D), None);
        assert_eq!(ime.join_surrogate(0xDE00), Some("😀".to_owned()));
        assert_eq!(ime.join_surrogate(0xDE00), None);
    }

    #[test]
    fn surrogate_joiner_drops_an_interrupted_high_unit() {
        let mut ime = ImeState::new();
        assert_eq!(ime.join_surrogate(0xD83D), None);
        ime.reset_pending_surrogate();
        assert_eq!(ime.join_surrogate(0xDE00), None);
    }

    #[test]
    fn pending_surrogate_reset_is_idempotent() {
        let mut ime = ImeState::new();
        ime.pending_high_surrogate = Some(0xD83D);
        ime.reset_pending_surrogate();
        assert_eq!(ime.pending_high_surrogate, None);
        ime.reset_pending_surrogate();
        assert_eq!(ime.pending_high_surrogate, None);
    }

    #[test]
    fn callback_depth_rejects_lifetime_mutation() {
        let mut ime = ImeState::new();
        assert!(ime.ensure_mutation_allowed("client change").is_ok());
        ime.callback_depth = 1;
        assert_eq!(
            ime.ensure_mutation_allowed("client change").unwrap_err().to_string(),
            "client change is not allowed during a text input callback",
        );
    }

    #[test]
    fn client_replacement_clears_pending_surrogate() {
        let mut ime = ImeState::new();
        ime.pending_high_surrogate = Some(0xD83D);
        ime.replace_client(None);
        assert_eq!(ime.pending_high_surrogate, None);
    }

    #[test]
    fn focus_loss_clears_pending_surrogate() {
        let mut ime = ImeState::new();
        ime.focused = true;
        ime.pending_high_surrogate = Some(0xD83D);
        let revision = ime.composition_revision;
        ime.set_focused(false);
        assert!(!ime.focused);
        assert_eq!(ime.pending_high_surrogate, None);
        assert_eq!(ime.composition_revision, revision + 1);
    }

    #[test]
    fn composition_start_marks_active_and_advances_revision() {
        let mut ime = ImeState::new();
        ime.app_has_marked_text = true;
        ime.pending_high_surrogate = Some(0xD83D);
        let revision = ime.composition_revision;
        ime.start_composition();
        assert!(ime.composition_active);
        assert!(!ime.app_has_marked_text);
        assert_eq!(ime.pending_high_surrogate, None);
        assert_eq!(ime.composition_revision, revision + 1);
    }

    #[test]
    fn composition_end_clears_pending_surrogate() {
        let mut ime = ImeState::new();
        ime.composition_active = true;
        ime.app_has_marked_text = true;
        ime.finalizing = true;
        ime.pending_high_surrogate = Some(0xD83D);
        let revision = ime.composition_revision;
        assert_eq!(ime.clear_composition_state(), revision + 1);
        assert!(!ime.composition_active);
        assert!(!ime.app_has_marked_text);
        assert!(!ime.finalizing);
        assert_eq!(ime.pending_high_surrogate, None);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;

    use windows::Win32::UI::Input::Ime::{
        CS_INSERTCHAR, CS_NOMOVECARET, GCS_COMPATTR, GCS_COMPCLAUSE, GCS_COMPSTR, GCS_CURSORPOS, GCS_DELTASTART, GCS_RESULTSTR,
        IME_COMPOSITION_STRING,
    };

    fn utf16_units(value: &str) -> Vec<u16> {
        value.encode_utf16().collect()
    }

    fn u32_bytes(values: &[u32]) -> Vec<u8> {
        values.iter().flat_map(|value| value.to_ne_bytes()).collect()
    }

    #[derive(Default)]
    struct FakeSource {
        result: Vec<u16>,
        composition: Vec<u16>,
        attributes: Vec<u8>,
        clauses: Vec<u8>,
        cursor: Option<usize>,
        fail_on: Option<IME_COMPOSITION_STRING>,
    }

    impl FakeSource {
        fn ensure_available(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<()> {
            anyhow::ensure!(self.fail_on != Some(which), "injected {which:?} failure");
            Ok(())
        }
    }

    impl CompositionSource for FakeSource {
        fn bytes(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<Vec<u8>> {
            self.ensure_available(which)?;
            Ok(match which {
                GCS_COMPATTR => self.attributes.clone(),
                GCS_COMPCLAUSE => self.clauses.clone(),
                _ => Vec::new(),
            })
        }

        fn utf16(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<Vec<u16>> {
            self.ensure_available(which)?;
            Ok(match which {
                GCS_RESULTSTR => self.result.clone(),
                GCS_COMPSTR => self.composition.clone(),
                _ => Vec::new(),
            })
        }

        fn cursor(&self) -> Option<usize> {
            self.cursor
        }
    }

    #[test]
    fn u32_decoder_rejects_misaligned_data() {
        assert!(decode_u32_bytes(&[0, 0, 0]).is_err());
    }

    #[test]
    fn composition_attributes_preserve_all_known_imm32_values() {
        let attributes = [
            (ATTR_INPUT, TextCompositionAttribute::Input),
            (ATTR_TARGET_CONVERTED, TextCompositionAttribute::TargetConverted),
            (ATTR_CONVERTED, TextCompositionAttribute::Converted),
            (ATTR_TARGET_NOTCONVERTED, TextCompositionAttribute::TargetNotConverted),
            (ATTR_INPUT_ERROR, TextCompositionAttribute::InputError),
            (ATTR_FIXEDCONVERTED, TextCompositionAttribute::FixedConverted),
        ];
        for (raw, attribute) in attributes {
            assert_eq!(composition_attribute_from_raw(u8::try_from(raw).unwrap()), attribute);
        }
        assert_eq!(TextCompositionAttribute::Input as u32, 0);
        assert_eq!(TextCompositionAttribute::TargetConverted as u32, 1);
        assert_eq!(TextCompositionAttribute::Converted as u32, 2);
        assert_eq!(TextCompositionAttribute::TargetNotConverted as u32, 3);
        assert_eq!(TextCompositionAttribute::InputError as u32, 4);
        assert_eq!(TextCompositionAttribute::FixedConverted as u32, 5);
        assert_eq!(TextCompositionAttribute::Unspecified as u32, 255);
        assert_ne!(
            TextCompositionAttribute::TargetConverted,
            TextCompositionAttribute::TargetNotConverted
        );
    }

    #[test]
    fn composition_segments_keep_clause_ranges_and_attributes() {
        let segments = segments_from_parts(
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
            segments,
            vec![
                TextCompositionSegment {
                    range: TextRange { location: 0, length: 2 },
                    attribute: TextCompositionAttribute::Input,
                },
                TextCompositionSegment {
                    range: TextRange { location: 2, length: 2 },
                    attribute: TextCompositionAttribute::TargetConverted,
                },
            ],
        );
    }

    #[test]
    fn malformed_clauses_fall_back_to_whole_preedit() {
        assert_eq!(
            segments_from_parts(&[u8::try_from(ATTR_INPUT).unwrap(); 3], &[1, 3], 3),
            fallback_segments(3),
        );
    }

    #[test]
    fn reserved_composition_attribute_becomes_unspecified() {
        assert_eq!(composition_attribute_from_raw(42), TextCompositionAttribute::Unspecified);
    }

    #[test]
    fn partial_preedit_flags_refetch_complete_preedit() {
        let source = FakeSource {
            composition: utf16_units("かな"),
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
            result: utf16_units("確定"),
            composition: utf16_units("つぎ"),
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
            composition: utf16_units("かな"),
            cursor: Some(99),
            ..Default::default()
        };
        let clamped = CompositionSnapshot::read(&source, GCS_COMPSTR.0).unwrap();
        assert_eq!(clamped.preedit.unwrap().selected, TextRange { location: 2, length: 0 });
    }

    #[test]
    fn hidden_cursor_renders_preedit_without_selection() {
        let source = FakeSource {
            composition: utf16_units("かな"),
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
    fn optional_composition_metadata_failure_uses_unspecified_fallback() {
        let source = FakeSource {
            composition: utf16_units("かな"),
            fail_on: Some(GCS_COMPATTR),
            ..Default::default()
        };
        let snapshot = CompositionSnapshot::read(&source, GCS_COMPSTR.0).unwrap();
        assert_eq!(snapshot.preedit.unwrap().segments, fallback_segments(2));
    }

    #[test]
    fn core_read_failure_is_returned_before_any_action() {
        let source = FakeSource {
            fail_on: Some(GCS_COMPSTR),
            ..Default::default()
        };
        assert!(CompositionSnapshot::read(&source, GCS_COMPSTR.0).is_err());
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
            let next = self.revision.get() + 1;
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
    fn apply_commits_result_before_starting_next_preedit() {
        let sink = FakeSink::default();
        let source = FakeSource {
            result: utf16_units("確定"),
            composition: utf16_units("つぎ"),
            cursor: Some(1),
            ..Default::default()
        };
        apply_owned_composition(&sink, &source, GCS_RESULTSTR.0 | GCS_COMPSTR.0);
        assert_eq!(&*sink.callbacks.borrow(), &["insert", "set_marked", "update_windows"]);
        assert_eq!(&*sink.inserted.borrow(), &["確定"]);
        assert!(sink.app_marked.get());
    }

    #[test]
    fn apply_distinguishes_empty_preedit_and_cancel() {
        let empty_preedit = FakeSink::default();
        empty_preedit.app_marked.set(true);
        apply_owned_composition(&empty_preedit, &FakeSource::default(), GCS_COMPSTR.0);
        assert_eq!(&*empty_preedit.callbacks.borrow(), &["discard", "update_windows"]);
        assert!(!empty_preedit.app_marked.get());

        let cancel = FakeSink::default();
        cancel.composition_active.set(true);
        cancel.app_marked.set(true);
        apply_owned_composition(&cancel, &FakeSource::default(), 0);
        assert_eq!(&*cancel.callbacks.borrow(), &["discard"]);
        assert!(!cancel.composition_active.get());
        assert!(!cancel.app_marked.get());
    }

    #[test]
    fn apply_stops_after_reentrant_end_during_insert() {
        let sink = FakeSink {
            reenter_on: Some(ReenterOn::Insert),
            ..Default::default()
        };
        sink.app_marked.set(true);
        apply_composition(
            &sink,
            CompositionSnapshot {
                result: Some("commit".to_owned()),
                preedit: Some(PreeditSnapshot {
                    text: "stale".to_owned(),
                    selected: TextRange { location: 0, length: 0 },
                    segments: Vec::new(),
                }),
                cancelled: false,
            },
        );
        assert_eq!(&*sink.callbacks.borrow(), &["insert"]);
        assert!(!sink.app_marked.get());
    }

    #[test]
    fn apply_stops_before_positioning_after_reentrant_focus_loss() {
        let sink = FakeSink {
            reenter_on: Some(ReenterOn::SetMarked),
            ..Default::default()
        };
        apply_composition(
            &sink,
            CompositionSnapshot {
                result: None,
                preedit: Some(PreeditSnapshot {
                    text: "preedit".to_owned(),
                    selected: TextRange { location: 0, length: 0 },
                    segments: Vec::new(),
                }),
                cancelled: false,
            },
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
        apply_owned_composition(&sink, &source, GCS_COMPSTR.0);
        assert_eq!(sink.revision.get(), 7);
        assert!(sink.composition_active.get());
        assert!(sink.app_marked.get());
        assert!(sink.callbacks.borrow().is_empty());
    }

    #[test]
    fn owned_post_end_result_is_still_applied() {
        let sink = FakeSink::default();
        let source = FakeSource {
            result: utf16_units("한"),
            ..Default::default()
        };
        apply_owned_composition(&sink, &source, GCS_RESULTSTR.0);
        assert_eq!(&*sink.callbacks.borrow(), &["insert"]);
    }

    #[test]
    fn finalizing_result_routes_to_insert_only() {
        let sink = FakeSink::default();
        let source = FakeSource {
            result: utf16_units("確定"),
            composition: utf16_units("つぎ"),
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
            composition: utf16_units("つぎ"),
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
