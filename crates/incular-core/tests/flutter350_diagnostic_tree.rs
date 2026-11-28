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

#[test]
fn compact_default_has_essential_but_not_detailed_fields() {
    let actual = tree().to_json_iterative(Options::default(), &());
    assert_eq!(
        Value::Object(actual),
        json!({
            "description": "TestTree#fixture",
            "shouldIndent": true,
            "widgetRuntimeType": "TestTree#fixture",
        })
    );
}

#[test]
fn compact_positive_depth_retains_descendants_but_omits_properties_and_leaf_lists() {
    let actual = tree().to_json_iterative(
        options(1, true),
        &TestDelegate {
            no_properties_below_root: true,
            ..Default::default()
        },
    );
    let children = actual["children"].as_array().unwrap();
    assert_eq!(children.len(), 3);
    assert!(children[0].get("children").is_none());
    assert_eq!(children[1]["children"].as_array().unwrap().len(), 3);
    assert!(children[1]["children"][2].get("properties").is_none());
    assert!(actual.get("properties").is_none());
    assert!(children[2].get("children").is_none());
}

#[test]
fn compact_calls_delegates_breadth_first_and_only_marks_the_truncated_parent() {
    use std::cell::RefCell;

    struct CompactOrder(RefCell<Vec<String>>);
    impl Delegate for CompactOrder {
        fn additional_properties(&self, _node: &Node) -> Map<String, Value> {
            panic!("the compact hook, not the full-detail hook, is expected");
        }
        fn additional_compact_properties(&self, node: &Node) -> Map<String, Value> {
            let ty = node.node_type().to_owned();
            self.0.borrow_mut().push(ty.clone());
            Map::from_iter([("nodeType".into(), json!(ty))])
        }
        fn truncate_nodes<'a>(&'a self, nodes: &mut Vec<&'a Node>, owner: &'a Node) {
            if owner.node_type() == "B" {
                nodes.truncate(1);
            }
        }
    }

    let root = Node::new("[root]", "Root").with_children(vec![
        Node::new("A-debugInfo", "A").with_children(vec![Node::new("X-1", "X")]),
        Node::new("B-debugInfo", "B")
            .with_children(vec![Node::new("Y-1", "Y"), Node::new("Z-1", "Z")]),
    ]);
    let delegate = CompactOrder(RefCell::new(Vec::new()));
    let actual = root.to_json_iterative(options(1, true), &delegate);
    assert_eq!(
        *delegate.0.borrow(),
        ["Root", "A", "B", "X", "Y", "DiagnosticsProperty<void>"]
    );
    assert_eq!(actual["widgetRuntimeType"], "RootWidget");
    assert_eq!(actual["children"][0]["widgetRuntimeType"], "A");
    assert_eq!(actual["children"][1]["truncated"], true);
    assert!(actual.get("truncated").is_none());
    assert_eq!(actual["children"][1]["children"][1]["description"], "...");
    assert!(
        actual["children"][1]["children"][1]
            .get("truncated")
            .is_none()
    );
}

#[test]
fn shallow_tree_summary_keeps_visible_properties_and_never_includes_children() {
    let root = Node::new("TestTree#00000", "DiagnosticableTreeNode")
        .with_properties(vec![
            string("stringProperty1", "value1"),
            Node::property(&Property::double("doubleProperty1", Some(42.5))),
            Node::property(&Property::double("roundedProperty", Some(1.0 / 3.0))),
            Node::property(
                &Property::string("DO_NOT_SHOW", Some("DO_NOT_SHOW".into()))
                    .level(DiagnosticLevel::Hidden)
                    .quoted(false),
            ),
            Node::property(
                &Property::string("DEBUG_ONLY", Some("DEBUG_ONLY".into()))
                    .level(DiagnosticLevel::Debug)
                    .quoted(false),
            ),
        ])
        .with_children(vec![Node::new("node A", "DiagnosticableTreeNode")]);
    assert_eq!(
        root.to_string_shallow(DiagnosticLevel::Info),
        "TestTree#00000(stringProperty1: value1, doubleProperty1: 42.5, roundedProperty: 0.3)",
    );
    assert_eq!(
        root.to_string_shallow(DiagnosticLevel::Debug),
        "TestTree#00000(stringProperty1: value1, doubleProperty1: 42.5, roundedProperty: 0.3, DEBUG_ONLY: DEBUG_ONLY)",
    );
}

#[derive(Debug, Clone, PartialEq)]
enum Object {
    Rectangle(incular_core::Rect),
    Color(incular_core::Color),
}

impl incular_core::DiagnosticValue for Object {
    const TYPE_NAME: &'static str = "Object";

    fn diagnostic_text(&self) -> String {
        match self {
            Self::Rectangle(rect) => rect.diagnostic_text(),
            Self::Color(color) => color.diagnostic_text(),
        }
    }

    fn to_diagnostic_json(&self) -> Value {
        Value::Null
    }
}

#[test]
fn iterable_properties_keep_typed_members_distinct_from_display_and_wire_text() {
    let ints = Property::iterable("ints", Some(vec![1_i64, 2, 3]));
    assert_eq!(ints.value(), Some(&vec![1, 2, 3]));
    assert!(!ints.is_filtered(DiagnosticLevel::Info));
    assert_eq!(ints.to_string(), "ints: 1, 2, 3");

    let doubles = Property::iterable("doubles", Some(vec![1.0_f64, 2.0, 3.0]));
    assert_eq!(doubles.value(), Some(&vec![1.0, 2.0, 3.0]));
    assert!(!doubles.is_filtered(DiagnosticLevel::Info));
    assert_eq!(doubles.to_string(), "doubles: 1.0, 2.0, 3.0");

    let rounded = Property::iterable("fraction", Some(vec![1.0 / 3.0_f64]));
    assert_eq!(rounded.to_string(), "fraction: 0.3");
    assert_eq!(rounded.to_json()["values"], json!(["0.3333333333333333"]));

    let empty = Property::<Vec<Object>>::iterable("name", Some(vec![]));
    assert_eq!(empty.value(), Some(&vec![]));
    assert!(!empty.is_filtered(DiagnosticLevel::Info));
    assert_eq!(empty.to_string(), "name: []");
    assert_eq!(
        empty.to_json(),
        json!({
            "name": "name", "description": "[]", "type": "IterableProperty<Object>",
            "propertyType": "Iterable<Object>", "style": "singleLine", "defaultLevel": "info",
            "missingIfNull": false, "ifEmpty": "[]", "values": [],
        })
    );

    let missing = Property::<Vec<Object>>::iterable("list", None);
    assert!(missing.value().is_none());
    assert!(!missing.is_filtered(DiagnosticLevel::Info));
    assert_eq!(missing.to_string(), "list: null");
    let mut expected = empty.to_json().as_object().unwrap().clone();
    expected.remove("values");
    expected.insert("name".into(), json!("list"));
    expected.insert("description".into(), json!("null"));
    expected.insert("value".into(), Value::Null);
    assert_eq!(missing.to_json(), Value::Object(expected.clone()));

    let hidden = missing.default_value(None);
    assert!(hidden.value().is_none());
    assert_eq!(hidden.effective_level(), DiagnosticLevel::Fine);
    assert!(hidden.is_filtered(DiagnosticLevel::Info));
    assert_eq!(hidden.to_string(), "list: null");
    expected.insert("defaultValue".into(), json!("null"));
    expected.insert("level".into(), json!("fine"));
    assert_eq!(hidden.to_json(), Value::Object(expected));
}

#[test]
fn iterable_deep_format_respects_its_own_style_and_compact_parent_style() {
    use incular_core::{Color, DiagnosticsTreeStyle as Style, Offset, Rect, Size};

    let rect = Object::Rectangle(Rect::from_origin_size(Offset::ZERO, Size::new(20.0, 20.0)));
    let white = Object::Color(Color::WHITE);
    let rect_text = "Rect.fromLTRB(0.0, 0.0, 20.0, 20.0)";
    let white_text = "Color(alpha: 1.0000, red: 1.0000, green: 1.0000, blue: 1.0000, colorSpace: ColorSpace.sRGB)";
    let minimum = DiagnosticLevel::Debug;
    let tree = || Node::new("TestTree#00000", "DiagnosticableTreeNode");

    let objects = vec![rect, white.clone()];
    let property = Property::iterable("objects", Some(objects.clone()));
    assert_eq!(property.value(), Some(&objects));
    assert!(!property.is_filtered(DiagnosticLevel::Info));
    assert_eq!(
        property.to_string(),
        format!("objects: {rect_text}, {white_text}")
    );
    assert_eq!(property.to_json()["values"], json!([rect_text, white_text]));

    let multi = property.clone().style(Style::Whitespace);
    assert_eq!(multi.value(), Some(&objects));
    assert!(!multi.is_filtered(DiagnosticLevel::Info));
    assert_eq!(
        multi.to_string(),
        format!("objects:\n{rect_text}\n{white_text}")
    );
    assert_eq!(multi.to_json()["values"], json!([rect_text, white_text]));
    assert_eq!(multi.to_json()["style"], "whitespace");
    assert_eq!(
        Node::property(&multi).to_string_deep_with_widths(minimum, 100, 65),
        format!("objects:\n  {rect_text}\n  {white_text}\n")
    );
    assert_eq!(
        tree()
            .with_properties(vec![Node::property(&multi)])
            .to_string_deep_with_widths(minimum, 100, 65),
        format!("TestTree#00000\n   objects:\n     {rect_text}\n     {white_text}\n")
    );
    for property in [property, multi] {
        assert_eq!(
            tree()
                .style(Style::SingleLine)
                .with_properties(vec![
                    Node::property(&property),
                    Node::property(&Property::integer("foo", Some(42))),
                ])
                .to_string_deep(minimum),
            format!("TestTree#00000(objects: [{rect_text}, {white_text}], foo: 42)")
        );
    }

    let solo = Property::iterable("object", Some(vec![white.clone()])).style(Style::Whitespace);
    assert_eq!(solo.value(), Some(&vec![white]));
    assert!(!solo.is_filtered(DiagnosticLevel::Info));
    assert_eq!(solo.to_string(), format!("object: {white_text}"));
    assert_eq!(solo.to_json()["values"], json!([white_text]));
    assert_eq!(
        Node::property(&solo).to_string_deep_with_widths(minimum, 100, 65),
        format!("object: {white_text}\n")
    );
    assert_eq!(
        tree()
            .with_properties(vec![Node::property(&solo)])
            .to_string_deep_with_widths(minimum, 120, 65),
        format!("TestTree#00000\n   object: {white_text}\n")
    );
}

// Adapted from Flutter's diagnostics_test.dart (see fixtures/flutter350_LICENSE).
#[test]
fn error_diagnostic_styles_reproduce_exact_reference_wraps_and_fixed_lines() {
    use incular_core::DiagnosticsTreeStyle as Style;

    let long_message =
        ["This is a very long message that must wrap as it cannot fit on one line."; 3].join(" ");
    let fixed_message = ["Message that is not allowed to wrap even though it is very long."; 3]
        .into_iter()
        .chain(["Message that is not allowed to wrap."])
        .collect::<Vec<_>>()
        .join(" ");
    let long_label = ["This property has a very long property name that will be allowed to wrap unlike most property names"; 2].join(". ");
    let short_url = "https://goo.gl/";
    let long_url = "http://someverylongurl.com/that-might-be-tempting-to-wrap-even-though-it-is-a-url-so-should-not-wrap.html";
    let matrix = concat!(
        "[1.0, 0.0, 0.0, 0.0]\n",
        "[1.0, 1.0, 0.0, 0.0]\n",
        "[1.0, 0.0, 1.0, 0.0]\n",
        "[1.0, 0.0, 0.0, 1.0]\n",
    );
    for (root, style, expected) in [
        (
            Style::Error,
            Style::SingleLine,
            include_str!("fixtures/diagnostics/error-inline.txt"),
        ),
        (
            Style::Sparse,
            Style::Flat,
            include_str!("fixtures/diagnostics/sparse-flat.txt"),
        ),
        (
            Style::Error,
            Style::ErrorProperty,
            include_str!("fixtures/diagnostics/error-property.txt"),
        ),
    ] {
        let message = |text: &str| Node::property(&Property::message(text).style(style));
        let marker = || message("--- example property at max length --");
        let field = |name, text: &str| {
            Node::property(&Property::new(name, Some(text.to_owned())).style(style))
        };
        let tree = Node::new("TestTree#00000", "DiagnosticableTreeNode")
            .style(root)
            .with_properties(vec![
                marker(),
                message(&long_message),
                marker(),
                Node::property(&Property::new("", Some(fixed_message.clone())).allow_wrap(false)),
                marker(),
                message(&format!("{long_label}:\n  {long_url}")),
                message(&format!("{long_label}:\n  {short_url}")),
                message(&format!("Click on the following url:\n  {long_url}")),
                message(&format!("Click on the following url\n  {short_url}")),
                marker(),
                field("multi-line value", matrix),
                marker(),
                field(&long_label, &long_message),
                marker(),
                field(&long_label, matrix),
                marker(),
                Node::property(
                    &Property::message_property(
                        "diagnosis",
                        "insufficient data to draw conclusion (less than five repaints)",
                    )
                    .style(style),
                ),
            ]);
        assert_eq!(
            tree.to_string_deep_with_widths(DiagnosticLevel::Debug, 40, 40),
            expected,
            "{style:?}"
        );
    }
}

#[test]
fn wrap_budget_counts_utf16_columns_and_preserves_indivisible_text() {
    use incular_core::DiagnosticsTreeStyle as Style;
    let tree = Node::new("root", "Tree")
        .style(Style::Flat)
        .with_properties(vec![
            Node::property(&Property::message("👋👋 aa bb").style(Style::Flat)),
            Node::property(&Property::message("  unbreakable-url\nplain text").style(Style::Flat)),
            Node::property(
                &Property::message("many words stay together")
                    .style(Style::Flat)
                    .allow_wrap(false),
            ),
        ]);
    assert_eq!(
        tree.to_string_deep_with_widths(DiagnosticLevel::Info, 7, 7),
        "root:\n👋👋 aa\nbb\n  unbreakable-url\nplain\ntext\nmany words stay together\n"
    );
}
