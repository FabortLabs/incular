use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size};
use incular_image::ImageHandle;
use incular_rendering::{BlendMode, ColorFilter, PaintCommand};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{BoxDecoration, BoxFit, DecoratedBox, DecorationImage, SizedBox};

fn red() -> DecorationImage {
    DecorationImage::new(ImageHandle::from_rgba8(1, 1, vec![255, 0, 0, 255]).unwrap())
}

fn commands(mut image: DecorationImage, fill: bool) -> Vec<PaintCommand> {
    if fill {
        image.fit = Some(BoxFit::Fill);
    }
    commands_in(image, Size::new(16., 16.))
}

fn commands_in(image: DecorationImage, size: Size) -> Vec<PaintCommand> {
    let mut tree = WidgetTree::new();
    tree.mount(
        DecoratedBox::new(SizedBox::shrink())
            .decoration(
                BoxDecoration::new()
                    .color(Color::rgba(0, 0, 255, 255))
                    .image(image),
            )
            .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(size)).unwrap();
    tree.update_compositor(incular_core::time::Instant::now())
        .unwrap();
    tree.paint().commands().to_vec()
}

fn sample_center(commands: &[PaintCommand]) -> [u8; 4] {
    let mut scopes = Vec::new();
    let mut rgb = None;
    for command in commands {
        match command {
            PaintCommand::PushColorFilter { filter, .. } => scopes.push((*filter, 1.0)),
            PaintCommand::PushOpacity { alpha, .. } => {
                scopes.push((ColorFilter::identity(), *alpha))
            }
            PaintCommand::PopEffect | PaintCommand::PopOpacity => {
                scopes.pop().unwrap();
            }
            PaintCommand::Image { image, .. } => {
                let pixel = image.decoded().pixels();
                let scale = |value| f32::from(value) / 255.;
                let mut sample = [
                    scale(pixel[0]),
                    scale(pixel[1]),
                    scale(pixel[2]),
                    scale(pixel[3]),
                ];
                for (filter, opacity) in scopes.iter().rev() {
                    sample = filter.apply(sample);
                    sample[3] *= opacity;
                }
                let over = [0.0, 0.0, 1.0];
                let blend = |index: usize| {
                    (255.0 * (sample[index] * sample[3] + over[index] * (1.0 - sample[3]))).round()
                        as u8
                };
                rgb = Some([blend(0), blend(1), blend(2), 255]);
            }
            _ => {}
        }
    }
    assert!(scopes.is_empty());
    rgb.unwrap()
}

#[test]
fn decoration_image_inversion_applies_after_its_color_filter() {
    let filtered = red().invert_colors(true).color_filter(ColorFilter::blend(
        Color::rgba(0, 255, 0, 255),
        BlendMode::SrcIn,
    ));
    assert_eq!(sample_center(&commands(filtered, true)), [255, 0, 255, 255]);
}

#[test]
fn decoration_opacity_multiplies_the_filtered_alpha_once() {
    let mut filtered = red().color_filter(ColorFilter::blend(
        Color::rgba(0, 255, 0, 128),
        BlendMode::SrcIn,
    ));
    filtered.opacity = 0.5;
    assert_eq!(sample_center(&commands(filtered, true)), [0, 64, 191, 255]);
}

#[test]
fn an_inverted_decoration_does_not_filter_its_background() {
    let list = commands(red().invert_colors(true), true);
    let filter = list
        .iter()
        .position(|item| matches!(item, PaintCommand::PushColorFilter { .. }))
        .unwrap();
    assert!(
        list[..filter]
            .iter()
            .any(|item| matches!(item, PaintCommand::Rect { .. } | PaintCommand::RRect { .. }))
    );
    assert_eq!(sample_center(&list), [0, 255, 255, 255]);
}

#[test]
fn a_filter_that_affects_transparent_pixels_is_bounded_to_the_image_draw() {
    let filtered = red().color_filter(ColorFilter::blend(
        Color::rgba(0, 255, 0, 255),
        BlendMode::Src,
    ));
    let list = commands(filtered, false);
    let bounds = list
        .iter()
        .find_map(|item| match item {
            PaintCommand::PushColorFilter { bounds, .. } => Some(*bounds),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        bounds,
        Rect::from_origin_size(Offset::new(7.5, 7.5), Size::new(1., 1.))
    );
}

#[test]
fn retained_nine_slice_keeps_geometry_filter_and_opacity_together() {
    use incular_config::Alignment;
    use incular_rendering::{FilterQuality, ImageSampling};
    use incular_widgets::ImageRepeat;
    let tuple = |x, y, w, h| Rect::from_origin_size(Offset::new(x, y), Size::new(w, h));
    let bitmap = ImageHandle::from_rgba8(100, 100, [255, 0, 0, 255].repeat(10000)).unwrap();
    let mut image = DecorationImage::new(bitmap.clone())
        .invert_colors(true)
        .color_filter(ColorFilter::blend(
            Color::rgba(0, 255, 0, 255),
            BlendMode::Src,
        ));
    image.fit = Some(BoxFit::Contain);
    image.alignment = Alignment::BOTTOM_LEFT;
    image.center_slice = Some(tuple(10., 20., 30., 40.));
    image.repeat = ImageRepeat::RepeatY;
    image.opacity = 0.5;
    image.filter_quality = FilterQuality::High;
    let commands = commands_in(image, Size::new(100., 100.));
    let tiles = commands
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Image {
                image,
                source,
                destination,
                sampling,
                ..
            } => {
                assert_eq!(image.id(), bitmap.id());
                assert_eq!(*sampling, ImageSampling::Linear);
                Some((*source, *destination))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(tiles.len(), 9);
    assert_eq!(
        tiles[4],
        (tuple(10., 20., 30., 40.), tuple(10., 20., 30., 40.))
    );
    assert_eq!(tiles[0].1.origin, Offset::ZERO);
    assert_eq!(tiles[8].1, tuple(40., 60., 60., 40.));
    assert_eq!(sample_center(&commands), [128, 0, 255, 255]);
}
