use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{DisplayList, LayerId, LayerTree, PaintCommand};

fn picture(tree: &mut LayerTree, x: f32) -> LayerId {
    let rect = Rect::from_origin_size(Offset::new(x, 0.), Size::new(2., 2.));
    let mut list = DisplayList::new();
    list.push(PaintCommand::Rect {
        rect,
        color: Color::WHITE,
    });
    tree.create_picture(list, rect)
}

fn pass(list: &DisplayList) -> Option<(LayerId, f32, u64)> {
    list.commands().iter().find_map(|command| match command {
        PaintCommand::PushOpacity {
            layer,
            alpha,
            generation,
            ..
        } => Some((*layer, *alpha, *generation)),
        _ => None,
    })
}

#[test]
fn an_opacity_layer_without_children_never_submits_a_pass() {
    for alpha in [0., 128. / 255., 1.] {
        let mut tree = LayerTree::new();
        let opacity = tree.create_opacity(alpha);
        tree.set_root(opacity);
        let list = tree.flatten();
        assert!(
            list.commands().is_empty(),
            "empty opacity at alpha {alpha} submitted work"
        );
        assert!(tree.flattened_pictures().is_empty());
        assert!(
            tree.contains(opacity),
            "no draw does not mean its identity is removed"
        );
    }
}

#[test]
fn detaching_the_last_picture_ends_its_opacity_pass_and_rebinding_starts_new_source() {
    let alpha = 128. / 255.;
    let mut tree = LayerTree::new();
    let opacity = tree.create_opacity(alpha);
    let first = picture(&mut tree, 0.);
    tree.set_children(opacity, vec![first]);
    tree.set_root(opacity);
    let original = tree.flatten();
    let (id, value, generation) = pass(&original).unwrap();
    assert_eq!((id, value), (opacity, alpha));
    assert_eq!(tree.flattened_pictures()[0].layer, first);

    tree.set_children(opacity, vec![]);
    tree.remove(first);
    let empty = tree.flatten();
    assert!(pass(&empty).is_none());
    assert!(tree.flattened_pictures().is_empty());
    assert!(!tree.contains(first));
    assert_eq!(pass(&original), Some((opacity, alpha, generation)));

    let second = picture(&mut tree, 5.);
    tree.set_children(opacity, vec![second]);
    let (new_id, value, new_generation) = pass(&tree.flatten()).unwrap();
    assert_eq!((new_id, value), (opacity, alpha));
    assert_ne!(new_generation, generation);
    assert_eq!(tree.flattened_pictures()[0].layer, second);
}
