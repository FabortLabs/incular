//! A GridView keeps a requested tile, not the rest of its former row.

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Directionality, GridView, KeepAlive, ScrollController, Semantics, SliverGridDelegate, Widget,
    internal::Key,
};

fn scene(controller: &ScrollController, keep: bool) -> Widget {
    let tiles = (0..20).map(|i| {
        let child = Semantics::new(Widget::box_(Size::new(50., 50.), Color::WHITE))
            .label(format!("tile {i}"))
            .container(true);
        KeepAlive::new(
            i == 1 && keep,
            Widget::from(child).with_key(Key::from(i as u64)),
        )
    });
    Directionality::new(
        TextDirection::Ltr,
        GridView::new(
            SliverGridDelegate::fixed_cross_axis_count(2).main_axis_extent(50.),
            tiles,
        )
        .cache_extent(0.)
        .controller(controller.clone()),
    )
    .into()
}

#[test]
fn parked_grid_tile_keeps_its_identity_without_retaining_its_unrequested_sibling() {
    let controller = ScrollController::new();
    let mut runtime = Runtime::new(scene(&controller, true)).unwrap();
    let view = Constraints::tight(Size::new(100., 100.));
    runtime.run_frame(view).unwrap();
    let sibling = Key::from(0_u64);
    let target = Key::from(1_u64);
    let first_sibling = runtime.tree().element_with_key(&sibling).unwrap();
    let first_target = runtime.tree().element_with_key(&target).unwrap();
    assert!(controller.jump_to(300.));
    runtime.run_frame(view).unwrap();
    assert_eq!(runtime.tree().element_with_key(&target), Some(first_target));
    assert!(!runtime.tree().element_exists(first_sibling));
    assert!(
        !runtime
            .tree()
            .semantics()
            .iter()
            .any(|(_, n)| n.label.as_deref() == Some("tile 1"))
    );
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, scene(&controller, true))
        .unwrap();
    runtime.run_frame(view).unwrap();
    assert_eq!(runtime.tree().element_with_key(&target), Some(first_target));
    assert!(controller.jump_to(0.));
    runtime.run_frame(view).unwrap();
    assert_eq!(runtime.tree().element_with_key(&target), Some(first_target));
    assert_ne!(
        runtime.tree().element_with_key(&sibling).unwrap(),
        first_sibling
    );

    assert!(controller.jump_to(300.));
    runtime.run_frame(view).unwrap();
    runtime
        .schedule_update(root, scene(&controller, false))
        .unwrap();
    runtime.run_frame(view).unwrap();
    assert!(!runtime.tree().element_exists(first_target));
}
