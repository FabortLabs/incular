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
    tree.layout(Constraints::loose(Size::new(100., 600.)))
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

#[test]
fn explicit_key_reaches_both_layers_and_is_removed_on_update() {
    let key = BackdropKey::new();
    let build = |enabled: bool| {
        Column::new([0, 1].map(|_| {
            let filter =
                BackdropFilter::blur(40., Widget::box_(Size::new(100., 200.), Color::WHITE));
            let filter = if enabled {
                filter.backdrop_key(key)
            } else {
                filter
            };
            incular_widgets::ClipRect::new(filter)
        }))
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(true)).unwrap();
    assert_eq!(keys(&mut tree), [Some(key), Some(key)]);
    tree.update(root, build(false)).unwrap();
    assert_eq!(keys(&mut tree), [None, None]);
}

#[test]
fn grouped_filters_share_the_key_of_their_nearest_group() {
    let mut tree = WidgetTree::new();
    tree.mount(BackdropGroup::new(Column::new([filter(true), filter(true)])).into())
        .unwrap();
    let stages = keys(&mut tree);
    assert_eq!(stages.len(), 2);
    assert!(stages[0].is_some());
    assert_eq!(stages[0], stages[1]);
}

#[test]
fn backdrop_without_a_child_does_not_composite_at_zero_area() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(BackdropFilter::without_child(GaussianBlur::uniform(40.)).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    let render = tree.render_id(root).unwrap();
    assert_eq!(tree.render_size(render), Some(Size::ZERO));
    assert!(
        tree.paint()
            .commands()
            .iter()
            .all(|command| !matches!(command, PaintCommand::PushBackdropFilter { .. }))
    );
}

#[test]
fn configured_blur_resolves_local_bounds_again_on_resize() {
    use incular_core::{Offset, Rect};
    use incular_rendering::{ImageFilter, ImageFilterConfig, TileMode};

    let config = ImageFilterConfig::blur_with_sampling(4., 2., TileMode::Mirror, true);
    let widget = || {
        BackdropFilter::filter_config(
            config.clone(),
            Widget::box_(Size::new(100., 100.), Color::WHITE),
        )
        .unwrap()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget().into()).unwrap();
    for size in [Size::new(20., 12.), Size::new(42., 18.)] {
        tree.layout(Constraints::tight(size)).unwrap();
        let painted = tree.paint();
        let filter = painted.commands().iter().find_map(|command| match command {
            PaintCommand::PushBackdropFilter { image_filter, .. } => image_filter.as_deref(),
            _ => None,
        });
        let Some(ImageFilter::SampledBlur(blur)) = filter else {
            panic!("layout-aware blur must reach the flattened scene");
        };
        assert_eq!(
            blur.bounds,
            Some(Rect::from_origin_size(Offset::ZERO, size))
        );
        assert_eq!(blur.tile_mode, TileMode::Mirror);
        assert_eq!(blur.blur, GaussianBlur::new(4., 2.));
    }
    tree.update(root, widget().enabled(false).into()).unwrap();
    tree.layout(Constraints::tight(Size::new(42., 18.)))
        .unwrap();
    assert!(
        tree.paint()
            .commands()
            .iter()
            .all(|command| !matches!(command, PaintCommand::PushBackdropFilter { .. }))
    );
}
