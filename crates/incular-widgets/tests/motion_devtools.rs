#![cfg(feature = "devtools")]

//! DevTools shows a motion site's cost tier, layers and values.

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_devtools_protocol::{DebugValue, DevWindowId};
use incular_widgets::internal::{ElementId, WidgetTree};
use incular_widgets::motion::{MotionExt, Transition};
use incular_widgets::{Column, Widget};

fn motion_properties(tree: &WidgetTree, root: ElementId) -> Vec<Vec<(String, String)>> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        let dev = tree.devtools_id_for_element(id).expect("devtools id");
        let details = tree
            .devtools_node_details(id, dev, DevWindowId::new(1, 0))
            .expect("details");
        let motion: Vec<(String, String)> = details
            .properties
            .iter()
            .filter(|property| property.name.starts_with("motion."))
            .filter_map(|property| match &property.value {
                DebugValue::Str(value) => Some((property.name.clone(), value.clone())),
                _ => None,
            })
            .collect();
        if !motion.is_empty() {
            out.push(motion);
        }
        stack.extend(tree.children(id).unwrap_or(&[]).iter().rev().copied());
    }
    out
}

fn get<'a>(props: &'a [(String, String)], name: &str) -> &'a str {
    props
        .iter()
        .find(|(key, _)| key == name)
        .map_or("", |(_, value)| value.as_str())
}

#[test]
fn sites_report_their_tier_and_values() {
    let scaled = Widget::box_(Size::new(50., 50.), Color::WHITE)
        .motion(|m| m.rest(|p| p.scale(1.5)).transition(Transition::instant()));
    let sized = Widget::box_(Size::new(50., 50.), Color::WHITE)
        .with_key("sized")
        .motion(|m| m.layout());
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(Column::new([scaled, sized]).into())
        .expect("mount");
    tree.layout(Constraints::loose(Size::new(200., 200.)))
        .expect("layout");
    let sites = motion_properties(&tree, root);
    assert_eq!(sites.len(), 2, "{sites:?}");
    assert_eq!(get(&sites[0], "motion.tier"), "composite");
    assert!(get(&sites[0], "motion.layers").contains("transform"));
    assert!(
        get(&sites[0], "motion.values").contains("Scale=1.500"),
        "{sites:?}"
    );
    assert!(
        get(&sites[1], "motion.tier").starts_with("layout"),
        "{sites:?}"
    );
}
