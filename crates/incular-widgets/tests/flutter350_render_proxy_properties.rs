use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_rendering::{DisplayList, PaintCommand};
use incular_widgets::{
    AspectRatio, ConstrainedBox, IntrinsicWidth, LimitedBox, Opacity, OpacityController, SizedBox,
    Widget,
    internal::{ElementId, WidgetTree},
};

fn size(tree: &WidgetTree, element: ElementId) -> Size {
    tree.render_size(tree.render_id(element).unwrap()).unwrap()
}

#[test]
fn additional_constraints_can_be_replaced_without_replacing_the_proxy() {
    let height = Constraints::new(0., f32::INFINITY, 10., 10.);
    let width = Constraints::new(10., 10., 0., f32::INFINITY);
    let initial = ConstrainedBox::new(height, SizedBox::shrink());
    assert_eq!(initial.additional_constraints(), height);

    let mut tree = WidgetTree::new();
    let root = tree.mount(initial.clone().into()).unwrap();
    let proxy = tree.render_id(root);
    let view = Constraints::at_most(100., 100.);
    tree.layout(view).unwrap();
    assert_eq!(size(&tree, root), Size::new(0., 10.));

    let updated = initial.with_additional_constraints(width);
    assert_eq!(updated.additional_constraints(), width);
    tree.update(root, updated.into()).unwrap();
    tree.layout(view).unwrap();
    assert_eq!(tree.render_id(root), proxy);
    assert_eq!(size(&tree, root), Size::new(10., 0.));
}

#[test]
fn limited_box_maxima_default_to_infinity_and_limit_only_unbounded_axes() {
    let initial = LimitedBox::new(SizedBox::new().width(80.).height(70.));
    assert_eq!(initial.width_limit(), f32::INFINITY);
    assert_eq!(initial.height_limit(), f32::INFINITY);
    let mut tree = WidgetTree::new();
    let root = tree.mount(initial.clone().into()).unwrap();
    let proxy = tree.render_id(root);
    tree.layout(Constraints::default()).unwrap();
    assert_eq!(size(&tree, root), Size::new(80., 70.));

    let limited = initial.max_width(0.).max_height(1.);
    assert_eq!(limited.width_limit(), 0.);
    assert_eq!(limited.height_limit(), 1.);
    tree.update(root, limited.into()).unwrap();
    tree.layout(Constraints::default()).unwrap();
    assert_eq!(tree.render_id(root), proxy);
    assert_eq!(size(&tree, root), Size::new(0., 1.));

    tree.layout(Constraints::at_most(100., 100.)).unwrap();
    assert_eq!(size(&tree, root), Size::new(80., 70.));
}

#[test]
fn subsequent_aspect_ratios_update_the_same_render_box() {
    let mut descriptor = AspectRatio::new(1., SizedBox::shrink());
    let mut tree = WidgetTree::new();
    let root = tree.mount(descriptor.clone().into()).unwrap();
    let proxy = tree.render_id(root);
    let viewport = Constraints::at_most(120., 100.);
    for (ratio, width) in [(1., 100.), (0.2, 20.), (1.2, 120.)] {
        descriptor = descriptor.with_ratio(ratio);
        assert_eq!(descriptor.ratio(), ratio);
        tree.update(root, descriptor.clone().into()).unwrap();
        tree.layout(viewport).unwrap();
        assert_eq!(tree.render_id(root), proxy);
        let actual = size(&tree, root);
        assert!((actual.width - width).abs() < 0.001);
        assert!((actual.height - 100.).abs() < 0.001);
    }
}

#[test]
fn intrinsic_steps_are_optional_and_each_rounds_its_own_axis() {
    let initial = IntrinsicWidth::new(SizedBox::new().width(13.).height(17.));
    let width = initial.clone().step_width(10.);
    let both = width.clone().step_height(10.);
    let mut tree = WidgetTree::new();
    let root = tree.mount(initial.clone().into()).unwrap();
    let proxy = tree.render_id(root);
    for (descriptor, steps, expected) in [
        (initial, [None, None], Size::new(13., 17.)),
        (width, [Some(10.), None], Size::new(20., 17.)),
        (both, [Some(10.), Some(10.)], Size::new(20., 20.)),
    ] {
        assert_eq!([descriptor.width_step(), descriptor.height_step()], steps);
        tree.update(root, descriptor.into()).unwrap();
        tree.layout(Constraints::at_most(100., 100.)).unwrap();
        assert_eq!(tree.render_id(root), proxy);
        assert_eq!(size(&tree, root), expected);
    }
}

#[test]
fn opacity_defaults_to_one_and_a_live_setter_suppresses_paint_at_zero() {
    let alpha = OpacityController::new();
    assert_eq!(alpha.opacity(), 1.);
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            Opacity::controlled(
                alpha.clone(),
                Widget::box_(Size::new(10., 10.), Color::WHITE),
            )
            .into(),
        )
        .unwrap();
    let proxy = tree.render_id(root);
    let constraints = Constraints::at_most(20., 20.);
    tree.layout(constraints).unwrap();
    let at = Instant::now();
    tree.update_compositor(at).unwrap();
    let draws = |list: &DisplayList| {
        list.commands().iter().any(|command| {
            matches!(
                command,
                PaintCommand::Rect {
                    color: Color::WHITE,
                    ..
                }
            )
        })
    };
    assert!(draws(&tree.paint()));
    assert!(alpha.set_opacity(0.));
    assert_eq!(alpha.opacity(), 0.);
    tree.update_compositor(at).unwrap();
    assert_eq!(tree.render_id(root), proxy);
    assert_eq!(size(&tree, root), Size::new(10., 10.));
    assert!(!draws(&tree.paint()));
}
