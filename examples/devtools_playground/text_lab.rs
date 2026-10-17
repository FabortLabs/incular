//! Section 6: text of several shapes for the text-cache diagnostics.
use super::theme::*;
use incular::controls_prelude::{TextArea, TextField};
use incular::prelude::*;
use incular::text::TextEditingController;

const LONG: &str = "Incular lays text out once, caches the shaped runs, and reuses them while the \
string, style and width stay the same. This paragraph is deliberately long so it wraps over \
several lines; resize the window to force new layouts and watch the text cache miss counters \
move in the DevTools Rendering view, then settle back to hits.";

#[derive(Clone)]
pub struct TextLab {
    pub single: TextEditingController,
    pub multi: TextEditingController,
}

impl TextLab {
    pub fn new() -> Self {
        Self {
            single: TextEditingController::with_text("Type here"),
            multi: TextEditingController::with_text("Multi-line\nnotes go here"),
        }
    }
}

pub fn view(lab: &TextLab) -> Widget {
    let paragraphs = card(
        "Paragraphs",
        "Several short blocks.",
        420.,
        Column::new([
            body("The quick brown fox jumps over the lazy dog."),
            gap(6.),
            body("Pack my box with five dozen liquor jugs."),
            gap(6.),
            body("Sphinx of black quartz, judge my vow."),
            gap(6.),
            muted("Ligatures: office, affine, waffle. Numerals: 0123456789."),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start),
    );

    let wrapping = card(
        "Long wrapping paragraph",
        "Soft-wrapped inside a 380 px column.",
        420.,
        SizedBox::new().width(380.).child(
            Text::new(LONG).style(
                TextStyle::new()
                    .font_size(14.)
                    .line_height_multiplier(1.45)
                    .color(INK),
            ),
        ),
    );

    let sizes = card(
        "Mixed font sizes",
        "Each size is its own cache entry.",
        420.,
        Column::new(
            [11., 14., 18., 24., 32., 44.]
                .into_iter()
                .map(|size| text(format!("Size {size}"), size, FontWeight::W600, INK))
                .collect::<Vec<Widget>>(),
        )
        .cross_axis_alignment(CrossAxisAlignment::Start),
    );

    let fields = card(
        "Text fields",
        "Editing re-lays out text on every keystroke.",
        420.,
        Column::new([
            Widget::from(TextField::new(lab.single.clone()).placeholder("Single line"))
                .accessibility_label("Playground text field"),
            gap(10.),
            Widget::from(TextArea::new(lab.multi.clone()).placeholder("Multi-line"))
                .accessibility_label("Playground text area"),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Stretch),
    );

    page(
        "Text",
        "Shape, wrap and edit text, then read the text-cache hit rate in Rendering.",
        vec![paragraphs, wrapping, sizes, fields],
    )
}
