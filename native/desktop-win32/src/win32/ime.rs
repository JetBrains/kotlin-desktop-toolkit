use windows::Win32::{
    Foundation::HWND,
    UI::Input::Ime::{HIMC, ImmGetContext, ImmReleaseContext},
};

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
