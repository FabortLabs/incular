//! Flutter Transform can change where a child paints without moving hits.
use incular_config::{Alignment, Constraints};
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Rect, Size, Transform as Affine, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{GestureDetector, Positioned, Stack, Transform, Widget, internal::Key};
use std::{cell::RefCell, rc::Rc, time::Duration};

#[test]
fn a_retained_hit_flag_changes_pivoted_gesture_coordinates_not_visual_geometry() {
    let points = Rc::new(RefCell::new(Vec::new()));
    let scene = |enabled| {
        let gesture = Widget::from(
            GestureDetector::new(Widget::box_(Size::new(40., 30.), Color::WHITE))
                .on_tap(|| {})
                .on_tap_down({
                    let points = points.clone();
                    move |details| points.borrow_mut().push(details.local_position)
                }),
        )
        .with_key(Key::Value(17));
        let transform = Transform::new(
            Affine::translation(Offset::new(50., 0.)).then(Affine::scale(2.)),
            gesture,
        )
        .origin(Offset::new(3., 4.))
        .alignment(Alignment::CENTER);
        let transform = if enabled {
            transform
        } else {
            transform.transform_hit_tests(false)
        };
        Stack::new([Positioned::new(transform).left(60.).top(70.)]).into()
    };
    let mut r = Runtime::new(scene(true)).unwrap();
    let viewport = Constraints::tight(Size::new(200., 200.));
    let now = Instant::now();
    r.run_frame_at(viewport, now).unwrap();
    let root = r.tree().root().unwrap();
    let key = Key::Value(17);
    let child = r.tree().element_with_key(&key).unwrap();
    let visual = Rect::from_origin_size(Offset::new(87., 51.), Size::new(80., 60.));
    let mut sequence = 0;
    for enabled in [true, false] {
        if !enabled {
            r.schedule_update(root, scene(false)).unwrap();
        }
        r.run_frame_at(viewport, now + Duration::from_millis(sequence))
            .unwrap();
        assert_eq!(r.tree().element_with_key(&key), Some(child));
        assert_eq!(r.tree().element_bounds(child), Some(visual));
        for position in [Offset::new(65., 74.), Offset::new(120., 78.)] {
            sequence += 1;
            for (phase, buttons) in [
                (PointerPhase::Down, PRIMARY_POINTER_BUTTON),
                (PointerPhase::Up, 0),
            ] {
                let _ = r.handle_input_at(
                    InputEvent::PointerWithMetadata {
                        pointer: sequence,
                        device: 1,
                        kind: PointerDeviceKind::Touch,
                        buttons,
                        button: Some(PRIMARY_POINTER_BUTTON),
                        position,
                        phase,
                        sample: PointerSampleMetadata::EMPTY,
                    },
                    now + Duration::from_millis(sequence),
                );
            }
        }
        let expected = if enabled {
            Offset::new(16.5, 13.5)
        } else {
            Offset::new(5., 4.)
        };
        assert_eq!(*points.borrow(), [expected]);
        points.borrow_mut().clear();
    }
}
