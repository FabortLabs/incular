use incular_config::{Clip, Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{ClipCoverage, DisplayList, PaintCommand};
use incular_widgets::{
    ConstraintsTransformBox as TransformBox, CustomPaint, CustomPainter, SizedBox, Text, Widget,
    internal::{TreeError, WidgetTree},
};
use std::{cell::Cell, rc::Rc};

fn size(tree: &WidgetTree, element: incular_widgets::internal::ElementId) -> Size {
    tree.render_size(tree.render_id(element).unwrap()).unwrap()
}

#[test]
fn replacing_width_unconstrained_with_fully_unconstrained_preserves_a_tight_parent() {
    let scene = |transform: fn(Constraints) -> Constraints| {
        TransformBox::new(transform, SizedBox::square(200.)).into()
    };
    let constraint = Constraints::tight(Size::new(200., 200.));
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(scene(TransformBox::width_unconstrained))
        .unwrap();
    let render = tree.render_id(root).unwrap();
    for transform in [
        TransformBox::width_unconstrained,
        TransformBox::unconstrained,
    ] {
        tree.update(root, scene(transform)).unwrap();
        tree.layout(constraint).unwrap();
        assert_eq!(tree.render_id(root), Some(render));
        assert_eq!(size(&tree, root), Size::new(200., 200.));
        assert_eq!(
            size(&tree, tree.children(root).unwrap()[0]),
            Size::new(200., 200.)
        );
    }
}

#[test]
fn invalid_transformed_constraints_return_the_layout_error_before_measuring_the_child() {
    let invalid = |_: Constraints| Constraints::try_new(0., f32::INFINITY, 200., -1.);
    let mut tree = WidgetTree::new();
    tree.mount(TransformBox::try_new(invalid, SizedBox::new().height(0.)).into())
        .unwrap();
    let err = tree.layout(Constraints::default()).unwrap_err();
    assert!(matches!(
        &err,
        TreeError::InvalidWidgetConfiguration {
            widget: "ConstraintsTransformBox",
            ..
        }
    ));
    assert!(
        err.to_string().contains("is not normalized")
            || err.to_string().contains("are not normalized")
    );
}

#[test]
fn transformed_child_can_overflow_and_every_clip_mode_is_observed() {
    let scene = |clip| -> Widget {
        let wrapper = TransformBox::new(
            TransformBox::max_width_unconstrained,
            Widget::box_(Size::new(200., 200.), Color::WHITE),
        );
        if let Some(clip) = clip {
            wrapper.clip_behavior(clip).into()
        } else {
            wrapper.into()
        }
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(None)).unwrap();
    let viewport = Size::new(100., 100.);
    for (setting, coverage) in [
        (None, None),
        (Some(Clip::None), None),
        (Some(Clip::HardEdge), Some(ClipCoverage::HardEdge)),
        (Some(Clip::AntiAlias), Some(ClipCoverage::AntiAlias)),
        (
            Some(Clip::AntiAliasWithSaveLayer),
            Some(ClipCoverage::AntiAliasWithSaveLayer),
        ),
    ] {
        let before = tree.diagnostics().unclipped_constraint_overflows;
        tree.update(root, scene(setting)).unwrap();
        tree.layout(Constraints::loose(viewport)).unwrap();
        assert_eq!(size(&tree, root), viewport);
        let child = tree.children(root).unwrap()[0];
        assert_eq!(size(&tree, child), Size::new(200., 100.));
        assert_eq!(
            tree.element_bounds(child).unwrap().origin,
            Offset::new(-50., 0.)
        );
        assert_eq!(
            tree.diagnostics().unclipped_constraint_overflows - before,
            u64::from(coverage.is_none() && cfg!(debug_assertions))
        );
        let list = tree.paint();
        let clip = list.commands().iter().find_map(|command| match command {
            PaintCommand::PushClip { rect } => Some((*rect, ClipCoverage::HardEdge)),
            PaintCommand::PushClipRRect {
                rrect, coverage, ..
            } => Some((rrect.rect, *coverage)),
            _ => None,
        });
        assert_eq!(
            clip,
            coverage.map(|mode| (Rect::from_origin_size(Offset::ZERO, viewport), mode))
        );
    }
}

#[test]
fn width_transform_keeps_a_paragraph_single_line_even_in_a_narrow_parent() {
    let text = || Text::new("a".repeat(100)).text_direction(TextDirection::Ltr);
    let constraints = Constraints::at_most(30., 1000.);
    let mut normal = WidgetTree::new();
    let plain = normal.mount(text().into()).unwrap();
    normal.layout(constraints).unwrap();

    let mut tree = WidgetTree::new();
    let root = tree
        .mount(TransformBox::new(TransformBox::max_width_unconstrained, text()).into())
        .unwrap();
    tree.layout(constraints).unwrap();
    let box_size = size(&tree, root);
    assert_eq!(box_size.width, 30.);
    assert!(size(&normal, plain).height >= 2. * box_size.height);
    assert!(size(&tree, tree.children(root).unwrap()[0]).width > 30.);
}

struct Trace(Rc<Cell<usize>>);
impl CustomPainter for Trace {
    fn paint(&self, _: Size) -> DisplayList {
        self.0.set(self.0.get() + 1);
        DisplayList::default()
    }
    fn should_repaint(&self, _: &Self) -> bool {
        true
    }
}

#[test]
fn a_zero_size_constraints_transform_still_calls_its_painter() {
    let calls = Rc::new(Cell::new(0));
    let child = CustomPaint::from_painter(Size::ZERO, Trace(calls.clone()));
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(TransformBox::new(TransformBox::max_width_unconstrained, child).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    assert_eq!(size(&tree, root), Size::ZERO);
    let _ = tree.paint();
    assert_eq!(calls.get(), 1);
}

#[test]
fn an_empty_constraints_transform_has_no_child_or_layout_callback_on_a_zero_area_view() {
    use incular_widgets::{Center, Directionality, internal::Key};
    const TOKEN: Key = Key::Value(810);
    let calls = Rc::new(Cell::new(0));
    let base = TransformBox::empty({
        let calls = calls.clone();
        move |incoming| {
            calls.set(calls.get() + 1);
            TransformBox::width_unconstrained(incoming)
        }
    });
    let scene = |child: bool| {
        let probe = if child {
            base.clone().child(SizedBox::square(25.))
        } else {
            base.clone()
        };
        Directionality::new(
            TextDirection::Ltr,
            Center::new(Widget::from(probe).with_key(TOKEN)),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(false)).unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    let box_id = tree.element_with_key(&TOKEN).unwrap();
    let render = tree.render_id(box_id).unwrap();
    assert_eq!(size(&tree, box_id), Size::ZERO);
    assert!(tree.children(box_id).unwrap().is_empty());
    assert_eq!(calls.get(), 0);
    let _ = tree.paint();

    for child in [true, false] {
        tree.update(root, scene(child)).unwrap();
        tree.layout(Constraints::tight(Size::new(100., 100.)))
            .unwrap();
        assert_eq!(tree.render_id(box_id), Some(render));
        assert_eq!(tree.children(box_id).unwrap().len(), usize::from(child));
        assert_eq!(
            size(&tree, box_id),
            if child {
                Size::new(25., 25.)
            } else {
                Size::ZERO
            }
        );
        assert_eq!(calls.get(), 1);
    }
}

#[test]
fn unconstrained_box_contains_the_placeholder_on_a_zero_area_view() {
    use incular_widgets::{Center, Directionality, Placeholder, UnconstrainedBox, internal::Key};
    const TOKEN: Key = Key::Value(811);
    let proxy = Widget::from(UnconstrainedBox::new(Placeholder::new())).with_key(TOKEN);
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, Center::new(proxy)).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    let proxy = tree.element_with_key(&TOKEN).unwrap();
    assert_eq!(size(&tree, proxy), Size::ZERO);
    assert_eq!(tree.children(proxy).unwrap().len(), 1);
    let _ = tree.paint();
}

#[test]
fn logical_constraint_alignment_tracks_ambient_direction_and_explicit_override() {
    use incular_config::AlignmentDirectional;
    use incular_widgets::{Directionality, internal::Key};
    const CHILD: Key = Key::Value(812);
    const PROXY: Key = Key::Value(813);
    let proxy = |explicit: Option<TextDirection>| {
        let child = Widget::from(SizedBox::new().width(200.).height(50.)).with_key(CHILD);
        let widget = TransformBox::new(TransformBox::max_width_unconstrained, child)
            .alignment(AlignmentDirectional::TOP_START)
            .clip_behavior(Clip::HardEdge);
        Widget::from(explicit.map_or_else(
            || widget.clone(),
            |direction| widget.clone().text_direction(direction),
        ))
        .with_key(PROXY)
    };
    let scene = |direction, explicit| Directionality::new(direction, proxy(explicit)).into();
    let mut tree = WidgetTree::new();
    assert!(matches!(
        tree.mount(proxy(None)),
        Err(TreeError::MissingDirectionality {
            widget: "ConstraintsTransformBox",
            ..
        })
    ));
    let root = tree.mount(scene(TextDirection::Ltr, None)).unwrap();
    let viewport = Constraints::tight(Size::new(100., 100.));
    tree.layout(viewport).unwrap();
    let id = tree.element_with_key(&PROXY).unwrap();
    let render = tree.render_id(id);
    for (inherited, explicit, expected_x) in [
        (TextDirection::Ltr, None, 0.),
        (TextDirection::Rtl, None, -100.),
        (TextDirection::Rtl, Some(TextDirection::Ltr), 0.),
    ] {
        tree.update(root, scene(inherited, explicit)).unwrap();
        tree.layout(viewport).unwrap();
        assert_eq!(tree.render_id(id), render);
        let child = tree.element_with_key(&CHILD).unwrap();
        assert_eq!(
            tree.element_bounds(child).unwrap().origin,
            Offset::new(expected_x, 0.)
        );
    }
}

#[test]
fn both_axis_overflow_reports_only_when_the_requested_clip_is_none() {
    let viewport = Size::new(100., 100.);
    let rect = Rect::from_origin_size(Offset::ZERO, viewport);
    for (setting, expected) in [
        (None, None),
        (Some(Clip::None), None),
        (Some(Clip::HardEdge), Some(ClipCoverage::HardEdge)),
        (Some(Clip::AntiAlias), Some(ClipCoverage::AntiAlias)),
        (
            Some(Clip::AntiAliasWithSaveLayer),
            Some(ClipCoverage::AntiAliasWithSaveLayer),
        ),
    ] {
        let make = |size, clip| -> Widget {
            let proxy = TransformBox::new(
                TransformBox::unconstrained,
                Widget::box_(size, Color::WHITE),
            )
            .text_direction(TextDirection::Ltr);
            match clip {
                Some(clip) => proxy.clip_behavior(clip).into(),
                None => proxy.into(),
            }
        };
        let mut tree = WidgetTree::new();
        let root = tree.mount(make(Size::new(200., 200.), setting)).unwrap();
        let view = Constraints::loose(viewport);
        tree.layout(view).unwrap();
        assert_eq!(size(&tree, root), viewport);
        let child = tree.children(root).unwrap()[0];
        assert_eq!(
            tree.element_bounds(child),
            Some(Rect::from_origin_size(
                Offset::new(-50., -50.),
                Size::new(200., 200.)
            ))
        );
        assert_eq!(
            tree.diagnostics().unclipped_constraint_overflows,
            u64::from(expected.is_none() && cfg!(debug_assertions))
        );

        let clip = |display: &DisplayList| {
            display.commands().iter().find_map(|op| match *op {
                PaintCommand::PushClip { rect } => Some((rect, ClipCoverage::HardEdge)),
                PaintCommand::PushClipRRect {
                    rrect,
                    coverage,
                    isolation,
                } => {
                    assert_eq!(
                        isolation.is_some(),
                        coverage == ClipCoverage::AntiAliasWithSaveLayer
                    );
                    Some((rrect.rect, coverage))
                }
                _ => None,
            })
        };
        let display = tree.paint();
        assert_eq!(clip(&display), expected.map(|coverage| (rect, coverage)));
        assert!(display.commands().iter().any(|op| matches!(
            op,
            PaintCommand::Rect {
                color: Color::WHITE,
                ..
            }
        )));

        let render = tree.render_id(root);
        tree.update(root, make(Size::new(80., 80.), setting))
            .unwrap();
        tree.layout(view).unwrap();
        assert_eq!(tree.render_id(root), render);
        assert_eq!(
            clip(&tree.paint()),
            None,
            "a fitting child needs no retained clip"
        );
    }
}
