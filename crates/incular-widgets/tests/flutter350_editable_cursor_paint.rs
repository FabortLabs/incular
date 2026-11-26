//! Flutter's RenderEditable cursor and selection paint ordering.
use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Size, time::Instant};
use incular_rendering::PaintCommand;
use incular_text::{TextAffinity, TextEditingController, TextSelection};
use incular_widgets::{
    EditableText, Widget,
    internal::{ElementId, WidgetTree},
};

const RED: Color = Color::rgba(220, 18, 8, 255);
const VIEW: Size = Size {
    width: 200.,
    height: 50.,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Painted {
    Caret,
    Selection,
    Text,
}

struct Editor {
    tree: WidgetTree,
    root: ElementId,
    controller: TextEditingController,
}

fn field(controller: &TextEditingController, above: bool) -> Widget {
    EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .cursor_color(RED)
        .selection_color(Color::BLACK)
        .show_cursor(true)
        .paint_cursor_above_text(above)
        .into()
}

impl Editor {
    fn new(selection: TextSelection, above: bool) -> Self {
        let controller = TextEditingController::with_text("test");
        controller.set_selection_with_affinity(selection, TextAffinity::Upstream);
        let mut tree = WidgetTree::new();
        let root = tree.mount(field(&controller, above)).unwrap();
        tree.layout(Constraints::tight(VIEW)).unwrap();
        tree.set_focused(root, true, Instant::now()).unwrap();
        Self {
            tree,
            root,
            controller,
        }
    }

    fn set_above(&mut self, above: bool) {
        self.tree
            .update(self.root, field(&self.controller, above))
            .unwrap();
    }

    fn paint(&mut self) -> Vec<Painted> {
        self.tree.layout(Constraints::tight(VIEW)).unwrap();
        self.controller.reset_caret(Instant::now());
        self.tree
            .paint()
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::Rect { color, .. } if *color == RED => Some(Painted::Caret),
                PaintCommand::Rect { color, .. } if *color == Color::BLACK => {
                    Some(Painted::Selection)
                }
                PaintCommand::GlyphRun { .. } => Some(Painted::Text),
                _ => None,
            })
            .collect()
    }
}

#[test]
fn selected_text_draws_above_its_only_rectangle_not_a_caret() {
    let mut editor = Editor::new(TextSelection::new(0, 3), false);
    assert_eq!(editor.paint(), [Painted::Selection, Painted::Text]);
}

#[test]
fn a_retained_cursor_can_switch_between_painting_above_and_below_its_text() {
    let mut editor = Editor::new(TextSelection::collapsed(2), true);
    assert_eq!(editor.paint(), [Painted::Text, Painted::Caret]);

    editor.set_above(false);
    assert_eq!(editor.paint(), [Painted::Caret, Painted::Text]);
    assert_eq!(editor.controller.selection(), TextSelection::collapsed(2));
}

#[test]
fn a_null_or_invalid_selection_does_not_paint_a_caret() {
    let mut editor = Editor::new(TextSelection::collapsed(2), true);
    assert_eq!(editor.paint(), [Painted::Text, Painted::Caret]);

    editor.controller.clear_selection();
    assert!(editor.controller.selection_if_set().is_none());
    assert_eq!(editor.paint(), [Painted::Text]);

    editor.controller.set_selection(TextSelection::collapsed(0));
    assert_eq!(editor.paint(), [Painted::Text, Painted::Caret]);
    editor.controller.set_text("test");
    assert!(editor.controller.selection_if_set().is_none());
    assert_eq!(editor.paint(), [Painted::Text]);
}
