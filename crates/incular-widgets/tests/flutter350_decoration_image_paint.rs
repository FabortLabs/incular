//! Selected pinned Flutter `painting/paint_image_test.dart` registrations.

use incular_config::{Alignment, Constraints, EdgeInsets};
use incular_core::{Offset, Rect, Size, Transform};
use incular_image::{ImageHandle, ImageId};
use incular_rendering::PaintCommand;
use incular_widgets::internal::WidgetTree;
use incular_widgets::{BoxDecoration, BoxFit, Container, DecorationImage, Padding, Widget};

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn pixels(width: u32, height: u32) -> ImageHandle {
    ImageHandle::from_rgba8(width, height, vec![255; (width * height * 4) as usize]).unwrap()
}

fn decoration_draws(
    image: DecorationImage,
    size: Size,
    origin: Offset,
) -> Vec<(ImageId, Rect, Rect)> {
    let mut tree = WidgetTree::new();
    let content = Container::new()
        .width(size.width)
        .height(size.height)
        .decoration(BoxDecoration::new().image(image));
    let root: Widget = Padding::new(EdgeInsets::only(origin.x, origin.y, 0., 0.), content).into();
    tree.mount(root).unwrap();
    tree.layout(Constraints::tight(Size::new(
        origin.x + size.width,
        origin.y + size.height,
    )))
    .unwrap();
    tree.update_compositor(incular_core::time::Instant::now())
        .unwrap();
    let mut transforms = vec![Transform::IDENTITY];
    let mut draws = Vec::new();
    for command in tree.paint().commands() {
        let current = *transforms.last().unwrap();
        match command {
            PaintCommand::PushTransform { transform } => transforms.push(current.then(*transform)),
            PaintCommand::PopTransform => {
                transforms.pop();
            }
            PaintCommand::Image {
                image,
                source,
                destination,
                ..
            } => {
                draws.push((
                    image.id(),
                    *source,
                    current.transform_rect_bbox(*destination),
                ));
            }
            _ => {}
        }
    }
    draws
}

#[test]
fn cover_and_align_keeps_pixel_crop_and_world_destination_separate() {
    let image = pixels(300, 300);
    let id = image.id();
    let mut image = DecorationImage::new(image);
    image.fit = Some(BoxFit::Cover);
    image.alignment = Alignment::CENTER_LEFT;
    let draws = decoration_draws(image, Size::new(200., 100.), Offset::new(50., 75.));
    assert_eq!(
        draws,
        [(id, rect(0., 75., 300., 150.), rect(50., 75., 200., 100.))]
    );
}

fn nine_slice(bounds: Size, slice: Rect) -> Vec<(ImageId, Rect, Rect)> {
    let image = pixels(208, 142);
    let id = image.id();
    let mut image = DecorationImage::new(image);
    image.center_slice = Some(slice);
    image.scale = 3.;
    let draws = decoration_draws(image, bounds, Offset::ZERO);
    assert!(!draws.is_empty(), "the nine-slice paint must draw");
    assert!(
        draws.iter().all(|(image, _, _)| *image == id),
        "draw uses the original bitmap"
    );
    draws
}

#[test]
fn nine_slice_tolerates_noninteger_pixel_to_logical_round_trip() {
    let draws = nine_slice(Size::new(200., 100.), rect(15., 15., 2., 2.));
    assert_eq!(draws.len(), 9);
    assert_eq!(draws[4].1, rect(45., 45., 6., 6.));
    assert_eq!(draws[0].2, rect(0., 0., 15., 15.));
    assert_eq!(draws[8].2.origin.x + draws[8].2.size.width, 200.);
    assert_eq!(draws[8].2.origin.y + draws[8].2.size.height, 100.);
}

#[test]
fn nine_slice_can_paint_even_when_a_destination_axis_is_smaller_than_its_borders() {
    nine_slice(Size::new(100., 46.), rect(15.5, 15.5, 1., 1.));
}
