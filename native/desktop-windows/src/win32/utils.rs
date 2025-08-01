use windows::Win32::Foundation::{LPARAM, WPARAM};

pub(crate) trait WLParamUtil {
    type T;

    #[allow(non_snake_case)]
    fn LOWORD(&self) -> Self::T;

    #[allow(non_snake_case)]
    fn HIWORD(&self) -> Self::T;
}

impl WLParamUtil for WPARAM {
    type T = usize;

    fn LOWORD(&self) -> Self::T {
        self.0 & 0xffff
    }

    fn HIWORD(&self) -> Self::T {
        (self.0 >> 16) & 0xffff
    }
}

impl WLParamUtil for LPARAM {
    type T = isize;

    fn LOWORD(&self) -> Self::T {
        self.0 & 0xffff
    }

    fn HIWORD(&self) -> Self::T {
        (self.0 >> 16) & 0xffff
    }
}
