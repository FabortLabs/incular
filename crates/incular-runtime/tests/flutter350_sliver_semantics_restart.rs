//! The host accessibility lifecycle suspends and restarts sliver semantics.

use std::time::Duration;

use incular_accessibility::{MobileAccessibilityProjection, MobileAccessibilityUpdate};
use incular_config::{Constraints, TextDirection};
use incular_core::{Size, time::Instant};
use incular_runtime::Application;
use incular_semantics::SemanticRole;
use incular_widgets::{
    CustomScrollView, Directionality, Semantics, SizedBox, Sliver, SliverSemantics,
    SliverToBoxAdapter,
};

#[test]
fn semantics_restart_schedules_one_frame_and_recreates_the_sliver_label() {
    let mut app = Application::new(|_| {
        let slivers: [Box<dyn Sliver>; 1] = [Box::new(SliverSemantics::new(
            || {
                Semantics::default()
                    .label("test1")
                    .text_direction(TextDirection::Ltr)
            },
            SliverToBoxAdapter::new(SizedBox::new().height(10.)),
        ))];
        Directionality::new(TextDirection::Ltr, CustomScrollView::new(slivers)).into()
    })
    .unwrap();
    let window = app.primary_window();
    let constraints = Constraints::tight(Size::new(800., 600.));
    let now = Instant::now();
    let mut projection = MobileAccessibilityProjection::new();

    let label_in = |update: MobileAccessibilityUpdate| {
        let label = update
            .nodes
            .iter()
            .find(|node| node.semantic.label.as_deref() == Some("test1"))
            .expect("one sliver label in the projected tree");
        assert_eq!(
            label.semantic.state.text_direction,
            Some(TextDirection::Ltr)
        );
        let scroll = update
            .nodes
            .iter()
            .find(|node| node.semantic.role == SemanticRole::ScrollView)
            .expect("scrolling semantic ancestor");
        assert!(scroll.semantic.state.has_implicit_scrolling);
        assert_eq!(scroll.children, [label.native_id]);
        label.native_id
    };
    app.run_window_frame_at(window, constraints, now)
        .unwrap()
        .unwrap();
    let old = label_in(
        app.sync_semantics_projection(window, &mut projection)
            .unwrap(),
    );
    app.set_semantics_enabled(window, false);
    app.run_window_frame_at(window, constraints, now + Duration::from_millis(16))
        .unwrap()
        .unwrap();
    assert!(!app.frame_requested(window));
    let cleared = app
        .sync_semantics_projection(window, &mut projection)
        .unwrap();
    assert!(cleared.nodes.is_empty());
    assert!(cleared.removed.contains(&old));

    app.set_semantics_enabled(window, true);
    assert!(app.frame_requested(window));
    app.run_window_frame_at(window, constraints, now + Duration::from_millis(32))
        .unwrap()
        .unwrap();
    let new = label_in(
        app.sync_semantics_projection(window, &mut projection)
            .unwrap(),
    );
    assert_ne!(old, new, "detached accessibility nodes are not reused");
    assert!(!app.frame_requested(window));
}
