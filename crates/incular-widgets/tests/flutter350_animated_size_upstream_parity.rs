//! Flutter 3.50.0 animated_size_test.dart public sizes, paint and lifecycle.
use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_rendering::PaintCommand;
use incular_widgets::{
    AnimatedSize, SizedBox, Text, TextDirection, Widget,
    internal::{ElementId, WidgetTree},
};
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

fn widget(side: f32, on_end: Rc<Cell<usize>>) -> Widget {
    AnimatedSize::new(
        Duration::from_millis(200),
        SizedBox::new().width(side).height(side),
    )
    .on_end(move || on_end.set(on_end.get() + 1))
    .into()
}

struct Fixture {
    tree: WidgetTree,
    root: ElementId,
    epoch: Instant,
    constraints: Constraints,
    end_calls: Rc<Cell<usize>>,
}

impl Fixture {
    fn new(side: f32, constraints: Constraints) -> Self {
        let mut tree = WidgetTree::new();
        let end_calls = Rc::new(Cell::new(0));
        let root = tree.mount(widget(side, end_calls.clone())).unwrap();
        let mut probe = Self {
            tree,
            root,
            epoch: Instant::now(),
            constraints,
            end_calls,
        };
        probe.frame(0);
        probe.frame(16);
        probe
    }

    fn frame(&mut self, ms: u64) -> Size {
        self.tree
            .update_compositor(self.epoch + Duration::from_millis(ms))
            .unwrap();
        self.tree.layout(self.constraints).unwrap();
        self.tree.element_bounds(self.root).unwrap().size
    }

    fn update(&mut self, side: f32, ms: u64) {
        self.tree
            .update(self.root, widget(side, self.end_calls.clone()))
            .unwrap();
        self.frame(ms);
        self.frame(ms); // Start pending controller on the same logical instant.
    }

    fn has_clip(&mut self) -> bool {
        self.tree
            .paint()
            .commands()
            .iter()
            .any(|cmd| matches!(cmd, PaintCommand::PushClip { .. }))
    }
}

const LARGE: Size = Size {
    width: 800.,
    height: 600.,
};

#[test]
fn stable_child_grows_then_shrinks_and_clips_only_when_it_overflows() {
    let mut probe = Fixture::new(100., Constraints::loose(LARGE));
    assert_eq!(probe.frame(24), Size::new(100., 100.));
    probe.update(200., 1000);
    let grow_half = probe.frame(1100);
    let grow_clip = probe.has_clip();
    let grew = probe.frame(1200);
    assert!(!probe.has_clip(), "the settled child fits exactly");
    probe.update(100., 1300);
    let shrink_half = probe.frame(1400);
    let shrink_clip = probe.has_clip();
    let shrank = probe.frame(1500);

    assert_eq!(
        [grow_half, grew, shrink_half, shrank],
        [
            Size::new(150., 150.),
            Size::new(200., 200.),
            Size::new(150., 150.),
            Size::new(100., 100.)
        ]
    );
    assert_eq!(
        [grow_clip, shrink_clip],
        [true, false],
        "clip only if the child overflows"
    );
}

#[test]
fn each_completed_size_change_calls_on_end_once() {
    let mut probe = Fixture::new(100., Constraints::loose(LARGE));
    assert_eq!(probe.end_calls.get(), 0);
    probe.update(200., 1000);
    assert_eq!(probe.end_calls.get(), 0);
    probe.frame(1100);
    probe.frame(1200);
    probe.frame(1216);
    assert_eq!(probe.end_calls.get(), 1);
    probe.update(100., 1300);
    probe.frame(1400);
    probe.frame(1500);
    probe.frame(1516);
    assert_eq!(probe.end_calls.get(), 2);
}

#[test]
fn animated_size_does_not_exceed_its_tight_outer_box() {
    let size = Size::new(100., 100.);
    let mut probe = Fixture::new(100., Constraints::tight(size));
    assert_eq!(probe.frame(32), size);
    probe.update(200., 1000);
    assert_eq!(probe.frame(1100), size);
}

#[test]
fn unchanged_size_has_no_activation_or_completion_over_twenty_frames() {
    let mut probe = Fixture::new(100., Constraints::loose(LARGE));
    for i in 1..=20 {
        let start = 100 + 10 * i;
        assert_eq!(probe.frame(start), Size::new(100., 100.));
        assert!(
            !probe
                .tree
                .tick_animations(probe.epoch + Duration::from_millis(start)),
            "unchanged child started an animation at frame {i}"
        );
    }
    assert_eq!(probe.end_calls.get(), 0);
}

#[test]
fn animated_size_can_lay_out_a_text_child_at_zero_size() {
    let animated = AnimatedSize::new(
        Duration::from_millis(300),
        Text::new("X").text_direction(TextDirection::Ltr),
    );
    let mut tree = WidgetTree::new();
    let root = tree.mount(animated.into()).unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    let _ = tree.paint();
    assert_eq!(tree.element_bounds(root).unwrap().size, Size::ZERO);
}

#[test]
fn clip_none_paints_while_the_larger_child_overflows() {
    use incular_config::Clip;
    let side = |value| {
        Widget::from(
            AnimatedSize::new(
                Duration::from_millis(200),
                Widget::box_(Size::new(value, value), Color::WHITE),
            )
            .clip_behavior(Clip::None),
        )
    };
    let mut probe = Fixture::new(100., Constraints::loose(LARGE));
    probe.tree.update(probe.root, side(100.)).unwrap();
    probe.frame(32);
    probe.tree.update(probe.root, side(200.)).unwrap();
    probe.frame(1000);
    probe.frame(1000);
    assert_eq!(probe.frame(1100), Size::new(150., 150.));
    assert!(!probe.has_clip());
}

#[test]
fn animated_size_baseline_includes_its_childs_animated_vertical_offset() {
    use incular_config::Alignment;
    use incular_widgets::internal::Key;
    let child = Key::Value(91);
    let build = |large| -> Widget {
        let content = if large {
            Widget::from(
                SizedBox::new()
                    .width(100.)
                    .height(80.)
                    .child(Text::new("X").text_direction(TextDirection::Ltr)),
            )
        } else {
            Widget::from(SizedBox::new().width(100.).height(20.))
        }
        .with_key(child.clone());
        AnimatedSize::new(Duration::from_millis(200), content)
            .alignment(Alignment::BOTTOM_LEFT)
            .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(false)).unwrap();
    let mut probe = Fixture {
        tree,
        root,
        epoch: Instant::now(),
        constraints: Constraints::loose(LARGE),
        end_calls: Rc::new(Cell::new(0)),
    };
    probe.frame(0);
    let baseline = |t: &WidgetTree, element| t.baseline(t.render_id(element).unwrap());
    assert_eq!(baseline(&probe.tree, probe.root), None);
    probe.tree.update(probe.root, build(true)).unwrap();
    probe.frame(1000);
    probe.frame(1000);
    assert_eq!(probe.frame(1100), Size::new(100., 50.));
    let element = probe.tree.element_with_key(&child).unwrap();
    let own_y = probe.tree.element_bounds(probe.root).unwrap().origin.y;
    let child_y = probe.tree.element_bounds(element).unwrap().origin.y;
    assert_eq!(child_y - own_y, -30.);
    assert_eq!(
        baseline(&probe.tree, probe.root),
        Some(baseline(&probe.tree, element).unwrap() + child_y - own_y),
    );
}

#[test]
fn animated_size_inside_intrinsic_height_and_wrap_keeps_the_live_tween() {
    use incular_widgets::{Center, IntrinsicHeight, Wrap, internal::Key};

    let key = Key::from("intrinsic height probe");
    let make = |width, height| -> Widget {
        let animated = AnimatedSize::new(
            Duration::from_millis(200),
            SizedBox::new().width(width).height(height),
        )
        .curve(incular_animation::Curves::EASE_IN_OUT_BACK);
        Center::new(
            Widget::from(IntrinsicHeight::new(
                Wrap::new([animated]).text_direction(TextDirection::Ltr),
            ))
            .with_key(key.clone()),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(make(100., 100.)).unwrap();
    let epoch = Instant::now();
    let frame = |tree: &mut WidgetTree, millis| {
        tree.update_compositor(epoch + Duration::from_millis(millis))
            .unwrap();
        tree.layout(Constraints::tight(LARGE)).unwrap();
        tree.element_bounds(tree.element_with_key(&key).unwrap())
            .unwrap()
            .size
    };
    assert_eq!(frame(&mut tree, 0), Size::new(100., 100.));
    tree.update(root, make(150., 200.)).unwrap();
    assert_eq!(frame(&mut tree, 1_000), Size::new(100., 100.));
    frame(&mut tree, 1_000);
    for elapsed in (10..=190).step_by(10) {
        frame(&mut tree, 1_000 + elapsed);
    }
    assert_eq!(frame(&mut tree, 1_200), Size::new(150., 200.));

    tree.update(root, make(100., 100.)).unwrap();
    assert_eq!(frame(&mut tree, 1_200), Size::new(150., 200.));
    tree.update(root, make(111., 111.)).unwrap();
    assert_eq!(frame(&mut tree, 1_210), Size::new(111., 111.));
    tree.update(root, make(222., 222.)).unwrap();
    assert_eq!(frame(&mut tree, 1_220), Size::new(222., 222.));
}

#[test]
fn directional_alignment_mirrors_during_a_size_change_without_restarting_it() {
    use incular_config::AlignmentDirectional;
    use incular_core::Offset;
    use incular_widgets::{Center, Directionality, internal::Key};

    let outer = Key::from("animated size");
    let child = Key::from("animated child");
    let ended = Rc::new(Cell::new(0));
    let make = |direction, side| -> Widget {
        let animated = AnimatedSize::new(
            Duration::from_millis(200),
            Widget::from(SizedBox::new().width(side).height(side)).with_key(child.clone()),
        )
        .clip_behavior(incular_config::Clip::None)
        .alignment(AlignmentDirectional::CENTER_START)
        .on_end({
            let ended = ended.clone();
            move || ended.set(ended.get() + 1)
        });
        Directionality::new(
            direction,
            Center::new(Widget::from(animated).with_key(outer.clone())),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(make(TextDirection::Ltr, 100.)).unwrap();
    let epoch = Instant::now();
    let frame = |tree: &mut WidgetTree, millis| {
        tree.update_compositor(epoch + Duration::from_millis(millis))
            .unwrap();
        tree.layout(Constraints::tight(LARGE)).unwrap();
        let outer = tree
            .element_bounds(tree.element_with_key(&outer).unwrap())
            .unwrap();
        let child = tree
            .element_bounds(tree.element_with_key(&child).unwrap())
            .unwrap();
        (outer.size, child.origin - outer.origin)
    };
    assert_eq!(frame(&mut tree, 0), (Size::new(100., 100.), Offset::ZERO));
    tree.update(root, make(TextDirection::Ltr, 200.)).unwrap();
    frame(&mut tree, 1_000);
    frame(&mut tree, 1_000);
    assert_eq!(
        frame(&mut tree, 1_100),
        (Size::new(150., 150.), Offset::new(0., -25.))
    );
    tree.update(root, make(TextDirection::Rtl, 200.)).unwrap();
    assert_eq!(
        frame(&mut tree, 1_100),
        (Size::new(150., 150.), Offset::new(-50., -25.))
    );
    assert_eq!(ended.get(), 0);
    assert_eq!(
        frame(&mut tree, 1_200),
        (Size::new(200., 200.), Offset::ZERO)
    );
    assert_eq!(ended.get(), 0);
    frame(&mut tree, 1_201);
    assert_eq!(ended.get(), 1);
}

#[test]
fn clip_defaults_to_hard_edge_and_all_four_modes_update_the_same_retained_layout() {
    use incular_config::Clip;
    use incular_widgets::internal::RenderKind;

    let mut tree = WidgetTree::new();
    let make = |clip: Option<Clip>| -> Widget {
        let animated = AnimatedSize::new(
            Duration::from_millis(200),
            SizedBox::new().width(100.).height(100.),
        );
        match clip {
            Some(clip) => animated.clip_behavior(clip),
            None => animated,
        }
        .into()
    };
    let root = tree.mount(make(None)).unwrap();
    let constraints = Constraints::loose(LARGE);
    tree.layout(constraints).unwrap();
    let find_layout = |tree: &WidgetTree| {
        let mut candidates = vec![root];
        while let Some(element) = candidates.pop() {
            if let Some(render_id) = tree.render_id(element)
                && let Some(RenderKind::CustomLayout { delegate }) =
                    tree.render_object_kind(render_id)
                && let Some(clip) = delegate.overflow_clip()
            {
                return (render_id, clip);
            }
            candidates.extend(tree.children(element).unwrap_or_default());
        }
        panic!("animated size layout not mounted")
    };
    let (render, initial_clip) = find_layout(&tree);
    assert_eq!(initial_clip, Clip::HardEdge);
    for clip in [
        Clip::None,
        Clip::HardEdge,
        Clip::AntiAlias,
        Clip::AntiAliasWithSaveLayer,
    ] {
        tree.update(root, make(Some(clip))).unwrap();
        tree.layout(constraints).unwrap();
        assert_eq!(find_layout(&tree), (render, clip));
    }
}

#[test]
fn unstable_child_is_followed_each_frame_then_size_animates_again() {
    use incular_widgets::{AnimatedContainer, Center, internal::Key};
    let key = Key::from("animated size following an animated child");
    let build = |side, duration| {
        Center::new(
            Widget::from(AnimatedSize::new(
                Duration::from_millis(200),
                AnimatedContainer::new(Duration::from_millis(duration))
                    .width(side)
                    .height(side),
            ))
            .with_key(key.clone()),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(100., 100)).unwrap();
    let epoch = Instant::now();
    let frame = |tree: &mut WidgetTree, millis| {
        tree.update_compositor(epoch + Duration::from_millis(millis))
            .unwrap();
        tree.layout(Constraints::tight(LARGE)).unwrap();
        tree.element_bounds(tree.element_with_key(&key).unwrap())
            .unwrap()
            .size
    };
    assert_eq!(frame(&mut tree, 0), Size::new(100., 100.));
    tree.update(root, build(200., 100)).unwrap();
    assert_eq!(frame(&mut tree, 1_000), Size::new(100., 100.));
    frame(&mut tree, 1_000);
    frame(&mut tree, 1_001);
    assert_eq!(frame(&mut tree, 1_050), Size::new(150., 150.));
    assert_eq!(frame(&mut tree, 1_100), Size::new(200., 200.));
    assert_eq!(frame(&mut tree, 1_150), Size::new(200., 200.));

    tree.update(root, build(100., 1)).unwrap();
    assert_eq!(frame(&mut tree, 1_200), Size::new(200., 200.));
    frame(&mut tree, 1_200);
    frame(&mut tree, 1_201);
    frame(&mut tree, 1_201);
    assert_eq!(frame(&mut tree, 1_301), Size::new(150., 150.));
    assert_eq!(frame(&mut tree, 1_401), Size::new(100., 100.));
}

#[test]
fn reparenting_while_growing_preserves_retained_element_and_in_flight_size() {
    use incular_widgets::{Center, Directionality, GlobalKey, Row, internal::Key};

    let subtree = GlobalKey::new();
    let size_key = Key::from("reparented animated size");
    let a = Key::from("first row child");
    let b = Key::from("second row child");

    let build = |side: f32, first: bool| -> Widget {
        let animated = Widget::from(AnimatedSize::new(
            Duration::from_secs(1),
            SizedBox::new().width(side).height(side),
        ))
        .with_key(size_key.clone());
        let wrapper = Widget::from(Center::new(
            SizedBox::square(200.).child(Center::new(animated)),
        ))
        .with_key(subtree);
        let left = SizedBox::new().height(200.);
        let right = SizedBox::new().height(200.);
        let (left, right) = if first {
            (left.child(wrapper), right)
        } else {
            (left, right.child(wrapper))
        };
        Directionality::new(
            TextDirection::Ltr,
            Row::new([
                Widget::from(left).with_key(a.clone()),
                Widget::from(right).with_key(b.clone()),
            ]),
        )
        .into()
    };

    let mut tree = WidgetTree::new();
    let root = tree.mount(build(100., true)).unwrap();
    let epoch = Instant::now();
    let frame = |tree: &mut WidgetTree, millis| {
        tree.update_compositor(epoch + Duration::from_millis(millis))
            .unwrap();
        tree.layout(Constraints::tight(LARGE)).unwrap();
        tree.element_bounds(tree.element_with_key(&size_key).unwrap())
            .unwrap()
            .size
    };
    assert_eq!(frame(&mut tree, 0), Size::new(100., 100.));
    let retained = tree.element_with_key(&Key::Global(subtree)).unwrap();
    let size_element = tree.element_with_key(&size_key).unwrap();
    let original_parent = tree.parent(retained);

    tree.update(root, build(150., true)).unwrap();
    assert_eq!(frame(&mut tree, 1_000), Size::new(100., 100.));
    tree.update(root, build(150., false)).unwrap();
    assert_eq!(frame(&mut tree, 1_000), Size::new(100., 100.));
    assert_eq!(tree.element_with_key(&Key::Global(subtree)), Some(retained));
    assert_eq!(tree.element_with_key(&size_key), Some(size_element));
    assert_ne!(tree.parent(retained), original_parent);
    assert_eq!(frame(&mut tree, 1_500), Size::new(125., 125.));
    assert_eq!(frame(&mut tree, 2_001), Size::new(150., 150.));
}

fn animated_size_controller(
    tree: &WidgetTree,
    root: ElementId,
) -> incular_animation::AnimationController {
    use incular_widgets::internal::RenderKind;

    let mut pending = vec![root];
    while let Some(element) = pending.pop() {
        if let Some(RenderKind::AnimationTicker { controller, .. }) = tree
            .render_id(element)
            .and_then(|render| tree.render_object_kind(render))
        {
            return controller.clone();
        }
        pending.extend(tree.children(element).unwrap_or_default());
    }
    panic!("the AnimatedSize ticker was not found");
}

#[test]
fn unmount_disposes_the_controller_even_if_debug_and_descriptor_handles_survive() {
    use incular_widgets::Center;

    let description = Widget::from(AnimatedSize::new(
        Duration::from_millis(200),
        SizedBox::square(100.),
    ));
    let mut tree = WidgetTree::new();
    let root = tree.mount(Center::new(description.clone()).into()).unwrap();
    let constraints = Constraints::tight(LARGE);
    tree.layout(constraints).unwrap();
    let controller = animated_size_controller(&tree, root);
    assert!(!controller.is_disposed());

    tree.update(root, Center::new(SizedBox::shrink()).into())
        .unwrap();
    tree.layout(constraints).unwrap();
    assert!(controller.is_disposed());
    let error = controller.try_dispose().unwrap_err();
    assert!(
        error
            .to_string()
            .contains("dispose() called more than once")
    );

    tree.update(root, Center::new(description).into()).unwrap();
    tree.layout(constraints).unwrap();
    assert!(!animated_size_controller(&tree, root).is_disposed());
}

#[test]
fn a_descriptor_cloned_for_multiple_mounts_does_not_share_size_controllers() {
    use incular_widgets::Center;

    let description = Widget::from(AnimatedSize::new(
        Duration::from_millis(200),
        SizedBox::square(100.),
    ));
    let mut first = WidgetTree::new();
    let mut second = WidgetTree::new();
    let root1 = first
        .mount(Center::new(description.clone()).into())
        .unwrap();
    let root2 = second.mount(Center::new(description).into()).unwrap();
    let constraints = Constraints::tight(LARGE);
    first.layout(constraints).unwrap();
    second.layout(constraints).unwrap();
    let first_controller = animated_size_controller(&first, root1);
    let second_controller = animated_size_controller(&second, root2);
    first
        .update(root1, Center::new(SizedBox::shrink()).into())
        .unwrap();
    first.layout(constraints).unwrap();
    assert!(first_controller.is_disposed());
    assert!(!second_controller.is_disposed());
    assert_eq!(second.element_bounds(root2).unwrap().size, LARGE);
}

#[test]
fn width_only_size_change_ticks_on_the_existing_controller() {
    let build = |width| {
        AnimatedSize::new(
            Duration::from_millis(200),
            SizedBox::new().width(width).height(100.),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(100.)).unwrap();
    let epoch = Instant::now();
    let frame = |tree: &mut WidgetTree, millis| {
        tree.update_compositor(epoch + Duration::from_millis(millis))
            .unwrap();
        tree.layout(Constraints::loose(LARGE)).unwrap();
        tree.element_bounds(root).unwrap().size
    };
    assert_eq!(frame(&mut tree, 0), Size::new(100., 100.));
    tree.update(root, build(200.)).unwrap();
    frame(&mut tree, 1_000);
    frame(&mut tree, 1_000);
    assert_eq!(frame(&mut tree, 1_100), Size::new(150., 100.));
}

#[test]
fn dropping_the_tree_also_disposes_the_mounted_controller() {
    let description = Widget::from(AnimatedSize::new(
        Duration::from_millis(200),
        SizedBox::square(100.),
    ));
    let controller = {
        let mut tree = WidgetTree::new();
        let root = tree.mount(description.clone()).unwrap();
        tree.layout(Constraints::loose(LARGE)).unwrap();
        animated_size_controller(&tree, root)
    };
    assert!(controller.is_disposed());
}
