use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{Text, WidgetsApp};

fn draw(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn has_text(runtime: &Runtime, text: &str) -> bool {
    runtime
        .tree()
        .semantics()
        .iter()
        .any(|(_, node)| node.label.as_deref() == Some(text))
}

#[test]
fn an_unknown_named_route_is_generated_once_and_displays_its_fallback() {
    let generated = Rc::new(RefCell::new(Vec::new()));
    let unknown = Rc::new(RefCell::new(Vec::new()));
    let generator_calls = generated.clone();
    let fallback_calls = unknown.clone();

    let app = WidgetsApp::new(Text::new("home"))
        .on_generate_route(move |settings| {
            generator_calls
                .borrow_mut()
                .push(settings.location().to_owned());
            None
        })
        .on_unknown_route(move |settings| {
            fallback_calls
                .borrow_mut()
                .push(settings.location().to_owned());
            Text::new(format!("fallback: {}", settings.location())).into()
        });
    let controller = app.controller();
    let mut runtime = Runtime::new(app.into()).unwrap();
    draw(&mut runtime);
    assert!(has_text(&runtime, "home"));
    assert!(generated.borrow().is_empty());
    assert!(unknown.borrow().is_empty());

    controller.push_route("/unlisted").unwrap();
    draw(&mut runtime);
    assert!(has_text(&runtime, "fallback: /unlisted"));
    assert_eq!(&*generated.borrow(), &["/unlisted"]);
    assert_eq!(&*unknown.borrow(), &["/unlisted"]);

    draw(&mut runtime);
    assert_eq!(&*generated.borrow(), &["/unlisted"]);
    assert_eq!(&*unknown.borrow(), &["/unlisted"]);
    assert!(controller.dispatch_back());
    draw(&mut runtime);
    assert!(has_text(&runtime, "home"));
    assert!(!has_text(&runtime, "fallback: /unlisted"));
    assert_eq!(&*generated.borrow(), &["/unlisted"]);
    assert_eq!(&*unknown.borrow(), &["/unlisted"]);
}

#[test]
fn platform_route_switch_releases_the_stack_before_notifying_observers() {
    let app = WidgetsApp::new(Text::new("home")).route("/target", || Text::new("target").into());
    let controller = app.controller();
    let known = controller.clone();
    let reported = Rc::new(RefCell::new(Vec::new()));
    let log = reported.clone();
    let _subscription = controller.on_navigation_notification(move |notice| {
        assert_eq!(
            notice.route_information.as_ref(),
            Some(&known.current_route()),
        );
        log.borrow_mut()
            .push(known.current_route().location().to_owned());
    });
    let mut runtime = Runtime::new(app.into()).unwrap();
    draw(&mut runtime);

    controller.set_route_information("/target").unwrap();
    draw(&mut runtime);
    assert!(has_text(&runtime, "target"));
    assert_eq!(&*reported.borrow(), &["/target"]);
}

#[test]
fn refusing_an_unlisted_route_does_not_mutate_the_active_page_or_stack() {
    let app = WidgetsApp::new(Text::new("home")).on_generate_route(|_| None);
    let controller = app.controller();
    let mut runtime = Runtime::new(app.into()).unwrap();
    draw(&mut runtime);
    assert!(controller.push_route("/unlisted").is_err());
    draw(&mut runtime);
    assert_eq!(controller.route_stack().len(), 1);
    assert_eq!(controller.current_route().location(), "/");
    assert!(has_text(&runtime, "home"));
}
