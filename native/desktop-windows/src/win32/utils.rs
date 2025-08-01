macro_rules! LOWORD {
    ($arg:ident) => {
        ($arg.0 & 0xffff)
    };
}

macro_rules! HIWORD {
    ($arg:ident) => {
        (($arg.0 >> 16) & 0xffff)
    };
}

macro_rules! GET_Y_LPARAM {
    ($arg:ident) => {
        ((($arg.0 >> 16) & 0xffff) as i16)
    };
}

pub(crate) use {GET_Y_LPARAM, HIWORD, LOWORD};
