//! Direct and configurable filters on the same retained backdrop widget.
use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{
    BackdropKey, GaussianBlur, ImageFilter, ImageFilterConfig, PaintCommand, TileMode,
};
use incular_widgets::{BackdropFilter, Widget, internal::WidgetTree};

#[test]
fn switching_filter_forms_preserves_retained_geometry_key_and_stage_order() {
    let size = Size::new(40., 20.);
    let bounds = Constraints::tight(size);
    let key = BackdropKey::new();
    // Same direct values as the pinned RenderBackdropFilter getter/setter test.
    let zero_blur = ImageFilter::blur(0., 0.);
    let identity = ImageFilter::matrix(Transform::IDENTITY);
    let composed = ImageFilter::compose(zero_blur.clone(), identity.clone());
    let direct = BackdropFilter::image_filter(zero_blur.clone(), Widget::box_(size, Color::WHITE))
        .unwrap()
        .backdrop_key(key);
    assert_eq!(direct.direct_image_filter(), Some(zero_blur.clone()));
    assert_eq!(
        direct.filter_configuration(),
        ImageFilterConfig::new(zero_blur.clone())
    );

    let mut tree = WidgetTree::new();
    let root = tree.mount(direct.clone().into()).unwrap();
    tree.layout(bounds).unwrap();
    let render = tree.render_id(root).unwrap();
    let root_layer = tree.retained_root_layer().unwrap();
    let leaf = tree.children(root).unwrap()[0];

    let matrix_config = ImageFilterConfig::new(identity.clone());
    let configured_matrix = direct.with_filter_config(matrix_config.clone()).unwrap();
    assert_eq!(configured_matrix.direct_image_filter(), Some(identity));
    assert_eq!(configured_matrix.filter_configuration(), matrix_config);
    tree.update(root, configured_matrix.clone().into()).unwrap();

    let direct_compose = configured_matrix
        .with_image_filter(composed.clone())
        .unwrap();
    assert_eq!(direct_compose.direct_image_filter(), Some(composed.clone()));
    assert_eq!(
        direct_compose.filter_configuration(),
        ImageFilterConfig::new(composed)
    );

    // Nonidentity stages make the recorded order and bounds observable.
    let translation = ImageFilter::matrix(Transform::translation(Offset::new(4., 3.)));
    let active_blur = ImageFilter::blur(2., 3.);
    let direct_compose = direct_compose
        .with_image_filter(ImageFilter::compose(
            active_blur.clone(),
            translation.clone(),
        ))
        .unwrap();
    tree.update(root, direct_compose.clone().into()).unwrap();
    tree.layout(bounds).unwrap();
    let display = tree.paint();
    let ordered = display.commands().iter().find_map(|command| match command {
        PaintCommand::PushBackdropFilter {
            image_filter: Some(filter),
            backdrop_key,
            ..
        } => {
            assert_eq!(*backdrop_key, Some(key));
            Some(filter.stages())
        }
        _ => None,
    });
    assert_eq!(ordered, Some(vec![translation, active_blur]));

    let structured = ImageFilterConfig::blur_with_sampling(10., 10., TileMode::Clamp, true);
    let configured = direct_compose
        .with_filter_config(structured.clone())
        .unwrap();
    assert_eq!(configured.filter_configuration(), structured);
    assert_eq!(configured.direct_image_filter(), None);
    tree.update(root, configured.clone().into()).unwrap();
    tree.layout(bounds).unwrap();
    let display = tree.paint();
    let sampled = display.commands().iter().find_map(|command| match command {
        PaintCommand::PushBackdropFilter {
            image_filter,
            backdrop_key,
            ..
        } => {
            assert_eq!(*backdrop_key, Some(key));
            image_filter.as_deref()
        }
        _ => None,
    });
    let Some(ImageFilter::SampledBlur(sampled)) = sampled else {
        panic!("the layout-aware filter did not replace the direct stages");
    };
    assert_eq!(sampled.blur, GaussianBlur::uniform(10.));
    assert_eq!(
        sampled.bounds,
        Some(Rect::from_origin_size(Offset::ZERO, size))
    );
    assert_eq!(sampled.tile_mode, TileMode::Clamp);

    let legacy = configured.filter(GaussianBlur::uniform(4.));
    assert_eq!(
        legacy.direct_image_filter(),
        Some(ImageFilter::blur(4., 4.))
    );
    tree.update(root, legacy.into()).unwrap();
    tree.layout(bounds).unwrap();
    assert!(tree.paint().commands().iter().any(|command| matches!(
        command,
        PaintCommand::PushBackdropFilter { blur, image_filter: None, backdrop_key: Some(current), .. }
            if *current == key && *blur == GaussianBlur::uniform(4.)
    )));
    assert_eq!(tree.render_id(root), Some(render));
    assert_eq!(tree.retained_root_layer(), Some(root_layer));
    assert_eq!(tree.children(root).unwrap(), [leaf]);
    assert_eq!(tree.element_bounds(leaf).unwrap().size, size);
}
