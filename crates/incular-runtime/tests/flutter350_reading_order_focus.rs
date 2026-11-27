use TextDirection::{Ltr, Rtl};
use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Column, Directionality, Focus, FocusNode, FocusScope, FocusScopeNode, FocusTraversalGroup, Row,
    SizedBox, TextDirection, Widget,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn item(node: &FocusNode, extent: f32) -> Focus {
    Focus::new(SizedBox::square(extent)).node(node.clone())
}

#[test]
fn reading_order_can_find_the_first_focus_from_an_unfocused_later_node() {
    let [first, second] = std::array::from_fn(|_| FocusNode::new());
    let scope = FocusScopeNode::new();
    let subtree =
        FocusScope::new(Column::new([item(&first, 100.), item(&second, 100.)])).node(scope.clone());
    let root = Directionality::new(Ltr, FocusTraversalGroup::new(subtree).reading_order());
    let mut runtime = Runtime::new(root.into()).unwrap();
    frame(&mut runtime);
    assert!(!first.has_focus() && !second.has_focus());
    assert!(runtime.next_focus_from(&second));
    frame(&mut runtime);
    assert!(first.has_primary_focus());
    assert!(!second.has_focus());
    assert!(scope.scope_focus_node().has_focus());
}

#[test]
fn reading_order_start_can_choose_an_invoking_nodes_eligible_descendant() {
    for can_request in [true, false] {
        let [parent, child] = std::array::from_fn(|_| FocusNode::new());
        let focus = Focus::new(Focus::new(SizedBox::shrink()).node(child.clone()))
            .node(parent.clone())
            .can_request_focus(can_request);
        let root = Directionality::new(
            Ltr,
            FocusTraversalGroup::new(FocusScope::new(focus)).reading_order(),
        );
        let mut runtime = Runtime::new(root.into()).unwrap();
        frame(&mut runtime);
        assert!(runtime.next_focus_from(&parent));
        frame(&mut runtime);
        assert_eq!(parent.has_primary_focus(), can_request);
        assert_eq!(child.has_primary_focus(), !can_request);
    }
}

fn mixed_scene(
    nodes: &[FocusNode; 10],
    scope: &FocusScopeNode,
    direction: TextDirection,
) -> Widget {
    let plain = |index: usize| item(&nodes[index], 10.);
    let directed = |index, direction| Directionality::new(direction, plain(index));
    let first = Directionality::new(Ltr, Row::new([plain(0), plain(1), plain(2)]));
    let second = Directionality::new(
        Ltr,
        Row::new([directed(3, Rtl), directed(4, Rtl), directed(5, Ltr)]),
    );
    let third = Row::new([
        directed(6, Ltr),
        directed(7, Rtl),
        directed(8, Rtl),
        directed(9, Ltr),
    ]);
    let scope = FocusScope::new(Column::new([
        Widget::from(first),
        second.into(),
        third.into(),
    ]))
    .node(scope.clone());
    Directionality::new(direction, FocusTraversalGroup::new(scope).reading_order()).into()
}

#[test]
fn reading_order_recomputes_mixed_direction_runs_after_a_root_direction_change() {
    let nodes = std::array::from_fn(|_| FocusNode::new());
    let scope = FocusScopeNode::new();
    let mut runtime = Runtime::new(mixed_scene(&nodes, &scope, Rtl)).unwrap();
    frame(&mut runtime);
    let cycle = |runtime: &mut Runtime, expected: [usize; 10]| {
        for id in expected {
            assert!(runtime.next_focus_from(&nodes[0]));
            frame(runtime);
            assert_eq!(
                nodes.iter().position(FocusNode::has_primary_focus),
                Some(id)
            );
        }
    };
    cycle(&mut runtime, [0, 1, 2, 4, 3, 5, 6, 7, 8, 9]);
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, mixed_scene(&nodes, &scope, Ltr))
        .unwrap();
    frame(&mut runtime);
    cycle(&mut runtime, [0, 1, 2, 4, 3, 5, 6, 8, 7, 9]);
}

#[test]
fn overlapping_rtl_rectangles_sort_by_right_edge_not_insertion_order() {
    use incular_widgets::{Alignment, Positioned, Stack};
    let nodes: [FocusNode; 10] = std::array::from_fn(|_| FocusNode::new());
    let scene = |alignment, positioned| {
        Directionality::new(
            Rtl,
            FocusTraversalGroup::new(
                Stack::new(nodes.iter().enumerate().map(|(index, node)| {
                    let child = item(node, 10. * (index + 1) as f32);
                    if positioned {
                        Positioned::new(child)
                            .top(5. * (index * (index + 1)) as f32)
                            .left(5. * (9 - index) as f32)
                            .into()
                    } else {
                        Widget::from(child)
                    }
                }))
                .alignment(alignment),
            )
            .reading_order(),
        )
        .into()
    };
    let mut runtime = Runtime::new(scene(Alignment::TOP_LEFT, false)).unwrap();
    frame(&mut runtime);
    let cycle = |runtime: &mut Runtime, expected: [usize; 10]| {
        for index in expected {
            assert!(runtime.next_focus_from(&nodes[0]));
            frame(runtime);
            assert_eq!(
                nodes.iter().position(FocusNode::has_primary_focus),
                Some(index)
            );
        }
    };
    let back_to_front = [9, 8, 7, 6, 5, 4, 3, 2, 1, 0];
    cycle(&mut runtime, back_to_front);
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, scene(Alignment::CENTER, false))
        .unwrap();
    frame(&mut runtime);
    cycle(&mut runtime, back_to_front);
    runtime
        .schedule_update(root, scene(Alignment::CENTER, true))
        .unwrap();
    frame(&mut runtime);
    assert!(
        nodes[0].has_primary_focus(),
        "the same node survives a new Positioned parent"
    );
    cycle(&mut runtime, [1, 2, 3, 4, 5, 6, 7, 8, 9, 0]);
}
