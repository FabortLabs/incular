use incular_config::Constraints;
use incular_core::{Color, Size, Transform};
use incular_rendering::{
    ColorFilter, DisplayList, FilterQuality, ImageFilter, ImageSampling, PaintCommand,
};
use incular_widgets::{BackdropFilter, ClipRect, ImageFiltered, Widget, internal::WidgetTree};

const VIEW: Size = Size {
    width: 32.,
    height: 16.,
};

fn pixels() -> Widget {
    Widget::box_(VIEW, Color::rgba(241, 5, 10, 255))
}

fn paint(tree: &mut WidgetTree) -> DisplayList {
    tree.layout(Constraints::tight(VIEW)).unwrap();
    tree.paint()
}

#[test]
fn both_default_and_explicit_image_matrix_qualities_reach_the_retained_stage() {
    let matrix = Transform::scale_non_uniform(0.25, 1.);
    assert_eq!(
        ImageFilter::matrix_with_quality(matrix, FilterQuality::Medium),
        ImageFilter::matrix(matrix)
    );
    for (quality, expected) in [
        (FilterQuality::None, ImageSampling::Nearest),
        (FilterQuality::Low, ImageSampling::Linear),
        (FilterQuality::Medium, ImageSampling::LinearMipmap),
        (FilterQuality::High, ImageSampling::MatrixCubic),
    ] {
        let mut tree = WidgetTree::new();
        let mask = ImageFiltered::new(ImageFilter::matrix_with_quality(matrix, quality), pixels())
            .unwrap();
        tree.mount(mask.into()).unwrap();
        assert!(paint(&mut tree).commands().iter().any(|command| matches!(
            command,
            PaintCommand::PushMatrixFilter { matrix: sampled, sampling, .. } if *sampled == matrix && *sampling == expected
        )));
    }
}

#[test]
fn backdrop_quality_changes_invalidate_the_same_layer_without_losing_composition() {
    let matrix = Transform::scale_non_uniform(0.25, 1.);
    let scene = |quality| {
        let requested = ImageFilter::matrix_with_quality(matrix, quality);
        let tint = ImageFilter::color(ColorFilter::invert(1.));
        let composed = ImageFilter::compose(tint, requested);
        Widget::from(ClipRect::new(
            BackdropFilter::image_filter(composed, pixels()).unwrap(),
        ))
    };
    let state = |frame: DisplayList| {
        frame
            .commands()
            .iter()
            .find_map(|command| match command {
                PaintCommand::PushBackdropFilter {
                    layer,
                    image_filter: Some(filter),
                    generation,
                    ..
                } => {
                    let stages = filter.stages();
                    assert_eq!(stages.len(), 2);
                    assert!(matches!(stages[1], ImageFilter::Color(_)));
                    Some((*layer, *generation, stages[0].matrix_parts().unwrap()))
                }
                _ => None,
            })
            .expect("a composed backdrop filter")
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(FilterQuality::Low)).unwrap();
    let low = state(paint(&mut tree));
    assert_eq!(low.2, (matrix, FilterQuality::Low));

    tree.update(root, scene(FilterQuality::Medium)).unwrap();
    let medium = state(paint(&mut tree));
    assert_eq!(medium.0, low.0, "quality is data, not a new layer");
    assert_ne!(
        medium.1, low.1,
        "a mismatched quality must not reuse the old capture result"
    );
    assert_eq!(medium.2, (matrix, FilterQuality::Medium));

    tree.update(root, scene(FilterQuality::High)).unwrap();
    let high = state(paint(&mut tree));
    assert_eq!(high.0, low.0);
    assert_ne!(high.1, medium.1);
    assert_eq!(high.2, (matrix, FilterQuality::High));
}
