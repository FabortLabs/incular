use incular_core::Color;
use incular_image::ImageHandle;
use incular_rendering::Path;
use incular_widgets::internal::{ImageFit, ImageRepeat, PhysicalModel, PhysicalShape};
use incular_widgets::{
    Border, BorderDirectional, BorderRadius, BorderRadiusDirectional, BorderSide, BorderStyle,
    BoxDecoration, BoxShadow, ClipRSuperellipse, DecorationImage, GridPaper, ImageFiltered,
    ImageIcon, Radius, RawImage, SnapshotWidget, Text, Widget,
};
use std::sync::Arc;

fn image_handle() -> ImageHandle {
    ImageHandle::from_rgba8(1, 1, vec![255, 255, 255, 255]).unwrap()
}

#[test]
fn effect_builders_preserve_defaults_and_normalize_values() {
    let raw = RawImage::builder()
        .width(-4.0)
        .height(-8.0)
        .scale(0.0)
        .build();
    assert_eq!(
        raw,
        RawImage::new()
            .width(0.0)
            .height(0.0)
            // No explicit fit: paintImage chooses scaleDown normally,
            // but fill when a center slice is supplied.
            .scale(0.001)
            .repeat(ImageRepeat::NoRepeat)
    );

    let handle = image_handle();
    let icon = ImageIcon::builder()
        .image(handle.clone())
        .size(-2.0)
        .build();
    assert_eq!(icon, ImageIcon::new(handle).size(0.0));
    assert_eq!(Widget::from(icon).debug_type_name(), "Image");

    let grid = GridPaper::builder().build();
    assert_eq!(grid, GridPaper::default());

    let decoration = BoxDecoration::builder().build();
    assert_eq!(decoration, BoxDecoration::default());
}

#[test]
fn composition_builders_accept_arbitrary_widgets_and_keep_lowering_inputs() {
    let filtered = ImageFiltered::builder()
        .sigma(3.0)
        .child(Text::new("filtered"))
        .build();
    assert_eq!(filtered, ImageFiltered::blur(3.0, Text::new("filtered")));
    let filtered: Widget = filtered.into();
    assert_eq!(filtered.debug_type_name(), "ImageFiltered");

    let snapshot = SnapshotWidget::builder()
        .child(Text::new("snapshot"))
        .build();
    let _: Widget = snapshot.into();

    let clip = ClipRSuperellipse::builder()
        .radius(-4.0)
        .child(Text::new("clip"))
        .build();
    let clip: Widget = clip.into();
    // The direction-aware Builder lowers to LayoutBuilder and retains
    // a ClipPath delegate that resolves against the final child size.
    assert_eq!(clip.debug_type_name(), "LayoutBuilder");

    let physical = PhysicalModel::builder()
        .color(Color::WHITE)
        .elevation(-2.0)
        .child(Text::new("physical"))
        .build();
    let expected = PhysicalModel::new(Color::WHITE, Text::new("physical"));
    assert_eq!(Widget::from(physical), Widget::from(expected));

    let physical_shape = PhysicalShape::builder()
        .clipper(Arc::new(Path::builder().build()))
        .color(Color::WHITE)
        .elevation(-2.0)
        .child(Text::new("shape"))
        .build();
    let physical_shape: Widget = physical_shape.into();
    // The surface paints behind the child, which is clipped only when a clip
    // behavior is requested (Flutter default: none), so this is a Stack.
    assert_eq!(physical_shape.debug_type_name(), "Stack");

    let grid = GridPaper::builder().child(Text::new("grid")).build();
    let _: Widget = grid.into();
}

#[test]
fn painting_data_builders_match_existing_defaults_and_conversions() {
    let radius = Radius::builder().x(4.0).build();
    assert_eq!(radius, Radius::elliptical(4.0, 0.0));

    let border_radius = BorderRadius::builder().top_left(radius).build();
    assert_eq!(border_radius.top_right, Radius::ZERO);

    let directional = BorderRadiusDirectional::builder().top_start(radius).build();
    assert_eq!(directional.top_end, Radius::ZERO);

    let side = BorderSide::builder().width(-0.0).build();
    assert_eq!(side.width, 0.0);
    assert_eq!(side, BorderSide::new(Color::BLACK, 0.0, BorderStyle::Solid));

    let border = Border::builder()
        .top(side)
        .right(side)
        .bottom(side)
        .left(side)
        .build();
    assert_eq!(border, Border::all(side));

    let directional_border = BorderDirectional::builder().build();
    assert_eq!(directional_border, BorderDirectional::default());

    let shadow = BoxShadow::builder().color(Color::BLACK).build();
    assert_eq!(shadow.blur_radius, 0.0);

    // Flutter `DecorationImage.fit` is null by default; `paintImage` resolves
    // a null fit to scaleDown without a center slice (decoration_image.dart:561).
    let image = DecorationImage::builder().image(image_handle()).build();
    assert_eq!(image.fit, None);
    assert_eq!(image.resolved_fit(), ImageFit::ScaleDown);
    let sliced = DecorationImage::builder()
        .image(image_handle())
        .center_slice(incular_core::Rect::from_origin_size(
            incular_core::Offset::new(10., 5.),
            incular_core::Size::new(20., 10.),
        ))
        .build();
    assert_eq!(sliced.resolved_fit(), ImageFit::Fill);
    assert_eq!(
        DecorationImage::builder()
            .image(image_handle())
            .fit(ImageFit::Cover)
            .build()
            .resolved_fit(),
        ImageFit::Cover
    );
    assert_eq!(image.repeat, ImageRepeat::NoRepeat);

    let decoration = BoxDecoration::builder()
        .color(Color::WHITE)
        .image(image)
        .box_shadow([shadow])
        .build();
    assert!(decoration.is_valid());
    assert_eq!(decoration.box_shadow.len(), 1);
}

#[test]
fn decoration_image_defaults_to_medium_filter_quality() {
    // Flutter `DecorationImage.filterQuality` defaults to `FilterQuality.medium`.
    let image = incular_image::ImageHandle::from_rgba8(1, 1, vec![0, 0, 0, 255]).expect("image");
    let decoration = incular_widgets::DecorationImage::new(image.clone());
    assert_eq!(
        decoration.filter_quality,
        incular_rendering::FilterQuality::Medium
    );
    assert_eq!(
        incular_widgets::DecorationImage::builder()
            .image(image)
            .build()
            .filter_quality,
        incular_rendering::FilterQuality::Medium
    );
}

fn blur_stage(
    tree: &mut incular_widgets::internal::WidgetTree,
) -> Option<(incular_rendering::LayerId, incular_rendering::GaussianBlur)> {
    tree.paint()
        .commands()
        .iter()
        .find_map(|command| match command {
            incular_rendering::PaintCommand::PushBlur { layer, blur, .. } => Some((*layer, *blur)),
            _ => None,
        })
}

#[test]
fn disabled_image_filter_passes_its_child_through_unchanged() {
    use incular_config::Constraints;
    use incular_core::Size;
    use incular_rendering::GaussianBlur;
    use incular_widgets::Placeholder;
    use incular_widgets::internal::WidgetTree;

    let build = |enabled| {
        ImageFiltered::blur(5., Placeholder::new())
            .enabled(enabled)
            .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(false)).unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    tree.layout(constraints).unwrap();
    let original_child = tree.children(root).unwrap()[0];
    let original_child_render = tree.render_id(original_child);
    assert_eq!(blur_stage(&mut tree), None);
    assert!(!tree.paint().commands().is_empty());
    tree.update(root, build(true)).unwrap();
    tree.layout(constraints).unwrap();
    assert_eq!(tree.children(root).unwrap(), [original_child]);
    assert_eq!(tree.render_id(original_child), original_child_render);
    assert_eq!(blur_stage(&mut tree).unwrap().1, GaussianBlur::uniform(5.));
    tree.update(root, build(false)).unwrap();
    tree.layout(constraints).unwrap();
    assert_eq!(tree.children(root).unwrap(), [original_child]);
    assert_eq!(tree.render_id(original_child), original_child_render);
    assert_eq!(blur_stage(&mut tree), None);
}

#[test]
fn image_filter_reuses_the_same_layer_after_sigma_changes() {
    use incular_config::Constraints;
    use incular_core::Size;
    use incular_rendering::{GaussianBlur, ImageFilter};
    use incular_widgets::Placeholder;
    use incular_widgets::internal::WidgetTree;

    let build = |sigma| {
        ImageFiltered::new(ImageFilter::blur(sigma, sigma), Placeholder::new())
            .unwrap()
            .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(5.)).unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    tree.layout(constraints).unwrap();
    let render = tree.render_id(root);
    let (layer, first) = blur_stage(&mut tree).unwrap();
    assert_eq!(first, GaussianBlur::uniform(5.));
    tree.update(root, build(10.)).unwrap();
    tree.layout(constraints).unwrap();
    assert_eq!(tree.render_id(root), render);
    assert_eq!(
        blur_stage(&mut tree),
        Some((layer, GaussianBlur::uniform(10.)))
    );
}

#[test]
fn image_filter_with_a_shrink_wrapped_child_keeps_zero_size() {
    use incular_config::Constraints;
    use incular_core::Size;
    use incular_widgets::Placeholder;
    use incular_widgets::internal::WidgetTree;

    let mut tree = WidgetTree::new();
    let root = tree
        .mount(ImageFiltered::blur(5., Placeholder::new()).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    assert_eq!(
        tree.render_size(tree.render_id(root).unwrap()),
        Some(Size::ZERO)
    );
    let _ = tree.paint();
}
