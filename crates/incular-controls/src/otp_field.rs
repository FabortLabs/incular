//! One retained editor with fixed visual slots, rather than one editor per
//! digit. This keeps paste, selection, and accessibility coherent.
use incular_widgets::internal::TextEditingController;
use incular_widgets::{
    EditableText, LengthLimitingTextInputFormatter, MaxLengthEnforcement, Opacity, Positioned,
    Stack, TextInputFormatter, Widget,
};

use typed_builder::TypedBuilder;

#[derive(Clone, TypedBuilder)]
pub struct Root {
    #[builder(default = 6, setter(transform = |length: usize| length.max(1)))]
    length: usize,
    #[builder(default, setter(strip_option, into))]
    child: Option<Widget>,
    #[builder(default)]
    masked: bool,
}
impl Default for Root {
    fn default() -> Self {
        Self::builder().build()
    }
}
impl Root {
    #[must_use]
    pub fn new(length: usize) -> Self {
        Self::builder().length(length).build()
    }
    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }
    #[must_use]
    pub fn is_masked(&self) -> bool {
        self.masked
    }
    #[must_use]
    pub fn masked(mut self, value: bool) -> Self {
        self.masked = value;
        self
    }
    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }
}
impl From<Root> for Widget {
    fn from(value: Root) -> Self {
        let controller = TextEditingController::new();
        // Each cell represents one accepted token, including while the user
        // composes. The general text formatter follows host IME defaults.
        let limiter = LengthLimitingTextInputFormatter::with_enforcement(
            value.length,
            MaxLengthEnforcement::Enforced,
        );
        let editor: Widget = EditableText::new(controller)
            .obscure_text(value.masked)
            .edit_transform(move |old, next| limiter.format_edit_update(old, next))
            .into();
        match value.child {
            None => editor,
            Some(visual) => {
                // The invisible editor keeps its semantics: since Opacity follows
                // Flutter (a zero alpha drops semantics unless
                // `alwaysIncludeSemantics`, rendering/proxy_box.dart
                // `RenderOpacity.visitChildrenForSemantics`), opt back in.
                Stack::new([
                    visual,
                    Positioned::fill(Opacity::new(0.0, editor).always_include_semantics(true))
                        .into(),
                ])
                .into()
            }
        }
    }
}
