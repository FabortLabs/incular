//! Flutter's default, opaque, deferring and translucent Dismissible hits.

use incular_config::{Constraints, TextDirection};
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{Directionality, DismissDirection, Dismissible, SizedBox, Widget};
use std::{rc::Rc, time::Duration};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};

#[test]
fn hit_test_behavior_controls_the_listener_behind_the_row() {
    use incular_core::Color;
    use incular_widgets::{HitTestBehavior, Listener, Stack};
    use std::cell::Cell;

    let received = Rc::new(Cell::new(false));
    let scene = |behavior| {
        let received = received.clone();
        let behind = Listener::new(Widget::box_(Size::new(100., 100.), Color::WHITE))
            .on_pointer_down(move |_| received.set(true));
        let front = Dismissible::new(
            DismissDirection::Horizontal,
            SizedBox::from_size(Size::new(100., 100.)),
            |_| {},
        );
        let front = match behavior {
            Some(behavior) => front.behavior(behavior),
            None => front,
        };
        Directionality::new(
            TextDirection::Ltr,
            Stack::new([Widget::from(behind), Widget::from(front).with_key(1_u64)]),
        )
    };
    let mut runtime = Runtime::new(scene(None).into()).unwrap();
    let root = runtime.tree().root().unwrap();
    let mut time = Instant::now();
    let viewport = Constraints::tight(VIEW);
    for (attempt, (behavior, expected)) in [
        (None, false),
        (Some(HitTestBehavior::DeferToChild), true),
        (Some(HitTestBehavior::Opaque), false),
        (Some(HitTestBehavior::Translucent), true),
    ]
    .into_iter()
    .enumerate()
    {
        if attempt != 0 {
            runtime
                .schedule_update(root, scene(behavior).into())
                .unwrap();
        }
        runtime.run_frame_at(viewport, time).unwrap();
        received.set(false);
        for (phase, buttons) in [
            (PointerPhase::Down, PRIMARY_POINTER_BUTTON),
            (PointerPhase::Up, 0),
        ] {
            runtime.handle_input_at(
                InputEvent::PointerWithMetadata {
                    phase,
                    pointer: attempt as u64 + 1,
                    device: 1,
                    kind: PointerDeviceKind::Touch,
                    buttons,
                    button: Some(PRIMARY_POINTER_BUTTON),
                    sample: PointerSampleMetadata::EMPTY,
                    position: Offset::new(10., 10.),
                },
                time,
            );
        }
        assert_eq!(received.get(), expected, "{behavior:?}");
        time += Duration::from_millis(16);
    }
}
