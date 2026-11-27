use std::{cell::RefCell, rc::Rc};

use incular_core::{Offset, Rect, Size};
use incular_platform::{
    FlutterScribbleClient, FlutterScribbleRegistry,
    message_codecs::{JsonMethodCodec, MethodCall},
};
use serde_json::{Value, json};

#[derive(Clone)]
struct Client {
    rect: Rect,
    calls: Rc<RefCell<Vec<Offset>>>,
    force_hit: bool,
}

impl FlutterScribbleClient for Client {
    fn bounds(&self) -> Rect {
        self.rect
    }

    fn is_in_scribble_rect(&self, rect: Rect) -> bool {
        self.force_hit || self.rect.intersects(rect)
    }

    fn on_scribble_focus(&self, position: Offset) {
        self.calls.borrow_mut().push(position);
    }
}

fn client(x: f32, y: f32) -> Rc<Client> {
    Rc::new(Client {
        rect: Rect::from_origin_size(Offset::new(x, y), Size::new(100., 100.)),
        calls: Rc::default(),
        force_hit: false,
    })
}

fn request(registry: &mut FlutterScribbleRegistry, method: &str, args: Value) -> Value {
    let request = JsonMethodCodec::encode_method_call(&MethodCall {
        method: format!("TextInputClient.{method}"),
        arguments: args,
    })
    .unwrap();
    let reply = registry
        .reply(&request)
        .unwrap()
        .expect("recognized Scribble method");
    JsonMethodCodec::decode_envelope(&reply).unwrap()
}

#[test]
fn native_handwriting_interaction_begin_and_finish_change_the_session_state() {
    let mut registry = FlutterScribbleRegistry::new();
    assert!(!registry.in_progress());
    assert_eq!(
        request(&mut registry, "scribbleInteractionBegan", json!([1, 0, 1])),
        Value::Null
    );
    assert!(registry.in_progress());
    assert_eq!(
        request(
            &mut registry,
            "scribbleInteractionFinished",
            json!([1, 0, 1])
        ),
        Value::Null
    );
    assert!(!registry.in_progress());
}

#[test]
fn native_focus_targets_exactly_the_registered_element() {
    let mut registry = FlutterScribbleRegistry::new();
    let target = client(0., 0.);
    let other = client(0., 0.);
    registry.register_element("target", target.clone());
    registry.register_element("other", other.clone());
    assert!(target.calls.borrow().is_empty() && other.calls.borrow().is_empty());

    assert_eq!(
        request(&mut registry, "focusElement", json!(["target", 0.0, 0.0])),
        Value::Null
    );
    assert!(registry.unregister_element("target"));
    assert!(registry.unregister_element("other"));
    assert_eq!(&*target.calls.borrow(), &[Offset::ZERO]);
    assert!(other.calls.borrow().is_empty());
    request(&mut registry, "focusElement", json!(["target", 20, 30]));
    assert_eq!(target.calls.borrow().len(), 1);
}

#[test]
fn native_query_returns_overlapping_elements_in_registration_order() {
    let mut registry = FlutterScribbleRegistry::new();
    for (id, x, y) in [
        ("target1", 0., 0.),
        ("target2", 0., 100.),
        ("other1", 100., 0.),
        ("other2", 100., 100.),
    ] {
        registry.register_element(id, client(x, y));
    }
    assert_eq!(
        request(
            &mut registry,
            "requestElementsInRect",
            json!([0.0, 50.0, 50.0, 100.0])
        ),
        json!([
            ["target1", 0.0, 0.0, 100.0, 100.0],
            ["target2", 0.0, 100.0, 100.0, 100.0],
        ]),
    );
}

#[test]
fn invalid_geometry_does_not_create_a_host_target_or_change_replacement_order() {
    let mut registry = FlutterScribbleRegistry::new();
    let invalid = |rect| {
        Rc::new(Client {
            rect,
            calls: Rc::default(),
            force_hit: true,
        })
    };
    registry.register_element("a", client(0., 0.));
    registry.register_element("b", client(1., 0.));
    registry.register_element(
        "invalid",
        invalid(Rect::from_origin_size(
            Offset::new(f32::NAN, 0.),
            Size::new(10., 10.),
        )),
    );
    registry.register_element(
        "zero",
        invalid(Rect::from_origin_size(Offset::ZERO, Size::ZERO)),
    );
    registry.register_element("a", client(2., 0.));
    assert_eq!(
        request(
            &mut registry,
            "requestElementsInRect",
            json!([0, 0, 10, 10])
        ),
        json!([["a", 2.0, 0.0, 100.0, 100.0], ["b", 1.0, 0.0, 100.0, 100.0]])
    );
}
