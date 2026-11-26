use incular_config::{Constraints, TextDirection};
use incular_core::{Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::{Center, RawTooltip, RawTooltipController, Text, WidgetsApp};
use std::time::Duration;

fn scene(annotated: bool) -> (Runtime, RawTooltipController, Instant) {
    let controller = RawTooltipController::new();
    let tooltip = RawTooltip::with_animated_builder(Text::new("Bar"), |_| Text::new("Foo").into())
        .controller(controller.clone());
    let tooltip = if annotated {
        tooltip.semantics_tooltip("Foo")
    } else {
        tooltip.clear_semantics_tooltip()
    };
    let mut runtime = Runtime::new(WidgetsApp::new(Center::new(tooltip)).into()).unwrap();
    let now = Instant::now();
    frame(&mut runtime, now);
    (runtime, controller, now)
}

fn frame(runtime: &mut Runtime, at: Instant) {
    runtime
        .run_frame_at(Constraints::tight(Size::new(800., 600.)), at)
        .unwrap();
}

#[test]
fn the_closed_tooltip_annotation_is_on_the_anchor_label_node_not_its_own_node() {
    let (runtime, _, _) = scene(true);
    let semantics = runtime.tree().semantics();
    let bar: Vec<_> = semantics
        .iter()
        .filter_map(|(_, node)| (node.label.as_deref() == Some("Bar")).then_some(node))
        .collect();
    assert_eq!(bar.len(), 1, "{}", runtime.tree().semantics_debug_dump());
    assert_eq!(bar[0].state.tooltip.as_deref(), Some("Foo"));
    assert_eq!(bar[0].state.text_direction, Some(TextDirection::Ltr));
    assert_eq!(
        semantics
            .iter()
            .filter(|(_, node)| node.state.tooltip.as_deref() == Some("Foo"))
            .count(),
        1
    );
    assert!(
        !semantics
            .iter()
            .any(|(_, node)| node.label.as_deref() == Some("Foo"))
    );
}

#[test]
fn excluding_the_tooltip_does_not_erase_label_direction_or_the_visual_builder() {
    let (mut runtime, controller, now) = scene(false);
    let semantics = runtime.tree().semantics();
    let bar = semantics
        .iter()
        .find_map(|(_, node)| (node.label.as_deref() == Some("Bar")).then_some(node))
        .unwrap();
    assert_eq!(bar.state.text_direction, Some(TextDirection::Ltr));
    assert!(
        semantics
            .iter()
            .all(|(_, node)| node.state.tooltip.is_none())
    );
    assert!(
        !semantics
            .iter()
            .any(|(_, node)| node.label.as_deref() == Some("Foo"))
    );

    assert!(controller.ensure_visible_at(now + Duration::from_millis(10)));
    frame(&mut runtime, now + Duration::from_millis(10));
    frame(&mut runtime, now + Duration::from_millis(162));
    assert!(
        runtime
            .tree()
            .semantics()
            .iter()
            .any(|(_, node)| node.label.as_deref() == Some("Foo"))
    );
}
