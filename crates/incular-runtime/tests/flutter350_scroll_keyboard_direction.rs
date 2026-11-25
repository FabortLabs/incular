//! Retained keyboard scroll actions from Flutter 3.50 scrollable_helpers_test.dart.

use std::time::Duration;

use incular_config::{Axis, Constraints, RuntimeEnvironment, TextDirection};
use incular_core::{
    Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Offset, Rect, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_scroll::ScrollPhysics;
use incular_widgets::{
    CustomScrollView, Directionality, Focus, SizedBox, Sliver, SliverToBoxAdapter, Widget,
    internal::{Key, ScrollController},
};

fn keyboard_key(runtime: &mut Runtime, code: Code, name: NamedKey, modifiers: Modifiers) {
    let key = KeyboardKey::Named(name);
    let mut down = KeyboardEvent::key_down(key.clone(), code);
    let mut up = KeyboardEvent::key_up(key, code);
    down.modifiers = modifiers;
    up.modifiers = modifiers;
    let _ = runtime.dispatch_keyboard_event(down);
    let _ = runtime.dispatch_keyboard_event(up);
}

fn press_and_settle(
    runtime: &mut Runtime,
    view: Constraints,
    now: &mut Instant,
    code: Code,
    name: NamedKey,
    modifiers: Modifiers,
) {
    if modifiers.ctrl() {
        let mut ctrl =
            KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Control), Code::ControlLeft);
        ctrl.modifiers = Modifiers::CONTROL;
        let _ = runtime.dispatch_keyboard_event(ctrl);
    }
    keyboard_key(runtime, code, name, modifiers);
    if modifiers.ctrl() {
        let _ = runtime.dispatch_keyboard_event(KeyboardEvent::key_up(
            KeyboardKey::Named(NamedKey::Control),
            Code::ControlLeft,
        ));
    }
    for _ in 0..10 {
        *now += Duration::from_millis(16);
        runtime.run_frame_at(view, *now).unwrap();
    }
}

fn compare(
    axis: Axis,
    ambient: TextDirection,
    environment: TextDirection,
    reverse: bool,
    physics: Option<ScrollPhysics>,
    first_rect: Rect,
    steps: &[(Code, NamedKey, Modifiers, f32, Rect)],
) {
    let controller = ScrollController::new();
    let key = Key::from("Box 0");
    let rows = (0..20)
        .map(|index| {
            let mut box_ = SizedBox::new();
            if axis == Axis::Horizontal {
                box_ = box_.width(50.);
            } else {
                box_ = box_.height(50.);
            }
            let child = Widget::from(box_).with_key(Key::from(format!("Box {index}").as_str()));
            Box::new(SliverToBoxAdapter::new(
                Focus::new(child).autofocus(index == 0),
            )) as Box<dyn Sliver>
        })
        .collect::<Vec<_>>();
    let scroll = CustomScrollView::new(rows)
        .controller(controller.clone())
        .scroll_direction(axis)
        .reverse(reverse);
    let scroll = match physics {
        None => scroll,
        Some(physics) => scroll.physics(physics),
    };
    let root = Directionality::new(ambient, scroll);
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime.set_environment(RuntimeEnvironment {
        text_direction: environment,
        ..Default::default()
    });
    let view = Constraints::tight(Size::new(800., 600.));
    let mut now = Instant::now();
    runtime.run_frame_at(view, now).unwrap();
    now += Duration::from_millis(16);
    runtime.run_frame_at(view, now).unwrap();
    assert!(runtime.focused_element().is_some());
    let child = runtime.tree().element_with_key(&key).unwrap();
    assert_eq!(controller.offset(), 0.);
    assert_eq!(runtime.tree().element_bounds(child).unwrap(), first_rect);
    for &(code, name, modifiers, offset, bounds) in steps {
        press_and_settle(&mut runtime, view, &mut now, code, name, modifiers);
        assert_eq!(
            controller.offset(),
            offset,
            "{ambient:?} {environment:?} {code:?}"
        );
        assert_eq!(
            runtime.tree().element_bounds(child).unwrap(),
            bounds,
            "{ambient:?} {environment:?} {code:?}"
        );
    }
    if physics == Some(ScrollPhysics::NEVER_SCROLLABLE) {
        assert!(controller.jump_to(80.));
        runtime
            .run_frame_at(view, now + Duration::from_millis(16))
            .unwrap();
        assert_eq!(controller.offset(), 80.);
        assert_eq!(runtime.tree().element_bounds(child).unwrap(), v(-80.));
    }
}

fn h(x: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, 0.), Size::new(50., 600.))
}

#[test]
fn horizontal_ltr_keyboard_uses_its_physical_axis() {
    for environment in [TextDirection::Ltr, TextDirection::Rtl] {
        compare(
            Axis::Horizontal,
            TextDirection::Ltr,
            environment,
            false,
            None,
            h(0.),
            &[
                (
                    Code::ArrowRight,
                    NamedKey::ArrowRight,
                    Modifiers::CONTROL,
                    50.,
                    h(-50.),
                ),
                (
                    Code::ArrowLeft,
                    NamedKey::ArrowLeft,
                    Modifiers::CONTROL,
                    0.,
                    h(0.),
                ),
            ],
        );
    }
}

#[test]
fn horizontal_rtl_keyboard_uses_its_physical_axis() {
    for environment in [TextDirection::Ltr, TextDirection::Rtl] {
        compare(
            Axis::Horizontal,
            TextDirection::Rtl,
            environment,
            false,
            None,
            h(750.),
            &[
                (
                    Code::ArrowLeft,
                    NamedKey::ArrowLeft,
                    Modifiers::CONTROL,
                    50.,
                    h(800.),
                ),
                (
                    Code::ArrowRight,
                    NamedKey::ArrowRight,
                    Modifiers::CONTROL,
                    0.,
                    h(750.),
                ),
            ],
        );
    }
}

fn v(y: f32) -> Rect {
    Rect::from_origin_size(Offset::new(0., y), Size::new(800., 50.))
}

#[test]
fn reversed_vertical_keyboard_scrolls_by_a_line_and_a_clamped_page() {
    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
        compare(
            Axis::Vertical,
            direction,
            direction,
            true,
            None,
            v(550.),
            &[
                (
                    Code::ArrowUp,
                    NamedKey::ArrowUp,
                    Modifiers::CONTROL,
                    50.,
                    v(600.),
                ),
                (
                    Code::ArrowDown,
                    NamedKey::ArrowDown,
                    Modifiers::CONTROL,
                    0.,
                    v(550.),
                ),
                (
                    Code::PageUp,
                    NamedKey::PageUp,
                    Modifiers::empty(),
                    400.,
                    v(950.),
                ),
                (
                    Code::PageDown,
                    NamedKey::PageDown,
                    Modifiers::empty(),
                    0.,
                    v(550.),
                ),
            ],
        );
    }
}

#[test]
fn explicit_reverse_flips_the_resolved_horizontal_keyboard_direction() {
    compare(
        Axis::Horizontal,
        TextDirection::Ltr,
        TextDirection::Rtl,
        true,
        None,
        h(750.),
        &[
            (
                Code::ArrowLeft,
                NamedKey::ArrowLeft,
                Modifiers::CONTROL,
                50.,
                h(800.),
            ),
            (
                Code::ArrowRight,
                NamedKey::ArrowRight,
                Modifiers::CONTROL,
                0.,
                h(750.),
            ),
        ],
    );
    compare(
        Axis::Horizontal,
        TextDirection::Rtl,
        TextDirection::Rtl,
        true,
        None,
        h(0.),
        &[
            (
                Code::ArrowRight,
                NamedKey::ArrowRight,
                Modifiers::CONTROL,
                50.,
                h(-50.),
            ),
            (
                Code::ArrowLeft,
                NamedKey::ArrowLeft,
                Modifiers::CONTROL,
                0.,
                h(0.),
            ),
        ],
    );
}

#[test]
fn never_scrollable_ignores_keyboard_line_and_page_but_default_scrolls() {
    let steps = [
        (
            Code::ArrowDown,
            NamedKey::ArrowDown,
            Modifiers::CONTROL,
            50.,
            v(-50.),
        ),
        (
            Code::ArrowUp,
            NamedKey::ArrowUp,
            Modifiers::CONTROL,
            0.,
            v(0.),
        ),
        (
            Code::PageDown,
            NamedKey::PageDown,
            Modifiers::empty(),
            400.,
            v(-400.),
        ),
        (
            Code::PageUp,
            NamedKey::PageUp,
            Modifiers::empty(),
            0.,
            v(0.),
        ),
    ];
    compare(
        Axis::Vertical,
        TextDirection::Ltr,
        TextDirection::Ltr,
        false,
        None,
        v(0.),
        &steps,
    );
    compare(
        Axis::Vertical,
        TextDirection::Ltr,
        TextDirection::Ltr,
        false,
        Some(ScrollPhysics::NEVER_SCROLLABLE),
        v(0.),
        &steps.map(|(code, key, modifiers, _, _)| (code, key, modifiers, 0., v(0.))),
    );
}

#[test]
fn keyboard_crosses_center_with_signed_offsets_and_clamps_at_the_end() {
    let controller = ScrollController::new();
    let slivers = (0..20).map(|i| {
        let child = Widget::from(SizedBox::new().height(100.)).with_key(Key::from(i as u64));
        Box::new(SliverToBoxAdapter::new(
            Focus::new(child).autofocus(i == 10),
        )) as Box<dyn Sliver>
    });
    let mut runtime = Runtime::new(
        Directionality::new(
            TextDirection::Ltr,
            CustomScrollView::new(slivers)
                .center(10)
                .controller(controller.clone()),
        )
        .into(),
    )
    .unwrap();
    let view = Constraints::tight(Size::new(800., 600.));
    let mut time = Instant::now();
    runtime.run_frame_at(view, time).unwrap();
    time += Duration::from_millis(16);
    runtime.run_frame_at(view, time).unwrap();
    assert!(runtime.focused_element().is_some());
    let centered = runtime.tree().element_with_key(&Key::from(10_u64)).unwrap();
    let check = |runtime: &Runtime, offset: f32| {
        assert_eq!(controller.offset(), offset);
        assert_eq!(
            runtime.tree().element_bounds(centered).unwrap(),
            Rect::from_origin_size(Offset::new(0., -offset), Size::new(800., 100.))
        );
    };
    check(&runtime, 0.);
    for (code, key, target) in [
        (Code::ArrowDown, NamedKey::ArrowDown, 400.),
        (Code::ArrowUp, NamedKey::ArrowUp, -100.),
    ] {
        for _ in 0..10 {
            press_and_settle(&mut runtime, view, &mut time, code, key, Modifiers::CONTROL);
        }
        check(&runtime, target);
    }
}
