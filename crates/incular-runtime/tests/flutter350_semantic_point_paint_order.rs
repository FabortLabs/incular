use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{Offset, Size};
use incular_runtime::Runtime;
use incular_semantics::{OrdinalSortKey, SemanticAction};
use incular_widgets::{Directionality, Semantics, SizedBox, Stack, Widget};

#[test]
fn point_actions_use_inverse_paint_order_even_when_traversal_is_sorted_differently() {
    let limits = Constraints::tight(Size::new(200., 200.));
    for nested in [false, true] {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let build = |bottom_order, top_order, has_top_tap, swap_paint| {
            let make = |label, order, on_tap| {
                let descriptor = Semantics::new(SizedBox::expand())
                    .container(true)
                    .label(label)
                    .sort_key(OrdinalSortKey::new(order));
                if on_tap {
                    let calls = calls.clone();
                    descriptor.on_tap(move || calls.borrow_mut().push(label))
                } else {
                    descriptor
                }
            };
            let mut children = [
                Widget::from(make("painted first", bottom_order, true)).with_key("painted first"),
                Widget::from(make("painted last", top_order, has_top_tap)).with_key("painted last"),
            ];
            if swap_paint {
                children.swap(0, 1);
            }
            let stack = Stack::new(children);
            let content = if nested {
                Widget::from(
                    Semantics::new(stack)
                        .container(true)
                        .explicit_child_nodes(true)
                        .label("container"),
                )
            } else {
                stack.into()
            };
            Directionality::new(TextDirection::Ltr, content)
        };

        let mut runtime = Runtime::new(build(10., 0., true, false).into()).unwrap();
        let root = runtime.tree().root().unwrap();
        let center = Offset::new(100., 100.);
        for (bottom, top, tap, swap_paint, expected) in [
            (10., 0., true, false, "painted last"),
            (0., 10., true, false, "painted last"),
            (10., 0., false, false, "painted first"),
            (10., 0., true, false, "painted last"),
            (10., 0., true, true, "painted first"),
            (10., 0., true, false, "painted last"),
        ] {
            runtime
                .schedule_update(root, build(bottom, top, tap, swap_paint).into())
                .unwrap();
            runtime.run_frame(limits).unwrap();
            let tree = runtime.tree().semantics();
            let ids = if nested {
                &tree.node(tree.root().unwrap()).unwrap().children
            } else {
                tree.roots()
            };
            let labels = ids
                .iter()
                .map(|&id| tree.node(id).unwrap().label.as_deref().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(
                labels,
                if bottom < top {
                    ["painted first", "painted last"]
                } else {
                    ["painted last", "painted first"]
                }
            );
            if nested {
                let inspected = runtime.flutter_semantics_node_json(tree.root().unwrap()).unwrap();
                let id_for = |label| {
                    let id = ids.iter().find(|&&id| tree.node(id).unwrap().label.as_deref() == Some(label)).unwrap();
                    (u64::from(id.0.generation()) << 32) | u64::from(id.0.index())
                };
                let first = id_for("painted first");
                let last = id_for("painted last");
                assert_eq!(
                    inspected["childrenInTraversalOrder"],
                    if bottom < top { serde_json::json!([first, last]) } else { serde_json::json!([last, first]) },
                );
                assert_eq!(
                    inspected["childrenInHitTestOrder"],
                    if swap_paint { serde_json::json!([last, first]) } else { serde_json::json!([first, last]) },
                );
            }
            assert!(runtime.dispatch_semantic_action_at(center, SemanticAction::Activate));
            assert_eq!(
                calls.borrow_mut().pop(),
                Some(expected),
                "nested: {nested}, traversal keys: {bottom}, {top}"
            );
            assert!(calls.borrow().is_empty());
            assert!(
                !runtime
                    .dispatch_semantic_action_at(Offset::new(200., 100.), SemanticAction::Activate)
            );
        }
        runtime.set_platform_semantics_enabled(false);
        assert!(!runtime.dispatch_semantic_action_at(center, SemanticAction::Activate));
        runtime.set_platform_semantics_enabled(true);
        runtime.run_frame(limits).unwrap();
        assert!(runtime.dispatch_semantic_action_at(center, SemanticAction::Activate));
        assert_eq!(*calls.borrow(), ["painted last"]);
    }
}
