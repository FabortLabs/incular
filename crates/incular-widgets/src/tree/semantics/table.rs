//! Stable semantic rows and cells for a flat visual Table.

use super::*;

#[derive(Default)]
pub(in crate::tree) struct TableSemanticWrappers {
    rows: FastHashMap<usize, SemanticNodeId>,
    cells: FastHashMap<(usize, usize), SemanticNodeId>,
}

impl TableSemanticWrappers {
    pub(in crate::tree) fn into_ids(self) -> impl Iterator<Item = SemanticNodeId> {
        self.rows.into_values().chain(self.cells.into_values())
    }

    fn retain_current(&mut self, kept: &mut [SemanticNodeId]) -> bool {
        kept.sort_unstable_by_key(|id| id.0);
        self.rows.retain(|_, id| is_attached(kept, *id));
        self.cells.retain(|_, id| is_attached(kept, *id));
        !self.rows.is_empty() || !self.cells.is_empty()
    }
}

#[derive(Default)]
struct Cell {
    geometry: Rect,
    raw_ids: Vec<SemanticNodeId>,
    explicit: bool,
}

impl WidgetTree {
    /// Groups the semantic nodes owned by each visible cell into the table's
    /// physical rows and columns without inserting render/layout elements.
    pub(super) fn assemble_semantic_table(
        &mut self,
        table: ElementId,
        raw_edges: &[(u32, u32, ElementId)],
        result: &mut Vec<SemanticNodeId>,
        changes: &mut SyntheticSemanticChanges,
    ) {
        let entry = self.element_live(table, "semantic table stays mounted");
        let render = self.render_live(entry.render, "semantic table render stays mounted");
        let RenderKind::Table {
            columns,
            text_direction,
            row_spacing,
            ..
        } = *render.object.kind()
        else {
            return;
        };
        let columns = columns.max(1);
        let table_width = render.size.width;
        let world = self.render_world_transform(entry.render);
        let reveal_rows = !self.pointer_actions_blocked(table)
            && self
                .scrollable_viewport_chain(table)
                .into_iter()
                .any(|(_, _, _, _, physics)| physics.allows_implicit_scrolling());
        let mut slots = FastHashMap::default();
        let mut cells = Vec::with_capacity(entry.children.len());
        for &id in &entry.children {
            let node = self.element_live(id, "table cell stays mounted");
            let render = self.render_live(node.render, "table cell render stays mounted");
            slots.insert(id, cells.len());
            cells.push(Cell {
                geometry: Rect::from_origin_size(render.offset, render.size),
                ..Cell::default()
            });
        }
        for &(_, _, raw_element) in raw_edges {
            let id = self.semantic_ids[&raw_element];
            let mut cursor = raw_element;
            let mut assigned = false;
            while let Some(parent) = self.parent(cursor) {
                if parent == table {
                    if let Some(&slot) = slots.get(&cursor) {
                        assigned = true;
                        let cell = &mut cells[slot];
                        cell.raw_ids.push(id);
                        cell.explicit = self.elements.get(raw_element.0).is_some_and(|entry| {
                            entry
                                .widget
                                .semantic_properties()
                                .explicit
                                .as_ref()
                                .is_some_and(|semantics| {
                                    matches!(
                                        semantics.role,
                                        SemanticRole::Cell | SemanticRole::ColumnHeader
                                    )
                                })
                        });
                    }
                    break;
                }
                cursor = parent;
            }
            if !assigned {
                changes.dropped.push(id);
            }
        }

        result.clear();
        let mut wrappers = self
            .table_semantic_wrappers
            .remove(&table)
            .unwrap_or_default();
        let kept_from = changes.kept.len();
        let rtl = text_direction == TextDirection::Rtl;
        let mut y = 0.0;
        for (row_index, row) in cells.chunks_mut(columns).enumerate() {
            let height = row
                .iter()
                .map(|cell| cell.geometry.size.height)
                .fold(0.0, f32::max);
            if height > 0. {
                let mut columns = Vec::new();
                for visual in 0..row.len() {
                    let column = if rtl { row.len() - visual - 1 } else { visual };
                    let cell = &mut row[column];
                    if cell.geometry.size.width <= 0. {
                        changes.dropped.append(&mut cell.raw_ids);
                        continue;
                    }
                    if cell.raw_ids.is_empty() {
                        continue;
                    }
                    if cell.explicit
                        && let [id] = cell.raw_ids.as_slice()
                    {
                        columns.push(*id);
                        continue;
                    }
                    let bounds = world.transform_rect_bbox(Rect::from_origin_size(
                        Offset::new(cell.geometry.origin.x, y),
                        Size::new(cell.geometry.size.width, height),
                    ));
                    let index = (row_index, visual);
                    let id = write_wrapper(
                        &mut self.semantics,
                        wrappers.cells.get(&index).copied(),
                        SemanticRole::Cell,
                        false,
                        bounds,
                        std::mem::take(&mut cell.raw_ids),
                        &mut changes.detached,
                    );
                    wrappers.cells.insert(index, id);
                    columns.push(id);
                    changes.kept.push(id);
                }
                if !columns.is_empty() {
                    let bounds = world.transform_rect_bbox(Rect::from_origin_size(
                        Offset::new(0.0, y),
                        Size::new(table_width, height),
                    ));
                    let id = write_wrapper(
                        &mut self.semantics,
                        wrappers.rows.get(&row_index).copied(),
                        SemanticRole::Row,
                        reveal_rows,
                        bounds,
                        columns,
                        &mut changes.detached,
                    );
                    wrappers.rows.insert(row_index, id);
                    result.push(id);
                    changes.kept.push(id);
                }
            } else {
                for cell in row {
                    changes.dropped.append(&mut cell.raw_ids);
                }
            }
            y += height + row_spacing;
        }
        if wrappers.retain_current(&mut changes.kept[kept_from..]) {
            self.table_semantic_wrappers.insert(table, wrappers);
        }
    }

    /// Reveals a synthetic row using the latest bounds of its retained ID.
    /// Ordinary element-based semantic actions do not target synthetic rows.
    #[doc(hidden)]
    pub fn semantic_show_table_row_on_screen(&self, row: SemanticNodeId) -> bool {
        let Some(node) = self.semantics.node(row).filter(|node| {
            node.role == SemanticRole::Row
                && node.actions.contains(&SemanticActionKind::ShowOnScreen)
        }) else {
            return false;
        };
        self.table_semantic_wrappers
            .iter()
            .find_map(|(&table, wrappers)| {
                wrappers.rows.values().any(|id| *id == row).then_some(table)
            })
            .is_some_and(|table| {
                !self.pointer_actions_blocked(table)
                    && self.reveal_semantic_bounds(table, node.bounds)
            })
    }
}

fn write_wrapper(
    tree: &mut SemanticsTree,
    old: Option<SemanticNodeId>,
    role: SemanticRole,
    can_reveal: bool,
    bounds: Rect,
    children: Vec<SemanticNodeId>,
    detached: &mut Vec<SemanticNodeId>,
) -> SemanticNodeId {
    let existing = old.and_then(|id| tree.node(id).map(|node| (id, node)));
    if let Some((id, node)) = existing {
        if node.children != children {
            detached.extend_from_slice(&node.children);
        }
        if node.actions.contains(&SemanticActionKind::ShowOnScreen) == can_reveal {
            let _ = tree.update_geometry(id, bounds, &children);
            return id;
        }
    }
    let node = SemanticNode {
        id: old.unwrap_or_else(|| SemanticNodeId(ArenaId::from_parts(0, 0))),
        role,
        label: None,
        value: None,
        description: None,
        bounds,
        state: SemanticState::default(),
        actions: if can_reveal {
            vec![SemanticActionKind::ShowOnScreen]
        } else {
            Vec::new()
        },
        children,
    };
    if let Some((id, _)) = existing {
        let _ = tree.update(id, node);
        id
    } else {
        tree.insert(node)
    }
}
