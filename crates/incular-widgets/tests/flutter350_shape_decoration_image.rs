use incular_config::{AlignmentDirectional, Clip, Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_image::ImageHandle;
use incular_rendering::{FillRule, PaintCommand};
use incular_widgets::internal::{TreeError, WidgetTree};
use incular_widgets::{
    CircleBorder, DecoratedShapeBox, DecorationImage, Directionality, Positioned, ShapeDecoration,
    Stack, Widget,
};

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn pixels(width: u32, height: u32) -> ImageHandle {
    ImageHandle::from_rgba8(width, height, vec![255; (width * height * 4) as usize]).unwrap()
}

fn draws(
    image: DecorationImage,
    size: Size,
    origin: Offset,
    direction: Option<TextDirection>,
) -> Result<Vec<(Rect, Rect, bool)>, TreeError> {
    let child = DecoratedShapeBox::new(
        ShapeDecoration::new(CircleBorder::default()).image(image),
        Widget::box_(size, Color::TRANSPARENT),
    );
    let root: Widget = Stack::new([Positioned::new(child)
        .left(origin.x)
        .top(origin.y)
        .width(size.width)
        .height(size.height)])
    .clip_behavior(Clip::None)
    .into();
    let mut tree = WidgetTree::new();
    tree.mount(match direction {
        Some(direction) => Directionality::new(direction, root).into(),
        None => root,
    })?;
    tree.layout(Constraints::tight(Size::new(1200., 1200.)))?;
    tree.update_compositor(incular_core::time::Instant::now())
        .unwrap();

    let mut output = Vec::new();
    let mut transforms = vec![Transform::IDENTITY];
    let mut clips = Vec::new();
    for command in tree.paint().commands() {
        let current = *transforms.last().unwrap();
        match command {
            PaintCommand::PushTransform { transform } => transforms.push(current.then(*transform)),
            PaintCommand::PopTransform => {
                transforms.pop();
            }
            PaintCommand::PushClipPath { path, .. } => {
                let center = Offset::new(size.width / 2., size.height / 2.);
                assert!(path.contains(center, FillRule::NonZero));
                assert!(!path.contains(Offset::new(1., 1.), FillRule::NonZero));
                clips.push(true);
            }
            PaintCommand::PushClip { .. }
            | PaintCommand::PushClipRRect { .. }
            | PaintCommand::PushClipOval { .. } => clips.push(false),
            PaintCommand::PopClip => {
                clips.pop().expect("balanced clip stack");
            }
            PaintCommand::Image {
                source,
                destination,
                ..
            } => {
                assert!(
                    clips.contains(&true),
                    "the image must be inside the shape clip"
                );
                let a = current.transform_point(destination.origin);
                let b = current
                    .transform_point(destination.origin + Offset::new(destination.size.width, 0.));
                output.push((
                    *source,
                    current.transform_rect_bbox(*destination),
                    b.x < a.x,
                ));
            }
            _ => {}
        }
    }
    Ok(output)
}

#[test]
fn bottom_end_places_shape_images_in_rtl_and_uses_source_and_destination_rects() {
    let image =
        DecorationImage::new(pixels(100, 200)).alignment_geometry(AlignmentDirectional::BOTTOM_END);
    let rtl = draws(
        image.clone(),
        Size::new(1000., 1000.),
        Offset::new(20., -40.),
        Some(TextDirection::Rtl),
    )
    .unwrap();
    assert_eq!(
        rtl,
        [(rect(0., 0., 100., 200.), rect(20., 760., 100., 200.), false)]
    );
    let ltr = draws(
        image,
        Size::new(100., 200.),
        Offset::ZERO,
        Some(TextDirection::Ltr),
    )
    .unwrap();
    assert_eq!(
        ltr,
        [(rect(0., 0., 100., 200.), rect(0., 0., 100., 200.), false)]
    );
}

#[test]
fn directional_shape_image_requires_an_ambient_direction() {
    let image =
        DecorationImage::new(pixels(100, 200)).alignment_geometry(AlignmentDirectional::BOTTOM_END);
    let result = draws(image, Size::new(100., 100.), Offset::ZERO, None);
    assert!(
        matches!(
            &result,
            Err(TreeError::InvalidGeneratedChild { source, .. })
                if matches!(**source, TreeError::MissingDirectionality { widget: "DecorationImage", .. })
        ),
        "expected missing direction, got {result:?}"
    );
}

#[test]
fn shape_image_painter_mirrors_in_rtl_and_keeps_nine_slice() {
    let mut image = DecorationImage::new(pixels(40, 40));
    image.match_text_direction = true;
    image.center_slice = Some(rect(10., 10., 20., 20.));
    let size = Size::new(100., 100.);
    let ltr = draws(image.clone(), size, Offset::ZERO, Some(TextDirection::Ltr)).unwrap();
    let rtl = draws(image, size, Offset::ZERO, Some(TextDirection::Rtl)).unwrap();
    assert_eq!((ltr.len(), rtl.len()), (9, 9));
    assert!(ltr.iter().all(|draw| !draw.2));
    assert!(rtl.iter().all(|draw| draw.2));
    let top_left_pixels = rect(0., 0., 10., 10.);
    assert!(ltr.contains(&(top_left_pixels, rect(0., 0., 10., 10.), false)));
    assert!(rtl.contains(&(top_left_pixels, rect(90., 0., 10., 10.), true)));
}
