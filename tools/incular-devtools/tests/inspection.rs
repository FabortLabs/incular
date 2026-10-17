use incular_devtools_protocol::{LayerNodeWire, RuntimeDiagnosticsWire, SemanticsNodeWire};
use incular_devtools_ui::diagnostics::*;
use std::time::{Duration, Instant};

fn node(
    id: u64,
    parent: Option<u64>,
    children: &[u64],
    role: &str,
    label: &str,
) -> SemanticsNodeWire {
    SemanticsNodeWire {
        id,
        parent,
        children: children.to_vec(),
        role: role.into(),
        label: (!label.is_empty()).then(|| label.into()),
        ..Default::default()
    }
}

/// 1 -> (2 -> (4, 5), 3)
fn sample() -> TreeView<SemanticsNodeWire> {
    let mut view = TreeView::default();
    view.replace(
        vec![
            node(1, None, &[2, 3], "Window", "Gallery"),
            node(2, Some(1), &[4, 5], "Group", ""),
            node(3, Some(1), &[], "Button", "Save"),
            node(4, Some(2), &[], "Text", "Hello"),
            node(5, Some(2), &[], "Button", "Cancel"),
        ],
        false,
    );
    view
}

fn ids(view: &TreeView<SemanticsNodeWire>) -> Vec<(u64, u16)> {
    view.visible_rows()
        .iter()
        .map(|row| (view.nodes()[row.index].id, row.depth))
        .collect()
}

#[test]
fn rates_ignore_resets_and_zero_elapsed() {
    let second = Duration::from_secs(1);
    assert_eq!(per_second(10, 30, second * 2), Some(10.));
    assert_eq!(per_second(30, 10, second), None);
    assert_eq!(per_second(1, 2, Duration::ZERO), None);
    assert_eq!(hit_rate(0, 0), None);
    assert_eq!(hit_rate(3, 1), Some(75.));
}

#[test]
fn history_computes_rate_between_the_last_two_polls() {
    let start = Instant::now();
    let mut history = DiagnosticsHistory::default();
    let mut wire = RuntimeDiagnosticsWire::default();
    wire.widgets.rebuilds = 100;
    history.push(wire.clone(), start);
    assert_eq!(history.rate(|w| w.widgets.rebuilds), None);
    wire.widgets.rebuilds = 160;
    history.push(wire, start + Duration::from_secs(2));
    assert_eq!(history.rate(|w| w.widgets.rebuilds), Some(30.));
    history.reset_deltas();
    assert_eq!(history.rate(|w| w.widgets.rebuilds), None);
    assert_eq!(history.latest().unwrap().widgets.rebuilds, 160);
}

#[test]
fn formatting_is_compact() {
    assert_eq!(format_count(999), "999");
    assert_eq!(format_count(1234), "1 234");
    assert_eq!(format_count(12_345), "12.3k");
    assert_eq!(format_rate(None), "–");
    assert_eq!(format_rate(Some(0.0)), "0/s");
    assert_eq!(format_rate(Some(12.34)), "+12.3/s");
    assert_eq!(format_age(2_000), "just now");
    assert_eq!(format_age(125_000), "2m ago");
}

#[test]
fn first_load_opens_the_top_levels_and_collapse_hides_children() {
    let mut view = sample();
    assert_eq!(ids(&view), [(1, 0), (2, 1), (4, 2), (5, 2), (3, 1)]);
    view.collapse_all();
    assert_eq!(ids(&view), [(1, 0)]);
    view.toggle(1);
    assert_eq!(ids(&view), [(1, 0), (2, 1), (3, 1)]);
}

#[test]
fn search_keeps_ancestors_and_marks_matches() {
    let mut view = sample();
    view.collapse_all();
    view.set_query("cancel");
    let rows = view.visible_rows();
    let shown: Vec<_> = rows.iter().map(|row| view.nodes()[row.index].id).collect();
    assert_eq!(shown, [1, 2, 5]);
    assert!(
        rows.iter()
            .find(|row| view.nodes()[row.index].id == 5)
            .unwrap()
            .matched
    );
    assert!(!rows[0].matched);
    view.set_query("role-that-does-not-exist");
    assert!(view.visible_rows().is_empty());
}

#[test]
fn replace_keeps_selection_only_for_surviving_nodes() {
    let mut view = sample();
    assert!(view.select(5));
    assert!(!view.select(99));
    view.replace(vec![node(1, None, &[], "Window", "")], true);
    assert_eq!(view.selected, None);
    assert!(view.truncated);
    assert_eq!(view.len(), 1);
}

#[test]
fn malformed_cycles_and_orphans_do_not_hang() {
    let mut view = TreeView::default();
    view.replace(
        vec![
            node(1, Some(2), &[2], "A", ""),
            node(2, Some(1), &[1], "B", ""),
            node(7, Some(404), &[], "Orphan", ""),
        ],
        false,
    );
    // Only the orphan has a missing parent, so it is the sole root.
    assert_eq!(ids(&view), [(7, 0)]);
    assert!(view.ancestors(1).len() <= 2);
}

#[test]
fn reveal_expands_ancestors() {
    let mut view = sample();
    view.collapse_all();
    view.reveal(4);
    assert!(ids(&view).contains(&(4, 2)));
}

#[test]
fn layer_nodes_search_by_kind_and_summary() {
    let mut view = TreeView::default();
    view.replace(
        vec![
            LayerNodeWire {
                id: 1,
                kind: "Transform".into(),
                children: vec![2],
                ..Default::default()
            },
            LayerNodeWire {
                id: 2,
                parent: Some(1),
                kind: "Picture".into(),
                summary: "3 commands".into(),
                bounds: Some([1., 2., 3., 4.]),
                ..Default::default()
            },
        ],
        false,
    );
    view.set_query("3 commands");
    assert_eq!(view.visible_rows().len(), 2);
}

#[test]
fn highlight_bounds_rejects_empty_or_non_finite_rects() {
    assert_eq!(highlight_bounds([1., 2., 3., 4.]), Some([1., 2., 3., 4.]));
    assert_eq!(highlight_bounds([0., 0., 0., 4.]), None);
    assert_eq!(highlight_bounds([f32::NAN, 0., 1., 1.]), None);
}
