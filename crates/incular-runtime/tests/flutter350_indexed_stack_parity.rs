//! IndexedStack paint, input and retained layout probes from pinned Flutter 3.50.0.
use incular_config::{Constraints, StackFit};
use incular_core::{
    Code, Color, InputEvent, KeyboardEvent, KeyboardKey, Offset, PRIMARY_POINTER_BUTTON,
    PointerDeviceKind, PointerPhase, PointerSampleMetadata, Rect, Size,
};
use incular_rendering::DisplayList;
use incular_runtime::Runtime;
use incular_widgets::{
    Center, CustomPaint, CustomPainter, Directionality, FocusNode, GestureDetector, IndexedStack,
    KeyboardListener, LayoutBuilder, Positioned, SizedBox, Text, TextDirection, Widget,
    internal::Key,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
const STACK: u64 = 99;
fn stack(
    children: impl IntoIterator<Item = impl Into<Widget>>,
    index: impl Into<Option<usize>>,
) -> Widget {
    Widget::from(IndexedStack::new(children).index(index)).with_key(Key::Value(STACK))
}
fn scene(w: impl Into<Widget>) -> Widget {
    Directionality::new(TextDirection::Ltr, w).into()
}
fn host(w: impl Into<Widget>) -> Runtime {
    let mut r = Runtime::new(scene(w)).unwrap();
    frame(&mut r);
    r
}
fn frame(r: &mut Runtime) {
    r.run_frame(Constraints::tight(VIEW)).unwrap();
}
fn change(r: &mut Runtime, w: impl Into<Widget>) {
    let root = r.tree().root().unwrap();
    r.schedule_update(root, scene(w)).unwrap();
    frame(r);
}
fn bounds(r: &Runtime, i: u64) -> Rect {
    let t = r.tree();
    t.element_bounds(t.element_with_key(&Key::Value(i)).unwrap())
        .unwrap()
}
fn tap(r: &mut Runtime, n: u64, p: Offset) {
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = r.handle_input(InputEvent::PointerWithMetadata {
            pointer: n,
            device: 0,
            kind: PointerDeviceKind::Touch,
            buttons: if phase == PointerPhase::Down {
                PRIMARY_POINTER_BUTTON
            } else {
                0
            },
            button: Some(PRIMARY_POINTER_BUTTON),
            sample: PointerSampleMetadata::EMPTY,
            phase,
            position: p,
        });
    }
    frame(r);
}
fn box_() -> Widget {
    Widget::box_(Size::new(50., 50.), Color::WHITE)
}
#[test]
fn empty_root_mounts_without_exception() {
    host(stack(Vec::<Widget>::new(), 0));
}
#[test]
fn empty_centered_stack_mounts_without_exception() {
    host(Center::new(stack(Vec::<Widget>::new(), 0)));
}

#[test]
fn null_index_keeps_two_shrink_children_without_an_error() {
    let r = host(stack([SizedBox::new(), SizedBox::new()], None));
    assert_eq!(bounds(&r, STACK).size, VIEW);
    assert_eq!(
        IndexedStack::new(Vec::<Widget>::new()).get_selected_index(),
        Some(0)
    );
    assert_eq!(
        IndexedStack::builder()
            .index(None)
            .build()
            .get_selected_index(),
        None
    );
    assert_eq!(
        IndexedStack::builder()
            .index(1)
            .build()
            .get_selected_index(),
        Some(1)
    );
}

#[test]
fn null_index_still_sizes_to_child_but_cannot_receive_a_tap() {
    let taps = Rc::new(Cell::new(0));
    let child = GestureDetector::new(Widget::box_(Size::new(200., 200.), Color::WHITE)).on_tap({
        let taps = taps.clone();
        move || taps.set(taps.get() + 1)
    });
    let make = |index| Center::new(stack([child.clone()], index));
    let mut r = host(make(None));
    let visible_bounds = bounds(&r, STACK);
    assert_eq!(visible_bounds.size, Size::new(200., 200.));
    let center = visible_bounds.origin + Offset::new(100., 100.);
    tap(&mut r, 1, center);
    assert_eq!(taps.get(), 0);
    change(&mut r, make(Some(0)));
    tap(&mut r, 2, center);
    assert_eq!(taps.get(), 1, "selected branch is an active control");
    change(&mut r, make(None));
    tap(&mut r, 3, center);
    assert_eq!(
        taps.get(),
        1,
        "hiding again removes the retained hit target"
    );
}

struct Trace(usize, Rc<RefCell<Vec<usize>>>);
impl CustomPainter for Trace {
    fn paint(&self, _: Size) -> DisplayList {
        self.1.borrow_mut().push(self.0);
        DisplayList::default()
    }
    fn should_repaint(&self, _: &Self) -> bool {
        true
    }
}
#[test]
fn active_only_paints_and_other_children_remain_mounted() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let make = |i| {
        Center::new(stack(
            (0..3).map(|k| {
                Widget::from(
                    CustomPaint::new(Size::ZERO, DisplayList::default())
                        .painter(Trace(k, trace.clone()))
                        .child(Text::new(k.to_string())),
                )
                .with_key(Key::Value(k as u64))
            }),
            i,
        ))
    };
    let mut r = host(make(0));
    let mut actual = vec![std::mem::take(&mut *trace.borrow_mut())];
    for i in 1..3 {
        change(&mut r, make(i));
        actual.push(std::mem::take(&mut *trace.borrow_mut()));
        for k in 0..3 {
            assert!(r.tree().element_with_key(&Key::Value(k)).is_some());
        }
    }
    assert_eq!(actual, [vec![0], vec![1], vec![2]]);
}

#[test]
fn dirty_hidden_painter_runs_only_when_selected_and_then_reuses_picture() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let painter = |slot, label| {
        Widget::from(
            CustomPaint::new(Size::new(20., 20.), DisplayList::default())
                .painter(Trace(label, trace.clone())),
        )
        .with_key(Key::Value(slot))
    };
    let first = painter(0, 0);
    let old_hidden = painter(1, 1);
    let latest_hidden = painter(1, 11);
    let make = |index, second: &Widget| Center::new(stack([first.clone(), second.clone()], index));
    let mut r = host(make(0, &old_hidden));
    assert_eq!(trace.take(), [0]);
    let hidden_element = r.tree().element_with_key(&Key::Value(1)).unwrap();

    change(&mut r, make(0, &latest_hidden));
    assert!(trace.take().is_empty());
    assert_eq!(
        r.tree().element_with_key(&Key::Value(1)),
        Some(hidden_element)
    );

    change(&mut r, make(1, &latest_hidden));
    assert_eq!(trace.take(), [11]);
    change(&mut r, make(0, &latest_hidden));
    change(&mut r, make(1, &latest_hidden));
    assert!(trace.take().is_empty(), "unchanged pictures are reused");
}
#[test]
fn an_overlapping_tap_reaches_only_the_selected_index() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let make = |i| {
        Center::new(stack(
            (0..3).map(|k| {
                GestureDetector::new(box_()).on_tap({
                    let log = log.clone();
                    move || log.borrow_mut().push(k)
                })
            }),
            i,
        ))
    };
    let mut r = host(make(0));
    for (n, i) in [0, 2].into_iter().enumerate() {
        if n != 0 {
            change(&mut r, make(i));
        }
        let b = bounds(&r, STACK);
        tap(&mut r, n as u64 + 1, b.origin + Offset::new(25., 25.));
        assert_eq!(std::mem::take(&mut *log.borrow_mut()), [i]);
    }
}
#[test]
fn hidden_gesture_child_cannot_receive_pointer_events() {
    let taps = Rc::new(Cell::new(0));
    let make = |i| {
        stack(
            [
                box_(),
                Widget::from(GestureDetector::new(box_()).on_tap({
                    let taps = taps.clone();
                    move || taps.set(taps.get() + 1)
                }))
                .with_key(Key::Value(1)),
            ],
            i,
        )
    };
    let mut r = host(make(0));
    let target = bounds(&r, 1).origin + Offset::new(20., 20.);
    tap(&mut r, 1, target);
    let hidden = taps.get();
    change(&mut r, make(1));
    tap(&mut r, 2, target);
    assert_eq!([hidden, taps.get()], [0, 1]);
}
#[test]
fn programmatic_focus_stays_with_visible_child_and_routes_keys_to_it() {
    let nodes = [FocusNode::new(), FocusNode::new()];
    let presses = Rc::new(RefCell::new(Vec::new()));
    let make = |i| {
        stack(
            (0..2).map(|k| {
                Widget::from(
                    KeyboardListener::new(Text::new(k.to_string()))
                        .focus_node(nodes[k].clone())
                        .on_key_down({
                            let presses = presses.clone();
                            move |_| presses.borrow_mut().push(k)
                        }),
                )
                .with_key(Key::Value(k as u64))
            }),
            i,
        )
    };
    let mut r = host(make(0));
    let mut actual = Vec::new();
    for (step, i) in [0, 1, 1, 0].into_iter().enumerate() {
        if step == 2 {
            change(&mut r, make(1));
        }
        nodes[i].request_focus();
        frame(&mut r);
        let k = KeyboardKey::Character("a".into());
        let _ = r.handle_input(InputEvent::Key(KeyboardEvent::key_down(
            k.clone(),
            Code::KeyA,
        )));
        let _ = r.handle_input(InputEvent::Key(KeyboardEvent::key_up(k, Code::KeyA)));
        actual.push([nodes[0].has_focus(), nodes[1].has_focus()]);
    }
    assert_eq!(
        (actual, std::mem::take(&mut *presses.borrow_mut())),
        (
            vec![[true, false], [true, false], [false, true], [false, true]],
            vec![0, 0, 1, 1]
        )
    );
}
#[test]
fn positioned_child_keeps_its_explicit_origin_and_extent() {
    let child = Widget::from(SizedBox::new()).with_key(Key::Value(1));
    let p = Positioned::new(child)
        .left(10.)
        .top(20.)
        .width(30.)
        .height(40.);
    let control = host(incular_widgets::Stack::new([
        Widget::from(p.clone()),
        box_(),
    ]));
    let normal = bounds(&control, 1);
    assert_eq!(
        (normal.origin, normal.size),
        (Offset::new(10., 20.), Size::new(30., 40.))
    );
    let r = host(stack([Widget::from(p), box_()], 0));
    let b = bounds(&r, 1);
    assert_eq!(
        (b.origin, b.size),
        (Offset::new(10., 20.), Size::new(30., 40.))
    );
}
#[test]
fn positive_index_equal_to_child_count_is_rejected() {
    let attempt = std::panic::catch_unwind(|| {
        let mut r = Runtime::new(scene(stack([SizedBox::new(), SizedBox::new()], 2)))?;
        r.run_frame(Constraints::tight(VIEW)).map(|_| ())
    });
    assert!(
        !matches!(attempt, Ok(Ok(()))),
        "an out-of-range selection reached layout successfully"
    );
}
#[test]
fn each_fit_passes_its_distinct_constraints_to_the_child() {
    use incular_widgets::{ConstrainedBox, Placeholder};
    let outer = Constraints::new(2., 3., 5., 7.);
    let observed = Rc::new(RefCell::new(Vec::new()));
    let make = |fit| {
        Center::new(ConstrainedBox::new(
            outer,
            IndexedStack::new([LayoutBuilder::new({
                let observed = observed.clone();
                move |_, c| {
                    observed.borrow_mut().push(c);
                    Placeholder::new().into()
                }
            })])
            .fit(fit),
        ))
    };
    let mut r = host(make(StackFit::Loose));
    change(&mut r, make(StackFit::Expand));
    change(&mut r, make(StackFit::Passthrough));
    assert_eq!(
        *observed.borrow(),
        [
            Constraints::new(0., 3., 0., 7.),
            Constraints::tight(Size::new(3., 7.)),
            outer
        ]
    );
}
#[test]
fn retained_clip_changes_from_hard_edge_to_antialias() {
    use incular_config::Clip;
    use incular_widgets::internal::RenderKind;
    let make = |clip| {
        Widget::from(IndexedStack::new(Vec::<Widget>::new()).clip_behavior(clip))
            .with_key(Key::Value(STACK))
    };
    let mut r = host(stack(Vec::<Widget>::new(), 0));
    let inspect = |r: &Runtime| {
        let t = r.tree();
        let id = t
            .render_id(t.element_with_key(&Key::Value(STACK)).unwrap())
            .unwrap();
        let RenderKind::IndexedStack { clip_behavior, .. } = t.render_object_kind(id).unwrap()
        else {
            panic!("expected retained IndexedStack")
        };
        (id, *clip_behavior)
    };
    let (id, before) = inspect(&r);
    change(&mut r, make(Clip::AntiAlias));
    assert_eq!(before, Clip::HardEdge);
    assert_eq!(inspect(&r), (id, Clip::AntiAlias));
}

#[test]
fn display_list_contains_only_selected_child_color() {
    use incular_rendering::PaintCommand;
    let colors = [
        Color::rgba(211, 21, 31, 255),
        Color::rgba(17, 201, 61, 255),
        Color::rgba(23, 49, 219, 255),
    ];
    let make = |i| {
        Center::new(stack(
            colors.map(|c| Widget::box_(Size::new(50., 50.), c)),
            i,
        ))
    };
    let selection = |list: &DisplayList| {
        list.commands()
            .iter()
            .filter_map(|cmd| match cmd {
                PaintCommand::Rect { color, .. } if colors.contains(color) => Some(*color),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    for (i, color) in colors.into_iter().enumerate() {
        let mut cold = Runtime::new(scene(make(i))).unwrap();
        let (list, _) = cold.run_frame(Constraints::tight(VIEW)).unwrap();
        assert_eq!(selection(&list), [color], "fresh mount selecting {i}");
    }
    let mut r = Runtime::new(scene(make(0))).unwrap();
    let mut observed = Vec::new();
    for i in 0..3 {
        if i != 0 {
            let root = r.tree().root().unwrap();
            r.schedule_update(root, scene(make(i))).unwrap();
        }
        let (list, _) = r.run_frame(Constraints::tight(VIEW)).unwrap();
        observed.push(selection(&list));
    }
    assert_eq!(observed, colors.map(|color| vec![color]));
}

#[test]
fn hidden_focus_widget_cannot_claim_focus_via_external_node() {
    let nodes = [FocusNode::new(), FocusNode::new()];
    let make = |i| {
        stack(
            (0..2).map(|k| {
                incular_widgets::Focus::new(Text::new(k.to_string())).node(nodes[k].clone())
            }),
            i,
        )
    };
    let mut r = host(make(0));
    let mut actual = Vec::new();
    for (step, i) in [0, 1, 1, 0].into_iter().enumerate() {
        if step == 2 {
            change(&mut r, make(1));
        }
        nodes[i].request_focus();
        frame(&mut r);
        actual.push([nodes[0].has_focus(), nodes[1].has_focus()]);
    }
    assert_eq!(
        actual,
        [[true, false], [true, false], [false, true], [false, true]]
    );
}

#[test]
fn deselecting_detaches_retained_paint_and_focus_but_preserves_the_child() {
    use incular_rendering::PaintCommand;
    let marker = Color::rgba(23, 49, 219, 255);
    let node = FocusNode::new();
    let child = Widget::from(
        incular_widgets::Focus::new(Widget::box_(Size::new(30., 20.), marker)).node(node.clone()),
    )
    .with_key(Key::Value(1));
    let make = |index| Center::new(stack([child.clone()], index));
    let mut r = Runtime::new(scene(make(Some(0)))).unwrap();
    let contains_marker = |list: &DisplayList| {
        list.commands()
            .iter()
            .any(|command| matches!(command, PaintCommand::Rect { color, .. } if *color == marker))
    };
    let (first, _) = r.run_frame(Constraints::tight(VIEW)).unwrap();
    assert!(contains_marker(&first));
    let child_id = r.tree().element_with_key(&Key::Value(1)).unwrap();
    node.request_focus();
    frame(&mut r);
    assert!(node.has_focus());

    let root = r.tree().root().unwrap();
    r.schedule_update(root, scene(make(None))).unwrap();
    let (hidden, _) = r.run_frame(Constraints::tight(VIEW)).unwrap();
    assert!(!contains_marker(&hidden));
    assert!(!node.has_focus());
    node.request_focus();
    frame(&mut r);
    assert!(!node.has_focus());
    assert_eq!(r.tree().element_with_key(&Key::Value(1)), Some(child_id));

    r.schedule_update(root, scene(make(Some(0)))).unwrap();
    let (restored, _) = r.run_frame(Constraints::tight(VIEW)).unwrap();
    assert!(contains_marker(&restored));
    assert_eq!(r.tree().element_with_key(&Key::Value(1)), Some(child_id));
}
