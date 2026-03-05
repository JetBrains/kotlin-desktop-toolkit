#[derive(Debug)]
pub struct MimeTypes<'a> {
    pub val: Box<[&'a str]>,
}

impl<'a> MimeTypes<'a> {
    pub fn new(mime_types_str: &'a str) -> Self {
        if mime_types_str.is_empty() {
            Self { val: Box::new([]) }
        } else {
            Self {
                val: mime_types_str.split(',').collect(),
            }
        }
    }
}
