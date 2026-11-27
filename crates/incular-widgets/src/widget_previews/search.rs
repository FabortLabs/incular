//! Reactive search query and field policy for widget previews.

use std::{cell::Cell, rc::Rc};

use crate::{TextEditingController, TextEditingValue};
use incular_core::reactivity::DependencySource;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewSearchField {
    Group,
    Name,
    Script,
    Package,
}

impl PreviewSearchField {
    fn mask(self) -> u8 {
        1 << self as u8
    }
}

/// One shared search buffer for the mounted editor and preview filter.
/// Programmatic changes move the caret to the end and clear composition.
#[derive(Clone)]
pub struct PreviewSearch {
    editor: TextEditingController,
    fields: Rc<Cell<u8>>,
    field_changes: DependencySource,
}

impl Default for PreviewSearch {
    fn default() -> Self {
        Self {
            editor: TextEditingController::new(),
            fields: Rc::new(Cell::new(0b1111)),
            field_changes: DependencySource::default(),
        }
    }
}

impl PreviewSearch {
    #[must_use]
    pub fn editing_controller(&self) -> TextEditingController {
        self.editor.clone()
    }

    #[must_use]
    pub fn query(&self) -> String {
        self.editor.text()
    }

    pub fn set_query(&self, query: impl Into<String>) {
        let query = query.into();
        if !self.editor.with_value(|value| value.text == query) {
            self.editor.set_value(TextEditingValue::new(query));
        }
    }

    pub fn clear(&self) {
        self.set_query("");
    }

    #[must_use]
    pub fn field_enabled(&self, field: PreviewSearchField) -> bool {
        self.field_changes.track();
        self.fields.get() & field.mask() != 0
    }

    /// Returns false if disabling this field would leave no searchable fields.
    pub fn toggle_field(&self, field: PreviewSearchField) -> bool {
        let next = self.fields.get() ^ field.mask();
        if next == 0 {
            return false;
        }
        self.fields.set(next);
        self.field_changes.notify();
        true
    }

    /// Normalizes one query once, for filtering several preview descriptors.
    #[must_use]
    pub fn matcher(&self) -> PreviewSearchMatcher {
        self.field_changes.track();
        PreviewSearchMatcher {
            fields: self.fields.get(),
            term: self
                .editor
                .with_value(|value| value.text.trim().to_lowercase()),
        }
    }
}

pub struct PreviewSearchMatcher {
    fields: u8,
    term: String,
}

impl PreviewSearchMatcher {
    #[must_use]
    pub fn matches(
        &self,
        group: &str,
        name: Option<&str>,
        script_uri: &str,
        package: &str,
    ) -> bool {
        use PreviewSearchField as Field;
        self.term.is_empty()
            || [
                (Field::Group, group),
                (Field::Name, name.unwrap_or("")),
                (Field::Script, script_uri),
                (Field::Package, package),
            ]
            .into_iter()
            .any(|(field, value)| {
                self.fields & field.mask() != 0 && value.to_lowercase().contains(&self.term)
            })
    }
}
