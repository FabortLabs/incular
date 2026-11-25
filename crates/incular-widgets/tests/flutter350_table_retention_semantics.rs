//! Flutter 3.50.0 Table retention, repump and portable accessibility tree.
use incular_config::Constraints;
use incular_core::Size;
use incular_widgets::{
    Table, TableCell, Text, TextDirection, Widget,
    internal::{Key, WidgetTree},
};

fn frame(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let _ = tree.paint();
    tree.update_semantics();
}

fn text(value: &str) -> Text {
    Text::new(value).text_direction(TextDirection::Ltr)
}

#[test]
fn an_empty_table_can_complete_a_frame() {
    let mut tree = WidgetTree::new();
    tree.mount(Table::new(1, Vec::<Widget>::new()).into())
        .unwrap();
    frame(&mut tree);
}

#[test]
fn reshaping_unkeyed_rows_reuses_first_row_but_not_removed_row() {
    let table = |cols, labels: &[&str]| {
        Widget::from(Table::new(cols, labels.iter().map(|label| text(label))))
    };
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(table(3, &["A", "B", "C", "D", "E", "F", "G", "H", "I"]))
        .unwrap();
    frame(&mut tree);
    let render = |tree: &WidgetTree, slot| {
        tree.children(root)
            .and_then(|children| children.get(slot))
            .and_then(|&id| tree.render_id(id))
            .expect("mounted text cell")
    };
    let first = render(&tree, 0);
    let removed = render(&tree, 6);
    tree.update(root, table(4, &["a", "b", "c", "d", "e", "f", "g", "h"]))
        .unwrap();
    frame(&mut tree);
    assert_eq!(first, render(&tree, 0));
    assert_ne!(
        removed,
        render(&tree, 6),
        "third-row old cell is not the new second-row column three"
    );
}

#[test]
fn default_flex_columns_remain_equally_sized_when_labels_change() {
    let table = |labels: [&str; 9]| {
        Widget::from(Table::new(
            3,
            labels
                .into_iter()
                .enumerate()
                .map(|(key, label)| Widget::from(text(label)).with_key(Key::Value(key as u64))),
        ))
    };
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(table([
            "AAAAAA", "B", "C", "D", "EEE", "F", "G", "H", "III",
        ]))
        .unwrap();
    frame(&mut tree);
    tree.update(
        root,
        table(["AAA", "B", "C", "D", "E", "FFFFFF", "G", "H", "III"]),
    )
    .unwrap();
    frame(&mut tree);
    let size = |key| {
        tree.element_bounds(tree.element_with_key(&Key::Value(key)).unwrap())
            .unwrap()
            .size
    };
    assert_eq!([size(3), size(6), size(1)], [size(0); 3]);
}

#[test]
fn table_publishes_table_row_and_labeled_cell_relationships() {
    let cells = ["Data Cell 1", "Data Cell 2"].map(|label| TableCell::new(text(label)));
    let mut tree = WidgetTree::new();
    tree.mount(Table::new(2, cells).into()).unwrap();
    frame(&mut tree);
    let semantics = tree.semantics();
    let leaves: Vec<_> = semantics
        .iter()
        .map(|(_, node)| (format!("{:?}", node.role), node.label.clone()))
        .collect();
    assert!(
        leaves
            .iter()
            .any(|(_, label)| label.as_deref() == Some("Data Cell 1"))
    );
    assert!(
        leaves
            .iter()
            .any(|(_, label)| label.as_deref() == Some("Data Cell 2"))
    );
    let table = semantics
        .iter()
        .map(|(_, node)| node)
        .find(|node| format!("{:?}", node.role) == "Table")
        .unwrap_or_else(|| panic!("table role missing; retained roles and labels {leaves:?}"));
    let row = semantics.node(table.children[0]).unwrap();
    assert_eq!(table.children.len(), 1);
    assert_eq!(format!("{:?}", row.role), "Row");
    let actual: Vec<_> = row
        .children
        .iter()
        .map(|id| {
            let cell = semantics.node(*id).unwrap();
            (format!("{:?}", cell.role), cell.label.as_deref().unwrap())
        })
        .collect();
    assert_eq!(
        actual,
        [
            ("Cell".to_string(), "Data Cell 1"),
            ("Cell".to_string(), "Data Cell 2")
        ]
    );
}

#[test]
fn width_changes_preserve_each_surviving_rows_cells_in_both_directions() {
    let form = |columns, rows, generation| {
        Widget::from(Table::new(
            columns,
            (0..rows).flat_map(move |row| {
                (0..columns).map(move |column| text(&format!("{generation}.{row}.{column}")))
            }),
        ))
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(form(3, 3, 0)).unwrap();
    frame(&mut tree);
    let renders = |tree: &WidgetTree| {
        tree.children(root)
            .unwrap()
            .iter()
            .map(|&id| tree.render_id(id).unwrap())
            .collect::<Vec<_>>()
    };
    let before = renders(&tree);
    tree.update(root, form(4, 2, 1)).unwrap();
    frame(&mut tree);
    let widened = renders(&tree);
    assert_eq!(&widened[0..3], &before[0..3]);
    assert_eq!(&widened[4..7], &before[3..6]);
    assert!(!before.contains(&widened[3]));
    assert!(!before.contains(&widened[7]));

    tree.update(root, form(2, 4, 2)).unwrap();
    frame(&mut tree);
    let narrowed = renders(&tree);
    assert_eq!(&narrowed[0..2], &widened[0..2]);
    assert_eq!(&narrowed[2..4], &widened[4..6]);
    assert!(narrowed[4..].iter().all(|id| !widened.contains(id)));
    assert!(narrowed[4..].iter().all(|id| !before.contains(id)));
}
