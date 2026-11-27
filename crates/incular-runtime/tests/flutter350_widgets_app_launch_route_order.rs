use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{RouteInformation, Text, WidgetsApp};

fn mount(app: WidgetsApp, expected: &str) -> Runtime {
    let mut runtime = Runtime::new(app.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    assert!(
        runtime
            .tree()
            .semantics()
            .iter()
            .any(|(_, node)| node.label.as_deref() == Some(expected)),
        "expected mounted content {expected}",
    );
    runtime
}

#[test]
fn a_non_root_platform_launch_route_has_precedence_over_the_apps_default() {
    for (host, expected) in [("/", "/welcome"), ("/inbox/thread", "/inbox/thread")] {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let log = calls.clone();
        let app = WidgetsApp::empty()
            .initial_route("/welcome")
            .on_generate_route(|_| None)
            .on_generate_initial_routes(move |name| {
                log.borrow_mut().push(name.to_owned());
                vec![(
                    RouteInformation::new(name),
                    Text::new(format!("opened: {name}")).into(),
                )]
            });
        app.controller()
            .route_information_provider()
            .set_value(host);
        let _runtime = mount(app, &format!("opened: {expected}"));
        assert_eq!(&*calls.borrow(), &[expected]);
    }
}

#[test]
fn the_default_startup_factory_requests_prefixes_before_the_leaf() {
    for exists in [true, false] {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let log = calls.clone();
        // Do not install home, so the ordinary generator sees root requests.
        let app = WidgetsApp::empty()
            .initial_route("/a/b")
            .on_generate_route(move |info| {
                let route = info.location();
                log.borrow_mut().push(route.to_owned());
                (exists || route != "/a/b").then(|| Text::new(format!("generated: {route}")).into())
            });
        app.controller().clear_home();
        let controller = app.controller();
        let _runtime = mount(
            app,
            if exists {
                "generated: /a/b"
            } else {
                "generated: /"
            },
        );
        if exists {
            assert_eq!(&*calls.borrow(), &["/", "/a", "/a/b"]);
            assert_eq!(controller.route_stack().len(), 3);
        } else {
            assert_eq!(&*calls.borrow(), &["/", "/a", "/a/b", "/"]);
            assert_eq!(controller.route_stack().len(), 1);
        }
    }
}
