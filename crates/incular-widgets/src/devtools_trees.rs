//! Bounded, read-only DevTools snapshots of the semantics tree, compositor
//! layers, per-element work counters and runtime counters.
//!
//! Nothing here marks state dirty, runs layout, or retains handles: every
//! method copies a bounded amount of data out of the retained tree.

use crate::tree::WidgetTree;
use crate::util::truncate;
use incular_core::{Rect, Transform};
use incular_devtools_protocol::{
    CompositorCounters, DevWidgetId, LayerNodeWire, RuntimeDiagnosticsWire, SemanticsCounters,
    SemanticsNodeWire, TextCacheCounters, WidgetPipelineCounters, WorkCountEntry,
};
use incular_rendering::{LayerId, LayerKind};
use incular_semantics::{SemanticNode, SemanticNodeId};
use std::collections::HashSet;

/// Maximum semantics nodes or layers returned by one inspection request.
pub const MAX_INSPECTION_NODES: usize = 4096;
/// Maximum entries returned by one work-count request.
pub const MAX_WORK_COUNT_ENTRIES: usize = 200;
/// Longest label, value or description copied onto the wire.
const MAX_TEXT_CHARS: usize = 120;

fn semantic_key(id: SemanticNodeId) -> u64 {
    (u64::from(id.0.generation()) << 32) | u64::from(id.0.index())
}

fn rect_array(rect: Rect) -> [f32; 4] {
    [
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
    ]
}

fn semantics_node_wire(node: &SemanticNode, parent: Option<u64>) -> SemanticsNodeWire {
    let state = &node.state;
    let mut flags = Vec::new();
    let mut flag = |set: bool, name: &str| {
        if set {
            flags.push(name.to_owned());
        }
    };
    flag(state.enabled, "enabled");
    flag(state.focused, "focused");
    flag(state.focusable, "focusable");
    flag(state.selected, "selected");
    flag(state.expanded == Some(true), "expanded");
    flag(state.read_only, "read_only");
    flag(state.editable, "editable");
    flag(state.obscured, "obscured");
    flag(state.multiline, "multiline");
    flag(state.busy, "busy");
    flag(state.invalid, "invalid");
    flag(state.required, "required");
    flag(state.hidden, "hidden");
    flag(state.live_region, "live_region");
    if let Some(toggled) = state.toggled {
        flags.push(if toggled { "toggled" } else { "untoggled" }.to_owned());
    }
    if let Some(checked) = state.checked {
        flags.push(
            match checked {
                incular_semantics::CheckedState::Unchecked => "unchecked",
                incular_semantics::CheckedState::Checked => "checked",
                incular_semantics::CheckedState::Indeterminate => "mixed",
            }
            .to_owned(),
        );
    }
    SemanticsNodeWire {
        id: semantic_key(node.id),
        parent,
        children: node.children.iter().copied().map(semantic_key).collect(),
        role: format!("{:?}", node.role),
        label: node
            .label
            .as_deref()
            .map(|label| truncate(label, MAX_TEXT_CHARS)),
        // Obscured fields (passwords) never leave the process.
        value: if state.obscured {
            None
        } else {
            node.value
                .as_deref()
                .map(|value| truncate(value, MAX_TEXT_CHARS))
        },
        description: node
            .description
            .as_deref()
            .map(|text| truncate(text, MAX_TEXT_CHARS)),
        bounds: rect_array(node.bounds),
        flags,
        actions: node
            .actions
            .iter()
            .map(|action| format!("{action:?}"))
            .collect(),
    }
}

fn layer_summary(kind: &LayerKind, world: Transform) -> (&'static str, String, Option<[f32; 4]>) {
    let world_rect = |rect: Rect| Some(rect_array(world.transform_rect_bbox(rect)));
    match kind {
        LayerKind::Picture {
            display_list,
            bounds,
        } => (
            "Picture",
            format!("{} commands", display_list.len()),
            world_rect(*bounds),
        ),
        LayerKind::Transform { transform } => {
            let c = transform.to_kurbo().as_coeffs();
            (
                "Transform",
                format!(
                    "translate ({:.1}, {:.1}) scale ({:.2}, {:.2})",
                    c[4], c[5], c[0], c[3]
                ),
                None,
            )
        }
        LayerKind::ClipRect { rect } => ("ClipRect", String::new(), world_rect(*rect)),
        LayerKind::ClipRRect { rrect } => ("ClipRRect", String::new(), world_rect(rrect.rect)),
        LayerKind::ClipOval { rect } => ("ClipOval", String::new(), world_rect(*rect)),
        LayerKind::ClipPath { path, .. } => (
            "ClipPath",
            String::new(),
            path.bounds().and_then(world_rect),
        ),
        LayerKind::Opacity { alpha } => ("Opacity", format!("alpha {alpha:.2}"), None),
        LayerKind::Blur { blur } => (
            "Blur",
            format!("sigma ({:.1}, {:.1})", blur.sigma_x, blur.sigma_y),
            None,
        ),
        LayerKind::DropShadow { .. } => ("DropShadow", String::new(), None),
        LayerKind::ColorFilter { .. } => ("ColorFilter", String::new(), None),
        LayerKind::Blend { mode } => ("Blend", format!("{mode:?}"), None),
        LayerKind::ShaderMask { .. } => ("ShaderMask", String::new(), None),
        LayerKind::BackdropFilter { enabled, .. } => (
            "BackdropFilter",
            if *enabled { "enabled" } else { "disabled" }.to_owned(),
            None,
        ),
        LayerKind::AnnotatedRegion { sized, size, .. } => (
            "AnnotatedRegion",
            if *sized {
                format!("{:.0} x {:.0}", size.width, size.height)
            } else {
                String::new()
            },
            None,
        ),
        LayerKind::Leader { size, .. } => (
            "Leader",
            format!("{:.0} x {:.0}", size.width, size.height),
            None,
        ),
        LayerKind::Follower(_) => ("Follower", String::new(), None),
    }
}

impl WidgetTree {
    /// Bounded pre-order snapshot of the semantics tree. Sibling roots are
    /// included after the designated root. Returns `(nodes, truncated)`.
    #[must_use]
    pub fn devtools_semantics_nodes(&self) -> (Vec<SemanticsNodeWire>, bool) {
        let semantics = self.semantics();
        let mut parented = HashSet::new();
        for (_, node) in semantics.iter() {
            parented.extend(node.children.iter().copied());
        }
        let mut extra: Vec<SemanticNodeId> = semantics
            .iter()
            .map(|(id, _)| id)
            .filter(|id| Some(*id) != semantics.root() && !parented.contains(id))
            .collect();
        extra.sort_by_key(|id| id.0);
        let mut stack: Vec<(SemanticNodeId, Option<u64>)> =
            extra.into_iter().rev().map(|id| (id, None)).collect();
        if let Some(root) = semantics.root() {
            stack.push((root, None));
        }
        let mut seen = HashSet::new();
        let mut nodes = Vec::new();
        let mut truncated = false;
        while let Some((id, parent)) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            let Some(node) = semantics.node(id) else {
                continue;
            };
            if nodes.len() >= MAX_INSPECTION_NODES {
                truncated = true;
                break;
            }
            let wire = semantics_node_wire(node, parent);
            let key = wire.id;
            nodes.push(wire);
            for child in node.children.iter().rev() {
                stack.push((*child, Some(key)));
            }
        }
        (nodes, truncated)
    }

    /// Bounded pre-order snapshot of the retained compositor layers with
    /// window-space bounds where a layer has geometry. Returns
    /// `(layers, truncated)`.
    #[must_use]
    pub fn devtools_layer_nodes(&self) -> (Vec<LayerNodeWire>, bool) {
        let tree = self.compositor_layers();
        let mut layers = Vec::new();
        let mut truncated = false;
        let mut stack: Vec<(LayerId, Option<u64>, Transform)> = Vec::new();
        if let Some(root) = tree.root() {
            stack.push((root, None, Transform::IDENTITY));
        }
        while let Some((id, parent, world)) = stack.pop() {
            let Some((kind, children)) = tree.layer(id) else {
                continue;
            };
            if layers.len() >= MAX_INSPECTION_NODES {
                truncated = true;
                break;
            }
            let (name, summary, bounds) = layer_summary(kind, world);
            let child_world = match kind {
                LayerKind::Transform { transform } => world.then(*transform),
                _ => world,
            };
            layers.push(LayerNodeWire {
                id: id.raw(),
                parent,
                children: children.iter().map(|child| child.raw()).collect(),
                kind: name.to_owned(),
                summary,
                bounds,
            });
            for child in children.iter().rev() {
                stack.push((*child, Some(id.raw()), child_world));
            }
        }
        (layers, truncated)
    }

    /// Per-element work counters ranked by build count (then total work),
    /// bounded by `limit` and [`MAX_WORK_COUNT_ENTRIES`].
    #[must_use]
    pub fn devtools_work_counts(&self, limit: usize) -> Vec<WorkCountEntry> {
        let limit = limit.min(MAX_WORK_COUNT_ENTRIES);
        let mut ranked: Vec<_> = self.dev_elements().iter().collect();
        ranked.sort_unstable_by(|(left_id, left), (right_id, right)| {
            right
                .dev
                .builds
                .cmp(&left.dev.builds)
                .then_with(|| {
                    let work = |e: &crate::tree::Element| e.dev.layouts + e.dev.paints;
                    work(right).cmp(&work(left))
                })
                .then_with(|| left_id.index().cmp(&right_id.index()))
        });
        ranked
            .into_iter()
            .take(limit)
            .map(|(raw, element)| WorkCountEntry {
                id: DevWidgetId::new(u64::from(raw.index()), u64::from(raw.generation())),
                type_name: element.widget.kind().dev_type_name_widget(),
                builds: element.dev.builds,
                layouts: element.dev.layouts,
                paints: element.dev.paints,
                semantics: element.dev.semantic_updates,
                composites: element.dev.composites,
            })
            .collect()
    }

    /// Cumulative widget, text, semantics and compositor counters for this tree.
    #[must_use]
    pub fn devtools_runtime_counters(&self) -> RuntimeDiagnosticsWire {
        let d = self.diagnostics();
        let compositor = self.compositor_diagnostics();
        let text = self.text_diagnostics();
        let semantics = self.semantics_diagnostics();
        RuntimeDiagnosticsWire {
            widgets: WidgetPipelineCounters {
                mounts: d.mounts,
                unmounts: d.unmounts,
                rebuilds: d.rebuilds,
                layouts: d.layouts,
                paints: d.paints,
                composites: d.composites,
                animation_ticks: d.animation_ticks,
                scroll_offset_updates: d.scroll_offset_updates,
                reconciliation_fast_paths: d.reconciliation_fast_paths,
                layout_cache_hits: d.layout_cache_hits,
                display_lists_reused: d.display_lists_reused,
                compositor_only_updates: d.compositor_only_updates,
                lazy_layouts: d.lazy_layouts,
                items_built: d.items_built,
                items_reused: d.items_reused,
                child_list_scans: d.child_list_scans,
                identical_child_bailouts: d.identical_child_bailouts,
                elements_created: d.elements_created,
                elements_removed: d.elements_removed,
                elements_moved: d.elements_moved,
                dirty_requests: d.dirty_requests,
                dirty_queue_deduplicated: d.dirty_queue_deduplicated,
                elements_total: self.element_count() as u64,
                render_objects_total: self.render_object_count() as u64,
                layers_total: compositor.layers,
            },
            text: TextCacheCounters {
                layouts_requested: text.layouts_requested,
                cache_hits: text.cache_hits,
                cache_misses: text.cache_misses,
                font_resolution_requests: text.font_resolution_requests,
                font_resolution_cache_hits: text.font_resolution_cache_hits,
                font_resolution_cache_misses: text.font_resolution_cache_misses,
                coverage_queries: text.coverage_queries,
                fallback_runs: text.fallback_runs,
                fallback_fonts_used: text.fallback_fonts_used,
                missing_clusters: text.missing_clusters,
                shaping_runs: text.shaping_runs,
                paragraphs_considered: text.paragraphs_considered,
                paragraphs_reshaped: text.paragraphs_reshaped,
                parley_layouts_reused: text.parley_layouts_reused,
                documents_composed: text.documents_composed,
                documents_reused: text.documents_reused,
                width_reuses: text.width_reuses,
                cache_growths: text.cache_growths,
                cache_shrinks: text.cache_shrinks,
                paragraphs_carried: text.paragraphs_carried,
                shapes_reused: text.shapes_reused,
            },
            semantics: SemanticsCounters {
                nodes: self.semantics().len() as u64,
                nodes_created: semantics.nodes_created,
                nodes_removed: semantics.nodes_removed,
                nodes_updated: semantics.nodes_updated,
                property_updates: semantics.property_updates,
                geometry_updates: semantics.geometry_updates,
                native_updates: semantics.native_updates,
                actions_received: semantics.actions_received,
                revisions: semantics.revisions,
            },
            compositor: CompositorCounters {
                layers: compositor.layers,
                picture_layers_repainted: compositor.picture_layers_repainted,
                transform_updates: compositor.transform_updates,
                clip_updates: compositor.clip_updates,
                layers_culled: compositor.layers_culled,
                opacity_updates: compositor.opacity_updates,
                leader_publish_passes: compositor.leader_publish_passes,
                leaders_published: compositor.leaders_published,
            },
        }
    }
}
