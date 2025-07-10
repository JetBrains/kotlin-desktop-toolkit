#[allow(non_snake_case)]
pub const fn LOWORD(l: usize) -> usize {
    l & 0xffff
}

#[allow(non_snake_case)]
pub const fn HIWORD(l: usize) -> usize {
    (l >> 16) & 0xffff
}
