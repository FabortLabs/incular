//! CustomPainter's repaint source drives only mounted paint.
use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size, reactivity::DependencySource};
use incular_rendering::{Canvas, DisplayList, PaintCommand};
use incular_widgets::{
    CustomPaint, CustomPainter, CustomPainterSemantics, Semantics, SemanticsBuilder, Widget,
    internal::WidgetTree,
};

#[derive(Clone)]
struct WatchedPainter {
    source: DependencySource,
    color: Rc<Cell<Color>>,
    paint_calls: Rc<Cell<usize>>,
    semantics_calls: Rc<Cell<usize>>,
}

impl CustomPainter for WatchedPainter {
    fn paint(&self, size: Size) -> DisplayList {
        self.paint_calls.set(self.paint_calls.get() + 1);
        let mut canvas = Canvas::default();
        canvas.rect(Rect::from_origin_size(Offset::ZERO, size), self.color.get());
        canvas.finish()
    }

    fn should_repaint(&self, _: &Self) -> bool {
        false
    }

    fn repaint_source(&self) -> Option<DependencySource> {
        Some(self.source.clone())
    }

    fn semantics_builder(&self) -> Option<SemanticsBuilder> {
        let calls = self.semantics_calls.clone();
        Some(Rc::new(move |_| {
            calls.set(calls.get() + 1);
            vec![CustomPainterSemantics::new(
                Rect::from_origin_size(Offset::ZERO, Size::new(10., 10.)),
                Semantics::builder().label("status").build(),
            )]
        }))
    }
}

fn picture(tree: &mut WidgetTree) -> Color {
    tree.paint()
        .commands()
        .iter()
        .find_map(|command| match command {
            PaintCommand::Rect { color, .. } => Some(*color),
            _ => None,
        })
        .unwrap()
}

#[test]
fn notifications_repaint_without_layout_or_semantics_and_detach_old_source() {
    let red = Color::rgba(200, 0, 0, 255);
    let blue = Color::rgba(0, 0, 200, 255);
    let green = Color::rgba(0, 200, 0, 255);
    let source_a = DependencySource::default();
    let source_b = DependencySource::default();
    let color = Rc::new(Cell::new(red));
    let paint_calls = Rc::new(Cell::new(0));
    let semantics_calls = Rc::new(Cell::new(0));
    let view = Size::new(100., 60.);
    let descriptor = |source: &DependencySource| {
        Widget::from(
            incular_widgets::SizedBox::new().child(CustomPaint::from_painter(
                view,
                WatchedPainter {
                    source: source.clone(),
                    color: color.clone(),
                    paint_calls: paint_calls.clone(),
                    semantics_calls: semantics_calls.clone(),
                },
            )),
        )
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(descriptor(&source_a)).unwrap();
    tree.layout(Constraints::tight(view)).unwrap();
    assert_eq!(picture(&mut tree), red);
    tree.update_semantics();
    let before = tree.diagnostics();
    assert_eq!(source_a.subscriber_count(), 1);
    assert_eq!(paint_calls.get(), 1);
    assert_eq!(semantics_calls.get(), 1);

    color.set(blue);
    source_a.notify();
    assert_eq!(picture(&mut tree), blue);
    tree.update_semantics();
    assert_eq!(paint_calls.get(), 2);
    assert_eq!(semantics_calls.get(), 1);
    assert_eq!(tree.diagnostics().rebuilds, before.rebuilds);
    assert_eq!(tree.diagnostics().layouts, before.layouts);

    tree.update(root, descriptor(&source_b)).unwrap();
    tree.layout(Constraints::tight(view)).unwrap();
    assert_eq!(source_a.subscriber_count(), 0);
    assert_eq!(source_b.subscriber_count(), 1);
    let retained = picture(&mut tree);
    let count = paint_calls.get();
    color.set(green);
    source_a.notify();
    assert_eq!(picture(&mut tree), retained);
    assert_eq!(paint_calls.get(), count);
    source_b.notify();
    assert_eq!(picture(&mut tree), green);
    assert_eq!(paint_calls.get(), count + 1);
    tree.update(
        root,
        incular_widgets::SizedBox::new()
            .child(incular_widgets::SizedBox::shrink())
            .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(view)).unwrap();
    assert_eq!(source_b.subscriber_count(), 0);
}
