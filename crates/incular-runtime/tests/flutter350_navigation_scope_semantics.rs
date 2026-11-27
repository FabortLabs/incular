use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::{
    SemanticActionKind, SemanticAnnotationFlag, SemanticNodeId, SemanticsTree,
};
use incular_widgets::{Focus, FocusNode, FocusScope, Text, Widget, WidgetsApp};

fn mount(root: impl Into<Widget>) -> Runtime {
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    runtime
}

fn label(tree: &SemanticsTree, expected: &str) -> SemanticNodeId {
    tree.iter()
        .find_map(|(id, node)| (node.label.as_deref() == Some(expected)).then_some(id))
        .unwrap()
}

fn parent(tree: &SemanticsTree, id: SemanticNodeId) -> Option<SemanticNodeId> {
    tree.iter()
        .find_map(|(parent, node)| node.children.contains(&id).then_some(parent))
}

#[test]
fn the_app_navigation_scope_keeps_its_routes_semantic_scope_distinct() {
    let runtime = mount(WidgetsApp::new(Text::new("route content")));
    let tree = runtime.tree().semantics();
    let content = label(tree, "route content");
    let route = parent(tree, content).expect("a named route keeps its own explicit children");
    let scope = parent(tree, route).expect("the route sits inside the navigation scope");
    assert_ne!(route, scope);
    let route_node = tree.node(route).unwrap();
    assert_eq!(
        route_node
            .state
            .annotation_flags
            .get(SemanticAnnotationFlag::ScopesRoute),
        Some(true),
    );
    assert!(route_node.state.explicit_child_nodes);
    let focus_scope = tree.node(scope).unwrap();
    assert!(focus_scope.state.explicit_child_nodes);
    assert!(!focus_scope.state.focusable);
    assert!(!focus_scope.actions.contains(&SemanticActionKind::Focus));
}

#[test]
fn toggling_focus_scope_semantics_does_not_remove_a_descendants_keyboard_focus() {
    let node = FocusNode::new();
    for include in [true, false] {
        let child =
            Focus::new(Text::new("input").text_direction(incular_config::TextDirection::Ltr))
                .node(node.clone())
                .autofocus(true);
        let runtime = mount(FocusScope::new(child).include_semantics(include));
        let tree = runtime.tree().semantics();
        let focused_label = label(tree, "input");
        assert!(node.has_primary_focus());
        assert_eq!(parent(tree, focused_label).is_some(), include);
        assert!(tree.node(focused_label).unwrap().state.focusable);
    }
}

fn router(
    via_config: bool,
) -> (
    Runtime,
    std::rc::Rc<std::cell::RefCell<incular_runtime::RouteOutlet>>,
    incular_navigation::Navigator,
) {
    use incular_navigation::{Navigator, Page};
    use incular_runtime::RouteOutlet;
    use incular_widgets::{BasicRouterDelegate, Column, Router, RouterConfig};
    use std::{cell::RefCell, rc::Rc};

    let mut runtime = Runtime::new(Column::new(Vec::<Widget>::new()).into()).unwrap();
    let navigator = Navigator::new();
    navigator.push_page(Page::new("route", Text::new("route content")));
    let task_scope = runtime.spawner().scope();
    let outlet = Rc::new(RefCell::new(RouteOutlet::new(&navigator, &task_scope)));
    let routing = outlet.borrow().widget();
    let delegate = BasicRouterDelegate::<String>::new(move || routing.clone());
    let router = if via_config {
        Router::from_config(RouterConfig::new(delegate))
    } else {
        Router::new(delegate)
    };
    let root = runtime.tree().root().unwrap();
    let app: Widget = WidgetsApp::router(router).into();
    runtime
        .register_builder(root, move || Column::new([app.clone()]).into())
        .unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(constraints).unwrap();
    RouteOutlet::attach(&outlet, &mut runtime).unwrap();
    RouteOutlet::present_frame(&outlet, &mut runtime, constraints).unwrap();
    (runtime, outlet, navigator)
}

fn routed_semantic_chain(runtime: &Runtime) -> (SemanticNodeId, SemanticNodeId) {
    let tree = runtime.tree().semantics();
    let content = label(tree, "route content");
    let route = parent(tree, content).unwrap();
    let outer = parent(tree, route).unwrap();
    assert_ne!(outer, route);
    for id in [route, outer] {
        let scope = tree.node(id).unwrap();
        assert!(scope.state.explicit_child_nodes, "{scope:?}");
    }
    (route, outer)
}

#[test]
fn a_router_config_keeps_the_route_semantic_boundary_under_the_navigation_scope() {
    let (runtime, _, _) = router(true);
    let _ = routed_semantic_chain(&runtime);
}

#[test]
fn adding_a_router_overlay_does_not_reparent_the_routes_semantics() {
    use incular_navigation::{OverlayEntry, Route};
    use incular_runtime::RouteOutlet;
    use incular_widgets::SizedBox;

    let (mut runtime, outlet, navigator) = router(false);
    let chain = routed_semantic_chain(&runtime);
    navigator.push(
        Route::new("overlay", SizedBox::shrink())
            .overlay(vec![OverlayEntry::new(Text::new("overlay content"))]),
    );
    RouteOutlet::present_frame(
        &outlet,
        &mut runtime,
        Constraints::tight(Size::new(800., 600.)),
    )
    .unwrap();
    assert_eq!(routed_semantic_chain(&runtime), chain);
    let tree = runtime.tree().semantics();
    let overlay = label(tree, "overlay content");
    assert_eq!(parent(tree, overlay), Some(chain.1));
}
