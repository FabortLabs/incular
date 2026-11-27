use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    BasicRouterDelegate, Focus, FocusNode, GlobalKey, Router, RouterConfig, SizedBox, Widget,
    WidgetsApp,
};

fn mount(app: WidgetsApp) -> Runtime {
    let mut runtime = Runtime::new(app.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    runtime
}

fn content(node: &FocusNode, autofocus: bool) -> Widget {
    Focus::new(SizedBox::square(25.))
        .node(node.clone())
        .autofocus(autofocus)
        .into()
}

fn assert_navigator_scope(node: &FocusNode) -> incular_widgets::FocusScopeNode {
    let scope = node
        .host_enclosing_scope()
        .expect("the routed focus node has an enclosing focus scope");
    assert_eq!(
        scope.scope_focus_node().debug_label().as_deref(),
        Some("Navigator Scope")
    );
    scope
}

#[test]
fn widgets_app_router_wraps_the_routing_content_in_an_autofocusing_scope() {
    let focus = FocusNode::new();
    let body = content(&focus, false);
    let delegate = BasicRouterDelegate::<String>::new(move || body.clone());
    let _runtime = mount(WidgetsApp::router(Router::new(delegate)));
    let scope = assert_navigator_scope(&focus);
    assert!(scope.has_primary_focus());
    assert!(!focus.has_primary_focus());
}

#[test]
fn widgets_app_router_config_uses_the_same_navigation_scope() {
    let focus = FocusNode::new();
    let body = content(&focus, false);
    let delegate = BasicRouterDelegate::<String>::new(move || body.clone());
    let router = Router::from_config(RouterConfig::new(delegate));
    let _runtime = mount(WidgetsApp::router(router));
    let scope = assert_navigator_scope(&focus);
    assert!(scope.has_primary_focus());
    assert!(!focus.has_primary_focus());
}

#[test]
fn the_navigator_shaped_app_uses_the_same_focus_scope() {
    let focus = FocusNode::new();
    let _runtime = mount(WidgetsApp::new(content(&focus, false)));
    let scope = assert_navigator_scope(&focus);
    assert!(scope.has_primary_focus());
    assert!(!focus.has_primary_focus());
}

#[test]
fn widgets_app_builder_receives_the_navigation_scope_not_just_the_router() {
    let body_focus = FocusNode::new();
    let scope_key = GlobalKey::new();
    let body = content(&body_focus, true);
    let delegate = BasicRouterDelegate::<String>::new(move || body.clone());
    let app = WidgetsApp::router(Router::new(delegate))
        .builder(move |routing| routing.with_key(scope_key));
    let runtime = mount(app);
    let scope_id = runtime.tree().element_with_key(&scope_key.into()).unwrap();
    let scoped = runtime
        .tree()
        .focus_node_at(scope_id)
        .expect("builder received a FocusScope");
    assert_eq!(scoped.debug_label().as_deref(), Some("Navigator Scope"));
    assert_eq!(
        body_focus
            .host_enclosing_scope()
            .unwrap()
            .scope_focus_node(),
        scoped,
    );
    assert!(body_focus.has_primary_focus());
}
