use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Alignment, Column, Directionality, Focus, FocusNode, FocusScope, FocusScopeNode,
    FocusTraversalGroup, FocusTraversalOrder, Positioned, Row, SizedBox, Stack, TextDirection,
    Widget,
};

fn item(node: &FocusNode, extent: f32) -> Focus {
    Focus::new(SizedBox::square(extent)).node(node.clone())
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

#[test]
fn explicit_numerical_order_selects_the_initial_focus() {
    let [first, second] = std::array::from_fn(|_| FocusNode::new());
    let scope = FocusScopeNode::new();
    let column = Column::new([
        FocusTraversalOrder::new(item(&first, 100.)).order(2.),
        FocusTraversalOrder::new(item(&second, 100.)).order(1.),
    ]);
    let root = Directionality::new(
        TextDirection::Ltr,
        FocusTraversalGroup::new(FocusScope::new(column).node(scope.clone())).ordered(),
    );
    let mut runtime = Runtime::new(root.into()).unwrap();
    frame(&mut runtime);
    assert!(!first.has_focus() && !second.has_focus());
    assert!(runtime.next_focus_from(&second));
    frame(&mut runtime);
    assert!(!first.has_focus());
    assert!(second.has_primary_focus());
    assert!(scope.scope_focus_node().has_focus());
}

fn cycle(ordered: Option<bool>) {
    let nodes: [FocusNode; 10] = std::array::from_fn(|_| FocusNode::new());
    let row = Row::new(nodes.iter().enumerate().map(|(index, node)| {
        let focus = item(node, 10.);
        match ordered {
            Some(true) => Widget::from(FocusTraversalOrder::new(focus).order((10 - index) as f64)),
            Some(false) => Widget::from(
                FocusTraversalOrder::new(focus).lexical(((b'J' - index as u8) as char).to_string()),
            ),
            None => Widget::from(focus),
        }
    }));
    let root = Directionality::new(
        if ordered.is_some() {
            TextDirection::Ltr
        } else {
            TextDirection::Rtl
        },
        FocusTraversalGroup::new(FocusScope::new(row)).ordered_with_widget_order(),
    );
    let mut runtime = Runtime::new(root.into()).unwrap();
    frame(&mut runtime);

    let ordered_index = |index| if ordered.is_some() { 9 - index } else { index };
    for index in 0..10 {
        assert!(runtime.next_focus_from(&nodes[0]));
        frame(&mut runtime);
        assert!(nodes[ordered_index(index)].has_primary_focus());
    }
    for index in (0..9).rev() {
        assert!(runtime.previous_focus_from(&nodes[0]));
        frame(&mut runtime);
        assert!(nodes[ordered_index(index)].has_primary_focus());
    }
}

#[test]
fn absent_orders_use_the_selected_widget_fallback_even_in_rtl() {
    cycle(None);
}

#[test]
fn numeric_orders_advance_and_reverse_from_the_current_primary_node() {
    cycle(Some(true));
}

#[test]
fn lexical_orders_advance_and_reverse_from_the_current_primary_node() {
    cycle(Some(false));
}

#[test]
fn widget_fallback_differs_from_reading_order_for_equal_or_absent_orders() {
    for equal_orders in [false, true] {
        for use_widget_order in [false, true] {
            let [first, second] = std::array::from_fn(|_| FocusNode::new());
            let widget = |node: &FocusNode, left| {
                let item = item(node, 20.);
                let item = if equal_orders {
                    Widget::from(FocusTraversalOrder::new(item).order(1.))
                } else {
                    Widget::from(item)
                };
                Positioned::new(item).left(left).top(0.)
            };
            let group = FocusTraversalGroup::new(
                Stack::new([widget(&first, 10.), widget(&second, 100.)])
                    .alignment(Alignment::TOP_LEFT),
            );
            let root = Directionality::new(
                TextDirection::Rtl,
                if use_widget_order {
                    group.ordered_with_widget_order()
                } else {
                    group.ordered()
                },
            );
            let mut runtime = Runtime::new(root.into()).unwrap();
            frame(&mut runtime);
            assert!(runtime.next_focus_from(&first));
            frame(&mut runtime);
            assert_eq!(first.has_primary_focus(), use_widget_order);
            assert_eq!(second.has_primary_focus(), !use_widget_order);
        }
    }
}

#[test]
fn a_traversal_group_uses_its_own_order_not_its_members_orders() {
    let nodes: [FocusNode; 10] = std::array::from_fn(|_| FocusNode::new());
    let numbers = |indices: [usize; 3]| {
        Row::new(indices.map(|id| FocusTraversalOrder::new(item(&nodes[id], 10.)).order(id as f64)))
    };
    let widget_group = FocusTraversalGroup::new(numbers([9, 8, 7])).widget_order();
    let numeric_group = FocusTraversalGroup::new(numbers([4, 5, 6])).ordered_with_widget_order();
    let lexical_group = FocusTraversalGroup::new(Row::new([3, 2, 1, 0].map(|index| {
        FocusTraversalOrder::new(item(&nodes[index], 10.))
            .lexical(((b'A' + index as u8) as char).to_string())
    })))
    .ordered_with_widget_order();
    let inner = FocusTraversalGroup::new(Row::new([
        FocusTraversalOrder::new(widget_group).order(0.),
        FocusTraversalOrder::new(numeric_group).order(1.),
        FocusTraversalOrder::new(lexical_group).order(2.),
    ]))
    .ordered_with_widget_order();
    let outer = FocusTraversalGroup::new(FocusScope::new(inner)).widget_order();
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, outer).into()).unwrap();
    frame(&mut runtime);
    for expected in [9, 8, 7, 4, 5, 6, 0, 1, 2, 3] {
        assert!(runtime.next_focus_from(&nodes[0]));
        frame(&mut runtime);
        assert_eq!(
            nodes.iter().position(FocusNode::has_primary_focus),
            Some(expected)
        );
    }
}
