use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Lerp, Offset, PointerPhase, Size};
use incular_gestures::PointerEvent;
use incular_rendering::{Brush, CornerRadii, PaintCommand};
use incular_widgets::{
    BorderRadius, BorderRadiusDirectional, BoxDecoration, Container, DecoratedBox, Directionality,
    GestureDetector, Radius, SizedBox, Widget, internal::WidgetTree,
};
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

const SIZE: Size = Size {
    width: 1000.0,
    height: 1000.0,
};

fn decoration() -> BoxDecoration {
    BoxDecoration::new()
        .color(Color::BLACK)
        .directional_border_radius(BorderRadiusDirectional::only(
            Radius::circular(100.0),
            Radius::ZERO,
            Radius::ZERO,
            Radius::ZERO,
        ))
}

fn expected(direction: TextDirection) -> CornerRadii {
    match direction {
        TextDirection::Ltr => CornerRadii {
            top_left: 100.0,
            ..CornerRadii::ZERO
        },
        TextDirection::Rtl => CornerRadii {
            top_right: 100.0,
            ..CornerRadii::ZERO
        },
    }
}

fn check_fill(list: &incular_rendering::DisplayList, direction: TextDirection) {
    let rounded: Vec<_> = list
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::RRect {
                rrect,
                brush: Brush::Solid(Color::BLACK),
            } => Some(*rrect),
            _ => None,
        })
        .collect();
    let [rrect] = rounded.as_slice() else {
        panic!("expected one black rounded box, got {rounded:?}");
    };
    assert_eq!(rrect.rect.origin, Offset::ZERO);
    assert_eq!(rrect.rect.size, SIZE);
    assert_eq!(rrect.radii, expected(direction));
}

#[test]
fn paint_and_hit_test_resolve_the_same_leading_corner_in_both_directions() {
    let decoration = decoration();
    for (direction, left, right) in [
        (TextDirection::Rtl, true, false),
        (TextDirection::Ltr, false, true),
    ] {
        let painted = decoration
            .clone()
            .resolved_for_direction(direction)
            .paint_background(SIZE)
            .expect("background fill");
        check_fill(&painted, direction);
        assert_eq!(
            decoration.hit_test_for_direction(SIZE, Offset::new(10.0, 10.0), direction),
            left,
        );
        assert_eq!(
            decoration.hit_test_for_direction(SIZE, Offset::new(990.0, 10.0), direction),
            right,
        );
    }
}

#[test]
fn retained_widgets_resolve_the_same_decoration_after_direction_only_updates() {
    let decorated: Widget = DecoratedBox::new(SizedBox::shrink())
        .decoration(decoration())
        .into();
    let container: Widget = Container::new().decoration(decoration()).into();
    for child in [decorated, container] {
        let tapped = Rc::new(Cell::new(0));
        let child: Widget = GestureDetector::new(child)
            .on_tap({
                let tapped = tapped.clone();
                move || tapped.set(tapped.get() + 1)
            })
            .into();
        let root_for = |direction| Directionality::new(direction, child.clone()).into();
        let mut tree = WidgetTree::new();
        let root = tree.mount(root_for(TextDirection::Ltr)).unwrap();
        let start = Instant::now();
        for (step, direction) in [TextDirection::Ltr, TextDirection::Rtl, TextDirection::Ltr]
            .into_iter()
            .enumerate()
        {
            if step > 0 {
                tree.update(root, root_for(direction)).unwrap();
            }
            tree.layout(Constraints::tight(SIZE)).unwrap();
            check_fill(&tree.paint(), direction);
            for (edge, x) in [10.0, 990.0].into_iter().enumerate() {
                let before = tapped.get();
                for (phase, after) in [(PointerPhase::Down, 0), (PointerPhase::Up, 10)] {
                    tree.dispatch_gesture(PointerEvent {
                        pointer: 1,
                        position: Offset::new(x, 10.0),
                        phase,
                        time: start
                            + Duration::from_millis((step * 200 + edge * 50) as u64 + after),
                    });
                }
                let should_hit = if direction == TextDirection::Ltr {
                    edge == 1
                } else {
                    edge == 0
                };
                assert_eq!(
                    tapped.get(),
                    before + usize::from(should_hit),
                    "{direction:?} at x={x}"
                );
            }
        }
    }
}

#[test]
fn interpolation_keeps_physical_and_directional_radius_components_until_resolution() {
    let physical = BoxDecoration::new().border_radius(BorderRadius::only(
        Radius::circular(20.0),
        Radius::ZERO,
        Radius::ZERO,
        Radius::ZERO,
    ));
    let mixed = physical.lerp(&decoration(), 0.5);
    for (direction, left, right) in [
        (TextDirection::Ltr, 60.0, 0.0),
        (TextDirection::Rtl, 10.0, 50.0),
    ] {
        let resolved = mixed.clone().resolved_for_direction(direction);
        let radius = resolved.border_radius.unwrap();
        assert_eq!(radius.top_left, Radius::circular(left));
        assert_eq!(radius.top_right, Radius::circular(right));
        assert!(resolved.directional_border_radius.is_none());
    }
}
