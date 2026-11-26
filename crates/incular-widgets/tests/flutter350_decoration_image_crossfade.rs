use incular_config::Constraints;
use incular_core::{Color, Lerp, Size};
use incular_image::ImageHandle;
use incular_rendering::{BlendMode, PaintCommand};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{BoxDecoration, BoxFit, Container, DecorationImage, Widget};

#[test]
fn both_weighted_images_are_isolated_from_the_opaque_backdrop() {
    let image = |rgba| {
        let mut image = DecorationImage::new(ImageHandle::from_rgba8(1, 1, rgba).unwrap());
        image.fit = Some(BoxFit::Fill);
        image.opacity = 0.5;
        image
    };
    let red = image(vec![255, 0, 0, 255]);
    let green = image(vec![0, 255, 0, 255]);
    let ids = [red.image.id(), green.image.id()];
    let left = BoxDecoration::new().image(red);
    let right = BoxDecoration::new().image(green);
    let mut tree = WidgetTree::new();
    let size = Size::new(16., 16.);
    let root: Widget = Container::new()
        .color(Color::rgba(0, 0, 255, 255))
        .child(Container::new().decoration(left.lerp(&right, 0.5)))
        .into();
    tree.mount(root).unwrap();
    tree.layout(Constraints::tight(size)).unwrap();
    tree.update_compositor(incular_core::time::Instant::now())
        .unwrap();
    let list = tree.paint();
    let mut images = Vec::new();
    let mut blending = Vec::new();
    let mut opacity = Vec::new();
    for command in list.commands() {
        match command {
            PaintCommand::PushBlend { mode, .. } => blending.push(*mode),
            PaintCommand::PopEffect => {
                blending.pop().expect("blend has a matching push");
            }
            PaintCommand::PushOpacity { alpha, .. } => opacity.push(*alpha),
            PaintCommand::PopOpacity => {
                opacity.pop().expect("opacity has a matching push");
            }
            PaintCommand::Image { image, .. } => {
                images.push((
                    image.id(),
                    opacity.iter().product::<f32>(),
                    blending.clone(),
                ));
            }
            _ => {}
        }
    }
    assert_eq!(
        images,
        [
            (ids[0], 0.25, vec![BlendMode::SrcOver]),
            (ids[1], 0.25, vec![BlendMode::SrcOver, BlendMode::Plus]),
        ]
    );
}
