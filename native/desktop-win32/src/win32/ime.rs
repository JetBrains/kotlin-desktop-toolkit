use std::cell::Cell;

use windows::Win32::{
    Foundation::HWND,
    UI::Input::Ime::{HIMC, ImmGetContext, ImmReleaseContext},
};

use super::text_input_client::TextInputClient;

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

    pub(crate) const fn himc(&self) -> HIMC {
        self.himc
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
    pub(crate) client: Option<TextInputClient>,
    pub(crate) enabled: bool,
    pub(crate) focused: bool,
    pub(crate) composition_active: bool,
    pub(crate) app_has_marked_text: bool,
    pub(crate) finalizing: bool,
    pub(crate) composition_revision: u64,
    pub(crate) callback_depth: u32,
    pub(crate) pending_high_surrogate: u16,
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
            pending_high_surrogate: 0,
        }
    }

    pub(crate) const fn detached() -> Self {
        Self {
            enabled: false,
            ..Self::new()
        }
    }

    pub(crate) const fn is_active(self) -> bool {
        self.focused && self.enabled && self.client.is_some()
    }

    pub(crate) const fn advance_composition_revision(&mut self) -> u64 {
        self.composition_revision = self.composition_revision.checked_add(1).expect("IME composition revision overflow");
        self.composition_revision
    }

    pub(crate) fn join_surrogate(&mut self, unit: u16) -> Option<String> {
        let pending = self.pending_high_surrogate;
        if (0xD800..=0xDBFF).contains(&unit) {
            self.pending_high_surrogate = unit;
            return None;
        }
        self.pending_high_surrogate = 0;
        if (0xDC00..=0xDFFF).contains(&unit) {
            return (pending != 0).then(|| String::from_utf16_lossy(&[pending, unit]));
        }
        Some(String::from_utf16_lossy(&[unit]))
    }

    pub(crate) const fn reset_pending_surrogate(&mut self) {
        self.pending_high_surrogate = 0;
    }
}

pub(crate) struct ClientCallbackGuard<'a>(&'a Cell<ImeState>);

impl<'a> ClientCallbackGuard<'a> {
    pub(crate) fn enter(state: &'a Cell<ImeState>) -> Self {
        let mut ime = state.get();
        ime.callback_depth = ime.callback_depth.checked_add(1).expect("text input callback depth overflow");
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
        ime.pending_high_surrogate = 0xD83D;
        ime.reset_pending_surrogate();
        assert_eq!(ime.pending_high_surrogate, 0);
        ime.reset_pending_surrogate();
        assert_eq!(ime.pending_high_surrogate, 0);
    }
}
