use std::cell::Cell;

use incular_core::{
    DiagnosticLevel, DiagnosticsNode as Node, DiagnosticsProperty as Property,
    DiagnosticsSerializationDelegate as Delegate, DiagnosticsSerializationOptions as Options,
};
use serde_json::{Map, Value, json};

fn string(name: &str, value: &str) -> Node {
    Node::property(&Property::string(name, Some(value.into())).quoted(false))
}

fn tree() -> Node {
    let tree_node = || Node::new("TestTree#fixture", "DiagnosticableTreeNode");
    let child = |name: &str| tree_node().named(format!("child node {name}"));
    tree_node()
        .with_properties(vec![
            string("stringProperty1", "value1"),
            Node::property(&Property::double("doubleProperty1", Some(42.5))),
            Node::property(&Property::double("roundedProperty", Some(1.0 / 3.0))),
            Node::property(
                &Property::string("DO_NOT_SHOW", Some("DO_NOT_SHOW".into()))
                    .level(DiagnosticLevel::Hidden)
                    .quoted(false),
            ),
            Node::property(&Property::<String>::new("DO_NOT_SHOW_NULL", None).default_value(None)),
            Node::property(&Property::<String>::new("nullProperty", None)),
            Node::property(
                &Property::string("node_type", Some("<root node>".into()))
                    .quoted(false)
                    .show_name(false),
            ),
        ])
        .with_children(vec![
            child("A"),
            child("B")
                .with_properties(vec![string("p1", "v1"), string("p2", "v2")])
                .with_children(vec![
                    child("B1"),
                    child("B2").with_properties(vec![string("property1", "value1")]),
                    child("B3").with_properties(vec![
                        Node::property(
                            &Property::string("node_type", Some("<leaf node>".into()))
                                .quoted(false)
                                .show_name(false),
                        ),
                        Node::property(&Property::integer("foo", Some(42))),
                    ]),
                ]),
            child("C").with_properties(vec![string("foo", "multi\nline\nvalue!")]),
        ])
}

fn options(subtree_depth: usize, include_properties: bool) -> Options {
    Options {
        subtree_depth,
        include_properties,
    }
}

#[test]
fn default_emits_only_detailed_node_keys() {
    let actual = tree().to_json(Options::default(), &());
    assert_eq!(
        Value::Object(actual),
        json!({
            "description": "TestTree#fixture",
            "type": "DiagnosticableTreeNode",
            "hasChildren": true,
            "allowWrap": false,
        })
    );
}

#[test]
fn subtree_depth_one_includes_children_but_not_grandchildren() {
    let actual = tree().to_json(options(1, false), &());
    assert!(!actual.contains_key("properties"));
    let children = actual["children"].as_array().unwrap();
    assert_eq!(children.len(), 3);
    assert!(children.iter().all(|node| node.get("children").is_none()));
    assert!(children[0].get("hasChildren").is_none());
    assert_eq!(children[1]["hasChildren"], true);
}

#[test]
fn subtree_depth_five_includes_explicit_empty_children() {
    let actual = tree().to_json(options(5, false), &());
    assert!(!actual.contains_key("properties"));
    assert_eq!(actual["children"][0]["children"], json!([]));
    let grandchildren = actual["children"][1]["children"].as_array().unwrap();
    assert_eq!(grandchildren.len(), 3);
    assert_eq!(grandchildren[2]["name"], "child node B3");
    assert_eq!(actual["children"][2]["children"], json!([]));
}

#[test]
fn include_properties_keeps_even_hidden_and_null_values() {
    let actual = tree().to_json(options(0, true), &());
    assert!(!actual.contains_key("children"));
    let properties = actual["properties"].as_array().unwrap();
    assert_eq!(properties.len(), 7);
    assert_eq!(properties[3]["level"], "hidden");
    assert_eq!(properties[4]["defaultValue"], "null");
    assert_eq!(properties[5].get("value"), Some(&Value::Null));
    assert_eq!(properties[6]["showName"], false);
}

#[test]
fn include_properties_with_depth_one_preserves_per_child_counts() {
    let actual = tree().to_json(options(1, true), &());
    assert_eq!(actual["properties"].as_array().unwrap().len(), 7);
    let children = actual["children"].as_array().unwrap();
    assert_eq!(children.len(), 3);
    assert_eq!(
        children
            .iter()
            .map(|node| node["properties"].as_array().unwrap().len())
            .collect::<Vec<_>>(),
        [0, 2, 1]
    );
    assert_eq!(children[2]["properties"][0]["value"], "multi\nline\nvalue!");
}

#[derive(Default)]
struct TestDelegate {
    add_fields: bool,
    strings_only: bool,
    replacement: Option<Node>,
    replaced: Cell<bool>,
    empty_children_only: bool,
    promote_children: bool,
    limit: Option<usize>,
    no_properties_below_root: bool,
}

impl Delegate for TestDelegate {
    fn additional_properties(&self, _node: &Node) -> Map<String, Value> {
        if self.add_fields {
            Map::from_iter([("foo".into(), json!(true))])
        } else {
            Map::new()
        }
    }

    fn filter_children<'a>(&'a self, owner: &'a Node) -> Vec<&'a Node> {
        if self.promote_children {
            owner.children().iter().flat_map(Node::children).collect()
        } else {
            owner
                .children()
                .iter()
                .filter(|node| !self.empty_children_only || node.properties().is_empty())
                .collect()
        }
    }

    fn filter_properties<'a>(&'a self, owner: &'a Node) -> Vec<&'a Node> {
        if let Some(replacement) = &self.replacement
            && !self.replaced.replace(true)
        {
            return vec![replacement];
        }
        owner
            .properties()
            .iter()
            .filter(|node| !self.strings_only || node.node_type() == "StringProperty")
            .collect()
    }

    fn truncate_nodes<'a>(&'a self, nodes: &mut Vec<&'a Node>, _owner: &'a Node) {
        if let Some(limit) = self.limit {
            nodes.truncate(limit);
        }
    }

    fn options_for_node(&self, node: &Node, mut options: Options) -> Options {
        if self.no_properties_below_root {
            options.include_properties = false;
            return options; // The custom policy need not decrement depth.
        }
        ().options_for_node(node, options)
    }
}

#[test]
fn extra_fields_apply_to_root_properties_and_children() {
    let actual = tree().to_json(
        options(1, true),
        &TestDelegate {
            add_fields: true,
            ..Default::default()
        },
    );
    assert_eq!(actual["foo"], true);
    for (key, len) in [("properties", 7), ("children", 3)] {
        let nodes = actual[key].as_array().unwrap();
        assert_eq!(nodes.len(), len);
        assert!(nodes.iter().all(|node| node["foo"] == true));
    }
}

#[test]
fn property_filter_can_select_a_typed_sublist() {
    let actual = tree().to_json(
        options(0, true),
        &TestDelegate {
            strings_only: true,
            ..Default::default()
        },
    );
    let nodes = actual["properties"].as_array().unwrap();
    assert_eq!(nodes.len(), 3);
    assert!(nodes.iter().all(|node| node["type"] == "StringProperty"));
}

#[test]
fn property_filter_can_replace_nodes() {
    let actual = tree().to_json(
        options(0, true),
        &TestDelegate {
            replacement: Some(string("foo", "bar")),
            ..Default::default()
        },
    );
    let nodes = actual["properties"].as_array().unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["name"], "foo");
    assert_eq!(nodes[0]["value"], "bar");
}

#[test]
fn child_filter_can_select_a_sublist() {
    let actual = tree().to_json(
        options(1, false),
        &TestDelegate {
            empty_children_only: true,
            ..Default::default()
        },
    );
    let nodes = actual["children"].as_array().unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["name"], "child node A");
}

#[test]
fn child_filter_can_promote_descendants_in_order() {
    let actual = tree().to_json(
        options(1, false),
        &TestDelegate {
            promote_children: true,
            ..Default::default()
        },
    );
    let nodes = actual["children"].as_array().unwrap();
    assert_eq!(nodes.len(), 3);
    assert_eq!(nodes[0]["name"], "child node B1");
}

#[test]
fn truncation_marks_both_lists_without_marking_real_nodes() {
    let actual = tree().to_json(
        options(5, true),
        &TestDelegate {
            limit: Some(2),
            ..Default::default()
        },
    );
    for key in ["children", "properties"] {
        let nodes = actual[key].as_array().unwrap();
        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[2]["description"], "...");
        assert_eq!(nodes[2]["truncated"], true);
        assert!(
            nodes[..2]
                .iter()
                .all(|node| node.get("truncated").is_none())
        );
    }
}

#[test]
fn options_can_differ_below_root_for_both_lists() {
    let actual = tree().to_json(
        options(5, true),
        &TestDelegate {
            no_properties_below_root: true,
            ..Default::default()
        },
    );
    for (key, len) in [("properties", 7), ("children", 3)] {
        let nodes = actual[key].as_array().unwrap();
        assert_eq!(nodes.len(), len);
        assert!(nodes.iter().all(|node| node.get("properties").is_none()));
    }
}
