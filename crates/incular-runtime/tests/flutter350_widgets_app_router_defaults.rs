use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    BasicRouterDelegate, MemoryRouteInformationProvider, Router, RouterConfig, RouterError,
    SizedBox, StringRouteInformationParser, Text, WidgetsApp,
};

#[test]
fn a_direct_router_parser_uses_the_default_root_route_without_a_provider() {
    let displayed = Rc::new(RefCell::new(String::new()));
    let value = displayed.clone();
    let delegate =
        BasicRouterDelegate::<String>::new(move || Text::new(value.borrow().clone()).into());
    let paths = Rc::new(RefCell::new(Vec::new()));
    let received = paths.clone();
    delegate.on_set_new_route_path(move |path| {
        received.borrow_mut().push(path.clone());
        *displayed.borrow_mut() = path;
        Ok(())
    });
    let router =
        Router::with_default_route_information_provider(delegate, StringRouteInformationParser);
    let mut runtime = Runtime::new(WidgetsApp::router(router).into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    assert_eq!(&*paths.borrow(), &["/"]);
    assert!(
        runtime
            .tree()
            .semantics()
            .iter()
            .any(|(_, node)| node.label.as_deref() == Some("/"))
    );
}

#[test]
fn a_provider_without_a_parser_is_rejected_before_mounting() {
    let delegate = BasicRouterDelegate::<String>::new(|| SizedBox::shrink().into());
    delegate.set_configuration("initial".to_owned());
    let provider = MemoryRouteInformationProvider::new("initial");
    let result = RouterConfig::try_from_parts(Rc::new(delegate), Some(Rc::new(provider)), None);
    assert!(matches!(result, Err(RouterError::InvalidConfiguration(_))));
}
