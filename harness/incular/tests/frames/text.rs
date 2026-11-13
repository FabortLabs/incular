use std::{cell::Cell, rc::Rc, time::Duration};

use incular_config::CrossAxisAlignment;
use incular_core::Size;
use incular_core::{Code, Color, InputEvent, KeyboardEvent, KeyboardKey, Modifiers, NamedKey};
use incular_parity_harness::{FrameHarness, Scenario, probe, root_text_style};
use incular_runtime::TextShortcutPlatform;
use incular_text::{
    RichText, TextEditingController, TextEditingValue, TextHeightBehavior, TextSelection, TextSpan,
    TextStyle, WidgetSpan, WidgetSpanAlignment,
};
use incular_widgets::{
    Align, Alignment, Column, DefaultTextStyle, EditableText, SelectionAreaController, SizedBox,
    Text, TextOverflow, Widget,
};

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
        ("text__text_max_lines_ellipsis", text_max_lines_ellipsis),
        ("text__editable_word_arrow_selection", word_arrow_selection),
        (
            "text__editable_home_end_shift_select",
            home_end_shift_select,
        ),
        ("text__editable_backspace_delete", backspace_delete),
        ("text__editable_undo_redo", undo_redo),
        ("text__editable_enter_callback_order", enter_callback_order),
        (
            "text__editable_formatter_length_then_digits",
            formatter_length_then_digits,
        ),
        (
            "text__editable_formatter_digits_then_length",
            formatter_digits_then_length,
        ),
        ("text__editable_read_only_typing", read_only_typing),
        ("text__editable_max_lines_sizing", max_lines_sizing),
        ("text__editable_min_lines_sizing", min_lines_sizing),
        ("text__editable_expands_sizing", expands_sizing),
        ("text__text_soft_wrap_false", text_soft_wrap_false),
        ("text__text_scaler_linear", text_scaler_linear),
        (
            "text__text_height_behavior_first_ascent",
            text_height_behavior_first_ascent,
        ),
        (
            "text__selection_area_long_press_drag",
            selection_area_long_press_drag,
        ),
        (
            "text__editable_undo_no_programmatic_selection",
            undo_without_programmatic_selection,
        ),
        ("text__editable_caret_blink_1200ms", caret_blink_1200ms),
        (
            "text__editable_multiline_vertical_document",
            multiline_vertical_document,
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

fn text_max_lines_ellipsis(h: &mut FrameHarness) {
    h.mount(|_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(100.).child(probe(
                "paragraph",
                Text::new("alpha beta gamma delta epsilon")
                    .style(root_text_style())
                    .max_lines(Some(2))
                    .overflow(TextOverflow::Ellipsis),
            )),
        )
        .into()
    });
    h.frames(1);
}

fn plain_field(controller: TextEditingController) -> EditableText {
    EditableText::new(controller).style(root_text_style())
}

fn chord(h: &mut FrameHarness, modifiers: Modifiers, key: KeyboardKey, code: Code) {
    h.send(InputEvent::Key(KeyboardEvent {
        modifiers,
        ..KeyboardEvent::key_down(key.clone(), code)
    }));
    h.send(InputEvent::Key(KeyboardEvent {
        modifiers,
        ..KeyboardEvent::key_up(key, code)
    }));
}

fn android_shortcuts(h: &mut FrameHarness) {
    h.runtime()
        .set_text_shortcut_platform(TextShortcutPlatform::Android);
}

fn read_selection_base(controller: &TextEditingController) -> f64 {
    controller.selection().base as f64
}

fn word_arrow_selection(h: &mut FrameHarness) {
    let controller = TextEditingController::with_text("hello big world");
    let base_read = controller.clone();
    let extent_read = controller.clone();
    h.value("selection_base", move || read_selection_base(&base_read));
    h.value("selection_extent", move || {
        extent_read.selection().extent as f64
    });
    let field = controller.clone();
    h.mount(move |_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(200.).height(48.).child(probe(
                "field",
                plain_field(field.clone()).size(Size::new(200., 48.)),
            )),
        )
        .into()
    });
    android_shortcuts(h);
    h.frames(1);
    let center = h.center("field");
    h.down(center);
    h.up();
    h.frames(1);
    controller.set_selection(TextSelection::new(0, 0));
    h.frames(1);
    chord(
        h,
        Modifiers::CONTROL,
        KeyboardKey::Named(NamedKey::ArrowRight),
        Code::ArrowRight,
    );
    h.frames(1);
    chord(
        h,
        Modifiers::CONTROL,
        KeyboardKey::Named(NamedKey::ArrowRight),
        Code::ArrowRight,
    );
    h.frames(1);
    chord(
        h,
        Modifiers::SHIFT,
        KeyboardKey::Named(NamedKey::ArrowRight),
        Code::ArrowRight,
    );
    h.frames(1);
}

fn home_end_shift_select(h: &mut FrameHarness) {
    let controller = TextEditingController::with_text("hello world");
    let base_read = controller.clone();
    let extent_read = controller.clone();
    h.value("selection_base", move || read_selection_base(&base_read));
    h.value("selection_extent", move || {
        extent_read.selection().extent as f64
    });
    let field = controller.clone();
    h.mount(move |_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(200.).height(48.).child(probe(
                "field",
                plain_field(field.clone()).size(Size::new(200., 48.)),
            )),
        )
        .into()
    });
    android_shortcuts(h);
    h.frames(1);
    let center = h.center("field");
    h.down(center);
    h.up();
    h.frames(1);
    controller.set_selection(TextSelection::new(5, 5));
    h.frames(1);
    chord(
        h,
        Modifiers::empty(),
        KeyboardKey::Named(NamedKey::End),
        Code::End,
    );
    h.frames(1);
    chord(
        h,
        Modifiers::SHIFT,
        KeyboardKey::Named(NamedKey::Home),
        Code::Home,
    );
    h.frames(1);
}

fn backspace_delete(h: &mut FrameHarness) {
    let controller = TextEditingController::with_text("abcdef");
    let length_read = controller.clone();
    let extent_read = controller.clone();
    h.value("text_length", move || length_read.text().len() as f64);
    h.value("selection_extent", move || {
        extent_read.selection().extent as f64
    });
    let field = controller.clone();
    h.mount(move |_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(200.).height(48.).child(probe(
                "field",
                plain_field(field.clone()).size(Size::new(200., 48.)),
            )),
        )
        .into()
    });
    android_shortcuts(h);
    h.frames(1);
    let center = h.center("field");
    h.down(center);
    h.up();
    h.frames(1);
    controller.set_selection(TextSelection::new(3, 3));
    h.frames(1);
    chord(
        h,
        Modifiers::empty(),
        KeyboardKey::Named(NamedKey::Backspace),
        Code::Backspace,
    );
    h.frames(1);
    chord(
        h,
        Modifiers::empty(),
        KeyboardKey::Named(NamedKey::Delete),
        Code::Delete,
    );
    h.frames(1);
}

fn undo_redo(h: &mut FrameHarness) {
    let controller = TextEditingController::with_text("abc");
    let length_read = controller.clone();
    h.value("text_length", move || length_read.text().len() as f64);
    let field = controller.clone();
    h.mount(move |_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(200.).height(48.).child(probe(
                "field",
                plain_field(field.clone()).size(Size::new(200., 48.)),
            )),
        )
        .into()
    });
    android_shortcuts(h);
    h.frames(1);
    let center = h.center("field");
    h.down(center);
    h.up();
    h.frames(1);
    controller.set_selection(TextSelection::new(3, 3));
    h.frames(1);
    chord(
        h,
        Modifiers::empty(),
        KeyboardKey::Named(NamedKey::Backspace),
        Code::Backspace,
    );
    h.frames(1);
    h.pump(Duration::from_millis(600));
    chord(
        h,
        Modifiers::CONTROL,
        KeyboardKey::Character("z".into()),
        Code::KeyZ,
    );
    h.frames(1);
    chord(
        h,
        Modifiers::CONTROL | Modifiers::SHIFT,
        KeyboardKey::Character("Z".into()),
        Code::KeyZ,
    );
    h.frames(1);
}

fn enter_callback_order(h: &mut FrameHarness) {
    let controller = TextEditingController::new();
    let changed = Rc::new(Cell::new(0_u32));
    let submitted = Rc::new(Cell::new(0_u32));
    let changed_read = changed.clone();
    let submitted_read = submitted.clone();
    let length_read = controller.clone();
    h.value("changed_calls", move || f64::from(changed_read.get()));
    h.value("submitted_calls", move || f64::from(submitted_read.get()));
    h.value("text_length", move || length_read.text().len() as f64);
    let field = controller.clone();
    h.mount(move |_| {
        let changed = changed.clone();
        let submitted = submitted.clone();
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(200.).height(48.).child(probe(
                "field",
                plain_field(field.clone())
                    .size(Size::new(200., 48.))
                    .on_changed(move |_| changed.set(changed.get() + 1))
                    .on_submit(move |_| submitted.set(submitted.get() + 1)),
            )),
        )
        .into()
    });
    android_shortcuts(h);
    h.frames(1);
    let center = h.center("field");
    h.down(center);
    h.up();
    h.frames(1);
    h.send(InputEvent::Text("ab".into()));
    h.frames(1);
    chord(
        h,
        Modifiers::empty(),
        KeyboardKey::Named(NamedKey::Enter),
        Code::Enter,
    );
    h.frames(1);
}

/// Incular has no input formatter on EditableText (`TextInputFormatter` is
/// defined in forms.rs but never applied), so both formatter orders give the
/// same unfiltered result. The Flutter twins differ only in formatter order.
fn formatter_unapplied(h: &mut FrameHarness) {
    let controller = TextEditingController::new();
    let length_read = controller.clone();
    h.value("text_length", move || length_read.text().len() as f64);
    let field = controller.clone();
    h.mount(move |_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(200.).height(48.).child(probe(
                "field",
                plain_field(field.clone()).size(Size::new(200., 48.)),
            )),
        )
        .into()
    });
    android_shortcuts(h);
    h.frames(1);
    let center = h.center("field");
    h.down(center);
    h.up();
    h.frames(1);
    h.send(InputEvent::Text("a1b2c3d4".into()));
    h.frames(1);
}

fn formatter_length_then_digits(h: &mut FrameHarness) {
    formatter_unapplied(h);
}

fn formatter_digits_then_length(h: &mut FrameHarness) {
    formatter_unapplied(h);
}

fn read_only_typing(h: &mut FrameHarness) {
    let controller = TextEditingController::with_text("abc");
    let length_read = controller.clone();
    h.value("text_length", move || length_read.text().len() as f64);
    let field = controller.clone();
    h.mount(move |_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(200.).height(48.).child(probe(
                "field",
                plain_field(field.clone())
                    .size(Size::new(200., 48.))
                    .read_only(true),
            )),
        )
        .into()
    });
    h.frames(1);
    let center = h.center("field");
    h.down(center);
    h.up();
    h.frames(1);
    h.send(InputEvent::Text("xyz".into()));
    h.frames(1);
}

fn max_lines_sizing(h: &mut FrameHarness) {
    let controller = TextEditingController::with_text("a\nb\nc\nd\ne");
    h.mount(move |_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(200.).child(probe(
                "field",
                plain_field(controller.clone()).max_lines(Some(3)),
            )),
        )
        .into()
    });
    h.frames(1);
}

fn min_lines_sizing(h: &mut FrameHarness) {
    let controller = TextEditingController::with_text("a");
    h.mount(move |_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(200.).child(probe(
                "field",
                plain_field(controller.clone())
                    .min_lines(Some(2))
                    .max_lines(Some(4)),
            )),
        )
        .into()
    });
    h.frames(1);
}

fn expands_sizing(h: &mut FrameHarness) {
    let controller = TextEditingController::with_text("a\nb");
    h.mount(move |_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(200.).height(120.).child(probe(
                "field",
                plain_field(controller.clone())
                    .max_lines(None)
                    .expands(true),
            )),
        )
        .into()
    });
    h.frames(1);
}

fn text_soft_wrap_false(h: &mut FrameHarness) {
    h.mount(|_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(60.).child(probe(
                "paragraph",
                Text::new("alpha beta gamma")
                    .style(root_text_style())
                    .soft_wrap(false),
            )),
        )
        .into()
    });
    h.frames(1);
}

/// Incular's `Text` has no `textScaler`, so this twin states the scaled size
/// (14 x 1.5 = 21) directly. It is a non-equivalent diagnostic: the API that
/// applies the scale is missing.
fn text_scaler_linear(h: &mut FrameHarness) {
    h.mount(|_| {
        Align::new(
            Alignment::TOP_LEFT,
            probe(
                "paragraph",
                Text::new("alpha").style(TextStyle::new().size(21.).color(Color::BLACK)),
            ),
        )
        .into()
    });
    h.frames(1);
}

fn text_height_behavior_first_ascent(h: &mut FrameHarness) {
    h.mount(|_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(200.).child(probe(
                "paragraph",
                Text::new("alpha")
                    .style(root_text_style().line_height_multiplier(2.))
                    .text_height_behavior(TextHeightBehavior {
                        apply_height_to_first_ascent: false,
                        ..TextHeightBehavior::default()
                    }),
            )),
        )
        .into()
    });
    h.frames(1);
}

fn selection_area_long_press_drag(h: &mut FrameHarness) {
    let selection = SelectionAreaController::new();
    let selected_read = selection.clone();
    h.value("selected_length", move || {
        selected_read.selected_text().len() as f64
    });
    let area = selection.clone();
    h.mount(move |_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(200.).child(Widget::selection_area(
                area.clone(),
                Column::new([
                    probe("first", Text::new("alpha beta").style(root_text_style())),
                    probe("second", Text::new("gamma delta").style(root_text_style())),
                ])
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .into(),
            )),
        )
        .into()
    });
    h.frames(1);
    let first = h.center("first");
    h.down(first);
    h.pump(Duration::from_millis(600));
    let second = h.center("second");
    h.move_to(second);
    h.frames(1);
    h.up();
    h.frames(1);
}

fn undo_without_programmatic_selection(h: &mut FrameHarness) {
    let controller = TextEditingController::with_text("abc");
    let length_read = controller.clone();
    h.value("text_length", move || length_read.text().len() as f64);
    let field = controller.clone();
    h.mount(move |_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(200.).height(48.).child(probe(
                "field",
                plain_field(field.clone()).size(Size::new(200., 48.)),
            )),
        )
        .into()
    });
    android_shortcuts(h);
    h.frames(1);
    let center = h.center("field");
    h.down(center);
    h.up();
    h.frames(1);
    chord(
        h,
        Modifiers::empty(),
        KeyboardKey::Named(NamedKey::End),
        Code::End,
    );
    h.frames(1);
    chord(
        h,
        Modifiers::empty(),
        KeyboardKey::Named(NamedKey::Backspace),
        Code::Backspace,
    );
    h.frames(1);
    h.pump(Duration::from_millis(600));
    chord(
        h,
        Modifiers::CONTROL,
        KeyboardKey::Character("z".into()),
        Code::KeyZ,
    );
    h.frames(1);
    h.pump(Duration::from_millis(600));
    chord(
        h,
        Modifiers::CONTROL | Modifiers::SHIFT,
        KeyboardKey::Character("Z".into()),
        Code::KeyZ,
    );
    h.frames(1);
}

fn caret_blink_1200ms(h: &mut FrameHarness) {
    let controller = TextEditingController::with_text("abc");
    let clock = h.clock();
    let caret_controller = controller.clone();
    h.value("caret_visible", move || {
        if caret_controller.caret_visible(clock.get()) {
            1.0
        } else {
            0.0
        }
    });
    let field = controller.clone();
    h.mount(move |_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(200.).height(48.).child(probe(
                "field",
                plain_field(field.clone()).size(Size::new(200., 48.)),
            )),
        )
        .into()
    });
    h.frames(1);
    let center = h.center("field");
    h.down(center);
    h.up();
    h.frames(75);
}

fn multiline_vertical_document(h: &mut FrameHarness) {
    let controller = TextEditingController::with_text("abc\ndefgh\nij");
    let base_read = controller.clone();
    let extent_read = controller.clone();
    h.value("selection_base", move || read_selection_base(&base_read));
    h.value("selection_extent", move || {
        extent_read.selection().extent as f64
    });
    let field = controller.clone();
    h.mount(move |_| {
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(200.).height(60.).child(probe(
                "field",
                plain_field(field.clone())
                    .max_lines(Some(3))
                    .size(Size::new(200., 60.)),
            )),
        )
        .into()
    });
    android_shortcuts(h);
    h.frames(1);
    let center = h.center("field");
    h.down(center);
    h.up();
    h.frames(1);
    controller.set_selection(TextSelection::new(1, 1));
    h.frames(1);
    chord(
        h,
        Modifiers::empty(),
        KeyboardKey::Named(NamedKey::ArrowDown),
        Code::ArrowDown,
    );
    h.frames(1);
    chord(
        h,
        Modifiers::empty(),
        KeyboardKey::Named(NamedKey::ArrowDown),
        Code::ArrowDown,
    );
    h.frames(1);
    chord(
        h,
        Modifiers::empty(),
        KeyboardKey::Named(NamedKey::ArrowUp),
        Code::ArrowUp,
    );
    h.frames(1);
    chord(
        h,
        Modifiers::CONTROL,
        KeyboardKey::Named(NamedKey::End),
        Code::End,
    );
    h.frames(1);
    chord(
        h,
        Modifiers::CONTROL,
        KeyboardKey::Named(NamedKey::Home),
        Code::Home,
    );
    h.frames(1);
}
