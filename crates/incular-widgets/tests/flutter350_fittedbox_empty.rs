//! Pinned Flutter 3.50.0 FittedBox absence and zero-area behavior.

use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Size, time::Instant};
use incular_gestures::PointerEvent;
use incular_widgets::{
    BoxFit, Center, Directionality, FittedBox, Placeholder, SizedBox, TextDirection, Widget,
    internal::{Key, WidgetTree},
};
use std::time::Duration;

const FIT: Key = Key::Value(9280);
const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};

fn keyed(fit: FittedBox) -> Widget {
    Widget::from(fit).with_key(FIT)
}

fn fit_size(tree: &WidgetTree) -> Size {
    let element = tree.element_with_key(&FIT).unwrap();
    tree.render_size(tree.render_id(element).unwrap()).unwrap()
}

/// widgets/fitted_box_test.dart:102.
#[test]
fn a_childless_fitted_box_uses_the_smallest_loose_size() {
    let mut tree = WidgetTree::new();
    let empty = FittedBox::builder().fit(BoxFit::Cover).build();
    tree.mount(Center::new(keyed(empty)).into()).unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(fit_size(&tree), Size::ZERO);
    assert!(
        tree.children(tree.element_with_key(&FIT).unwrap())
            .unwrap()
            .is_empty()
    );
}

/// widgets/fitted_box_test.dart:524.
#[test]
fn a_childless_fitted_box_in_a_tight_frame_does_not_fail_on_pointer_input() {
    let mut tree = WidgetTree::new();
    let empty = keyed(FittedBox::builder().build());
    tree.mount(Center::new(SizedBox::square(200.).child(empty)).into())
        .unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(fit_size(&tree), Size::new(200., 200.));
    let element = tree.element_with_key(&FIT).unwrap();
    assert!(tree.children(element).unwrap().is_empty());
    let point = tree.element_bounds(element).unwrap().origin + Offset::new(100., 100.);
    let time = Instant::now();
    for (phase, after) in [(PointerPhase::Down, 0), (PointerPhase::Up, 10)] {
        tree.dispatch_gesture(PointerEvent {
            pointer: 1,
            position: point,
            phase,
            time: time + Duration::from_millis(after),
        });
    }
    let _ = tree.paint();
}

/// widgets/fitted_box_test.dart:537.
#[test]
fn a_zero_size_child_is_safe_under_tight_scale_down_and_loose_contain() {
    let mut tree = WidgetTree::new();
    let tight = |fit: FittedBox| Center::new(SizedBox::square(200.).child(keyed(fit))).into();
    let root = tree
        .mount(tight(
            FittedBox::new(SizedBox::shrink()).fit(BoxFit::ScaleDown),
        ))
        .unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(fit_size(&tree), Size::new(200., 200.));
    let _ = tree.paint();

    let contained = incular_widgets::ConstrainedBox::new(
        Constraints::loose(Size::new(200., 200.)),
        keyed(FittedBox::new(SizedBox::shrink())),
    );
    tree.update(root, Center::new(contained).into()).unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(fit_size(&tree), Size::ZERO);
    let _ = tree.paint();
}

/// widgets/fitted_box_test.dart:559.
#[test]
fn fitted_box_with_nonempty_child_finishes_a_zero_area_view() {
    let mut tree = WidgetTree::new();
    let fit = keyed(FittedBox::new(Placeholder::new()));
    tree.mount(Directionality::new(TextDirection::Ltr, Center::new(fit)).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    assert_eq!(fit_size(&tree), Size::ZERO);
    let _ = tree.paint();
}

#[test]
fn a_retained_fitted_box_rebinds_its_draw_when_a_child_comes_and_goes() {
    let scene = |present| {
        let fit = if present {
            FittedBox::new(Widget::box_(Size::new(30., 20.), Color::WHITE))
        } else {
            FittedBox::builder().build()
        };
        Center::new(keyed(fit)).into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(false)).unwrap();
    let element = tree.element_with_key(&FIT).unwrap();
    let render = tree.render_id(element).unwrap();
    for present in [false, true, false, true] {
        tree.update(root, scene(present)).unwrap();
        tree.layout(Constraints::tight(VIEW)).unwrap();
        assert_eq!(tree.render_id(element), Some(render));
        assert_eq!(
            fit_size(&tree),
            if present {
                Size::new(30., 20.)
            } else {
                Size::ZERO
            }
        );
        let list = tree.paint();
        assert_eq!(
            list.commands().iter().any(|op| matches!(
                op,
                incular_rendering::PaintCommand::Rect { color, .. } if *color == Color::WHITE
            )),
            present
        );
    }
}
