pub struct MimeTypes<'a> {
    pub val: Box<[&'a str]>,
}

impl<'a> MimeTypes<'a> {
    pub fn new(mime_types_str: &'a str) -> Self {
        Self {
            val: mime_types_str.split(',').collect(),
        }
    }
}
