use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Rect, Size};
use incular_rendering::{DisplayList, PaintCommand};
use incular_runtime::Runtime;
use incular_text::{
    ComposingRange, TextEditingController, TextEditingValue, TextSelection, TextStyle,
};
use incular_widgets::{EditableText, FocusNode};

const INK: Color = Color::rgba(27, 153, 84, 255);
const CONTENT: &str = "text composing text";

fn mount(
    read_only: bool,
    obscure: bool,
    autofocus: bool,
) -> (Runtime, FocusNode, TextEditingController) {
    let editing = TextEditingController::new();
    editing.set_value(TextEditingValue {
        text: CONTENT.to_owned(),
        selection: TextSelection::collapsed(14),
        composing: Some(ComposingRange::new(5, 14)),
    });
    let node = FocusNode::new();
    let field = EditableText::new(editing.clone())
        .focus_node(node.clone())
        .autofocus(autofocus)
        .style(TextStyle::new().font_size(14.).color(INK))
        .show_cursor(false)
        .read_only(read_only)
        .obscure_text(obscure)
        .text_direction(TextDirection::Ltr)
        .on_editing_complete(|| {});
    (Runtime::new(field.into()).unwrap(), node, editing)
}

fn frame(runtime: &mut Runtime) -> DisplayList {
    runtime
        .run_frame(Constraints::tight(Size::new(350., 50.)))
        .unwrap()
        .0
}

fn underlines(list: &DisplayList) -> Vec<Rect> {
    list.commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Rect { rect, color } if *color == INK => Some(*rect),
            _ => None,
        })
        .collect()
}

fn glyphs(list: &DisplayList) -> usize {
    list.commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::GlyphRun { run, color } if *color == INK => Some(run.glyphs.len()),
            _ => None,
        })
        .sum()
}

#[test]
fn mounted_editor_underlines_only_the_composing_range_and_clears_it_on_blur() {
    let (mut runtime, node, editing) = mount(false, false, true);
    let focused = frame(&mut runtime);
    assert!(node.has_primary_focus());
    let tree = runtime.tree();
    let layout = tree
        .text_layout(tree.render_id(tree.root().unwrap()).unwrap())
        .unwrap();
    let [(left, right)] = layout.lines[0].selection_spans(5, 14)[..] else {
        panic!("expected one visual composing range");
    };
    let lines = underlines(&focused);
    let [line] = lines.as_slice() else {
        panic!("expected one composing underline, got {lines:?}");
    };
    assert_eq!(line.origin.x, left);
    assert_eq!(line.size.width, right - left);
    assert!(line.origin.y > layout.lines[0].baseline);
    assert_eq!(line.size.height, 1.);

    node.unfocus();
    let blurred = frame(&mut runtime);
    assert!(!node.has_primary_focus());
    assert!(underlines(&blurred).is_empty());
    assert_eq!(glyphs(&blurred), glyphs(&focused));
    assert_eq!(editing.value().text, CONTENT);
}

#[test]
fn composing_underline_is_suppressed_for_unfocused_read_only_and_obscured_editors() {
    for (read_only, obscure, autofocus) in [
        (false, false, false),
        (true, false, true),
        (false, true, true),
    ] {
        let (mut runtime, node, _) = mount(read_only, obscure, autofocus);
        let display = frame(&mut runtime);
        assert_eq!(node.has_primary_focus(), autofocus);
        assert!(underlines(&display).is_empty());
    }
}

#[test]
fn transient_host_preedit_is_underlined_without_committing_it_to_the_controller() {
    use incular_core::{ImeEvent, InputEvent};

    let editing = TextEditingController::new();
    let node = FocusNode::new();
    let mut runtime = Runtime::new(
        EditableText::new(editing.clone())
            .focus_node(node)
            .autofocus(true)
            .show_cursor(false)
            .style(TextStyle::new().font_size(14.).color(INK))
            .text_direction(TextDirection::Ltr)
            .into(),
    )
    .unwrap();
    assert!(underlines(&frame(&mut runtime)).is_empty());
    let _ = runtime.handle_input(InputEvent::Ime(ImeEvent::Preedit {
        text: "λ界".to_owned(),
        selection: None,
    }));
    let preedit = frame(&mut runtime);
    assert_eq!(editing.preedit().as_deref(), Some("λ界"));
    assert!(editing.text().is_empty());
    assert_eq!(underlines(&preedit).len(), 1);
    assert_eq!(glyphs(&preedit), 2);

    let _ = runtime.handle_input(InputEvent::Ime(ImeEvent::Commit("λ界".to_owned())));
    let committed = frame(&mut runtime);
    assert!(underlines(&committed).is_empty());
    assert_eq!(editing.text(), "λ界");
    assert_eq!(glyphs(&committed), 2);
}

#[test]
fn a_composing_range_can_begin_inside_a_combining_grapheme() {
    let (mut runtime, node, editing) = mount(false, false, true);
    let _ = frame(&mut runtime);
    assert!(node.has_primary_focus());
    // Flutter's UTF-16 [1, 2) is the combining mark after ASCII 'A'.
    // The same scalar occupies UTF-8 bytes [1, 3).
    let value = TextEditingValue {
        text: "A\u{301}".to_owned(),
        selection: TextSelection { base: 1, extent: 3 },
        composing: Some(ComposingRange::new(1, 3)),
    };
    editing.set_value(value.clone());
    let display = frame(&mut runtime);
    assert!(glyphs(&display) > 0);
    assert_eq!(editing.value(), value);
    assert!(node.has_primary_focus());
}

#[test]
fn first_frame_autofocus_handles_a_preexisting_upstream_caret_in_a_multiline_editor() {
    use incular_text::{TextAffinity, TextAlign};
    use incular_widgets::SizedBox;

    let sample = "Now is the time for\nall good people\nto come to the aid\nof their country.";
    let editing = TextEditingController::with_text(sample);
    editing.set_selection_with_affinity(TextSelection::collapsed(0), TextAffinity::Upstream);
    let node = FocusNode::new();
    let scene = EditableText::new(editing.clone())
        .focus_node(node.clone())
        .autofocus(true)
        .select_all_on_focus(false)
        .text_direction(TextDirection::Ltr)
        .text_align(TextAlign::Right)
        .style(TextStyle::new().font_size(16.).color(INK))
        .cursor_color(Color::rgba(50, 100, 230, 255))
        .max_lines(Some(10))
        .selection_handles_builder(|_, _| SizedBox::shrink().into());
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let display = frame(&mut runtime);
    assert!(node.has_primary_focus());
    assert!(glyphs(&display) > 0);
    assert_eq!(
        editing.selection_if_set(),
        Some(TextSelection::collapsed(0))
    );
    assert_eq!(editing.caret_affinity(), TextAffinity::Upstream);
    let focused = runtime.focused_element().unwrap();
    assert!(
        runtime
            .tree()
            .text_field_selection_handle_geometry(focused)
            .is_some()
    );
    assert_eq!(editing.text(), sample);
}
