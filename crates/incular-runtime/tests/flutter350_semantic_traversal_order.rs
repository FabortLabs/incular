use std::f32::consts::FRAC_PI_2;

use incular_accessibility::AccessKitProjection;
use incular_config::{Alignment, Constraints, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::{OrdinalSortKey, Role, SemanticNodeId, SemanticsTree};
use incular_widgets::{
    Column, Directionality, Placeholder, Positioned, Row, Semantics, SizedBox, Stack, Text,
    Transform, Widget,
};

fn mount(scene: impl Into<Widget>) -> Runtime {
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let limits = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(limits).unwrap();
    let mut native = AccessKitProjection::new();
    native.activate();
    assert!(native.sync(runtime.tree().semantics(), 1.).is_some());
    let revision = runtime.tree().semantics().revision();
    runtime.run_frame(limits).unwrap();
    assert_eq!(runtime.tree().semantics().revision(), revision);
    assert!(native.sync(runtime.tree().semantics(), 1.).is_none());
    runtime
}

fn labels<'a>(tree: &'a SemanticsTree, ids: &[SemanticNodeId]) -> Vec<Option<&'a str>> {
    ids.iter()
        .map(|&id| tree.node(id).unwrap().label.as_deref())
        .collect()
}

fn text(label: &'static str) -> Widget {
    Text::new(label).into()
}

fn order(label: &'static str, order: f64) -> Widget {
    Semantics::new(text(label))
        .sort_key(OrdinalSortKey::new(order))
        .into()
}

#[test]
fn explicit_sort_numbers_order_nested_annotation_groups_independently() {
    let group = Semantics::new(Row::new([
        order("Label 3", 3.),
        order("Label 4", 2.),
        order("Label 5", 1.),
    ]))
    .sort_key(OrdinalSortKey::new(1.))
    .explicit_child_nodes(true);
    let scene = Semantics::new(Column::new([
        order("Label 1", 3.),
        order("Label 2", 2.),
        group.into(),
    ]))
    .sort_key(OrdinalSortKey::new(0.))
    .explicit_child_nodes(true);
    let runtime = mount(Directionality::new(TextDirection::Ltr, scene));
    let tree = runtime.tree().semantics();
    let [root] = tree.roots() else {
        panic!("one explicit root")
    };
    let children = &tree.node(*root).unwrap().children;
    assert_eq!(
        labels(tree, children),
        [None, Some("Label 2"), Some("Label 1")]
    );
    assert_eq!(
        labels(tree, &tree.node(children[0]).unwrap().children),
        [Some("Label 5"), Some("Label 4"), Some("Label 3")]
    );
}

#[test]
fn explicit_sort_numbers_can_override_spatial_order_in_a_row() {
    let scene = Row::new([
        order("Label 1", 3.),
        order("Label 2", 1.),
        order("Label 3", 2.),
    ]);
    let runtime = mount(Directionality::new(TextDirection::Ltr, scene));
    let tree = runtime.tree().semantics();
    assert_eq!(
        labels(tree, tree.roots()),
        [Some("Label 2"), Some("Label 3"), Some("Label 1")]
    );
}

#[test]
fn unkeyed_column_then_row_order_includes_each_text_with_its_direction() {
    let row: Widget = Row::new([text("Label 3"), text("Label 4"), text("Label 5")]).into();
    for transformed in [false, true] {
        let row = if transformed {
            Transform::rotation(FRAC_PI_2, row.clone()).into()
        } else {
            row.clone()
        };
        let runtime = mount(Directionality::new(
            TextDirection::Ltr,
            Column::new([text("Label 1"), text("Label 2"), row]),
        ));
        let tree = runtime.tree().semantics();
        assert_eq!(
            labels(tree, tree.roots()),
            [
                Some("Label 1"),
                Some("Label 2"),
                Some("Label 3"),
                Some("Label 4"),
                Some("Label 5")
            ],
            "transformed = {transformed}",
        );
        for &id in tree.roots() {
            assert_eq!(
                tree.node(id).unwrap().state.text_direction,
                Some(TextDirection::Ltr)
            );
        }
    }
}

#[test]
fn without_direction_unkeyed_overlap_uses_paint_order_not_position() {
    let mut children = vec![Widget::from(
        Semantics::new(Placeholder::new())
            .button(true)
            .identifier("placeholder"),
    )];
    for (name, x, y) in [
        ("box-0", 100., 200.),
        ("box-1", 200., 100.),
        ("box-2", 100., 100.),
        ("box-3", 0., 100.),
        ("box-4", 100., 10.),
    ] {
        let node = Semantics::new(SizedBox::square(30.))
            .button(true)
            .identifier(name);
        children.push(Positioned::new(node).left(x).top(y).into());
    }
    let runtime = mount(Stack::aligned(Alignment::CENTER, children));
    let tree = runtime.tree().semantics();
    let order = tree
        .roots()
        .iter()
        .map(|&id| {
            let node = tree.node(id).unwrap();
            assert_eq!(node.state.text_direction, None);
            assert_eq!(node.role, Role::Button);
            node.state.identifier.as_deref().unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        order,
        ["placeholder", "box-0", "box-1", "box-2", "box-3", "box-4"]
    );
}
