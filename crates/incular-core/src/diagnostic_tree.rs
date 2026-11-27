//! Structured diagnostic snapshots and serialization for inspection tools.

use std::collections::VecDeque;

use serde_json::{Map, Value, json};

use crate::{DiagnosticValue, DiagnosticsProperty};

/// A heterogeneous snapshot of a diagnostics node and its ordered relationships.
#[derive(Debug, Clone)]
pub struct DiagnosticsNode {
    details: Map<String, Value>,
    specialized: Map<String, Value>,
    properties: Vec<Self>,
    children: Vec<Self>,
}

impl DiagnosticsNode {
    /// Creates a tree node with Flutter's default tree formatting flags.
    #[must_use]
    pub fn new(description: impl Into<String>, node_type: impl Into<String>) -> Self {
        Self {
            details: Map::from_iter([
                ("description".into(), json!(description.into())),
                ("type".into(), json!(node_type.into())),
                ("allowWrap".into(), json!(false)),
            ]),
            specialized: Map::new(),
            properties: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Snapshots a typed scalar diagnostics property, including hidden values.
    #[must_use]
    pub fn property<T: DiagnosticValue>(property: &DiagnosticsProperty<T>) -> Self {
        let Value::Object(mut specialized) = property.to_json() else {
            unreachable!("a diagnostics property serializes to a JSON object")
        };
        let mut details = Map::from_iter([("allowNameWrap".into(), Value::Bool(true))]);
        for key in [
            "description",
            "type",
            "name",
            "showSeparator",
            "level",
            "showName",
            "emptyBodyDescription",
            "style",
            "allowTruncate",
            "linePrefix",
            "allowWrap",
            "allowNameWrap",
        ] {
            if let Some(value) = specialized.remove(key) {
                details.insert(key.into(), value);
            }
        }
        Self {
            details,
            specialized,
            properties: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Sets the label independently of the node description.
    #[must_use]
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.details.insert("name".into(), json!(name.into()));
        self
    }

    /// Adds a node-specific field to the detailed view after delegate fields.
    /// The compact view intentionally discards subclass fields.
    #[must_use]
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.specialized.insert(key.into(), value.into());
        self
    }

    /// Sets the ordered diagnostic properties.
    #[must_use]
    pub fn with_properties(mut self, properties: Vec<Self>) -> Self {
        self.properties = properties;
        self
    }

    /// Sets the ordered diagnostic children.
    #[must_use]
    pub fn with_children(mut self, children: Vec<Self>) -> Self {
        self.children = children;
        self
    }

    /// Returns the unfiltered diagnostic properties.
    #[must_use]
    pub fn properties(&self) -> &[Self] {
        &self.properties
    }

    /// Returns the unfiltered diagnostic children.
    #[must_use]
    pub fn children(&self) -> &[Self] {
        &self.children
    }

    /// Returns the diagnostic wire type.
    #[must_use]
    pub fn node_type(&self) -> &str {
        self.details["type"]
            .as_str()
            .expect("node type is a string")
    }

    /// Serializes the node with full details, honoring the delegate at each level.
    #[must_use]
    pub fn to_json(
        &self,
        options: DiagnosticsSerializationOptions,
        delegate: &impl DiagnosticsSerializationDelegate,
    ) -> Map<String, Value> {
        // Subclass data is added after the delegate and the base node fields.
        let mut details = self.details.clone();
        if !self.children.is_empty() {
            details.insert("hasChildren".into(), Value::Bool(true));
        }
        details.extend(delegate.additional_properties(self));
        if options.include_properties {
            let properties = delegate.filter_properties(self);
            details.insert(
                "properties".into(),
                self.serialize_list(properties, options, delegate),
            );
        }
        if options.subtree_depth > 0 {
            let children = delegate.filter_children(self);
            details.insert(
                "children".into(),
                self.serialize_list(children, options, delegate),
            );
        }
        details.extend(self.specialized.clone());
        details
    }

    /// Exports the compact inspector tree without recursive traversal. A
    /// positive depth enables descendants without decrementing that depth;
    /// the compact wire format never includes properties.
    #[must_use]
    pub fn to_json_iterative(
        &self,
        options: DiagnosticsSerializationOptions,
        delegate: &impl DiagnosticsSerializationDelegate,
    ) -> Map<String, Value> {
        type PendingNode = (Option<usize>, Map<String, Value>, Option<Vec<Value>>);
        let marker = Self::truncation_marker();
        let mut queue = VecDeque::from([(None, self)]);
        let mut pending: Vec<PendingNode> = Vec::new();
        // Visit breadth first so delegates observe the same order as Flutter.
        while let Some((parent, node)) = queue.pop_front() {
            let description = node.details["description"]
                .as_str()
                .expect("description is a string");
            let widget_type = if description == "[root]" {
                "RootWidget"
            } else {
                description
                    .split_once('-')
                    .map_or(description, |(prefix, _)| prefix)
            };
            let should_indent = !matches!(
                node.details.get("style").and_then(Value::as_str),
                Some("flat" | "error")
            );
            let mut entry = Map::from_iter([
                ("description".into(), json!(description)),
                ("shouldIndent".into(), json!(should_indent)),
                ("widgetRuntimeType".into(), json!(widget_type)),
            ]);
            let children = if options.subtree_depth > 0 && !node.children.is_empty() {
                let mut children = delegate.filter_children(node);
                let original_len = children.len();
                delegate.truncate_nodes(&mut children, node);
                if children.len() != original_len {
                    entry.insert("truncated".into(), Value::Bool(true));
                    children.push(&marker);
                }
                let len = children.len();
                let index = pending.len();
                queue.extend(children.into_iter().map(|child| (Some(index), child)));
                Some(Vec::with_capacity(len))
            } else {
                None
            };
            entry.extend(delegate.additional_compact_properties(node));
            pending.push((parent, entry, children));
        }

        // Move complete child entries into their parent bottom-up. No recursive
        // writes or deep Value clones are needed to assemble the response.
        while let Some((parent, mut entry, children)) = pending.pop() {
            if let Some(mut children) = children {
                children.reverse();
                entry.insert("children".into(), Value::Array(children));
            }
            if let Some(parent) = parent {
                pending[parent]
                    .2
                    .as_mut()
                    .expect("queued parents include children")
                    .push(Value::Object(entry));
            } else {
                return entry;
            }
        }
        unreachable!("the root always produces an entry")
    }

    fn truncation_marker() -> Self {
        Self::property(&DiagnosticsProperty::message("..."))
    }

    fn serialize_list<'a>(
        &'a self,
        mut nodes: Vec<&'a Self>,
        options: DiagnosticsSerializationOptions,
        delegate: &'a impl DiagnosticsSerializationDelegate,
    ) -> Value {
        let original_len = nodes.len();
        delegate.truncate_nodes(&mut nodes, self);
        let mut json: Vec<_> = nodes
            .iter()
            .map(|node| {
                Value::Object(node.to_json(delegate.options_for_node(node, options), delegate))
            })
            .collect();
        if original_len != nodes.len() {
            let marker = Self::truncation_marker();
            let mut entry = marker.to_json(delegate.options_for_node(&marker, options), delegate);
            entry.insert("truncated".into(), Value::Bool(true));
            json.push(Value::Object(entry));
        }
        Value::Array(json)
    }
}

/// Controls which relationships are serialized. The default is the current node only.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiagnosticsSerializationOptions {
    /// Whether to include the diagnostic properties array, even when empty.
    pub include_properties: bool,
    /// Remaining depth of diagnostic children to include.
    pub subtree_depth: usize,
}

/// Hooks for selecting diagnostic data before it reaches an inspection client.
pub trait DiagnosticsSerializationDelegate {
    /// Extra keys for this node, merged before serializing its relationships.
    fn additional_properties(&self, _node: &DiagnosticsNode) -> Map<String, Value> {
        Map::new()
    }

    /// Extra keys in the reduced inspector view. Defaults to the same keys
    /// as the detailed view, as with a delegate that ignores `fullDetails`.
    fn additional_compact_properties(&self, node: &DiagnosticsNode) -> Map<String, Value> {
        self.additional_properties(node)
    }

    /// Returns the selected children in wire order. May promote descendants.
    fn filter_children<'a>(&'a self, owner: &'a DiagnosticsNode) -> Vec<&'a DiagnosticsNode> {
        owner.children().iter().collect()
    }

    /// Returns the selected properties in wire order, including hidden ones by default.
    fn filter_properties<'a>(&'a self, owner: &'a DiagnosticsNode) -> Vec<&'a DiagnosticsNode> {
        owner.properties().iter().collect()
    }

    /// Limits a filtered list. Changing its length appends an explicit truncation marker.
    fn truncate_nodes<'a>(
        &'a self,
        _nodes: &mut Vec<&'a DiagnosticsNode>,
        _owner: &'a DiagnosticsNode,
    ) {
    }

    /// Chooses options for each property or child. The default decreases the depth by one.
    fn options_for_node(
        &self,
        _node: &DiagnosticsNode,
        mut options: DiagnosticsSerializationOptions,
    ) -> DiagnosticsSerializationOptions {
        options.subtree_depth = options.subtree_depth.saturating_sub(1);
        options
    }
}

impl DiagnosticsSerializationDelegate for () {}
