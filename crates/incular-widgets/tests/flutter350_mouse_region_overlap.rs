use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Offset, PointerPhase, Size};
use incular_gestures::{PointerDeviceKind, PointerEvent, RawPointerEvent};
use incular_widgets::{
    AbsorbPointer, Directionality, GestureDetector, HitTestBehavior, IgnorePointer, MouseRegion,
    Positioned, Stack, Widget, internal::WidgetTree,
};

fn check(behavior: HitTestBehavior, opaque: bool, expected_hover: &[&str], expected_taps: usize) {
    let hovered = Rc::new(RefCell::new(Vec::new()));
    let taps = Rc::new(Cell::new(0));
    let behind = GestureDetector::new(Widget::box_(Size::new(60., 60.), Color::WHITE))
        .behavior(HitTestBehavior::Opaque)
        .on_tap({
            let taps = taps.clone();
            move || taps.set(taps.get() + 1)
        });
    let region = |label, child: Widget| {
        MouseRegion::new(child).on_enter({
            let hovered = hovered.clone();
            move |_| hovered.borrow_mut().push(label)
        })
    };
    let front = GestureDetector::new(IgnorePointer::new(Widget::box_(
        Size::new(50., 50.),
        Color::WHITE,
    )))
    .behavior(HitTestBehavior::DeferToChild)
    .on_tap(|| {});
    let root = Directionality::new(
        TextDirection::Ltr,
        Stack::new([
            Widget::from(
                Positioned::new(region("behind", behind.into()))
                    .left(0.)
                    .top(0.),
            ),
            Widget::from(
                Positioned::new(
                    region("front", front.into())
                        .opaque(opaque)
                        .hit_test_behavior(behavior),
                )
                .left(20.)
                .top(20.),
            ),
        ]),
    );
    let mut tree = WidgetTree::new();
    tree.mount(root.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .unwrap();

    let point = Offset::new(25., 25.);
    let start = Instant::now();
    for (time, position) in [
        (start, Offset::new(80., 80.)),
        (start + Duration::from_millis(1), point),
    ] {
        tree.dispatch_raw_pointer(RawPointerEvent::new(
            1,
            1,
            PointerDeviceKind::Mouse,
            0,
            position,
            PointerPhase::Move,
            time,
        ));
    }
    assert_eq!(hovered.borrow().as_slice(), expected_hover);
    for (phase, ms) in [(PointerPhase::Down, 10), (PointerPhase::Up, 20)] {
        tree.dispatch_gesture(PointerEvent {
            pointer: 2,
            position: point,
            phase,
            time: start + Duration::from_millis(ms),
        });
    }
    assert_eq!(taps.get(), expected_taps);
}

#[test]
fn deferring_opaque_region_does_not_hit_if_its_child_declines() {
    check(HitTestBehavior::DeferToChild, true, &["behind"], 1);
}

#[test]
fn accepting_opaque_region_blocks_the_hover_and_tap_below() {
    check(HitTestBehavior::Opaque, true, &["front"], 0);
}

#[test]
fn nonopaque_mouse_region_does_not_preempt_the_behind_tap() {
    check(HitTestBehavior::Opaque, false, &["behind", "front"], 1);
}

#[test]
fn translucent_mouse_region_can_observe_without_taking_the_tap() {
    check(HitTestBehavior::Translucent, true, &["behind", "front"], 1);
}

fn opacity_log(opaque: Option<bool>, expected: &[&[&str]]) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let region = |name: &'static str, builder: MouseRegion| {
        let enter = log.clone();
        let hover = log.clone();
        let exit = log.clone();
        builder
            .on_enter(move |_| enter.borrow_mut().push(format!("enter{name}")))
            .on_hover(move |_| hover.borrow_mut().push(format!("hover{name}")))
            .on_exit(move |_| exit.borrow_mut().push(format!("exit{name}")))
    };
    let front = match opaque {
        Some(value) => MouseRegion::without_child().opaque(value),
        None => MouseRegion::without_child(),
    };
    let positioned = |offset, child| -> Widget {
        Positioned::new(child)
            .left(offset)
            .top(offset)
            .width(80.)
            .height(80.)
            .into()
    };
    let scene = incular_widgets::Align::new(
        incular_config::Alignment::TOP_LEFT,
        region(
            "A",
            MouseRegion::new(incular_widgets::SizedBox::from_dimensions(
                Some(150.),
                Some(150.),
                Some(Stack::new([
                    positioned(20., region("B", MouseRegion::without_child())),
                    positioned(50., region("C", front)),
                ])),
            )),
        ),
    );

    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, scene).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let start = Instant::now();
    let mut points = vec![(75., 75.)];
    if opaque.is_some() {
        points.extend([(25., 75.), (75., 75.), (125., 75.), (75., 75.)]);
    }
    points.push((160., 160.));
    for (i, ((x, y), expected)) in points.into_iter().zip(expected).enumerate() {
        tree.dispatch_raw_pointer(RawPointerEvent::new(
            1,
            1,
            PointerDeviceKind::Mouse,
            0,
            Offset::new(x, y),
            PointerPhase::Move,
            start + Duration::from_millis(i as u64),
        ));
        let actual = std::mem::take(&mut *log.borrow_mut());
        assert_eq!(&actual, expected, "at ({x}, {y})");
    }
}

#[test]
fn nonopaque_region_allows_surrounding_hover_callbacks_in_hit_order() {
    opacity_log(
        Some(false),
        &[
            &["enterA", "enterB", "enterC", "hoverC", "hoverB", "hoverA"],
            &["exitC", "hoverB", "hoverA"],
            &["enterC", "hoverC", "hoverB", "hoverA"],
            &["exitB", "hoverC", "hoverA"],
            &["enterB", "hoverC", "hoverB", "hoverA"],
            &["exitC", "exitB", "exitA"],
        ],
    );
}

#[test]
fn opaque_region_hides_siblings_but_not_ancestors() {
    opacity_log(
        Some(true),
        &[
            &["enterA", "enterC", "hoverC", "hoverA"],
            &["exitC", "enterB", "hoverB", "hoverA"],
            &["exitB", "enterC", "hoverC", "hoverA"],
            &["hoverC", "hoverA"],
            &["hoverC", "hoverA"],
            &["exitC", "exitA"],
        ],
    );
}

#[test]
fn mouse_region_opacity_defaults_to_true() {
    opacity_log(
        None,
        &[
            &["enterA", "enterC", "hoverC", "hoverA"],
            &["exitC", "exitA"],
        ],
    );
}

#[test]
fn an_empty_opaque_region_still_blocks_a_region_below_it() {
    let called = Rc::new(Cell::new(false));
    let bottom = {
        let enter = called.clone();
        let hover = called.clone();
        let exit = called.clone();
        MouseRegion::new(Widget::box_(Size::new(10., 10.), Color::WHITE))
            .on_enter(move |_| enter.set(true))
            .on_hover(move |_| hover.set(true))
            .on_exit(move |_| exit.set(true))
    };
    let mut tree = WidgetTree::new();
    tree.mount(
        Directionality::new(
            TextDirection::Ltr,
            Stack::new([
                Widget::from(incular_widgets::Align::new(
                    incular_config::Alignment::TOP_LEFT,
                    bottom,
                )),
                MouseRegion::without_child().into(),
            ]),
        )
        .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    let start = Instant::now();
    for (i, p) in [(20., 20.), (5., 5.), (20., 20.)].into_iter().enumerate() {
        tree.dispatch_raw_pointer(RawPointerEvent::new(
            1,
            1,
            PointerDeviceKind::Mouse,
            0,
            Offset::new(p.0, p.1),
            PointerPhase::Move,
            start + Duration::from_millis(i as u64),
        ));
    }
    assert!(!called.get());
}

#[test]
fn an_absorbing_overlay_blocks_only_its_half_open_box() {
    let taps = Rc::new(Cell::new(0));
    let behind = GestureDetector::new(Widget::box_(Size::new(100., 100.), Color::WHITE))
        .behavior(HitTestBehavior::Opaque)
        .on_tap({
            let taps = taps.clone();
            move || taps.set(taps.get() + 1)
        });
    let scene = Stack::new([
        Widget::from(behind),
        Positioned::new(AbsorbPointer::new(Widget::box_(
            Size::new(40., 30.),
            Color::WHITE,
        )))
        .left(10.)
        .top(50.)
        .into(),
    ]);
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, scene).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    let now = Instant::now();
    for (i, (x, y, expected)) in [
        (20., 20., 1),
        (10., 50., 1),
        (49.99, 79.99, 1),
        (50., 60., 2),
        (20., 80., 3),
        (9.99, 60., 4),
    ]
    .into_iter()
    .enumerate()
    {
        for (phase, step) in [(PointerPhase::Down, 0), (PointerPhase::Up, 10)] {
            tree.dispatch_gesture(PointerEvent {
                pointer: (i + 1) as u64,
                position: Offset::new(x, y),
                phase,
                time: now + Duration::from_millis(30 * i as u64 + step),
            });
        }
        assert_eq!(taps.get(), expected, "at ({x}, {y})");
    }
}
