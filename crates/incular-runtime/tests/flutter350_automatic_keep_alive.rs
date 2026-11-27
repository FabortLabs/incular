//! AutomaticKeepAlive requests travel from mounted descendants to the
//! sliver's cache. They are independent of the retained row's build.

use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    AutomaticKeepAlive, Builder, Directionality, GridView, KeepAliveHandle, KeepAliveNotification,
    ListView, NotificationDispatcher, ScrollController, Semantics, SliverGridDelegate, Widget,
    internal::Key,
};

#[derive(Clone, Copy, Debug)]
enum ScrollKind {
    FixedList,
    NaturalList,
    Grid,
}

fn assert_parked(runtime: &Runtime, key: &Key, id: incular_widgets::internal::ElementId) {
    assert_eq!(runtime.tree().element_with_key(key), Some(id));
    assert!(
        !runtime
            .tree()
            .semantics()
            .iter()
            .any(|(_, node)| { node.label.as_deref() == Some("retained row") })
    );
}

#[test]
fn a_row_survives_until_the_last_descendant_handle_is_released() {
    for kind in [
        ScrollKind::FixedList,
        ScrollKind::NaturalList,
        ScrollKind::Grid,
    ] {
        let controller = ScrollController::new();
        let source = Rc::new(RefCell::new(None));
        let row_key = Key::from("retained child");
        let row = {
            let source = source.clone();
            let row_key = row_key.clone();
            move |index| {
                let box_ = Widget::box_(Size::new(50., 40.), Color::WHITE);
                if index != 0 {
                    return box_;
                }
                let origin = Builder::new({
                    let source = source.clone();
                    move |context| {
                        *source.borrow_mut() = Some(NotificationDispatcher::of(context));
                        Semantics::new(box_.clone())
                            .container(true)
                            .label("retained row")
                            .into()
                    }
                });
                AutomaticKeepAlive::new(Widget::from(origin).with_key(row_key.clone())).into()
            }
        };
        let scroll: Widget = match kind {
            ScrollKind::Grid => GridView::builder(
                30,
                SliverGridDelegate::fixed_cross_axis_count(2).main_axis_extent(40.),
                row,
            )
            .controller(controller.clone())
            .cache_extent(0.)
            .into(),
            _ => {
                let list = ListView::builder(30, row)
                    .controller(controller.clone())
                    .cache_extent(0.);
                if matches!(kind, ScrollKind::FixedList) {
                    list.item_extent(40.).into()
                } else {
                    list.into()
                }
            }
        };
        let bounds = Constraints::tight(Size::new(100., 120.));
        let mut runtime =
            Runtime::new(Directionality::new(TextDirection::Ltr, scroll).into()).unwrap();
        runtime.run_frame(bounds).unwrap();
        let id = runtime
            .tree()
            .element_with_key(&row_key)
            .expect("initial row");
        let source = source
            .borrow()
            .clone()
            .expect("mounted notification sender");
        let first = KeepAliveHandle::new();
        assert!(!source.dispatch(&KeepAliveNotification::new(first.clone())));
        assert!(
            first.has_listeners(),
            "{kind:?}: dispatch reached its mounted ancestor"
        );

        // Request and scroll before the next frame, not one frame apart.
        assert!(controller.jump_to(320.));
        runtime.run_frame(bounds).unwrap();
        assert_parked(&runtime, &row_key, id);

        let second = KeepAliveHandle::new();
        assert!(!source.dispatch(&KeepAliveNotification::new(second.clone())));
        assert!(second.has_listeners());
        first.release();
        runtime.run_frame(bounds).unwrap();
        assert!(!first.has_listeners());
        assert_parked(&runtime, &row_key, id);

        second.release();
        runtime.run_frame(bounds).unwrap();
        assert!(!second.has_listeners());
        assert!(
            !runtime.tree().element_exists(id),
            "{kind:?}: disposed without scrolling"
        );
        assert_eq!(controller.offset(), 320.);

        let stale = KeepAliveHandle::new();
        source.dispatch(&KeepAliveNotification::new(stale.clone()));
        assert!(
            !stale.has_listeners(),
            "old sender cannot revive an unmounted row"
        );
        assert!(controller.jump_to(0.));
        runtime.run_frame(bounds).unwrap();
        assert_ne!(runtime.tree().element_with_key(&row_key), Some(id));
    }
}

#[test]
fn flutter_original_double_keep_alive_holds_the_whole_row() {
    use incular_core::{
        InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerPhase, PointerSampleMetadata,
        time::Instant,
    };
    use incular_widgets::{SizedBox, Stack, internal::PointerDeviceKind};
    use std::time::Duration;

    let origins: [_; 2] = std::array::from_fn(|_| Rc::new(RefCell::new(None)));
    let leaves = origins.iter().enumerate().map(|(index, origin)| {
        let origin = origin.clone();
        let leaf = Builder::new(move |context| {
            *origin.borrow_mut() = Some(NotificationDispatcher::of(context));
            Semantics::new(Widget::box_(Size::new(800., 400.), Color::WHITE))
                .container(true)
                .label(format!("leaf {index}"))
                .into()
        });
        Widget::from(leaf).with_key(index as u64)
    });
    let rows = [
        AutomaticKeepAlive::new(SizedBox::new().height(400.).child(Stack::new(leaves))),
        AutomaticKeepAlive::new(Widget::from(SizedBox::new().height(400.)).with_key(2_u64)),
        AutomaticKeepAlive::new(Widget::from(SizedBox::new().height(400.)).with_key(3_u64)),
    ];
    let scroll = ListView::new(rows).cache_extent(0.);
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, scroll).into()).unwrap();
    let mut time = Instant::now();
    let check = |runtime: &Runtime, expected: [bool; 4], first_row_visible| {
        assert_eq!(
            std::array::from_fn(|index| runtime
                .tree()
                .element_with_key(&Key::from(index as u64))
                .is_some()),
            expected,
        );
        for label in ["leaf 0", "leaf 1"] {
            assert_eq!(
                runtime
                    .tree()
                    .semantics()
                    .iter()
                    .any(|(_, node)| node.label.as_deref() == Some(label)),
                first_row_visible,
            );
        }
    };
    let drag = |runtime: &mut Runtime, time: &mut Instant, pointer, dy: f32| {
        for (phase, y) in [
            (PointerPhase::Down, 300.),
            (PointerPhase::Move, 300. + dy.signum() * 20.),
            (PointerPhase::Move, 300. + dy),
            (PointerPhase::Up, 300. + dy),
        ] {
            let _ = runtime.handle_input_at(
                InputEvent::PointerWithMetadata {
                    pointer,
                    device: 0,
                    kind: PointerDeviceKind::Touch,
                    buttons: if phase == PointerPhase::Up {
                        0
                    } else {
                        PRIMARY_POINTER_BUTTON
                    },
                    button: (phase != PointerPhase::Move).then_some(PRIMARY_POINTER_BUTTON),
                    sample: PointerSampleMetadata::EMPTY,
                    phase,
                    position: Offset::new(400., y),
                },
                *time,
            );
        }
        *time += Duration::from_millis(16);
        runtime.run_frame_at(bounds, *time).unwrap();
    };
    runtime.run_frame_at(bounds, time).unwrap();
    check(&runtime, [true, true, true, false], true);
    drag(&mut runtime, &mut time, 1, -1000.);
    check(&runtime, [false, false, true, true], false);
    drag(&mut runtime, &mut time, 2, 1000.);
    check(&runtime, [true, true, true, false], true);
    let identities = std::array::from_fn::<_, 2, _>(|index| {
        runtime
            .tree()
            .element_with_key(&Key::from(index as u64))
            .unwrap()
    });
    let handle = |index: usize| {
        let handle = KeepAliveHandle::new();
        let origin = origins[index].borrow();
        assert!(
            !origin
                .as_ref()
                .unwrap()
                .dispatch(&KeepAliveNotification::new(handle.clone()))
        );
        assert!(handle.has_listeners());
        handle
    };
    let first = handle(0);
    drag(&mut runtime, &mut time, 3, -1000.);
    check(&runtime, [true, true, true, true], false);
    assert!(
        identities
            .iter()
            .all(|&id| runtime.tree().element_exists(id))
    );

    let second = handle(1);
    time += Duration::from_millis(16);
    runtime.run_frame_at(bounds, time).unwrap();
    check(&runtime, [true, true, true, true], false);

    first.release();
    time += Duration::from_millis(16);
    runtime.run_frame_at(bounds, time).unwrap();
    check(&runtime, [true, true, true, true], false);

    second.release();
    time += Duration::from_millis(16);
    runtime.run_frame_at(bounds, time).unwrap();
    check(&runtime, [false, false, true, true], false);
}
