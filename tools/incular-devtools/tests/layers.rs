use incular_devtools_protocol::LayerNodeWire;
use incular_devtools_ui::diagnostics::{
    CacheHealth, TreeView, cache_health, is_identity_transform,
};

const IDENTITY: &str = "translate (0.0, 0.0) scale (1.00, 1.00)";

#[test]
fn identity_transforms_are_recognised_only_when_nothing_moves() {
    assert!(is_identity_transform("Transform", IDENTITY));
    assert!(is_identity_transform(
        "Transform",
        "translate (-0.0, 0.0) scale (1.00, 1.00)"
    ));
    assert!(!is_identity_transform(
        "Transform",
        "translate (12.5, 0.0) scale (1.00, 1.00)"
    ));
    assert!(!is_identity_transform(
        "Transform",
        "translate (0.0, 0.0) scale (2.00, 1.00)"
    ));
    assert!(!is_identity_transform("Picture", IDENTITY));
    assert!(!is_identity_transform("Transform", "42 commands"));
}

fn layer(
    id: u64,
    parent: Option<u64>,
    children: &[u64],
    kind: &str,
    summary: &str,
) -> LayerNodeWire {
    LayerNodeWire {
        id,
        parent,
        children: children.to_vec(),
        kind: kind.into(),
        summary: summary.into(),
        bounds: None,
    }
}

#[test]
fn hiding_splices_children_up_without_changing_the_snapshot() {
    let mut view = TreeView::default();
    view.replace(
        vec![
            layer(1, None, &[2], "Root", ""),
            layer(2, Some(1), &[3], "Transform", IDENTITY),
            layer(3, Some(2), &[4, 5], "Transform", IDENTITY),
            layer(4, Some(3), &[], "Picture", "9 commands"),
            layer(5, Some(3), &[], "Picture", "3 commands"),
        ],
        false,
    );
    view.expand_all();
    let hide = |node: &LayerNodeWire| is_identity_transform(&node.kind, &node.summary);
    let shown = view
        .visible_rows_hiding(&hide)
        .into_iter()
        .map(|row| (view.nodes()[row.index].id, row.depth))
        .collect::<Vec<_>>();
    assert_eq!(shown, vec![(1, 0), (4, 1), (5, 1)]);
    assert_eq!(view.visible_rows().len(), 5);
    assert_eq!(view.len(), 5);
}

#[test]
fn cold_text_cache_is_not_judged() {
    assert_eq!(cache_health(0, 54), CacheHealth::Cold);
    assert_eq!(cache_health(0, 0), CacheHealth::Cold);
    assert_eq!(cache_health(0, 400), CacheHealth::Poor(0.));
    assert!(matches!(cache_health(150, 100), CacheHealth::Fair(_)));
    assert!(matches!(cache_health(950, 50), CacheHealth::Good(_)));
}
