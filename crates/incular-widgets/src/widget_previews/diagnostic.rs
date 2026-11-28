//! Displays errors from a preview factory without compromising its neighbors.

use std::rc::Rc;

use incular_config::{CrossAxisAlignment, MainAxisSize};
use incular_core::ValueNotifier;
use incular_semantics::Role;

use crate::{
    Builder, Column, GestureDetector, HitTestBehavior, SelectionAreaController, Semantics,
    SingleChildScrollView, SizedBox, Text, Widget,
};

/// A source location as reported by the preview host or its symbolizer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewSourceLocation {
    pub uri: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

impl PreviewSourceLocation {
    fn is_linkable(&self) -> bool {
        self.uri.split_once(':').is_some_and(|(scheme, _)| {
            scheme.eq_ignore_ascii_case("file") || scheme.eq_ignore_ascii_case("package")
        })
    }
}

/// One diagnostic frame. Unparsed text remains visible but cannot navigate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewDiagnosticFrame {
    pub text: String,
    pub source: Option<PreviewSourceLocation>,
}

#[derive(Clone)]
struct EditorNavigation {
    available: ValueNotifier<bool>,
    open: Rc<dyn Fn(&PreviewSourceLocation)>,
}

/// The visible error and selectable stack trace for a failed preview factory.
/// The host supplies source locations and owns package URI resolution and
/// opening the active editor. No editor is contacted by the widget itself.
#[derive(Clone)]
pub struct PreviewDiagnostic {
    message: String,
    frames: Rc<[PreviewDiagnosticFrame]>,
    height: f32,
    editor: Option<EditorNavigation>,
}

impl PreviewDiagnostic {
    #[must_use]
    pub fn new(
        message: impl Into<String>,
        frames: impl IntoIterator<Item = PreviewDiagnosticFrame>,
        height: f32,
    ) -> Self {
        Self {
            message: message.into(),
            frames: frames.into_iter().collect(),
            height,
            editor: None,
        }
    }

    /// Mounts a fallible preview factory. A reported construction error is
    /// rendered in this slot rather than replacing or unmounting its peers.
    /// Returning a diagnostic does not catch panics or deferred layout errors.
    #[must_use]
    pub fn build_preview(factory: impl Fn() -> Result<Widget, Self> + 'static) -> Widget {
        Builder::new(move |_| factory().unwrap_or_else(Into::into)).into()
    }

    #[must_use]
    pub fn editor(
        mut self,
        available: ValueNotifier<bool>,
        navigate: impl Fn(&PreviewSourceLocation) + 'static,
    ) -> Self {
        self.editor = Some(EditorNavigation {
            available,
            open: Rc::new(navigate),
        });
        self
    }
}

fn frame_widget(frame: &PreviewDiagnosticFrame, editor: Option<&EditorNavigation>) -> Widget {
    let text = Text::new(frame.text.as_str());
    match (frame.source.as_ref(), editor) {
        (Some(source), Some(editor)) if source.is_linkable() => {
            let source = Rc::new(source.clone());
            let activate = {
                let source = source.clone();
                let editor = editor.clone();
                move || {
                    // Presses can end before the readiness rebuild is drawn.
                    if editor.available.value() {
                        (editor.open)(&source);
                    }
                }
            };
            Semantics::new(
                GestureDetector::new(text)
                    .behavior(HitTestBehavior::Opaque)
                    .on_tap(activate.clone()),
            )
            .role(Role::Link)
            .label(frame.text.as_str())
            .link_url(source.uri.as_str())
            .on_tap(activate)
            .into()
        }
        _ => text.into(),
    }
}

impl From<PreviewDiagnostic> for Widget {
    fn from(diagnostic: PreviewDiagnostic) -> Self {
        let frames = diagnostic.frames;
        let editor = diagnostic.editor;
        let trace = Builder::new(move |_| {
            let editor = editor.as_ref().filter(|editor| editor.available.value());
            Column::new(frames.iter().map(|frame| frame_widget(frame, editor)))
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .into()
        });
        let trace = Widget::selection_area(SelectionAreaController::new(), trace.into());
        let content = Column::new([
            Widget::from(Text::new(&format!(
                "Failed to initialize widget tree: {}",
                diagnostic.message
            ))),
            Text::new("Stacktrace:").into(),
            trace,
        ])
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start);
        SizedBox::new()
            .height(diagnostic.height)
            .child(SingleChildScrollView::new(content))
            .into()
    }
}
