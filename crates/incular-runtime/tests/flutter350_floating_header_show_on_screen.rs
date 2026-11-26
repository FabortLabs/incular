//! A floating persistent header can expand without moving its viewport.

use std::time::Duration;

use incular_animation::Curves;
use incular_config::{Axis, Constraints};
use incular_core::{Offset, Rect, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::{
    CustomScrollView, Directionality, PersistentHeaderShowOnScreenConfiguration, ScrollController,
    Scrollable, ShowOnScreen, SizedBox, Sliver, SliverHeaderScrollBehavior, SliverPersistentHeader,
    SliverToBoxAdapter, TextDirection, Widget,
};

fn bounds() -> Constraints {
    Constraints::tight(Size::new(400., 400.))
}

fn mount(
    axis: Axis,
    configuration: Option<PersistentHeaderShowOnScreenConfiguration>,
) -> (Runtime, ScrollController, Instant) {
    let controller = ScrollController::new().with_initial_scroll_offset(300.);
    let slivers = (0..20).map(|index| -> Box<dyn Sliver> {
        if index == 10 {
            let header = SliverPersistentHeader::with_delegate(100., 300., |_, _| {
                Widget::from(SizedBox::expand()).with_key("header")
            })
            .scroll_behavior(SliverHeaderScrollBehavior::FloatingPinned);
            Box::new(match configuration {
                Some(config) => header.show_on_screen_configuration(config),
                None => header,
            })
        } else {
            Box::new(SliverToBoxAdapter::new(SizedBox::square(300.)))
        }
    });
    let viewport = CustomScrollView::new(slivers)
        .scroll_direction(axis)
        .controller(controller.clone());
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, viewport).into()).unwrap();
    let now = Instant::now();
    runtime.run_frame_at(bounds(), now).unwrap();
    controller.jump_to(4500.);
    runtime
        .run_frame_at(bounds(), now + Duration::from_millis(16))
        .unwrap();
    assert_eq!(extent(&runtime, axis), 100.);
    assert_eq!(controller.offset(), 4500.);
    (runtime, controller, now)
}

fn reveal(runtime: &Runtime, origin: f32, extent: f32, duration: Duration) {
    let id = runtime.tree().element_with_key(&"header".into()).unwrap();
    let rect = Rect::from_origin_size(Offset::new(origin, origin), Size::new(extent, extent));
    assert_eq!(
        Scrollable::show_on_screen(
            runtime.tree(),
            id,
            ShowOnScreen::new()
                .rect(rect)
                .duration(duration)
                .curve(Curves::LINEAR)
        ),
        0
    );
}

fn extent(runtime: &Runtime, axis: Axis) -> f32 {
    let tree = runtime.tree();
    let id = tree.element_with_key(&"header".into()).unwrap();
    axis.main_extent(tree.element_bounds(id).unwrap().size)
}

fn pump(runtime: &mut Runtime, now: Instant, ms: u64) -> bool {
    runtime
        .run_frame_at(bounds(), now + Duration::from_millis(ms))
        .unwrap()
        .1
        .requested_another_frame
}

#[test]
fn configured_floating_header_reveals_full_height_without_scrolling() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        let (mut runtime, controller, now) = mount(axis, Some(Default::default()));
        reveal(&runtime, 0., 300., Duration::ZERO);
        pump(&mut runtime, now, 30);
        assert_eq!(extent(&runtime, axis), 300.);
        reveal(&runtime, -1., 300., Duration::ZERO);
        pump(&mut runtime, now, 46);
        assert_eq!(extent(&runtime, axis), 300.);
        assert_eq!(controller.offset(), 4500.);
    }
}

#[test]
fn second_reveal_does_not_retarget_a_running_expansion() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        let (mut runtime, controller, now) = mount(axis, Some(Default::default()));
        reveal(&runtime, 0., 150., Duration::from_secs(3));
        assert!(pump(&mut runtime, now, 32));
        assert!(pump(&mut runtime, now, 1032));
        let between = extent(&runtime, axis);
        let expected = 100. + 50. / 3.;
        assert!((between - expected).abs() < 0.0001, "{axis:?}: {between}");
        reveal(&runtime, 0., 300., Duration::ZERO);
        pump(&mut runtime, now, 3032);
        pump(&mut runtime, now, 3048);
        assert_eq!(extent(&runtime, axis), 150.);
        assert_eq!(controller.offset(), 4500.);
    }
}

#[test]
fn min_and_max_reveal_extents_never_contract_the_floating_header() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for (range, requested) in [
            ((f32::NEG_INFINITY, 200.), 300.),
            ((200., f32::INFINITY), 110.),
        ] {
            let configuration = PersistentHeaderShowOnScreenConfiguration::new(range.0, range.1);
            let (mut runtime, controller, now) = mount(axis, Some(configuration));
            for (origin, ms) in [(0., 32), (-1., 48)] {
                reveal(&runtime, origin, requested, Duration::ZERO);
                pump(&mut runtime, now, ms);
                assert_eq!(extent(&runtime, axis), 200., "{axis:?}: {range:?}");
                assert_eq!(controller.offset(), 4500.);
            }
            controller.jump_to(3050.);
            pump(&mut runtime, now, 64);
            assert_eq!(extent(&runtime, axis), 250.);
            reveal(&runtime, -1., requested, Duration::ZERO);
            pump(&mut runtime, now, 80);
            assert_eq!(extent(&runtime, axis), 250.);
            assert_eq!(controller.offset(), 3050.);
        }
    }
}

#[test]
fn a_floating_header_without_configuration_does_not_expand() {
    let (mut runtime, controller, now) = mount(Axis::Vertical, None);
    reveal(&runtime, 0., 300., Duration::ZERO);
    pump(&mut runtime, now, 32);
    assert_eq!(extent(&runtime, Axis::Vertical), 100.);
    assert_eq!(controller.offset(), 4500.);
}

#[test]
fn programmatic_reversal_does_not_float_but_an_active_user_drag_does() {
    use incular_core::{PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase};
    use incular_widgets::RawPointerEvent;

    for axis in [Axis::Vertical, Axis::Horizontal] {
        let (mut runtime, controller, now) = mount(axis, Some(Default::default()));
        controller.jump_to(4475.);
        pump(&mut runtime, now, 32);
        assert_eq!(extent(&runtime, axis), 100., "idle jump: {axis:?}");

        let center = Offset::new(200., 200.);
        let moved = center + axis.offset(25., 0.);
        for (phase, at, point) in [
            (PointerPhase::Down, 40, center),
            (PointerPhase::Move, 56, moved),
        ] {
            let _ = runtime.handle_raw_pointer_event(RawPointerEvent::new(
                1,
                0,
                PointerDeviceKind::Touch,
                PRIMARY_POINTER_BUTTON,
                point,
                phase,
                now + Duration::from_millis(at),
            ));
        }
        pump(&mut runtime, now, 56);
        assert_eq!(controller.offset(), 4450.);
        assert_eq!(extent(&runtime, axis), 125., "held drag: {axis:?}");
        let _ = runtime.handle_raw_pointer_event(RawPointerEvent::new(
            1,
            0,
            PointerDeviceKind::Touch,
            0,
            moved,
            PointerPhase::Up,
            now + Duration::from_millis(220),
        ));
        pump(&mut runtime, now, 220);
        assert_eq!(extent(&runtime, axis), 125.);
    }
}

#[test]
fn reveal_transforms_the_descendant_rect_through_a_wrapped_header() {
    use incular_config::EdgeInsets;
    use incular_widgets::{Padding, SliverOpacity};

    let ctl = ScrollController::new();
    let header = SliverPersistentHeader::with_delegate(100., 300., |_, _| {
        let leaf = Widget::from(SizedBox::expand()).with_key("padded descendant");
        Widget::from(Padding::new(EdgeInsets::only(0., 10., 0., 0.), leaf))
            .with_key("wrapped header")
    })
    .scroll_behavior(SliverHeaderScrollBehavior::FloatingPinned)
    .show_on_screen_configuration(PersistentHeaderShowOnScreenConfiguration::default());
    let slivers: [Box<dyn Sliver>; 3] = [
        Box::new(SliverToBoxAdapter::new(SizedBox::new().height(300.))),
        Box::new(SliverOpacity::new(0.5, header)),
        Box::new(SliverToBoxAdapter::new(SizedBox::new().height(700.))),
    ];
    let viewport = CustomScrollView::new(slivers).controller(ctl.clone());
    let mut rt = Runtime::new(Directionality::new(TextDirection::Ltr, viewport).into()).unwrap();
    let now = Instant::now();
    pump(&mut rt, now, 0);
    ctl.jump_to(900.);
    pump(&mut rt, now, 16);
    let tree = rt.tree();
    let header = tree.element_with_key(&"wrapped header".into()).unwrap();
    let leaf = tree.element_with_key(&"padded descendant".into()).unwrap();
    assert_eq!(tree.element_bounds(header).unwrap().size.height, 100.);
    assert_eq!(tree.element_bounds(leaf).unwrap().origin.y, 10.);

    assert_eq!(
        Scrollable::show_on_screen(
            tree,
            leaf,
            ShowOnScreen::new().rect(Rect::from_origin_size(Offset::ZERO, Size::new(100., 140.))),
        ),
        0
    );
    assert!(
        rt.frame_requested(),
        "presentation-only reveal schedules a frame"
    );
    pump(&mut rt, now, 32);
    assert_eq!(ctl.offset(), 900.);
    assert_eq!(rt.tree().element_bounds(header).unwrap().size.height, 150.);
}
