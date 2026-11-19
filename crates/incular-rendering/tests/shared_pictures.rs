use incular_core::{Offset, Rect, Size};
use incular_image::ImageHandle;
use incular_rendering::{DisplayList, ImageSampling, LayerKind, LayerTree, PaintCommand};
use std::sync::Arc;

#[test]
fn identical_thumbnail_layers_share_one_recording_and_repaint_independently() {
    let bounds = Rect::from_origin_size(Offset::ZERO, Size::new(16., 16.));
    let image = ImageHandle::from_rgba8(1, 1, vec![255; 4]).unwrap();
    let record = |destination| {
        let mut list = DisplayList::new();
        list.push(PaintCommand::Image {
            image: image.clone(),
            source: Rect::from_origin_size(Offset::ZERO, Size::new(1., 1.)),
            destination,
            sampling: ImageSampling::Linear,
        });
        list
    };
    let picture = |tree: &LayerTree, id| match tree.layer(id).unwrap().0 {
        LayerKind::Picture { display_list, .. } => Arc::clone(display_list),
        _ => unreachable!(),
    };
    let mut tree = LayerTree::new();
    let original = record(bounds);
    let mut layers = Vec::new();
    for _ in 0..100 {
        let id = tree.create_picture(DisplayList::new(), bounds);
        assert!(tree.update_picture_if_changed(id, &original, bounds));
        layers.push(id);
    }
    let retained = picture(&tree, layers[0]);
    assert!(
        layers
            .iter()
            .all(|&id| Arc::ptr_eq(&retained, &picture(&tree, id)))
    );
    let different = record(Rect::from_origin_size(Offset::ZERO, Size::new(8., 8.)));
    assert!(tree.update_picture_if_changed(layers[1], &different, bounds));
    assert_eq!(*picture(&tree, layers[1]), different);
    assert_eq!(*picture(&tree, layers[0]), original);
}
