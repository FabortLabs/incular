use incular_config::Constraints;
use incular_core::{Color, Lerp, Size};
use incular_image::{ImageHandle, ImageId};
use incular_rendering::{BlendMode, PaintCommand};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{BoxDecoration, BoxFit, Container, DecorationImage, Widget};

fn image(rgba: [u8; 4], opacity: f32) -> DecorationImage {
    let mut image = DecorationImage::new(ImageHandle::from_rgba8(1, 1, rgba.to_vec()).unwrap());
    image.fit = Some(BoxFit::Fill);
    image.opacity = opacity;
    image
}

fn mounted_images(
    decoration: BoxDecoration,
    background: Color,
) -> Vec<(ImageId, f32, Vec<BlendMode>)> {
    let mut tree = WidgetTree::new();
    let size = Size::new(16., 16.);
    let root: Widget = Container::new()
        .color(background)
        .child(Container::new().decoration(decoration))
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
    images
}

#[test]
fn both_weighted_images_are_isolated_from_the_opaque_backdrop() {
    let red = image([255, 0, 0, 255], 0.5);
    let green = image([0, 255, 0, 255], 0.5);
    let ids = [
        red.image.decoded().unwrap().id(),
        green.image.decoded().unwrap().id(),
    ];
    let left = BoxDecoration::new().image(red);
    let right = BoxDecoration::new().image(green);
    let images = mounted_images(left.lerp(&right, 0.5), Color::rgba(0, 0, 255, 255));
    assert_eq!(
        images,
        [
            (ids[0], 0.25, vec![BlendMode::SrcOver]),
            (ids[1], 0.25, vec![BlendMode::SrcOver, BlendMode::Plus]),
        ]
    );
}

#[test]
fn retargeted_crossfades_restore_each_nested_layer_with_source_over() {
    use BlendMode::{Plus, SrcOver};
    let red = image([255, 0, 0, 255], 1.);
    let green = image([0, 255, 0, 255], 1.);
    let blue = image([0, 0, 255, 255], 1.);
    let [red_id, green_id, blue_id] = [
        red.image.decoded().unwrap().id(),
        green.image.decoded().unwrap().id(),
        blue.image.decoded().unwrap().id(),
    ];
    let group = DecorationImage::lerp_optional(Some(&green), Some(&blue), 0.5).unwrap();
    let red_then_group = DecorationImage::lerp_optional(Some(&red), Some(&group), 0.5).unwrap();
    let group_then_red = DecorationImage::lerp_optional(Some(&group), Some(&red), 0.5).unwrap();
    assert_eq!(
        mounted_images(BoxDecoration::new().image(red_then_group), Color::BLACK),
        [
            (red_id, 0.5, vec![SrcOver]),
            (green_id, 0.25, vec![SrcOver, SrcOver, Plus]),
            (blue_id, 0.25, vec![SrcOver, SrcOver, Plus]),
        ]
    );
    assert_eq!(
        mounted_images(BoxDecoration::new().image(group_then_red), Color::BLACK),
        [
            (green_id, 0.25, vec![SrcOver, SrcOver]),
            (blue_id, 0.25, vec![SrcOver, SrcOver, Plus]),
            (red_id, 0.5, vec![SrcOver, Plus]),
        ]
    );
}
