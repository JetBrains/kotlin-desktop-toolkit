pub fn join_str_iter<'a>(iter: impl Iterator<Item = &'a str> + Clone, sep: &str) -> String {
    let string_len: usize = iter.clone().map(|e| e.len() + sep.len()).sum();
    let acc = String::with_capacity(string_len + 1); // allocate taking into account the possible null terminator
    iter.fold(acc, |mut acc, e| {
        acc.push_str(e);
        acc.push_str(sep);
        acc
    })
}
