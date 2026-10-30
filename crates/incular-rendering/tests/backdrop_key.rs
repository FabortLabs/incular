//! Flutter's `BackdropKey` on backdrop-filter layers.

use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{
    BackdropKey, BlendMode, DisplayList, GaussianBlur, LayerTree, PaintCommand,
};

fn keys(tree: &mut LayerTree) -> Vec<Option<BackdropKey>> {
    tree.flatten()
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushBackdropFilter { backdrop_key, .. } => Some(*backdrop_key),
            _ => None,
        })
        .collect()
}

#[test]
fn backdrop_filters_carry_their_shared_key() {
    let mut tree = LayerTree::new();
    let bounds = Rect::from_origin_size(Offset::ZERO, Size::new(10., 10.));
    let mut list = DisplayList::new();
    list.push(PaintCommand::Rect {
        rect: bounds,
        color: Color::WHITE,
    });
    let picture = tree.create_picture(list, bounds);
    let filter = tree.create_backdrop_filter(GaussianBlur::uniform(2.), BlendMode::SrcOver, true);
    tree.set_children(filter, vec![picture]);
    tree.set_root(filter);
    assert_eq!(keys(&mut tree), [None]);

    let key = BackdropKey::new();
    assert!(tree.update_backdrop_key(filter, Some(key)));
    // An unchanged key is no change.
    assert!(!tree.update_backdrop_key(filter, Some(key)));
    assert_eq!(keys(&mut tree), [Some(key)]);
    assert_ne!(key, BackdropKey::new());
}
