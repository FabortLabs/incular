use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Rect, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Center, Positioned, RawTooltip, RawTooltipController, SizedBox, Stack,
    TooltipComponentBuilder, TooltipPositionContext, Widget, WidgetsApp,
};
use std::{cell::Cell, rc::Rc, time::Duration};

const VIEW: Size = Size {
    width: 800.0,
    height: 600.0,
};

fn frame(runtime: &mut Runtime, at: Instant) {
    runtime.run_frame_at(Constraints::tight(VIEW), at).unwrap();
}

fn tooltip_builder(height: f32) -> TooltipComponentBuilder {
    TooltipComponentBuilder::new(move || {
        Widget::box_(Size::new(100.0, height), Color::BLACK).with_key("measured tooltip")
    })
}

fn tooltip_bounds(runtime: &Runtime) -> Rect {
    let element = runtime
        .tree()
        .element_with_key(&"measured tooltip".into())
        .expect("tooltip is mounted");
    runtime.tree().element_bounds(element).unwrap()
}

#[test]
fn raw_tooltip_default_and_preferred_geometry_matches_seven_flutter_positions() {
    for (x, y, height, prefer_below, expected_x, expected_y) in [
        (300., 0., 30., true, 250., 0.),
        (0., 0., 30., true, 10., 0.),
        (400., 300., 100., false, 350., 200.),
        (400., 299., 190., false, 350., 109.),
        (400., 300., 190., true, 350., 300.),
        (1600., 300., 20., true, 690., 300.),
        (780., 300., 20., true, 690., 300.),
    ] {
        let controller = RawTooltipController::new();
        let tooltip = RawTooltip::with_builder(SizedBox::shrink(), tooltip_builder(height))
            .controller(controller.clone())
            .prefer_below(prefer_below);
        let anchor = Positioned::builder().left(x).top(y).child(tooltip).build();
        let mut runtime = Runtime::new(WidgetsApp::new(Stack::new([anchor])).into()).unwrap();
        let now = Instant::now();
        frame(&mut runtime, now);
        assert!(controller.show_at(now + Duration::from_millis(10)));
        frame(&mut runtime, now + Duration::from_millis(2000));

        assert_eq!(
            tooltip_bounds(&runtime),
            Rect::from_origin_size(Offset::new(expected_x, expected_y), Size::new(100., height)),
            "anchor=({x}, {y}), height={height}, below={prefer_below}"
        );
    }
}

#[test]
fn pressed_tooltip_delegate_receives_the_measured_context_and_owns_placement() {
    let recorded = Rc::new(Cell::new(None::<TooltipPositionContext>));
    let tooltip = RawTooltip::with_builder(
        Widget::box_(Size::new(50., 50.), Color::WHITE),
        tooltip_builder(14.),
    )
    .position_delegate({
        let recorded = recorded.clone();
        move |info| {
            recorded.set(Some(info));
            Offset::new(
                info.target.x + info.target_size.width / 2.,
                info.target.y - info.target_size.height / 2. - info.tooltip_size.height,
            )
        }
    });
    let mut runtime = Runtime::new(WidgetsApp::new(Center::new(tooltip)).into()).unwrap();
    let start = Instant::now();
    frame(&mut runtime, start);
    let send = |runtime: &mut Runtime, phase, ms| {
        runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 7,
                device: 7,
                kind: PointerDeviceKind::Touch,
                buttons: if phase == PointerPhase::Down {
                    PRIMARY_POINTER_BUTTON
                } else {
                    0
                },
                button: Some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::EMPTY,
                phase,
                position: Offset::new(400., 300.),
            },
            start + Duration::from_millis(ms),
        );
    };
    send(&mut runtime, PointerPhase::Down, 10);
    frame(&mut runtime, start + Duration::from_millis(510));
    send(&mut runtime, PointerPhase::Up, 530);
    frame(&mut runtime, start + Duration::from_millis(1000));
    assert_eq!(
        recorded.get(),
        Some(TooltipPositionContext {
            target: Offset::new(400., 300.),
            target_size: Size::new(50., 50.),
            tooltip_size: Size::new(100., 14.),
            overlay_size: VIEW,
            prefer_below: true,
            vertical_offset: 0.,
        })
    );
    assert_eq!(tooltip_bounds(&runtime).origin, Offset::new(425., 261.));
}

#[test]
fn explicitly_anchored_tooltips_still_follow_the_incular_anchor_offsets() {
    use incular_widgets::Alignment;

    let controller = RawTooltipController::new();
    let tooltip = RawTooltip::with_builder(SizedBox::square(50.), tooltip_builder(20.))
        .controller(controller.clone())
        .position_anchors(
            Alignment::BOTTOM_RIGHT,
            Alignment::TOP_LEFT,
            Offset::new(8., 4.),
        );
    let mut runtime = Runtime::new(Center::new(tooltip).into()).unwrap();
    let now = Instant::now();
    frame(&mut runtime, now);
    assert!(controller.show_at(now + Duration::from_millis(10)));
    frame(&mut runtime, now + Duration::from_millis(20));
    assert_eq!(tooltip_bounds(&runtime).origin, Offset::new(433., 329.));
}

#[test]
fn ignore_pointer_tooltip_hides_when_the_mouse_leaves_its_anchor_for_the_bubble() {
    use incular_widgets::Text;

    for ignore_pointer in [true, false] {
        let tooltip = RawTooltip::with_builder(Text::new("Hover me"), tooltip_builder(40.))
            .ignore_pointer(ignore_pointer);
        let mut runtime = Runtime::new(WidgetsApp::new(Center::new(tooltip)).into()).unwrap();
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        frame(&mut runtime, at(0));
        let hover = |runtime: &mut Runtime, position, ms| {
            runtime.handle_input_at(
                InputEvent::PointerWithMetadata {
                    pointer: 1,
                    device: 1,
                    kind: PointerDeviceKind::Mouse,
                    buttons: 0,
                    button: None,
                    sample: PointerSampleMetadata::EMPTY,
                    phase: PointerPhase::Move,
                    position,
                },
                at(ms),
            );
        };
        hover(&mut runtime, Offset::ZERO, 10);
        hover(&mut runtime, Offset::new(400., 300.), 20);
        frame(&mut runtime, at(120));
        let bubble = tooltip_bounds(&runtime);
        let center = bubble.origin + Offset::new(bubble.size.width / 2., bubble.size.height / 2.);
        hover(&mut runtime, center, 130);
        frame(&mut runtime, at(140));
        frame(&mut runtime, at(500));
        frame(&mut runtime, at(1000));
        assert_eq!(
            runtime.tree().element_with_key(&"measured tooltip".into()).is_none(),
            ignore_pointer
        );
    }
}

#[test]
fn raw_tooltip_mounts_at_zero_viewport_size() {
    use incular_widgets::Text;

    let tooltip = Widget::from(RawTooltip::with_builder(
        Text::new("X"),
        TooltipComponentBuilder::new(|| Text::new("Y").into()),
    ))
    .with_key("zero size anchor");
    let mut runtime = Runtime::new(WidgetsApp::new(Center::new(tooltip)).into()).unwrap();
    runtime.run_frame(Constraints::tight(Size::ZERO)).unwrap();
    let element = runtime.tree().element_with_key(&"zero size anchor".into()).unwrap();
    assert_eq!(runtime.tree().element_bounds(element).unwrap().size, Size::ZERO);
}

#[test]
fn long_pressed_tooltip_paragraph_inherits_each_layout_direction() {
    use incular_widgets::{Directionality, Text, TextDirection, internal::RenderKind};

    let scene = |direction| {
        let tooltip = RawTooltip::with_builder(
            Widget::box_(Size::new(100., 100.), Color::rgba(0, 255, 0, 255)),
            TooltipComponentBuilder::new(|| {
                Widget::from(Text::new("A tooltip")).with_key("tooltip paragraph")
            }),
        )
        .semantics_tooltip("A tooltip");
        WidgetsApp::new(Directionality::new(direction, Center::new(tooltip))).into()
    };
    let mut runtime = Runtime::new(scene(TextDirection::Rtl)).unwrap();
    let start = Instant::now();
    frame(&mut runtime, start);
    let press = |runtime: &mut Runtime, at: Instant, id| {
        let send = |runtime: &mut Runtime, phase, offset| {
            runtime.handle_input_at(
                InputEvent::PointerWithMetadata {
                    pointer: id,
                    device: id,
                    kind: PointerDeviceKind::Touch,
                    buttons: if phase == PointerPhase::Down {
                        PRIMARY_POINTER_BUTTON
                    } else {
                        0
                    },
                    button: Some(PRIMARY_POINTER_BUTTON),
                    sample: PointerSampleMetadata::EMPTY,
                    phase,
                    position: Offset::new(400., 300.),
                },
                at + Duration::from_millis(offset),
            );
        };
        send(runtime, PointerPhase::Down, 0);
        frame(runtime, at + Duration::from_millis(501));
        send(runtime, PointerPhase::Up, 510);
        frame(runtime, at + Duration::from_millis(520));
    };
    let inspect = |runtime: &Runtime, expected| {
        let tree = runtime.tree();
        let text = tree.element_with_key(&"tooltip paragraph".into()).unwrap();
        let render = tree.render_id(text).unwrap();
        let Some(RenderKind::Text { text_direction, .. }) = tree.render_object_kind(render)
        else {
            panic!("expected the actual mounted tooltip paragraph");
        };
        assert_eq!(*text_direction, expected);
        assert!(tree.text_layout(render).is_some());
    };
    press(&mut runtime, start + Duration::from_millis(10), 31);
    inspect(&runtime, TextDirection::Rtl);
    frame(&mut runtime, start + Duration::from_secs(10));
    assert!(runtime.tree().element_with_key(&"tooltip paragraph".into()).is_none());
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, scene(TextDirection::Ltr)).unwrap();
    frame(&mut runtime, start + Duration::from_millis(10001));
    press(&mut runtime, start + Duration::from_secs(11), 32);
    inspect(&runtime, TextDirection::Ltr);
}
