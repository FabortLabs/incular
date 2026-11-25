use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::{Alignment, Clip, Constraints, EdgeInsets};
use incular_core::{Color, Offset, PointerPhase, Size, Transform};
use incular_gestures::PointerEvent;
use incular_widgets::{
    Align, BorderRadius, BoxDecoration, BoxShape, Container, GestureDetector, Radius, Widget,
    internal::{Key, WidgetTree},
};

fn taps(child: impl Into<Widget>, points: impl FnOnce(&WidgetTree) -> Vec<Offset>) -> Vec<bool> {
    let fired = Rc::new(Cell::new(false));
    let detector = GestureDetector::new(child).on_tap({
        let fired = fired.clone();
        move || fired.set(true)
    });
    let mut tree = WidgetTree::new();
    tree.mount(detector.into()).expect("mount");
    tree.layout(Constraints::tight(Size::new(100.0, 60.0)))
        .expect("layout");
    let _ = tree.paint();
    let start = Instant::now();
    points(&tree)
        .into_iter()
        .enumerate()
        .map(|(index, position)| {
            fired.set(false);
            for (phase, after) in [(PointerPhase::Down, 0), (PointerPhase::Up, 10)] {
                tree.dispatch_gesture(PointerEvent {
                    pointer: 1,
                    position,
                    phase,
                    time: start + Duration::from_millis(100 * index as u64 + after),
                });
            }
            fired.get()
        })
        .collect()
}

#[test]
fn container_only_accepts_taps_on_a_decoration_not_layout_alone() {
    let black = BoxDecoration::new().color(Color::BLACK);
    for container in [
        Container::new().decoration(black.clone()),
        Container::new().foreground_decoration(black),
        Container::new().color(Color::BLACK),
    ] {
        assert_eq!(taps(container, |_| vec![Offset::new(50.0, 30.0)]), [true]);
    }

    let key = Key::from("layout only");
    let transformed = Widget::from(
        Container::new()
            .alignment(Alignment::BOTTOM_RIGHT)
            .padding(EdgeInsets::all(2.0))
            .width(50.0)
            .height(50.0)
            .margin(EdgeInsets::all(2.0))
            .transform(Transform::rotation(1.0)),
    )
    .with_key(key.clone());
    let styled = Align::new(Alignment::CENTER, transformed);
    assert_eq!(
        taps(styled, |tree| {
            let (transform, size) = tree
                .element_geometry(tree.element_with_key(&key).expect("container"))
                .expect("laid out");
            vec![transform.transform_point(Offset::new(size.width / 2.0, size.height / 2.0))]
        }),
        [false]
    );
}

#[test]
fn decoration_hits_use_the_shape_without_clipping_the_child() {
    let circle = BoxDecoration::new().shape(BoxShape::Circle);
    for container in [
        Container::new().decoration(circle.clone()),
        Container::new().foreground_decoration(circle.clone()),
    ] {
        assert_eq!(
            taps(container, |_| {
                vec![Offset::new(50.0, 30.0), Offset::new(10.0, 30.0)]
            }),
            [true, false]
        );
    }

    let rounded = BoxDecoration::new().border_radius(BorderRadius::all(Radius::circular(20.0)));
    assert_eq!(
        taps(Container::new().decoration(rounded), |_| {
            vec![Offset::new(50.0, 30.0), Offset::new(1.0, 1.0)]
        }),
        [true, false]
    );

    let child = Widget::box_(Size::new(100.0, 60.0), Color::WHITE);
    for (mode, outside_taps) in [(Clip::None, true), (Clip::HardEdge, false)] {
        assert_eq!(
            taps(
                Container::new()
                    .decoration(circle.clone())
                    .child(child.clone())
                    .clip_behavior(mode),
                |_| vec![Offset::new(50.0, 30.0), Offset::new(10.0, 30.0)]
            ),
            [true, outside_taps]
        );
    }
}
