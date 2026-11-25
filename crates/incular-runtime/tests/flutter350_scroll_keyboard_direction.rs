//! Retained keyboard scroll actions from Flutter 3.50 scrollable_helpers_test.dart.

use std::time::Duration;

use incular_config::{Axis, Constraints, RuntimeEnvironment, TextDirection};
use incular_core::{
    Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Offset, Rect, Size, time::Instant,
};
use incular_runtime::Runtime;
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

fn compare(
    axis: Axis,
    ambient: TextDirection,
    environment: TextDirection,
    reverse: bool,
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
    let root = Directionality::new(
        ambient,
        CustomScrollView::new(rows)
            .controller(controller.clone())
            .scroll_direction(axis)
            .reverse(reverse),
    );
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
        if modifiers.ctrl() {
            let mut ctrl =
                KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Control), Code::ControlLeft);
            ctrl.modifiers = Modifiers::CONTROL;
            let _ = runtime.dispatch_keyboard_event(ctrl);
        }
        keyboard_key(&mut runtime, code, name, modifiers);
        if modifiers.ctrl() {
            let _ = runtime.dispatch_keyboard_event(KeyboardEvent::key_up(
                KeyboardKey::Named(NamedKey::Control),
                Code::ControlLeft,
            ));
        }
        for _ in 0..10 {
            now += Duration::from_millis(16);
            runtime.run_frame_at(view, now).unwrap();
        }
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
