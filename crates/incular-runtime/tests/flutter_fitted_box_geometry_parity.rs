//! Flutter 3.50.0 widgets/fitted_box_test.dart retained geometry and hits.

use std::{cell::Cell, rc::Rc};

use incular_config::{Alignment, Constraints};
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Rect, Size, Transform,
};
use incular_rendering::{Brush, PaintCommand};
use incular_runtime::Runtime;
use incular_widgets::{
    BoxFit, Center, ColoredBox, FittedBox, Listener, SizedBox, Widget, internal::Key,
};

const INSIDE: u64 = 1;
const OUTSIDE: u64 = 2;

fn frame_constraints() -> Constraints {
    Constraints::tight(Size::new(800., 600.))
}

fn geometry(runtime: &Runtime, key: u64) -> (Transform, Size) {
    let tree = runtime.tree();
    let id = tree.element_with_key(&Key::from(key)).expect("fixture key");
    tree.element_geometry(id).expect("fixture geometry")
}

fn scene(fit: BoxFit, tight_height: bool) -> Widget {
    let inner = Widget::from(
        SizedBox::new()
            .width(100.)
            .height(50.)
            .child(ColoredBox::new(Color::BLACK, SizedBox::shrink())),
    )
    .with_key(INSIDE);
    let outer = Widget::from(FittedBox::new(inner).fit(fit)).with_key(OUTSIDE);
    let bound = if tight_height {
        SizedBox::square(200.)
    } else {
        SizedBox::new().width(200.)
    };
    Center::new(bound.child(outer)).into()
}

// fitted_box_test.dart:41, "Can contain child".
#[test]
fn tight_square_fitted_box_contains_the_child_without_changing_its_layout_size() {
    let mut runtime = Runtime::new(scene(BoxFit::Contain, true)).expect("mount");
    runtime.run_frame(frame_constraints()).expect("layout");
    let (inside, child_size) = geometry(&runtime, INSIDE);
    let (outside, outer_size) = geometry(&runtime, OUTSIDE);
    assert_eq!(child_size, Size::new(100., 50.));
    assert_eq!(outer_size, Size::new(200., 200.));
    assert_eq!(
        inside.transform_point(Offset::new(100., 0.)),
        outside.transform_point(Offset::new(200., 50.)),
    );
}

// fitted_box_test.dart:71, "Child can cover". In addition to the upstream
// center-point assertion, inspect default clip-free overflow.
#[test]
fn cover_center_aligns_the_child_and_lets_default_paint_overflow() {
    let mut runtime = Runtime::new(scene(BoxFit::Cover, true)).expect("mount");
    let (list, _) = runtime.run_frame(frame_constraints()).expect("layout");
    let (inside, child_size) = geometry(&runtime, INSIDE);
    let (outside, outer_size) = geometry(&runtime, OUTSIDE);
    assert_eq!(child_size, Size::new(100., 50.));
    assert_eq!(outer_size, Size::new(200., 200.));
    assert_eq!(
        inside.transform_point(Offset::new(50., 25.)),
        outside.transform_point(Offset::new(100., 100.)),
    );
    assert_eq!(
        inside.transform_point(Offset::ZERO),
        Offset::new(200., 200.)
    );
    assert_eq!(
        inside.transform_point(Offset::new(100., 50.)),
        Offset::new(600., 400.),
    );
    let mut transforms = vec![Transform::IDENTITY];
    let mut painted = Vec::new();
    for command in list.commands() {
        let current = *transforms.last().expect("paint transform");
        match command {
            PaintCommand::PushTransform { transform } => transforms.push(current.then(*transform)),
            PaintCommand::PopTransform => {
                transforms.pop();
            }
            PaintCommand::Rect { rect, color } if *color == Color::BLACK => {
                painted.push(current.transform_rect_bbox(*rect));
            }
            PaintCommand::RRect {
                rrect,
                brush: Brush::Solid(color),
            } if *color == Color::BLACK => {
                painted.push(current.transform_rect_bbox(rrect.rect));
            }
            PaintCommand::PushClip { .. } => {
                panic!("the default FittedBox adds no rectangular clip")
            }
            _ => {}
        }
    }
    assert_eq!(
        painted,
        [Rect::from_origin_size(
            Offset::new(200., 200.),
            Size::new(400., 200.)
        )],
    );
}

// fitted_box_test.dart:396, "Big child into small fitted box - hit testing".
#[test]
fn scaled_down_fitted_child_receives_pointer_down_at_its_transformed_center() {
    let pressed = Rc::new(Cell::new(false));
    let child = Listener::new(ColoredBox::new(Color::BLACK, SizedBox::shrink())).on_pointer_down({
        let pressed = Rc::clone(&pressed);
        move |_| pressed.set(true)
    });
    let fit = FittedBox::new(SizedBox::square(1000.).child(child)).alignment(Alignment::CENTER);
    let scene = Center::new(SizedBox::square(100.).child(fit));
    let mut runtime = Runtime::new(scene.into()).expect("mount");
    runtime.run_frame(frame_constraints()).expect("layout");
    assert!(!pressed.get());
    for (phase, buttons) in [
        (PointerPhase::Down, PRIMARY_POINTER_BUTTON),
        (PointerPhase::Up, 0),
    ] {
        let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 0,
            kind: PointerDeviceKind::Touch,
            buttons,
            button: Some(PRIMARY_POINTER_BUTTON),
            sample: PointerSampleMetadata::default(),
            phase,
            position: Offset::new(400., 300.),
        });
    }
    assert!(pressed.get(), "hit follows the fitted paint position");
}

// fitted_box_test.dart:487, "Switching to and from BoxFit.scaleDown causes relayout".
#[test]
fn switching_between_scale_down_and_contain_recomputes_retained_height() {
    let mut runtime = Runtime::new(scene(BoxFit::ScaleDown, false)).expect("mount");
    runtime
        .run_frame(frame_constraints())
        .expect("scale-down frame");
    let root = runtime.tree().root().expect("scene root");
    let key = Key::from(OUTSIDE);
    let fitted = runtime.tree().element_with_key(&key).expect("fitted box");
    let start = geometry(&runtime, OUTSIDE).1.height;
    let changes = [BoxFit::Contain, BoxFit::ScaleDown].map(|fit| {
        runtime
            .tree_mut()
            .update(root, scene(fit, false))
            .expect("update fit");
        runtime
            .run_frame(frame_constraints())
            .expect("update frame");
        assert_eq!(runtime.tree().element_with_key(&key), Some(fitted));
        geometry(&runtime, OUTSIDE).1.height
    });
    assert_eq!([start, changes[0], changes[1]], [50., 100., 50.]);
}
