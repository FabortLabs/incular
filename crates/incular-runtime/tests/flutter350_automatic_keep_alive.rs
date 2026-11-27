//! AutomaticKeepAlive requests travel from mounted descendants to the
//! sliver's cache. They are independent of the retained row's build.

use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::{Constraints, TextDirection};
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerPhase, PointerSampleMetadata, Size,
    time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    AutomaticKeepAlive, Builder, Directionality, GridView, KeepAliveHandle, KeepAliveNotification,
    ListView, NotificationDispatcher, ScrollController, Semantics, SliverGridDelegate, Widget,
    internal::{Key, PointerDeviceKind},
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

fn drag(runtime: &mut Runtime, bounds: Constraints, time: &mut Instant, pointer: u64, dy: f32) {
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
}

#[test]
fn flutter_original_double_keep_alive_holds_the_whole_row() {
    use incular_widgets::{SizedBox, Stack};

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
    let scroll = ListView::new(rows)
        .cache_extent(0.)
        .add_automatic_keep_alives(false);
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
    runtime.run_frame_at(bounds, time).unwrap();
    check(&runtime, [true, true, true, false], true);
    drag(&mut runtime, bounds, &mut time, 1, -1000.);
    check(&runtime, [false, false, true, true], false);
    drag(&mut runtime, bounds, &mut time, 2, 1000.);
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
    drag(&mut runtime, bounds, &mut time, 3, -1000.);
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

fn compare_original_grid_or_list(kind: ScrollKind, implied: bool) {
    use incular_widgets::GlobalKey;

    let origin = Rc::new(RefCell::new(None));
    let keys = [(); 100].map(|_| GlobalKey::new());
    let rows: Vec<_> = keys
        .iter()
        .enumerate()
        .map(|(index, &key)| {
            let origin = origin.clone();
            let element = Builder::new(move |context| {
                if index == 60 {
                    *origin.borrow_mut() = Some(NotificationDispatcher::of(context));
                }
                let height = if matches!(kind, ScrollKind::Grid) {
                    24.6
                } else {
                    12.3
                };
                let child = Widget::box_(Size::new(100., height), Color::WHITE);
                if index == 60 {
                    Semantics::new(child)
                        .container(true)
                        .label("retained row")
                        .into()
                } else {
                    child
                }
            });
            let leaf = Widget::from(element).with_key(key);
            if implied {
                leaf
            } else {
                AutomaticKeepAlive::new(leaf).into()
            }
        })
        .collect();
    let scroll: Widget = match kind {
        ScrollKind::Grid => GridView::new(
            SliverGridDelegate::fixed_cross_axis_count(2).child_aspect_ratio(400. / 24.6),
            rows,
        )
        .add_automatic_keep_alives(implied)
        .cache_extent(0.)
        .into(),
        _ => {
            let list = ListView::new(rows)
                .add_automatic_keep_alives(implied)
                .cache_extent(0.);
            if matches!(kind, ScrollKind::FixedList) {
                list.item_extent(12.3).into()
            } else {
                list.into()
            }
        }
    };
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, scroll).into()).unwrap();
    let mut time = Instant::now();
    let get =
        |runtime: &Runtime, index: usize| runtime.tree().element_with_key(&Key::from(keys[index]));
    let check = |runtime: &Runtime, present: &[usize], missing: &[usize]| {
        for &index in present {
            assert!(
                get(runtime, index).is_some(),
                "{kind:?} implied={implied}: missing {index}"
            );
        }
        for &index in missing {
            assert!(
                get(runtime, index).is_none(),
                "{kind:?} implied={implied}: unexpected {index}"
            );
        }
    };
    runtime.run_frame_at(bounds, time).unwrap();
    check(&runtime, &[3, 30], &[59, 60, 61, 90]);
    drag(&mut runtime, bounds, &mut time, 1, -300.);
    check(&runtime, &[30, 59, 60, 61], &[3, 90]);

    let id = get(&runtime, 60).unwrap();
    let handle = KeepAliveHandle::new();
    assert!(
        !origin
            .borrow()
            .as_ref()
            .unwrap()
            .dispatch(&KeepAliveNotification::new(handle.clone()))
    );
    assert!(handle.has_listeners());
    drag(&mut runtime, bounds, &mut time, 2, 300.);
    check(&runtime, &[3, 30, 60], &[59, 61, 90]);
    assert_parked(&runtime, &Key::from(keys[60]), id);

    handle.release();
    time += Duration::from_millis(16);
    runtime.run_frame_at(bounds, time).unwrap();
    check(&runtime, &[3, 30], &[59, 60, 61, 90]);
}

#[test]
fn flutter_original_fixed_list_explicit_and_implied_automatic_keep_alive() {
    for implied in [false, true] {
        compare_original_grid_or_list(ScrollKind::FixedList, implied);
    }
}

#[test]
fn flutter_original_variable_list_explicit_and_implied_automatic_keep_alive() {
    for implied in [false, true] {
        compare_original_grid_or_list(ScrollKind::NaturalList, implied);
    }
}

#[test]
fn flutter_original_grid_explicit_and_implied_automatic_keep_alive() {
    for implied in [false, true] {
        compare_original_grid_or_list(ScrollKind::Grid, implied);
    }
}

#[test]
fn flutter_original_implied_list_listener_detaches_when_called() {
    for enabled in [true, false] {
        let source = Rc::new(RefCell::new(None));
        let leaf = Builder::new({
            let source = source.clone();
            move |context| {
                *source.borrow_mut() = Some(NotificationDispatcher::of(context));
                Widget::box_(Size::new(50., 40.), Color::WHITE)
            }
        });
        let view = ListView::builder(1, move |_| Widget::from(leaf.clone()))
            .add_automatic_keep_alives(enabled);
        let bounds = Constraints::tight(Size::new(100., 100.));
        let mut runtime =
            Runtime::new(Directionality::new(TextDirection::Ltr, view).into()).unwrap();
        runtime.run_frame(bounds).unwrap();
        let handle = KeepAliveHandle::new();
        assert!(!handle.has_listeners());
        source
            .borrow()
            .as_ref()
            .unwrap()
            .dispatch(&KeepAliveNotification::new(handle.clone()));
        assert_eq!(handle.has_listeners(), enabled);
        handle.release();
        assert!(!handle.has_listeners());
    }
}

#[test]
fn flutter_original_initial_build_can_request_keep_alive_before_scroll() {
    use incular_core::DragStartBehavior;
    use incular_widgets::{GlobalKey, SizedBox, Text};

    let controller = ScrollController::new();
    let key = GlobalKey::new();
    let handle = KeepAliveHandle::new();
    let keep = handle.clone();
    let list = ListView::builder(50, move |index| {
        if index == 0 {
            let keep = keep.clone();
            return Widget::from(Builder::new(move |context| {
                KeepAliveNotification::new(keep.clone()).dispatch(context);
                let text =
                    Widget::from(Text::new("keep me alive")).with_key("retained first label");
                SizedBox::new().height(48.).child(text).into()
            }))
            .with_key(key);
        }
        SizedBox::new()
            .height(44.)
            .child(Text::new(format!("FooBar {index}")))
            .into()
    })
    .controller(controller.clone())
    .drag_start_behavior(DragStartBehavior::Down);
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, list).into()).unwrap();
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut time = Instant::now();
    let semantics_has = |runtime: &Runtime, label: &str| {
        runtime
            .tree()
            .semantics()
            .iter()
            .any(|(_, node)| node.label.as_deref() == Some(label))
    };
    assert!(!handle.has_listeners());
    runtime.run_frame_at(bounds, time).unwrap();
    let id = runtime.tree().element_with_key(&Key::from(key)).unwrap();
    let label_key = Key::from("retained first label");
    let label = runtime.tree().element_with_key(&label_key).unwrap();
    assert!(handle.has_listeners());
    for label in ["keep me alive", "FooBar 1", "FooBar 2"] {
        assert!(semantics_has(&runtime, label), "{label}");
    }
    drag(&mut runtime, bounds, &mut time, 1, -1000.);
    assert_eq!(runtime.tree().element_with_key(&Key::from(key)), Some(id));
    assert_eq!(runtime.tree().element_with_key(&label_key), Some(label));
    assert!(handle.has_listeners());
    for label in ["keep me alive", "FooBar 1", "FooBar 2"] {
        assert!(!semantics_has(&runtime, label), "{label}");
    }
}

#[test]
fn flutter_original_initial_keep_alive_survives_a_jump_past_many_rows() {
    use incular_widgets::{GlobalKey, SizedBox, Text};
    const COUNT: usize = 250;
    let keys = Rc::new([(); COUNT].map(|_| GlobalKey::new()));
    let controller = ScrollController::new();
    let list = ListView::builder(COUNT, {
        let keys = keys.clone();
        move |index| {
            let child: Widget = if index.is_multiple_of(2) {
                let handle = KeepAliveHandle::new();
                Builder::new(move |context| {
                    KeepAliveNotification::new(handle.clone()).dispatch(context);
                    SizedBox::new()
                        .height(48.)
                        .child(Text::new("keep me alive"))
                        .into()
                })
                .into()
            } else {
                SizedBox::new()
                    .height(44.)
                    .child(Text::new(format!("FooBar {index}")))
                    .into()
            };
            child.with_key(keys[index])
        }
    })
    .controller(controller.clone());
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, list).into()).unwrap();
    let bounds = Constraints::tight(Size::new(800., 600.));
    let retained =
        |runtime: &Runtime, index: usize| runtime.tree().element_with_key(&Key::from(keys[index]));
    let visible = |runtime: &Runtime, index: usize| {
        retained(runtime, index)
            .and_then(|id| runtime.tree().element_bounds(id))
            .is_some_and(|rect| rect.origin.y < 600. && rect.origin.y + rect.size.height > 0.)
    };
    runtime.run_frame(bounds).unwrap();
    assert_eq!(
        (0..COUNT)
            .step_by(2)
            .filter(|&index| visible(&runtime, index))
            .count(),
        7,
    );
    assert!(visible(&runtime, 1));
    assert!(visible(&runtime, 3));
    let first = retained(&runtime, 0).unwrap();

    assert!(controller.jump_to(3025.));
    runtime.run_frame(bounds).unwrap();
    assert_eq!(retained(&runtime, 0), Some(first));
    assert_eq!(
        (0..COUNT)
            .step_by(2)
            .filter(|&index| retained(&runtime, index).is_some())
            .count(),
        23,
    );
    assert!(!visible(&runtime, 1));
    assert!(!visible(&runtime, 3));
    assert!(visible(&runtime, 73));
}
