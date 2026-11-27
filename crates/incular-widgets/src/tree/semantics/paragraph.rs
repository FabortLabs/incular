//! Independent semantic children of one retained rich-text render box.

use super::*;
use incular_semantics::SemanticTextAttributes;
use incular_text::{InlineSpan, TextLayout, TextSpan, combine_inline_span_semantics};

#[derive(Clone, Debug, PartialEq)]
pub(in crate::tree) enum ParagraphSemanticPart {
    Text {
        bytes: std::ops::Range<usize>,
        label: String,
        identifier: Option<String>,
        attributes: Option<Arc<SemanticTextAttributes>>,
    },
    Placeholder(usize),
}

impl ParagraphSemanticPart {
    pub(in crate::tree) fn for_spans(span: &TextSpan) -> Option<Arc<[Self]>> {
        fn has_identifier(span: &TextSpan) -> bool {
            span.semantics_identifier.is_some()
                || span
                    .children
                    .iter()
                    .any(|child| matches!(child, InlineSpan::Text(span) if has_identifier(span)))
        }
        if !has_identifier(span) {
            return None;
        }
        let spans = span.semantics_information();
        Some({
            let mut offset = 0;
            let mut placeholder_index = 0;
            combine_inline_span_semantics(&spans)
                .into_iter()
                .filter_map(|span| {
                    let start = offset;
                    offset += span.text.len();
                    if span.is_placeholder {
                        let slot = placeholder_index;
                        placeholder_index += 1;
                        return Some(Self::Placeholder(slot));
                    }
                    if start == offset {
                        return None;
                    }
                    let attributes: Vec<_> = span
                        .string_attributes
                        .into_iter()
                        .map(super::super::descriptors::paragraph_string_attribute)
                        .collect();
                    Some(Self::Text {
                        bytes: start..offset,
                        label: span.label,
                        identifier: span.identifier.map(str::to_owned),
                        attributes: (!attributes.is_empty()).then(|| {
                            Arc::new(SemanticTextAttributes {
                                label: attributes,
                                ..Default::default()
                            })
                        }),
                    })
                })
                .collect()
        })
    }
}

impl WidgetTree {
    /// Assembles text fragments without adding render boxes or changing layout.
    /// Existing nodes are reused in reading order; losing a trailing fragment
    /// drops its ID instead of retaining it for a later, unrelated fragment.
    pub(super) fn assemble_paragraph_semantics(
        &mut self,
        paragraph: ElementId,
        parent: SemanticNodeId,
        edges: &[(u32, u32, ElementId)],
        child_ids: &mut Vec<SemanticNodeId>,
        changes: &mut SyntheticSemanticChanges,
    ) {
        let entry = self.element_live(paragraph, "semantic paragraph remains live");
        let Some(parts) = entry.widget.semantic_properties().paragraph_parts.clone() else {
            // Old IDs are disposed by the detached-child sweep.
            self.paragraph_semantic_nodes.remove(&paragraph);
            return;
        };
        let (direction, inline) = match entry.widget.kind() {
            WidgetKind::Text { .. } => {
                let render =
                    self.render_live(entry.render, "semantic paragraph render remains live");
                let RenderKind::Text {
                    text_direction,
                    inline,
                    ..
                } = render.object.kind()
                else {
                    return;
                };
                (*text_direction, inline.clone())
            }
            _ => return,
        };
        let render = self.render_live(entry.render, "semantic paragraph render remains live");
        let Some(layout) = render.text_layout().cloned() else {
            return;
        };
        let size = render.size;
        let world = self.render_world_transform(entry.render);
        // Inline children may contribute zero, one, or several independent
        // semantics nodes. Allocate the slot map only for a paragraph that
        // has an actual inline widget represented in the current child list.
        let completed_slots = inline.as_ref().filter(|_| !edges.is_empty()).map(|_| {
            layout
                .placeholders
                .iter()
                .map(|box_| box_.index)
                .collect::<FastHashSet<_>>()
        });
        let mut children_by_slot = FastHashMap::<usize, Vec<SemanticNodeId>>::default();
        for ((_, _, child), &semantic) in edges.iter().zip(child_ids.iter()) {
            let mut direct = *child;
            let mut owner = self.parent(direct);
            let slot = loop {
                match owner {
                    Some(owner) if owner != paragraph => {
                        direct = owner;
                    }
                    Some(_) => {
                        break inline.as_ref().and_then(|inline| {
                            let position = entry
                                .children
                                .iter()
                                .position(|&element| element == direct)?;
                            inline.slots.get(position).copied()
                        });
                    }
                    None => break None,
                }
                owner = self.parent(direct);
            };
            if let Some(slot) =
                slot.filter(|&slot| {
                    inline.as_ref().zip(completed_slots.as_ref()).is_some_and(
                        |(inline, completed)| completed.contains(&inline.placeholders[slot].index),
                    )
                })
            {
                children_by_slot.entry(slot).or_default().push(semantic);
            } else {
                // A truncated placeholder has no exposed widget children.
                changes.dropped.push(semantic);
            }
        }
        child_ids.clear();
        let mut old = self
            .paragraph_semantic_nodes
            .remove(&paragraph)
            .unwrap_or_default()
            .into_iter();
        let mut current = Vec::new();
        for part in parts.iter() {
            let (bytes, label, identifier, attributes) = match part {
                ParagraphSemanticPart::Text {
                    bytes,
                    label,
                    identifier,
                    attributes,
                } => (bytes, label, identifier, attributes),
                ParagraphSemanticPart::Placeholder(slot) => {
                    if let Some(nodes) = children_by_slot.get(slot) {
                        child_ids.extend(nodes.iter().copied());
                    }
                    continue;
                }
            };
            let Some(bounds) = paragraph_text_bounds(&layout, bytes, size) else {
                continue;
            };
            let node = SemanticNode {
                id: parent,
                role: SemanticRole::Text,
                label: Some(label.clone()),
                value: None,
                description: None,
                bounds: world.transform_rect_bbox(bounds),
                state: SemanticState {
                    identifier: identifier.clone(),
                    text_attributes: attributes.clone(),
                    text_direction: Some(direction),
                    ..SemanticState::default()
                },
                actions: Vec::new(),
                children: Vec::new(),
            };
            let id = if let Some(id) = old.next() {
                let _ = self.semantics.update(id, SemanticNode { id, ..node });
                id
            } else {
                self.semantics.insert(node)
            };
            current.push(id);
            child_ids.push(id);
        }
        changes.kept.extend(current.iter().copied());
        if !current.is_empty() {
            self.paragraph_semantic_nodes.insert(paragraph, current);
        }
    }
}

fn paragraph_text_bounds(
    layout: &TextLayout,
    bytes: &std::ops::Range<usize>,
    size: Size,
) -> Option<Rect> {
    let end = bytes.end.min(layout.retained_source_end());
    if bytes.start >= end {
        return None;
    }
    let first_line = layout.lines.partition_point(|line| line.end <= bytes.start);
    let (mut left, mut top, mut right, mut bottom) = (
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );
    for line in &layout.lines[first_line..] {
        if end <= line.start {
            break;
        }
        for (span_start, span_end) in line.selection_spans(bytes.start, end) {
            left = left.min(span_start);
            right = right.max(span_end);
            top = top.min(line.top);
            bottom = bottom.max(line.top + line.height);
        }
    }
    if !left.is_finite() {
        return None;
    }
    // Flutter pads the span range on each side for easier accessibility
    // targeting; the padding does not affect glyphs or pointer hit testing.
    let width = (right - left).min(size.width);
    let height = (bottom - top).min(size.height);
    let left = left.max(0.);
    let top = top.max(0.);
    let rounded_left = left.floor() - 4.;
    let rounded_top = top.floor() - 4.;
    let rounded_right = (left + width).ceil() + 4.;
    let rounded_bottom = (top + height).ceil() + 4.;
    Some(Rect::from_origin_size(
        Offset::new(rounded_left, rounded_top),
        Size::new(rounded_right - rounded_left, rounded_bottom - rounded_top),
    ))
}
