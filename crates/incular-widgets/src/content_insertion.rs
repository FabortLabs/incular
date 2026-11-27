//! App handling of rich content supplied by the virtual keyboard.

use std::{rc::Rc, sync::Arc};

use incular_platform::KeyboardInsertedContent;

pub const DEFAULT_CONTENT_INSERTION_MIME_TYPES: &[&str] = &[
    "image/png",
    "image/bmp",
    "image/jpg",
    "image/tiff",
    "image/gif",
    "image/jpeg",
    "image/webp",
];

#[derive(Clone)]
pub struct ContentInsertionConfiguration {
    pub(crate) allowed_mime_types: Arc<[String]>,
    pub(crate) on_content_inserted: Rc<dyn Fn(KeyboardInsertedContent)>,
}

impl ContentInsertionConfiguration {
    #[must_use]
    pub fn new(callback: impl Fn(KeyboardInsertedContent) + 'static) -> Self {
        Self {
            allowed_mime_types: DEFAULT_CONTENT_INSERTION_MIME_TYPES
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            on_content_inserted: Rc::new(callback),
        }
    }

    pub fn with_allowed_mime_types(
        mut self,
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, EmptyContentInsertionMimeTypes> {
        let types: Arc<[String]> = values.into_iter().map(Into::into).collect();
        if types.is_empty() {
            return Err(EmptyContentInsertionMimeTypes);
        }
        self.allowed_mime_types = types;
        Ok(self)
    }

    #[must_use]
    pub fn allowed_mime_types(&self) -> &[String] {
        &self.allowed_mime_types
    }
}

impl PartialEq for ContentInsertionConfiguration {
    fn eq(&self, other: &Self) -> bool {
        self.allowed_mime_types == other.allowed_mime_types
            && Rc::ptr_eq(&self.on_content_inserted, &other.on_content_inserted)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EmptyContentInsertionMimeTypes;

impl std::fmt::Display for EmptyContentInsertionMimeTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the keyboard content MIME type list must not be empty")
    }
}

impl std::error::Error for EmptyContentInsertionMimeTypes {}
