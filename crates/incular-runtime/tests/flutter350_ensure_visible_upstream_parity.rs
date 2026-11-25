//! Flutter 3.50.0 ensure_visible_test.dart, exact SingleChildScrollView bounds.
use incular_config::{Axis, Constraints};
use incular_core::{Color, Rect, Size, Transform as Affine, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::{
    Center, Directionality, EnsureVisible, ListBody, ScrollController,
    ScrollPositionAlignmentPolicy, Scrollable, SingleChildScrollView, SizedBox, TextDirection,
    Transform, Widget,
    internal::{ElementId, Key},
};
use std::time::Duration;

struct Fixture {
    runtime: Runtime,
    now: Instant,
    controller: ScrollController,
    axis: Axis,
}

impl Fixture {
    fn new(axis: Axis, reverse: bool, rotated: bool) -> Self {
        let controller = ScrollController::new();
        let children = (0u64..7).map(|i| -> Widget {
            let leaf = || Widget::from(SizedBox::new().width(200.).height(200.));
            if !rotated {
                return leaf().with_key(Key::Value(i));
            }
            if i != 3 {
                return leaf();
            }
            let marker = Widget::box_(Size::new(100., 100.), Color::WHITE).with_key(Key::Value(0));
            SizedBox::new()
                .height(200.)
                .child(Center::new(Transform::new(
                    Affine::rotation(std::f32::consts::PI),
                    marker,
                )))
                .into()
        });
        let scene = Directionality::new(
            TextDirection::Ltr,
            Center::new(
                SizedBox::new().width(600.).height(400.).child(
                    SingleChildScrollView::new(ListBody::new(children).main_axis(axis))
                        .scroll_direction(axis)
                        .reverse(reverse)
                        .controller(controller.clone()),
                ),
            ),
        );
        let mut probe = Self {
            runtime: Runtime::new(scene.into()).unwrap(),
            now: Instant::now(),
            controller,
            axis,
        };
        probe.frame(0);
        probe
    }

    fn frame(&mut self, ms: u64) {
        self.now += Duration::from_millis(ms);
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(800., 600.)), self.now)
            .unwrap();
    }

    fn target(&self, id: u64) -> ElementId {
        self.runtime
            .tree()
            .element_with_key(&Key::Value(id))
            .unwrap()
    }

    fn rect(&self, id: u64) -> Rect {
        self.runtime.tree().element_bounds(self.target(id)).unwrap()
    }

    fn edge(&self, id: u64, trailing: bool) -> f32 {
        let rect = self.rect(id);
        match self.axis {
            Axis::Vertical => rect.origin.y + if trailing { rect.size.height } else { 0. },
            Axis::Horizontal => rect.origin.x + if trailing { rect.size.width } else { 0. },
        }
    }

    fn reveal(&mut self, id: u64, options: EnsureVisible) {
        Scrollable::ensure_visible(self.runtime.tree(), self.target(id), options);
        self.frame(0);
    }

    fn animated_reveal(&mut self, id: u64) {
        self.reveal(id, EnsureVisible::new().duration(Duration::from_secs(1)));
        self.frame(1020);
    }
}

fn forward(axis: Axis, expected: [f32; 5]) {
    let mut p = Fixture::new(axis, false, false);
    let mut out = Vec::new();
    for (id, alignment, trailing) in [
        (3, 0., false),
        (6, 0., false),
        (4, 1., true),
        (0, 1., false),
    ] {
        p.reveal(id, EnsureVisible::new().alignment(alignment));
        out.push(p.edge(id, trailing));
    }
    p.animated_reveal(3);
    out.push(p.edge(3, false));
    assert_eq!(out, expected);
}

#[test]
fn vertical_alignment_clamping_and_animated_reveal() {
    forward(Axis::Vertical, [100., 300., 500., 100., 100.]);
}
#[test]
fn horizontal_alignment_clamping_and_animated_reveal() {
    forward(Axis::Horizontal, [100., 500., 700., 100., 100.]);
}

fn reverse(axis: Axis, mut expected: Vec<f32>, before: [f32; 2], after: [f32; 2]) {
    let mut p = Fixture::new(axis, true, false);
    let mut out = Vec::new();
    for (id, alignment, trailing) in [(3, 0., true), (0, 0., true), (2, 1., false), (6, 1., true)] {
        p.reveal(id, EnsureVisible::new().alignment(alignment));
        out.push(p.edge(id, trailing));
    }
    p.animated_reveal(3);
    out.push(p.edge(3, true));

    p.controller.jump_to(0.);
    p.frame(0);
    let (outlier, vertical) = if axis == Axis::Vertical {
        (4, true)
    } else {
        (3, false)
    };
    out.extend([p.edge(outlier, vertical), p.edge(6, vertical)]);
    let leading =
        EnsureVisible::new().alignment_policy(ScrollPositionAlignmentPolicy::KeepVisibleAtStart);
    p.reveal(6, leading.clone());
    p.reveal(5, leading.clone());
    if !vertical {
        p.reveal(4, leading.clone());
    }
    out.extend([p.edge(outlier, vertical), p.edge(6, vertical)]);
    p.reveal(outlier, leading);
    out.extend([p.edge(outlier, vertical), p.edge(6, vertical)]);
    p.reveal(
        6,
        EnsureVisible::new().alignment_policy(ScrollPositionAlignmentPolicy::KeepVisibleAtEnd),
    );
    out.extend([p.edge(outlier, vertical), p.edge(6, vertical)]);
    expected.extend(before);
    expected.extend(before);
    expected.extend(after);
    expected.extend(before);
    assert_eq!(out, expected);
}

#[test]
fn reversed_vertical_uses_logical_edges_for_all_policies() {
    reverse(
        Axis::Vertical,
        vec![500., 300., 100., 500., 500.],
        [100., 500.],
        [300., 700.],
    );
}
#[test]
fn reversed_horizontal_uses_logical_edges_for_all_policies() {
    reverse(
        Axis::Horizontal,
        vec![700., 300., 100., 700., 700.],
        [-100., 500.],
        [100., 700.],
    );
}

#[test]
fn rotated_child_reveals_transformed_bounds_at_both_edges() {
    let mut p = Fixture::new(Axis::Vertical, false, true);
    p.reveal(0, EnsureVisible::new());
    let start = p.edge(0, false); // Rotated local bottomRight is on the world top edge.
    p.reveal(0, EnsureVisible::new().alignment(1.));
    let end = p.edge(0, true); // Rotated local topLeft is on the world bottom edge.
    assert!(
        (start - 100.).abs() <= 0.1 && (end - 500.).abs() <= 0.1,
        "transformed edges {:?}",
        (start, end)
    );
}

fn scroll_window(scroll: impl Into<Widget>) -> Widget {
    Directionality::new(
        TextDirection::Ltr,
        Center::new(SizedBox::new().width(600.).height(400.).child(scroll)),
    )
    .into()
}

#[test]
fn nested_different_axis_scrollables_reveal_the_same_target_in_both() {
    use incular_core::Offset;
    use incular_widgets::{Column, Row};

    let inner = ScrollController::new();
    let outer = ScrollController::new();
    let rows = (0u64..7).map(|y| {
        Widget::from(Row::new((0u64..7).map(|x| {
            Widget::from(SizedBox::new().width(200.).height(200.)).with_key(Key::Value(y * 7 + x))
        })))
    });
    let inner_view = SingleChildScrollView::new(Column::new(rows)).controller(inner);
    let outer_view = SingleChildScrollView::new(inner_view)
        .scroll_direction(Axis::Horizontal)
        .controller(outer.clone());
    let mut probe = Fixture {
        runtime: Runtime::new(scroll_window(outer_view)).unwrap(),
        now: Instant::now(),
        controller: outer,
        axis: Axis::Horizontal,
    };
    probe.frame(0);
    let mut observed = Vec::new();
    for (x, y, alignment) in [
        (0, 0, 0.),
        (3, 0, 0.),
        (3, 0, 0.5),
        (6, 0, 0.),
        (0, 2, 0.),
        (3, 2, 0.),
        (3, 2, 0.5),
    ] {
        let key = y * 7 + x;
        probe.reveal(key, EnsureVisible::new().alignment(alignment));
        observed.push(probe.rect(key).origin);
    }
    assert_eq!(
        observed,
        [
            Offset::new(100., 100.),
            Offset::new(100., 100.),
            Offset::new(300., 100.),
            Offset::new(500., 100.),
            Offset::new(100., 100.),
            Offset::new(100., 100.),
            Offset::new(300., 200.),
        ]
    );
}

fn list_forward(axis: Axis, initial_offsets: [f32; 5], expected: [f32; 5]) {
    use incular_widgets::ListView;
    let controller = ScrollController::new();
    let rows = (0..7)
        .map(|i| Widget::from(SizedBox::new().width(200.).height(200.)).with_key(Key::Value(i)));
    let list = ListView::new(rows)
        .scroll_direction(axis)
        .controller(controller.clone());
    let mut probe = Fixture {
        runtime: Runtime::new(scroll_window(list)).unwrap(),
        now: Instant::now(),
        controller,
        axis,
    };
    probe.frame(0);
    let mut out = Vec::new();
    for (step, (id, align, trailing)) in [
        (3, 0., false),
        (6, 0., false),
        (4, 1., true),
        (0, 1., false),
        (3, 0., false),
    ]
    .into_iter()
    .enumerate()
    {
        probe.controller.jump_to(initial_offsets[step]);
        probe.frame(0);
        if probe
            .runtime
            .tree()
            .element_with_key(&Key::Value(id))
            .is_none()
        {
            out.push(None);
            continue;
        }
        if step == 4 {
            probe.animated_reveal(id);
        } else {
            probe.reveal(id, EnsureVisible::new().alignment(align));
        }
        out.push(
            probe
                .runtime
                .tree()
                .element_with_key(&Key::Value(id))
                .map(|_| probe.edge(id, trailing)),
        );
    }
    assert_eq!(
        out,
        expected.map(Some),
        "None represents an unmounted original target"
    );
}

#[test]
fn list_view_vertical_reveals_cached_targets_at_the_original_offsets() {
    list_forward(
        Axis::Vertical,
        [480., 1083., 735., 123., 523.],
        [100., 300., 500., 100., 100.],
    );
}
#[test]
fn list_view_horizontal_reveals_cached_targets_at_the_original_offsets() {
    list_forward(
        Axis::Horizontal,
        [23., 843., 415., 46., 211.],
        [100., 500., 700., 100., 100.],
    );
}
