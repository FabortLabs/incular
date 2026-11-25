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
