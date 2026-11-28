use std::{cell::Cell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size, reactivity::DependencySource};
use incular_rendering::{DisplayList, PaintCommand};
use incular_text::{TextEditingController, TextStyle};
use incular_widgets::{
    EditableText, EditableTextPaintContext, EditableTextPainter,
    EditableTextPainterDelegate as Delegate, Widget, internal::WidgetTree,
};

const INK: Color = Color::rgba(27, 153, 84, 255);
const BACK: Color = Color::rgba(52, 86, 120, 18);
const FRONT: Color = Color::rgba(84, 96, 27, 120);
const VIEW: Size = Size {
    width: 100.,
    height: 36.,
};

struct Painter {
    color: Color,
    repaint: bool,
    calls: Rc<Cell<usize>>,
    context: Rc<Cell<Option<EditableTextPaintContext>>>,
    source: Option<DependencySource>,
}

impl EditableTextPainter for Painter {
    fn paint(&self, context: EditableTextPaintContext, canvas: &mut DisplayList) {
        self.calls.set(self.calls.get() + 1);
        self.context.set(Some(context));
        canvas.push(PaintCommand::Rect {
            rect: Rect::from_origin_size(Offset::new(1., 1.), Size::new(3., 3.)),
            color: self.color,
        });
    }

    fn should_repaint(&self, _previous: Option<&Self>) -> bool {
        self.repaint
    }

    fn repaint_source(&self) -> Option<DependencySource> {
        self.source.clone()
    }
}

fn painter(color: Color, repaint: bool, source: Option<DependencySource>) -> Painter {
    Painter {
        color,
        repaint,
        source,
        calls: Rc::default(),
        context: Rc::default(),
    }
}

fn field(
    controller: &TextEditingController,
    background: Option<Delegate>,
    foreground: Option<Delegate>,
    offset: Option<f32>,
) -> Widget {
    EditableText::new(controller.clone())
        .style(TextStyle::new().font_size(12.).color(INK))
        .text_direction(TextDirection::Ltr)
        .show_cursor(false)
        .fixed_scroll_offset(offset)
        .painter_delegates(background, foreground)
        .into()
}

fn editor() -> TextEditingController {
    let controller = TextEditingController::new();
    controller.set_text("test");
    controller
}

fn commands(tree: &mut WidgetTree) -> Vec<Color> {
    tree.paint()
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Rect { color, .. } if *color == BACK || *color == FRONT => Some(*color),
            PaintCommand::GlyphRun { color, .. } if *color == INK => Some(*color),
            _ => None,
        })
        .collect()
}

#[test]
fn actual_glyph_paint_is_between_the_editor_painters_and_the_slots_can_swap() {
    let editor = editor();
    let mut tree = WidgetTree::new();
    let bg = Delegate::new(painter(BACK, true, None));
    let fg = Delegate::new(painter(FRONT, true, None));
    let root = tree
        .mount(field(&editor, None, Some(fg.clone()), None))
        .unwrap();
    for (background, foreground, wanted) in [
        (None, Some(fg.clone()), vec![INK, FRONT]),
        (Some(bg.clone()), None, vec![BACK, INK]),
        (Some(bg.clone()), Some(fg.clone()), vec![BACK, INK, FRONT]),
        (Some(fg.clone()), Some(bg), vec![FRONT, INK, BACK]),
    ] {
        tree.update(root, field(&editor, background, foreground, None))
            .unwrap();
        tree.layout(Constraints::tight(VIEW)).unwrap();
        assert_eq!(commands(&mut tree), wanted);
    }
}

#[test]
fn replacing_each_slot_honors_should_repaint_and_updates_the_latest_delegate() {
    for foreground in [false, true] {
        let editor = editor();
        let mut tree = WidgetTree::new();
        let descriptor = |delegate: Delegate| {
            if foreground {
                field(&editor, None, Some(delegate), None)
            } else {
                field(&editor, Some(delegate), None, None)
            }
        };
        let before = painter(BACK, true, None);
        let first_calls = before.calls.clone();
        let root = tree.mount(descriptor(Delegate::new(before))).unwrap();
        tree.layout(Constraints::tight(VIEW)).unwrap();
        let prior = commands(&mut tree);
        assert_eq!(first_calls.get(), 1);
        let replacement = painter(FRONT, false, None);
        let quiet_calls = replacement.calls.clone();
        let saved = tree.diagnostics();
        tree.update(root, descriptor(Delegate::new(replacement)))
            .unwrap();
        tree.layout(Constraints::tight(VIEW)).unwrap();
        assert_eq!(commands(&mut tree), prior);
        assert_eq!(quiet_calls.get(), 0);
        assert_eq!(tree.diagnostics().layouts, saved.layouts);

        let next = painter(FRONT, true, None);
        let next_calls = next.calls.clone();
        tree.update(root, descriptor(Delegate::new(next))).unwrap();
        tree.layout(Constraints::tight(VIEW)).unwrap();
        assert!(commands(&mut tree).contains(&FRONT));
        assert_eq!(next_calls.get(), 1);
        assert_eq!(tree.diagnostics().layouts, saved.layouts);
    }
}

#[test]
fn same_painter_in_both_slots_paints_twice_in_order_and_can_share_a_signal() {
    let source = DependencySource::default();
    let editor = editor();
    let mut tree = WidgetTree::new();
    let shared = painter(BACK, true, Some(source.clone()));
    let calls = shared.calls.clone();
    let shared = Delegate::new(shared);
    let root = tree
        .mount(field(&editor, Some(shared.clone()), Some(shared), None))
        .unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(commands(&mut tree), [BACK, INK, BACK]);
    assert_eq!(calls.get(), 2);
    assert_eq!(source.subscriber_count(), 1);

    let before = tree.diagnostics();
    source.notify();
    assert_eq!(commands(&mut tree), [BACK, INK, BACK]);
    assert_eq!(calls.get(), 4);
    assert_eq!(tree.diagnostics().layouts, before.layouts);
    assert_eq!(tree.diagnostics().rebuilds, before.rebuilds);

    tree.update(root, field(&editor, None, None, None)).unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(commands(&mut tree), [INK]);
    assert_eq!(source.subscriber_count(), 0);
}

#[test]
fn auxiliary_paint_stays_in_the_viewport_when_the_text_has_scrolled() {
    let editor = editor();
    let mut tree = WidgetTree::new();
    let p = painter(BACK, true, None);
    let calls = p.calls.clone();
    let context = p.context.clone();
    let retained = Delegate::new(p);
    let root = tree
        .mount(field(&editor, Some(retained.clone()), None, Some(0.)))
        .unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(commands(&mut tree), [BACK, INK]);
    let before = context.get().unwrap();
    assert_eq!(before.scroll_offset, Offset::ZERO);
    tree.update(root, field(&editor, Some(retained), None, Some(1000.)))
        .unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    let list = tree.paint();
    assert!(list.commands().iter().any(|command| matches!(
        command,
        PaintCommand::Rect { rect, color }
            if *color == BACK && rect.origin == Offset::new(1., 1.)
    )));
    let after = context.get().unwrap();
    assert_eq!(after.size, before.size);
    assert_eq!(after.text_origin.x, before.text_origin.x - 1000.);
    assert_eq!(after.scroll_offset, Offset::new(1000., 0.));
    assert_eq!(calls.get(), 2);
}

#[test]
fn explicitly_repainting_the_editor_repaints_its_installed_painter() {
    let editor = editor();
    let mut tree = WidgetTree::new();
    let painter = painter(BACK, true, None);
    let calls = painter.calls.clone();
    let root = tree
        .mount(field(&editor, Some(Delegate::new(painter)), None, None))
        .unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(commands(&mut tree), [BACK, INK]);
    assert_eq!(calls.get(), 1);
    tree.mark_paint(root).unwrap();
    assert_eq!(commands(&mut tree), [BACK, INK]);
    assert_eq!(calls.get(), 2);
}

#[test]
fn changing_the_foreground_source_keeps_the_old_background_subscription() {
    let background_source = DependencySource::default();
    let old_foreground_source = DependencySource::default();
    let next_foreground_source = DependencySource::default();
    let editor = editor();
    let mut tree = WidgetTree::new();
    let background = painter(BACK, true, Some(background_source.clone()));
    let calls = background.calls.clone();
    let background = Delegate::new(background);
    let old = Delegate::new(painter(FRONT, true, Some(old_foreground_source.clone())));
    let new = Delegate::new(painter(FRONT, false, Some(next_foreground_source.clone())));
    let root = tree
        .mount(field(&editor, Some(background.clone()), Some(old), None))
        .unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(commands(&mut tree), [BACK, INK, FRONT]);
    tree.update(root, field(&editor, Some(background), Some(new), None))
        .unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(commands(&mut tree), [BACK, INK, FRONT]);
    assert_eq!(background_source.subscriber_count(), 1);
    assert_eq!(old_foreground_source.subscriber_count(), 0);
    assert_eq!(next_foreground_source.subscriber_count(), 1);
    assert_eq!(calls.get(), 1);
    old_foreground_source.notify();
    assert_eq!(commands(&mut tree), [BACK, INK, FRONT]);
    assert_eq!(calls.get(), 1);
    background_source.notify();
    assert_eq!(commands(&mut tree), [BACK, INK, FRONT]);
    assert_eq!(calls.get(), 2);
}

#[test]
fn painter_notification_updates_only_its_pass_and_does_not_paint_editor_text() {
    let source_bg = DependencySource::default();
    let source_fg = DependencySource::default();
    let editor = editor();
    let mut tree = WidgetTree::new();
    let bg = painter(BACK, true, Some(source_bg.clone()));
    let fg = painter(FRONT, true, Some(source_fg.clone()));
    let bg_calls = bg.calls.clone();
    let fg_calls = fg.calls.clone();
    let root = tree
        .mount(field(
            &editor,
            Some(Delegate::new(bg)),
            Some(Delegate::new(fg)),
            None,
        ))
        .unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(commands(&mut tree), [BACK, INK, FRONT]);

    let baseline = tree.diagnostics();
    source_bg.notify();
    assert_eq!(commands(&mut tree), [BACK, INK, FRONT]);
    assert_eq!((bg_calls.get(), fg_calls.get()), (2, 1));
    assert_eq!(tree.diagnostics().paints, baseline.paints);
    assert_eq!(tree.diagnostics().layouts, baseline.layouts);

    source_fg.notify();
    assert_eq!(commands(&mut tree), [BACK, INK, FRONT]);
    assert_eq!((bg_calls.get(), fg_calls.get()), (2, 2));
    assert_eq!(tree.diagnostics().paints, baseline.paints);

    // A normal editor repaint in the same frame has priority and records each
    // pass once, against the new editor picture.
    source_fg.notify();
    tree.mark_paint(root).unwrap();
    assert_eq!(commands(&mut tree), [BACK, INK, FRONT]);
    assert_eq!((bg_calls.get(), fg_calls.get()), (3, 3));
    assert_eq!(tree.diagnostics().paints, baseline.paints + 1);
    assert_eq!(commands(&mut tree), [BACK, INK, FRONT]);
    assert_eq!((bg_calls.get(), fg_calls.get()), (3, 3));
}

#[test]
fn aux_changes_advance_the_isolated_clip_generation_and_preserve_its_layer() {
    let source = DependencySource::default();
    let editor = editor();
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            EditableText::new(editor)
                .fixed_scroll_offset(1000.)
                .text_direction(TextDirection::Ltr)
                .clip_behavior(incular_config::Clip::AntiAliasWithSaveLayer)
                .show_cursor(false)
                .painter(painter(BACK, true, Some(source.clone())))
                .into(),
        )
        .unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    let clip = |tree: &mut WidgetTree| {
        tree.paint()
            .commands()
            .iter()
            .find_map(|command| match command {
                PaintCommand::PushClipRRect {
                    isolation: Some(isolation),
                    ..
                } => Some(*isolation),
                _ => None,
            })
            .expect("overflow has an isolated editor clip")
    };
    let first = clip(&mut tree);
    let previous = tree.diagnostics().paints;
    source.notify();
    let next = clip(&mut tree);
    assert_eq!(next.layer, first.layer);
    assert_eq!(next.generation, first.generation + 1);
    assert_eq!(tree.diagnostics().paints, previous);
    tree.mark_paint(root).unwrap();
    assert_eq!(clip(&mut tree).generation, next.generation + 1);
}

#[test]
fn changing_one_pass_command_count_does_not_shift_the_other_pass() {
    struct Variable {
        count: Rc<Cell<usize>>,
        source: DependencySource,
        color: Color,
    }
    impl EditableTextPainter for Variable {
        fn paint(&self, _context: EditableTextPaintContext, canvas: &mut DisplayList) {
            for x in 0..self.count.get() {
                canvas.push(PaintCommand::Rect {
                    rect: Rect::from_origin_size(Offset::new(x as f32, 1.), Size::new(1., 1.)),
                    color: self.color,
                });
            }
        }
        fn repaint_source(&self) -> Option<DependencySource> {
            Some(self.source.clone())
        }
    }
    let editor = editor();
    let mut tree = WidgetTree::new();
    let bg_count = Rc::new(Cell::new(1));
    let fg_count = Rc::new(Cell::new(1));
    let bg_source = DependencySource::default();
    let fg_source = DependencySource::default();
    let bg = Delegate::new(Variable {
        count: bg_count.clone(),
        source: bg_source.clone(),
        color: BACK,
    });
    let fg = Delegate::new(Variable {
        count: fg_count.clone(),
        source: fg_source.clone(),
        color: FRONT,
    });
    tree.mount(field(&editor, Some(bg), Some(fg), None))
        .unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(commands(&mut tree), [BACK, INK, FRONT]);

    bg_count.set(2);
    bg_source.notify();
    assert_eq!(commands(&mut tree), [BACK, BACK, INK, FRONT]);
    fg_count.set(2);
    fg_source.notify();
    assert_eq!(commands(&mut tree), [BACK, BACK, INK, FRONT, FRONT]);
    bg_count.set(0);
    bg_source.notify();
    assert_eq!(commands(&mut tree), [INK, FRONT, FRONT]);
}
