//! A fixed-extent list item or page preserves requested state only.

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Directionality, KeepAlive, ListView, PageController, PageView, ScrollController, Semantics,
    Widget, internal::Key,
};

fn scene(controller: &ScrollController, page: bool, keep: bool) -> Widget {
    let rows = (0..12).map(|i| {
        let widget = Semantics::new(Widget::box_(Size::new(100., 50.), Color::WHITE))
            .label(format!("tile {i}"))
            .container(true);
        KeepAlive::new(
            i == 0 && keep,
            Widget::from(widget).with_key(Key::from(i as u64)),
        )
    });
    let view: Widget = if page {
        PageView::new(rows)
            .page_snapping(false)
            .controller(PageController::from(controller.clone()))
            .into()
    } else {
        ListView::new(rows)
            .item_extent(50.)
            .cache_extent(0.)
            .controller(controller.clone())
            .into()
    };
    Directionality::new(TextDirection::Ltr, view).into()
}

#[test]
fn fixed_extent_views_retain_hide_and_then_release_the_requested_child() {
    let bounds = Constraints::tight(Size::new(100., 100.));
    for page in [false, true] {
        let controller = ScrollController::new();
        let mut runtime = Runtime::new(scene(&controller, page, true)).unwrap();
        runtime.run_frame(bounds).unwrap();
        let key = Key::from(0_u64);
        let first = runtime.tree().element_with_key(&key).unwrap();
        let visible = |runtime: &Runtime| {
            runtime
                .tree()
                .semantics()
                .iter()
                .any(|(_, n)| n.label.as_deref() == Some("tile 0"))
        };
        assert!(visible(&runtime));
        assert!(controller.jump_to(300.));
        runtime.run_frame(bounds).unwrap();
        assert_eq!(
            runtime.tree().element_with_key(&key),
            Some(first),
            "page={page}"
        );
        assert!(!visible(&runtime));

        let root = runtime.tree().root().unwrap();
        // A new descriptor still asking for the same keyed item must not
        // lose its parked identity; revoking the same request removes it.
        runtime
            .schedule_update(root, scene(&controller, page, true))
            .unwrap();
        runtime.run_frame(bounds).unwrap();
        assert_eq!(
            runtime.tree().element_with_key(&key),
            Some(first),
            "after rebuild, page={page}"
        );
        assert!(!visible(&runtime));
        runtime
            .schedule_update(root, scene(&controller, page, false))
            .unwrap();
        runtime.run_frame(bounds).unwrap();
        assert!(!runtime.tree().element_exists(first));
        assert!(controller.jump_to(0.));
        runtime.run_frame(bounds).unwrap();
        assert_ne!(runtime.tree().element_with_key(&key).unwrap(), first);
        assert!(visible(&runtime));
    }
}
