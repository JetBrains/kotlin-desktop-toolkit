macro_rules! LOWORD {
    ($arg:expr) => {
        ($arg as usize & 0xffff) as u16
    };
}

macro_rules! HIWORD {
    ($arg:expr) => {
        (($arg as usize >> 16) & 0xffff) as u16
    };
}

macro_rules! LOBYTE {
    ($arg:expr) => {
        ($arg as usize & 0xff) as u8
    };
}

macro_rules! GET_X_LPARAM {
    ($arg:expr) => {
        (($arg as usize & 0xffff) as i16) as i32
    };
}

macro_rules! GET_Y_LPARAM {
    ($arg:expr) => {
        ((($arg as usize >> 16) & 0xffff) as i16) as i32
    };
}

pub(crate) use {GET_X_LPARAM, GET_Y_LPARAM, HIWORD, LOBYTE, LOWORD};
