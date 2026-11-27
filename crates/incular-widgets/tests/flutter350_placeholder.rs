use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Size};
use incular_rendering::{Brush, PaintCommand};
use incular_widgets::{
    Center, Directionality, Placeholder, Positioned, SizedBox, Stack, Widget,
    internal::{Key, WidgetTree},
};

const BOX: Key = Key::Value(100);

fn placeholder(widget: Placeholder) -> Widget {
    Widget::from(widget).with_key(BOX.clone())
}

fn frame(widget: impl Into<Widget>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    tree
}

fn size(tree: &WidgetTree) -> Size {
    let element = tree.element_with_key(&BOX).unwrap();
    tree.element_bounds(element).unwrap().size
}

#[test]
fn bounded_axes_fill_and_only_unbounded_axes_use_fallbacks() {
    let leaf = || placeholder(Placeholder::new());
    assert_eq!(size(&frame(leaf())), Size::new(800., 600.));
    assert_eq!(size(&frame(Center::new(leaf()))), Size::new(800., 600.));
    let positioned = |child| Stack::new([child]).text_direction(TextDirection::Ltr);
    assert_eq!(
        size(&frame(positioned(
            Positioned::new(leaf()).top(0.).bottom(0.)
        ))),
        Size::new(400., 600.)
    );
    assert_eq!(
        size(&frame(positioned(
            Positioned::new(leaf()).left(0.).right(0.)
        ))),
        Size::new(800., 400.)
    );
    let custom = placeholder(
        Placeholder::new()
            .fallback_width(200.)
            .fallback_height(300.),
    );
    assert_eq!(
        size(&frame(positioned(Positioned::new(custom).top(0.)))),
        Size::new(200., 300.)
    );
}

#[test]
fn paints_the_cross_as_a_local_path_with_the_exact_color_and_stroke_width() {
    let green = Color::rgba(0, 255, 0, 255);
    for (widget, color, width) in [
        (Placeholder::new(), Color::rgba(69, 90, 100, 255), 2.),
        (Placeholder::new().color(green), green, 2.),
        (
            Placeholder::new().stroke_width(10.),
            Color::rgba(69, 90, 100, 255),
            10.,
        ),
    ] {
        let mut tree = frame(placeholder(widget));
        let paint = tree.paint();
        let paths: Vec<_> = paint
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::StrokePath {
                    path,
                    brush: Brush::Solid(color),
                    stroke,
                } => Some((path.tight_bounds().unwrap().size, *color, stroke.width)),
                _ => None,
            })
            .collect();
        assert_eq!(paths, [(Size::new(800., 600.), color, width)]);
    }
}

#[test]
fn zero_area_placeholder_can_complete_layout_and_paint() {
    let mut tree = frame(Directionality::new(
        TextDirection::Ltr,
        Center::new(SizedBox::shrink().child(placeholder(Placeholder::new()))),
    ));
    assert_eq!(size(&tree), Size::ZERO);
    let _ = tree.paint();
}

#[test]
fn a_placeholder_mounts_its_optional_label_on_update() {
    use incular_widgets::Text;

    let label_key = Key::Value(101);
    let mut tree = frame(placeholder(Placeholder::new()));
    assert!(tree.element_with_key(&label_key).is_none());

    let text = Widget::from(Text::new("Label")).with_key(label_key.clone());
    let root = tree.root().unwrap();
    tree.update(
        root,
        placeholder(Placeholder::new().child(Directionality::new(TextDirection::Ltr, text))),
    )
    .unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    assert!(tree.element_with_key(&label_key).is_some());
}

#[test]
fn an_outline_is_pointer_transparent_but_its_child_is_interactive() {
    use incular_core::{Offset, PointerDeviceKind, PointerPhase};
    use incular_gestures::PointerEvent;
    use incular_widgets::{GestureDetector, HitTestBehavior};
    use std::{
        cell::Cell,
        rc::Rc,
        time::{Duration, Instant},
    };

    let make_target = |size_box, taps: &Rc<Cell<u32>>| {
        let taps = Rc::clone(taps);
        GestureDetector::new(size_box)
            .behavior(HitTestBehavior::Opaque)
            .on_tap(move || taps.set(taps.get() + 1))
    };
    let tap = |tree: &mut WidgetTree, position| {
        let now = Instant::now();
        for (phase, millis) in [(PointerPhase::Down, 0), (PointerPhase::Up, 10)] {
            let _ = tree.dispatch_pointer_gesture_in_window(
                0,
                1,
                PointerDeviceKind::Touch,
                PointerEvent {
                    position,
                    phase,
                    time: now + Duration::from_millis(millis),
                    pointer: 1,
                },
            );
        }
    };
    let taps = Rc::new(Cell::new(0));
    let child = make_target(SizedBox::new().width(20.).height(30.), &taps);
    let mut tree = frame(Center::new(placeholder(Placeholder::new().child(child))));
    assert_eq!(size(&tree), Size::new(20., 30.));
    tap(&mut tree, Offset::new(400., 300.));
    assert_eq!(taps.get(), 1);

    let behind = make_target(SizedBox::expand(), &taps);
    let mut tree = frame(
        Stack::new([
            Positioned::fill(behind),
            Positioned::fill(placeholder(Placeholder::new())),
        ])
        .text_direction(TextDirection::Ltr),
    );
    tap(&mut tree, Offset::new(10., 10.));
    assert_eq!(taps.get(), 2);
}

#[test]
fn layout_builder_updates_both_placeholder_paints_across_constraint_changes() {
    use incular_widgets::{Column, LayoutBuilder};
    use std::{cell::RefCell, rc::Rc};

    let snapshots = Rc::new(RefCell::new(Vec::new()));
    let scene = |fixed| {
        let snapshots = Rc::clone(&snapshots);
        let layout = LayoutBuilder::new(move |_, constraints| {
            snapshots.borrow_mut().push(constraints);
            let size = if fixed { 10. } else { 20. };
            Column::new((102..=103).map(|key| {
                SizedBox::square(size)
                    .child(Widget::from(Placeholder::new()).with_key(Key::Value(key)))
            }))
            .into()
        });
        let wrapper = if fixed {
            SizedBox::square(100.)
        } else {
            SizedBox::new()
        };
        Widget::from(Center::new(wrapper.child(layout)))
    };
    let mut tree = frame(scene(false));
    let root = tree.root().unwrap();
    for (current, extent, expected_constraints) in [
        (false, 20., Constraints::loose(Size::new(800., 600.))),
        (true, 10., Constraints::tight(Size::new(100., 100.))),
    ] {
        if current {
            tree.update(root, scene(true)).unwrap();
            tree.layout(Constraints::tight(Size::new(800., 600.)))
                .unwrap();
        }
        assert_eq!(snapshots.borrow().last(), Some(&expected_constraints));
        let expected = Size::new(extent, extent);
        for key in 102..=103 {
            let element = tree.element_with_key(&Key::Value(key)).unwrap();
            assert_eq!(tree.element_bounds(element).unwrap().size, expected);
        }
        let paint = tree.paint();
        let strokes: Vec<_> = paint
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::StrokePath { path, .. } => {
                    path.tight_bounds().map(|bounds| bounds.size)
                }
                _ => None,
            })
            .collect();
        assert_eq!(strokes, [expected, expected]);
    }
}
