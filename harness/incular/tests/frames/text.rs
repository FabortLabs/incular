use std::{cell::Cell, rc::Rc, time::Duration};

use incular_core::Size;
use incular_parity_harness::{FrameHarness, Scenario, probe, root_text_style};
use incular_text::{
    RichText, TextEditingController, TextEditingValue, TextSpan, TextStyle, WidgetSpan,
    WidgetSpanAlignment,
};
use incular_widgets::{Align, Alignment, DefaultTextStyle, EditableText, SizedBox, Text};

pub fn scenarios() -> Vec<Scenario> {
    vec![
        ("text__rich_span_style_metrics", rich_span_style_metrics),
        ("text__widget_span_layout", widget_span_layout),
        ("text__editing_selection_offsets", editing_selection_offsets),
        (
            "text__default_text_style_max_lines",
            default_text_style_max_lines,
        ),
        (
            "text__editable_initial_selection_focus",
            editable_initial_selection_focus,
        ),
        (
            "text__editable_programmatic_controller_update",
            editable_programmatic_controller_update,
        ),
        (
            "text__editable_long_press_word_selection",
            editable_long_press_word_selection,
        ),
    ]
}

fn rich_span_style_metrics(h: &mut FrameHarness) {
    h.mount(|_| {
        let span = TextSpan::new("Small ")
            .style(root_text_style())
            .child(TextSpan::new("Large").style(TextStyle::new().size(30.)));
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new()
                .width(220.)
                .child(probe("paragraph", RichText::new(span))),
        )
        .into()
    });
    h.frames(1);
}

fn widget_span_layout(h: &mut FrameHarness) {
    h.mount(|_| {
        let span = TextSpan::new("before ")
            .style(root_text_style())
            .child(WidgetSpan::sized(7, Size::new(40., 40.)).alignment(WidgetSpanAlignment::Middle))
            .child(TextSpan::new(" after"));
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new()
                .width(220.)
                .child(probe("paragraph", RichText::new(span))),
        )
        .into()
    });
    h.frames(1);
}

fn editing_selection_offsets(h: &mut FrameHarness) {
    let controller = TextEditingController::from_value(TextEditingValue::new("😀x"));
    let text_read = controller.clone();
    let selection_read = controller.clone();
    h.value("text_length", move || text_read.text().len() as f64);
    h.value("selection_extent", move || {
        selection_read.selection().extent as f64
    });

    h.mount(move |_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(200.).height(48.).child(probe(
                "field",
                EditableText::new(controller.clone())
                    .size(Size::new(200., 48.))
                    .style(root_text_style()),
            )),
        )
        .into()
    });
    h.frames(1);
}

fn default_text_style_max_lines(h: &mut FrameHarness) {
    h.mount(|_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(220.).child(DefaultTextStyle::new(
                root_text_style(),
                probe("paragraph", Text::new("first\nsecond")),
            )),
        )
        .into()
    });
    h.frames(1);
}

fn editable_initial_selection_focus(h: &mut FrameHarness) {
    let controller = TextEditingController::with_text("abc");
    let text_read = controller.clone();
    let selection_read = controller.clone();
    h.value("selection_valid", move || {
        let text = text_read.text();
        let selection = text_read.selection();
        if selection.base <= text.len() && selection.extent <= text.len() {
            1.0
        } else {
            0.0
        }
    });
    h.value("selection_collapsed", move || {
        if selection_read.selection().is_collapsed() {
            1.0
        } else {
            0.0
        }
    });
    let extent_read = controller.clone();
    h.value("selection_extent", move || {
        extent_read.selection().extent as f64
    });

    h.mount(move |_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(200.).height(48.).child(probe(
                "field",
                EditableText::new(controller.clone())
                    .size(Size::new(200., 48.))
                    .style(root_text_style()),
            )),
        )
        .into()
    });
    h.frames(1);
    let center = h.center("field");
    h.down(center);
    h.up();
    h.frames(1);
}

fn editable_programmatic_controller_update(h: &mut FrameHarness) {
    let controller = TextEditingController::new();
    let changed_calls = Rc::new(Cell::new(0_u32));
    let text_read = controller.clone();
    let changed_read = changed_calls.clone();
    h.value("text_length", move || text_read.text().len() as f64);
    h.value("changed_calls", move || f64::from(changed_read.get()));

    let field_controller = controller.clone();
    h.mount(move |_| {
        let changed = changed_calls.clone();
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(200.).height(48.).child(probe(
                "field",
                EditableText::new(field_controller.clone())
                    .size(Size::new(200., 48.))
                    .style(root_text_style())
                    .edit_transform(|_, next| {
                        TextEditingValue::new(
                            next.text
                                .chars()
                                .filter(|character| character.is_ascii_digit())
                                .collect::<String>(),
                        )
                    })
                    .on_changed(move |_| changed.set(changed.get() + 1)),
            )),
        )
        .into()
    });
    h.frames(1);
    controller.set_text("a1b2");
    h.frames(1);
}

fn editable_long_press_word_selection(h: &mut FrameHarness) {
    let controller = TextEditingController::with_text("hello world");
    let start_read = controller.clone();
    let end_read = controller.clone();
    let collapsed_read = controller.clone();
    h.value("selection_start", move || {
        start_read.selection().start() as f64
    });
    h.value("selection_end", move || end_read.selection().end() as f64);
    h.value("selection_collapsed", move || {
        if collapsed_read.selection().is_collapsed() {
            1.0
        } else {
            0.0
        }
    });

    h.mount(move |_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(200.).height(48.).child(probe(
                "field",
                EditableText::new(controller.clone())
                    .size(Size::new(200., 48.))
                    .style(root_text_style()),
            )),
        )
        .into()
    });
    h.frames(1);
    let point = incular_core::Offset::new(16., 24.);
    h.down(point);
    h.pump(Duration::from_millis(600));
    h.up();
    h.frames(1);
}
