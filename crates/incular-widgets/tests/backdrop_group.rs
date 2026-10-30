//! Flutter's `BackdropGroup`, `BackdropFilter.grouped` and `BackdropKey`.

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_rendering::{BackdropKey, GaussianBlur, PaintCommand};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{BackdropFilter, BackdropGroup, Column, Widget};

fn filter(grouped: bool) -> Widget {
    let child = Widget::box_(Size::new(10., 10.), Color::WHITE);
    if grouped {
        BackdropFilter::grouped(GaussianBlur::uniform(4.), child).into()
    } else {
        BackdropFilter::blur(4., child).into()
    }
}

fn keys(tree: &mut WidgetTree) -> Vec<Option<BackdropKey>> {
    tree.layout(Constraints::loose(Size::new(100., 200.)))
        .expect("layout");
    tree.paint()
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushBackdropFilter { backdrop_key, .. } => Some(*backdrop_key),
            _ => None,
        })
        .collect()
}

fn scene() -> Widget {
    Column::new([
        Widget::from(BackdropGroup::new(Column::new([
            filter(true),
            filter(true),
        ]))),
        BackdropGroup::new(filter(true)).into(),
        filter(false),
        // Without a group a grouped filter has no key.
        filter(true),
    ])
    .into()
}

#[test]
fn grouped_filters_share_their_groups_key() {
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene()).expect("mount");
    let first = keys(&mut tree);
    assert_eq!(first.len(), 5);
    assert!(first[0].is_some());
    assert_eq!(first[0], first[1]);
    assert!(first[2].is_some());
    assert_ne!(first[0], first[2]);
    assert_eq!(first[3], None);
    assert_eq!(first[4], None);
    // A group keeps its key across rebuilds.
    tree.update(root, scene()).expect("update");
    assert_eq!(keys(&mut tree), first);
}

#[test]
fn an_explicit_key_is_shared_across_groups_and_filters() {
    let key = BackdropKey::new();
    let mut tree = WidgetTree::new();
    tree.mount(
        Column::new([
            Widget::from(BackdropGroup::new(filter(true)).backdrop_key(key)),
            BackdropFilter::blur(4., Widget::box_(Size::new(10., 10.), Color::WHITE))
                .backdrop_key(key)
                .into(),
        ])
        .into(),
    )
    .expect("mount");
    assert_eq!(keys(&mut tree), [Some(key), Some(key)]);
}
