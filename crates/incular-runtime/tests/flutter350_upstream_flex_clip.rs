use incular_config::{Axis, Clip, Constraints, MainAxisSize, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{ClipCoverage, DisplayList, PaintCommand};
use incular_runtime::Runtime;
use incular_widgets::{Expanded, Flex, SizedBox, Widget};

fn view(mode: Option<Clip>) -> Widget {
    let mut flex = Flex::new(
        Axis::Vertical,
        [Widget::box_(Size::new(200., 200.), Color::WHITE)],
    )
    .text_direction(TextDirection::Ltr);
    if let Some(mode) = mode {
        flex = flex.clip_behavior(mode);
    }
    flex.into()
}

fn paint(runtime: &mut Runtime, height: f32) -> DisplayList {
    runtime
        .run_frame(Constraints::at_most(100., height))
        .unwrap()
        .0
}

fn clip(list: &DisplayList) -> Vec<(ClipCoverage, Rect, bool)> {
    list.commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushClip { rect } => Some((ClipCoverage::HardEdge, *rect, false)),
            PaintCommand::PushClipRRect {
                rrect,
                coverage,
                isolation,
            } => Some((*coverage, rrect.rect, isolation.is_some())),
            _ => None,
        })
        .collect()
}

#[test]
fn flex_records_the_configured_clip_only_when_the_main_axis_overflows() {
    let mut runtime = Runtime::new(view(None)).unwrap();
    let root = runtime.tree().root().unwrap();
    let bounds = Rect::from_origin_size(Offset::ZERO, Size::new(100., 100.));
    assert_eq!(clip(&paint(&mut runtime, 100.)), []);

    for (mode, coverage, isolated) in [
        (Clip::HardEdge, ClipCoverage::HardEdge, false),
        (Clip::AntiAlias, ClipCoverage::AntiAlias, false),
        (
            Clip::AntiAliasWithSaveLayer,
            ClipCoverage::AntiAliasWithSaveLayer,
            true,
        ),
    ] {
        runtime.schedule_update(root, view(Some(mode))).unwrap();
        let overflow = paint(&mut runtime, 100.);
        assert_eq!(clip(&overflow), [(coverage, bounds, isolated)]);
        let commands = overflow.commands();
        let push = commands
            .iter()
            .position(|c| {
                matches!(
                    c,
                    PaintCommand::PushClip { .. } | PaintCommand::PushClipRRect { .. }
                )
            })
            .unwrap();
        let pop = commands
            .iter()
            .position(|c| matches!(c, PaintCommand::PopClip))
            .unwrap();
        assert!(commands[push + 1..pop].iter().any(|command| matches!(
            command,
            PaintCommand::Rect {
                color: Color::WHITE,
                ..
            }
        )));

        let fitting = paint(&mut runtime, 250.);
        assert_eq!(clip(&fitting), []);
        assert!(fitting.commands().iter().any(|command| matches!(
            command,
            PaintCommand::Rect {
                color: Color::WHITE,
                ..
            }
        )));
        assert_eq!(
            clip(&paint(&mut runtime, 100.)),
            [(coverage, bounds, isolated)]
        );
        assert_eq!(runtime.tree().root(), Some(root));
    }

    runtime
        .schedule_update(root, view(Some(Clip::None)))
        .unwrap();
    assert_eq!(clip(&paint(&mut runtime, 100.)), []);
}

#[test]
fn tightly_constrained_flex_keeps_the_parent_size() {
    let mut runtime = Runtime::new(
        Flex::horizontal([Widget::box_(Size::ZERO, Color::TRANSPARENT)])
            .text_direction(TextDirection::Ltr)
            .into(),
    )
    .unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(200., 200.)))
        .unwrap();
    let tree = runtime.tree();
    assert_eq!(
        tree.element_bounds(tree.root().unwrap()).unwrap().size,
        Size::new(200., 200.)
    );
}

#[test]
fn shrink_wrapped_overflow_keeps_parent_bounds_and_clips_the_real_child() {
    let build = |max_width| {
        incular_widgets::OverflowBox::new(
            Flex::horizontal([Widget::box_(Size::new(438.85715, 100.), Color::WHITE)])
                .main_axis_size(MainAxisSize::Min)
                .clip_behavior(Clip::HardEdge)
                .text_direction(TextDirection::Ltr),
        )
        .min_width(0.)
        .max_width(max_width)
        .min_height(0.)
        .max_height(400.)
        .into()
    };
    let pinned_max = 438.85714285714283_f64 as f32;
    assert_eq!(438.8571428571429_f64 as f32, pinned_max);
    let mut runtime = Runtime::new(build(pinned_max)).unwrap();
    let root = runtime.tree().root().unwrap();

    let check = |runtime: &mut Runtime, max_width, expected_clip: bool| {
        let list = paint(runtime, 100.);
        let tree = runtime.tree();
        let flex = tree.children(root).unwrap()[0];
        assert_eq!(
            tree.element_bounds(flex).unwrap().size,
            Size::new(max_width, 100.)
        );
        let clips = clip(&list);
        assert_eq!(clips.len(), usize::from(expected_clip));
        if let Some((coverage, bounds, isolation)) = clips.first() {
            assert_eq!(*coverage, ClipCoverage::HardEdge);
            assert_eq!(bounds.size, Size::new(max_width, 100.));
            assert!(!isolation);
        }
    };
    check(&mut runtime, pinned_max, false);
    let lower = f32::from_bits(pinned_max.to_bits() - 1);
    runtime.schedule_update(root, build(lower)).unwrap();
    check(&mut runtime, lower, true);
}

#[test]
fn fractional_flex_allocations_do_not_activate_an_overflow_clip() {
    let mut runtime = Runtime::new(
        Flex::horizontal([
            Expanded::new(SizedBox::from_size(Size::new(1., 20.))),
            Expanded::new(SizedBox::from_size(Size::new(1., 20.))),
            Expanded::new(SizedBox::from_size(Size::new(1., 20.))),
        ])
        .clip_behavior(Clip::HardEdge)
        .text_direction(TextDirection::Ltr)
        .into(),
    )
    .unwrap();
    let list = runtime
        .run_frame(Constraints::at_most(500., 100.))
        .unwrap()
        .0;
    let tree = runtime.tree();
    let root = tree.root().unwrap();
    assert_eq!(
        tree.element_bounds(root).unwrap().size,
        Size::new(500., 20.)
    );
    let children = tree.children(root).unwrap();
    for (index, child) in children.iter().enumerate() {
        let bounds = tree.element_bounds(*child).unwrap();
        assert!((bounds.size.width - 500. / 3.).abs() < 0.0001);
        assert!((bounds.origin.x - index as f32 * (500. / 3.)).abs() < 0.0001);
    }
    assert_eq!(clip(&list), []);
}

#[test]
fn hard_edge_flex_clips_separate_semantics_and_keeps_labels_merged_inside() {
    use incular_widgets::{Center, Directionality, MergeSemantics, Text};

    let child = |label| {
        SizedBox::new()
            .width(75.)
            .height(14.)
            .child(Text::new(label))
    };
    for merge in [false, true] {
        let row = if merge {
            Flex::horizontal([
                Widget::from(child("1")),
                MergeSemantics::new(Flex::horizontal([child("2"), child("3")])).into(),
            ])
        } else {
            Flex::horizontal([child("1"), child("2"), child("3")])
        }
        .clip_behavior(Clip::HardEdge);
        let screen = Directionality::new(
            TextDirection::Ltr,
            Center::new(SizedBox::new().width(100.).child(row)),
        );
        let mut runtime = Runtime::new(screen.into()).unwrap();
        runtime
            .run_frame(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
        #[cfg(debug_assertions)]
        {
            let reports = runtime.tree_mut().take_flex_overflow_diagnostics();
            assert_eq!(reports.len(), 1);
            assert_eq!(reports[0].pixels, 125.);
            assert_eq!(reports[0].axis, Axis::Horizontal);
            assert_eq!(
                reports[0].to_string(),
                "A RenderFlex overflowed by 125 pixels on the right."
            );
        }
        let semantics = runtime.tree().semantics();
        let actual = semantics
            .iter()
            .filter_map(|(_, node)| node.label.as_deref().map(|label| (label, node.bounds)))
            .collect::<Vec<_>>();
        let expected_label = if merge { "2\n3" } else { "2" };
        assert_eq!(
            actual,
            [
                (
                    "1",
                    Rect::from_origin_size(Offset::new(350., 293.), Size::new(75., 14.))
                ),
                (
                    expected_label,
                    Rect::from_origin_size(Offset::new(425., 293.), Size::new(25., 14.))
                ),
            ],
            "merge={merge}"
        );
    }
}

#[test]
fn retained_semantic_bounds_follow_flex_resize_and_clip_policy_changes() {
    use incular_widgets::Text;

    let make = |mode| {
        Flex::horizontal(["1", "2", "3"].map(|label| {
            SizedBox::new()
                .width(75.)
                .height(14.)
                .child(Text::new(label).text_direction(TextDirection::Ltr))
        }))
        .text_direction(TextDirection::Ltr)
        .clip_behavior(mode)
        .into()
    };
    let mut runtime = Runtime::new(make(Clip::HardEdge)).unwrap();
    let frame = |runtime: &mut Runtime, width| {
        runtime
            .run_frame(Constraints::tight(Size::new(width, 14.)))
            .unwrap();
    };
    let observed = |runtime: &Runtime| {
        let semantics = runtime.tree().semantics();
        semantics
            .iter()
            .filter_map(|(id, node)| {
                node.label
                    .as_ref()
                    .map(|label| (id, label.clone(), node.bounds.size.width))
            })
            .collect::<Vec<_>>()
    };
    let widths = |runtime: &Runtime| {
        observed(runtime)
            .into_iter()
            .map(|(_, _, width)| width)
            .collect::<Vec<_>>()
    };

    frame(&mut runtime, 100.);
    assert_eq!(widths(&runtime), [75., 25.]);
    let first = observed(&runtime)[0].0;
    frame(&mut runtime, 130.);
    assert_eq!(
        widths(&runtime),
        [75., 55.],
        "an unchanged child follows its ancestor clip"
    );
    frame(&mut runtime, 150.);
    assert_eq!(
        widths(&runtime),
        [75., 75.],
        "touching the edge does not publish item 3"
    );
    frame(&mut runtime, 225.);
    assert_eq!(widths(&runtime), [75., 75., 75.]);
    frame(&mut runtime, 100.);
    assert_eq!(widths(&runtime), [75., 25.]);

    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, make(Clip::None)).unwrap();
    frame(&mut runtime, 100.);
    assert_eq!(widths(&runtime), [75., 75., 75.]);
    runtime.schedule_update(root, make(Clip::HardEdge)).unwrap();
    frame(&mut runtime, 100.);
    assert_eq!(widths(&runtime), [75., 25.]);
    assert_eq!(
        observed(&runtime)[0].0,
        first,
        "the visible node keeps identity"
    );
}

#[cfg(debug_assertions)]
#[test]
fn flex_overflow_is_reported_once_when_painted_not_when_laid_out_offstage() {
    use incular_widgets::{Offstage, internal::WidgetTree};

    let scene = |offstage| -> Widget {
        let children = (0..3).map(|_| SizedBox::from_size(Size::new(14., 75.)));
        Offstage::new(Flex::new(Axis::Vertical, children))
            .offstage(offstage)
            .into()
    };
    let constraints = Constraints::tight(Size::new(14., 100.));
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(true)).unwrap();
    tree.layout(constraints).unwrap();
    assert!(tree.take_flex_overflow_diagnostics().is_empty());
    let _ = tree.paint();
    assert!(tree.take_flex_overflow_diagnostics().is_empty());

    tree.update(root, scene(false)).unwrap();
    tree.layout(constraints).unwrap();
    assert!(tree.take_flex_overflow_diagnostics().is_empty());
    let _ = tree.paint();
    let reports = tree.take_flex_overflow_diagnostics();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].axis, Axis::Vertical);
    assert_eq!(
        reports[0].to_string(),
        "A RenderFlex overflowed by 125 pixels on the bottom."
    );

    for height in [225., 100., 110.] {
        tree.layout(Constraints::tight(Size::new(14., height)))
            .unwrap();
        let _ = tree.paint();
        assert!(tree.take_flex_overflow_diagnostics().is_empty());
    }
}
