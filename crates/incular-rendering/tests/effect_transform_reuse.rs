//! Re-flattening a scene whose effect layers did not change hands out the
//! same shared transforms instead of allocating new ones each frame.

use std::sync::Arc;

use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{
    BlendMode, Brush, DisplayList, ImageSampling, LayerId, LayerTree, PaintCommand,
};

struct Scene {
    tree: LayerTree,
    root: LayerId,
    opacity: LayerId,
}

fn picture(tree: &mut LayerTree) -> LayerId {
    tree.create_picture(
        DisplayList::new(),
        Rect::from_origin_size(Offset::ZERO, Size::new(10., 10.)),
    )
}

/// A resampled transform and a shader mask beside an opacity layer whose
/// alpha changes, so every flatten walks the effect layers again.
fn scene() -> Scene {
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::translation(Offset::new(5., 0.)));
    let scaled = tree.create_transform(Transform::scale(2.));
    assert!(tree.update_transform_sampling(scaled, Some(ImageSampling::Nearest)));
    let scaled_child = picture(&mut tree);
    tree.set_children(scaled, vec![scaled_child]);
    let mask = tree.create_shader_mask(
        Brush::from(Color::WHITE),
        BlendMode::SrcOver,
        Size::new(10., 10.),
        Transform::translation(Offset::new(1., 1.)),
    );
    let mask_child = picture(&mut tree);
    tree.set_children(mask, vec![mask_child]);
    let opacity = tree.create_opacity(1.);
    let opacity_child = picture(&mut tree);
    tree.set_children(opacity, vec![opacity_child]);
    tree.set_children(root, vec![scaled, mask, opacity]);
    tree.set_root(root);
    Scene {
        tree,
        root,
        opacity,
    }
}

fn stage(list: &DisplayList) -> Arc<Transform> {
    list.commands()
        .iter()
        .find_map(|command| match command {
            PaintCommand::PushMatrixFilter { stage, .. } => Some(Arc::clone(stage)),
            _ => None,
        })
        .expect("matrix filter stage")
}

fn mask_transform(list: &DisplayList) -> Arc<Transform> {
    list.commands()
        .iter()
        .find_map(|command| match command {
            PaintCommand::PushShaderMask { mask_transform, .. } => Some(Arc::clone(mask_transform)),
            _ => None,
        })
        .expect("shader mask")
}

#[test]
fn unchanged_effect_transforms_are_shared_across_flattens() {
    let mut s = scene();
    let first = s.tree.flatten();
    assert!(s.tree.update_opacity(s.opacity, 0.5));
    let second = s.tree.flatten();
    assert!(Arc::ptr_eq(&stage(&first), &stage(&second)));
    assert!(Arc::ptr_eq(
        &mask_transform(&first),
        &mask_transform(&second)
    ));
}

#[test]
fn a_moved_stage_gets_its_new_transform() {
    let mut s = scene();
    let first = stage(&s.tree.flatten());
    assert!(
        s.tree
            .update_transform(s.root, Transform::translation(Offset::new(9., 0.)))
    );
    let moved = stage(&s.tree.flatten());
    assert!(!Arc::ptr_eq(&first, &moved));
    assert_eq!(*moved, Transform::translation(Offset::new(9., 0.)));
    assert_eq!(*first, Transform::translation(Offset::new(5., 0.)));
}
