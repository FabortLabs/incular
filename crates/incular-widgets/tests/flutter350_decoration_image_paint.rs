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

#[test]
fn scaled_center_outside_either_image_axis_paints_the_full_bitmap() {
    let bitmap = pixels(300, 300);
    let id = bitmap.id();
    for slice in [
        rect(50., 40., 200., 220.),
        rect(50., 40., 200., 40.),
        rect(50., 40., 40., 220.),
        rect(-1., 40., 40., 40.),
    ] {
        let mut image = DecorationImage::new(bitmap.clone());
        image.scale = 2.;
        image.center_slice = Some(slice);
        assert_eq!(
            decoration_draws(image, Size::new(420., 400.), Offset::new(10., 20.)),
            [(id, rect(0., 0., 300., 300.), rect(10., 20., 420., 400.))],
            "slice {slice:?}"
        );
    }
}

fn close_rect(actual: Rect, expected: Rect) {
    for (actual, expected) in [
        (actual.origin.x, expected.origin.x),
        (actual.origin.y, expected.origin.y),
        (actual.size.width, expected.size.width),
        (actual.size.height, expected.size.height),
    ] {
        assert!((actual - expected).abs() < 0.0001, "{actual} != {expected}");
    }
}

fn bottom_right_scale_checks(fit: BoxFit) {
    let bitmap = pixels(100, 100);
    for scale in [1., 2., 3., 4.] {
        let mut image = DecorationImage::new(bitmap.clone());
        image.fit = Some(fit);
        image.scale = scale;
        image.alignment = Alignment::BOTTOM_RIGHT;
        let draws = decoration_draws(image, Size::new(250., 250.), Offset::new(30., 30.));
        let [(key, source, destination)] = draws.as_slice() else {
            panic!("one image draw expected at scale {scale}: {draws:?}");
        };
        assert_eq!((*key, *source), (bitmap.id(), rect(0., 0., 100., 100.)));
        let extent = 100. / scale;
        close_rect(
            *destination,
            rect(280. - extent, 280. - extent, extent, extent),
        );
    }
}

#[test]
fn none_selects_all_pixels_and_places_the_scaled_image_at_bottom_right() {
    bottom_right_scale_checks(BoxFit::None);
}

#[test]
fn scale_down_keeps_an_image_that_already_fits_at_its_logical_size() {
    bottom_right_scale_checks(BoxFit::ScaleDown);
}

#[test]
fn scale_down_fits_the_smaller_axis_without_cropping_the_source() {
    let bitmap = pixels(100, 100);
    let mut image = DecorationImage::new(bitmap.clone());
    image.scale = 2.;
    image.fit = Some(BoxFit::ScaleDown);
    image.alignment = Alignment::BOTTOM_RIGHT;
    assert_eq!(
        decoration_draws(image, Size::new(250., 20.), Offset::new(30., 30.)),
        [(
            bitmap.id(),
            rect(0., 0., 100., 100.),
            rect(260., 30., 20., 20.)
        )]
    );
}

#[test]
fn fit_and_source_scale_do_not_shift_the_default_center_alignment() {
    let bitmap = pixels(100, 100);
    for fit in [
        BoxFit::Contain,
        BoxFit::Cover,
        BoxFit::FitWidth,
        BoxFit::FitHeight,
        BoxFit::None,
        BoxFit::ScaleDown,
    ] {
        let mut image = DecorationImage::new(bitmap.clone());
        image.scale = 3.;
        image.fit = Some(fit);
        let draws = decoration_draws(image, Size::new(250., 250.), Offset::new(30., 30.));
        let [(id, _, destination)] = draws.as_slice() else {
            panic!("one centered image expected for {fit:?}: {draws:?}");
        };
        assert_eq!(*id, bitmap.id());
        let center = destination.origin
            + Offset::new(destination.size.width / 2., destination.size.height / 2.);
        assert!((center.x - 155.).abs() < 0.0001);
        assert!((center.y - 155.).abs() < 0.0001);
    }
}

#[test]
fn repeat_x_with_fit_height_paints_two_full_width_tiles() {
    let bitmap = pixels(100, 100);
    let mut image = DecorationImage::new(bitmap.clone());
    image.alignment = Alignment::TOP_LEFT;
    image.fit = Some(BoxFit::FitHeight);
    image.repeat = incular_widgets::ImageRepeat::RepeatX;
    let draws = decoration_draws(image, Size::new(400., 200.), Offset::new(30., 30.));
    assert_eq!(
        draws,
        [
            (
                bitmap.id(),
                rect(0., 0., 100., 100.),
                rect(30., 30., 200., 200.)
            ),
            (
                bitmap.id(),
                rect(0., 0., 100., 100.),
                rect(230., 30., 200., 200.)
            ),
        ]
    );
}

#[test]
fn repeat_y_with_fit_width_paints_two_full_height_tiles() {
    let bitmap = pixels(100, 100);
    let mut image = DecorationImage::new(bitmap.clone());
    image.alignment = Alignment::TOP_LEFT;
    image.fit = Some(BoxFit::FitWidth);
    image.repeat = incular_widgets::ImageRepeat::RepeatY;
    let draws = decoration_draws(image, Size::new(200., 400.), Offset::new(30., 30.));
    assert_eq!(
        draws,
        [
            (
                bitmap.id(),
                rect(0., 0., 100., 100.),
                rect(30., 30., 200., 200.)
            ),
            (
                bitmap.id(),
                rect(0., 0., 100., 100.),
                rect(30., 230., 200., 200.)
            ),
        ]
    );
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Ready(incular_widgets::ImageStream);

impl incular_widgets::ImageProvider for Ready {
    fn load(&self) -> Result<ImageHandle, incular_image::ImageError> {
        panic!("the decoration must use the synchronously ready provider stream");
    }
    fn resolve(&self, _: &incular_widgets::ImageConfiguration) -> incular_widgets::ImageStream {
        self.0.clone()
    }
}

#[test]
fn a_decoration_applies_its_scale_to_a_synchronously_resolved_provider() {
    let bitmap = pixels(100, 100);
    let mut image =
        DecorationImage::from_provider(Ready(incular_widgets::ImageStream::ready(bitmap.clone())));
    image.scale = 4.;
    image.alignment = Alignment::TOP_LEFT;
    assert_eq!(
        decoration_draws(image, Size::new(100., 100.), Offset::ZERO),
        [(
            bitmap.id(),
            rect(0., 0., 100., 100.),
            rect(0., 0., 25., 25.)
        )]
    );
}
