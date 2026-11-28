use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    SECONDARY_POINTER_BUTTON, Size, TERTIARY_POINTER_BUTTON,
};
use incular_runtime::Runtime;
use incular_widgets::{Align, Alignment, DragDropContext, Draggable, SizedBox, Widget};

type ButtonFilter = fn(u32) -> bool;

#[test]
fn a_local_drag_applies_the_configured_mask_before_it_joins_the_arena() {
    let p = PRIMARY_POINTER_BUTTON;
    let s = SECONDARY_POINTER_BUTTON;
    let t = TERTIARY_POINTER_BUTTON;
    let only_secondary: ButtonFilter = |mask| mask == SECONDARY_POINTER_BUTTON;
    let primary_or_tertiary: ButtonFilter =
        |mask| (mask & (PRIMARY_POINTER_BUTTON | TERTIARY_POINTER_BUTTON)) != 0;
    let reject: ButtonFilter = |_| false;
    for (filter, buttons, allowed) in [
        (None, p, true),
        (None, s, false),
        (None, p | s, false),
        (Some(only_secondary), p, false),
        (Some(only_secondary), s, true),
        (Some(primary_or_tertiary), p, true),
        (Some(primary_or_tertiary), t, true),
        (Some(primary_or_tertiary), s | t, true),
        (Some(reject), p, false),
    ] {
        let context = DragDropContext::new();
        let starts = Rc::new(Cell::new(0));
        let calls = Rc::new(Cell::new(0));
        let mut source = Draggable::new(
            context.clone(),
            1,
            Widget::box_(Size::new(80., 50.), Color::BLACK),
        )
        .feedback(|_| SizedBox::square(20.).into())
        .on_start({
            let starts = starts.clone();
            move |_| starts.set(starts.get() + 1)
        });
        if let Some(filter) = filter {
            let calls = calls.clone();
            source = source.allowed_buttons_filter(move |mask| {
                calls.set(calls.get() + 1);
                assert_eq!(mask, buttons);
                filter(mask)
            });
        }
        let mut runtime = Runtime::new(Align::new(Alignment::CENTER, source).into()).unwrap();
        let view = Constraints::tight(Size::new(800., 600.));
        let now = Instant::now();
        runtime.run_frame_at(view, now).unwrap();
        for (i, phase) in [PointerPhase::Down, PointerPhase::Up]
            .into_iter()
            .enumerate()
        {
            let time = now + Duration::from_millis(16 * (i as u64 + 1));
            let _ = runtime.handle_input_at(
                InputEvent::PointerWithMetadata {
                    pointer: 1,
                    device: 1,
                    kind: PointerDeviceKind::Mouse,
                    phase,
                    position: Offset::new(400., 300.),
                    buttons: if phase == PointerPhase::Down {
                        buttons
                    } else {
                        0
                    },
                    button: Some(if buttons == s {
                        s
                    } else if buttons & t != 0 {
                        t
                    } else {
                        p
                    }),
                    sample: Default::default(),
                },
                time,
            );
            runtime.run_frame_at(view, time).unwrap();
            assert_eq!(
                context.feedback().is_some(),
                allowed && phase == PointerPhase::Down,
                "mask {buttons} phase {phase:?}"
            );
        }
        assert_eq!(starts.get(), usize::from(allowed), "mask {buttons}");
        assert_eq!(calls.get(), usize::from(filter.is_some()), "mask {buttons}");
    }
}
