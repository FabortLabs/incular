use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    BasicRouterDelegate, MemoryRouteInformationProvider, RootBackButtonDispatcher, Router,
    RouterConfig, StringRouteInformationParser, Text, WidgetsApp,
};

#[derive(Clone, Copy)]
enum RouteWiring {
    ProviderParser,
    DelegateOnly,
    ExplicitConfiguration,
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn visible(runtime: &Runtime, label: &str) -> bool {
    runtime
        .tree()
        .semantics()
        .iter()
        .any(|(_, node)| node.label.as_deref() == Some(label))
}

fn verify_system_pop_route(wiring: RouteWiring) {
    let content = Rc::new(RefCell::new("initial".to_owned()));
    let for_build = content.clone();
    let delegate =
        BasicRouterDelegate::<String>::new(move || Text::new(for_build.borrow().clone()).into());
    let set = content.clone();
    delegate.on_set_new_route_path(move |path| {
        *set.borrow_mut() = path.trim_start_matches('/').to_owned();
        Ok(())
    });
    let pops = Rc::new(Cell::new(0));
    let called = pops.clone();
    delegate.on_pop_route(move || {
        called.set(called.get() + 1);
        *content.borrow_mut() = "popped".to_owned();
        true
    });
    let router = match wiring {
        RouteWiring::ProviderParser => Router::with_provider_parser(
            delegate,
            MemoryRouteInformationProvider::new("initial"),
            StringRouteInformationParser,
        ),
        RouteWiring::DelegateOnly => {
            delegate.set_configuration("initial".to_owned());
            Router::new(delegate)
        }
        RouteWiring::ExplicitConfiguration => Router::from_config(
            RouterConfig::with_provider_parser(
                delegate,
                MemoryRouteInformationProvider::new("initial"),
                StringRouteInformationParser,
            )
            .with_back_button_dispatcher(RootBackButtonDispatcher::new()),
        ),
    };

    let mut runtime = Runtime::new(WidgetsApp::router(router).into()).unwrap();
    frame(&mut runtime);
    assert!(visible(&runtime, "initial"));
    assert!(!visible(&runtime, "popped"));
    assert!(runtime.handle_back_button());
    frame(&mut runtime);
    assert_eq!(pops.get(), 1);
    assert!(!visible(&runtime, "initial"));
    assert!(visible(&runtime, "popped"));
}

#[test]
fn provider_and_parser_router_receives_the_system_pop_route() {
    verify_system_pop_route(RouteWiring::ProviderParser);
}

#[test]
fn delegate_only_router_handles_back_without_a_parser() {
    verify_system_pop_route(RouteWiring::DelegateOnly);
}

#[test]
fn a_router_configuration_keeps_the_supplied_root_dispatcher_live() {
    verify_system_pop_route(RouteWiring::ExplicitConfiguration);
}
