use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{RouteInformation, Text, WidgetsApp};

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
fn a_custom_startup_stack_bypasses_the_named_factory_and_pops_back_to_its_first_page() {
    let initial = Rc::new(RefCell::new(Vec::new()));
    let calls = initial.clone();
    let app = WidgetsApp::empty()
        .initial_route("/abc")
        .on_generate_route(|_| Some(Text::new("regular page").into()))
        .on_generate_initial_routes(move |name| {
            calls.borrow_mut().push(name.to_owned());
            vec![
                (
                    RouteInformation::default(),
                    Text::new("non-regular page one").into(),
                ),
                (
                    RouteInformation::default(),
                    Text::new("non-regular page two").into(),
                ),
            ]
        });
    let controller = app.controller();
    let mut runtime = Runtime::new(app.into()).unwrap();
    draw(&mut runtime);
    assert_eq!(&*initial.borrow(), &["/abc"]);
    assert_eq!(controller.route_stack().len(), 2);
    assert!(has_text(&runtime, "non-regular page two"));
    assert!(!has_text(&runtime, "non-regular page one"));
    assert!(!has_text(&runtime, "regular page"));

    assert!(controller.dispatch_back());
    draw(&mut runtime);
    assert_eq!(controller.route_stack().len(), 1);
    assert!(has_text(&runtime, "non-regular page one"));
    assert!(!has_text(&runtime, "non-regular page two"));
    assert!(!has_text(&runtime, "regular page"));
    assert_eq!(&*initial.borrow(), &["/abc"]);
}

#[test]
fn subsequent_platform_deep_links_use_named_routes_without_regenerating_startup() {
    let count = Rc::new(std::cell::Cell::new(0));
    let calls = count.clone();
    let app = WidgetsApp::new(Text::new("home"))
        .route("/regular", || Text::new("regular page").into())
        .on_generate_initial_routes(move |_| {
            calls.set(calls.get() + 1);
            vec![(RouteInformation::new("/custom"), Text::new("custom").into())]
        });
    let controller = app.controller();
    let mut runtime = Runtime::new(app.into()).unwrap();
    draw(&mut runtime);
    assert!(has_text(&runtime, "custom"));
    assert_eq!(count.get(), 1);

    controller.set_route_information("/regular").unwrap();
    draw(&mut runtime);
    assert!(has_text(&runtime, "regular page"));
    assert!(!has_text(&runtime, "custom"));
    assert_eq!(count.get(), 1);
}

#[test]
fn a_restored_startup_prefix_keeps_the_right_generated_page_even_with_duplicate_names() {
    use incular_core::{RestorationBackend, RestorationKey, RestorationScope};
    use std::collections::HashMap;

    #[derive(Default)]
    struct Storage(RefCell<HashMap<Vec<RestorationKey>, serde_json::Value>>);

    impl RestorationBackend for Storage {
        fn read_value(&self, path: &[RestorationKey]) -> Option<serde_json::Value> {
            self.0.borrow().get(path).cloned()
        }

        fn write_value(&self, path: &[RestorationKey], value: serde_json::Value) {
            self.0.borrow_mut().insert(path.to_vec(), value);
        }

        fn remove_value(&self, path: &[RestorationKey]) {
            self.0.borrow_mut().remove(path);
        }
    }

    let storage = Rc::new(Storage::default());
    let make_app = || {
        let app = WidgetsApp::empty().on_generate_initial_routes(|_| {
            vec![
                (RouteInformation::new("/same"), Text::new("first").into()),
                (RouteInformation::new("/same"), Text::new("second").into()),
            ]
        });
        app.controller().set_restoration_scope(
            RestorationScope::root(storage.clone()),
            RestorationKey::new("startup").unwrap(),
        );
        app
    };

    {
        let app = make_app();
        let controller = app.controller();
        let mut runtime = Runtime::new(app.into()).unwrap();
        draw(&mut runtime);
        assert!(has_text(&runtime, "second"));
        assert!(controller.dispatch_back());
        draw(&mut runtime);
        assert!(has_text(&runtime, "first"));
    }

    let app = make_app();
    let restored = app.controller();
    let mut runtime = Runtime::new(app.into()).unwrap();
    draw(&mut runtime);
    assert!(has_text(&runtime, "first"));
    assert!(!has_text(&runtime, "second"));
    assert_eq!(restored.route_stack().len(), 1);
}
