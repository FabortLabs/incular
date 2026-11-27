use std::{
    cell::Cell,
    convert::Infallible,
    future::{Future, ready},
    pin::pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use incular_core::{Offset, Rect, Size};
use incular_platform::{
    FlutterPlatformMessenger, SystemContextMenuCommand as Command,
    SystemContextMenuController as Controller, SystemContextMenuError as Error,
    SystemContextMenuItem as Item, SystemContextMenus as Menus, TextInputClientId,
    message_codecs::{JsonMethodCodec, MethodCall},
};
use serde_json::{Value, json};

const CLIENT: Option<TextInputClientId> = Some(TextInputClientId::new(1));

#[derive(Default)]
struct Wire(Vec<MethodCall<Value>>);

impl FlutterPlatformMessenger for Wire {
    type Error = Infallible;

    fn send_platform_message(
        &mut self,
        channel: &'static str,
        bytes: Vec<u8>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>> {
        assert_eq!(channel, "flutter/platform");
        self.0
            .push(JsonMethodCodec::decode_method_call(&bytes).unwrap());
        ready(Ok(None))
    }
}

impl Wire {
    fn send(&mut self, command: Option<Command>) {
        if let Some(command) = command {
            let mut cx = Context::from_waker(Waker::noop());
            assert!(matches!(
                pin!(command.send(self)).poll(&mut cx),
                Poll::Ready(Ok(()))
            ));
        }
    }

    fn expect(&mut self, method: &str, arguments: Value) {
        assert_eq!(
            std::mem::take(&mut self.0),
            [MethodCall {
                method: method.into(),
                arguments
            }]
        );
    }
}

fn rect(x: f32, length: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, x), Size::new(length, length))
}

fn show_args(x: f32, length: f32, types: Value) -> Value {
    json!({
        "targetRect": {"x": x, "y": x, "width": length, "height": length},
        "items": types,
    })
}

fn item(kind: &str) -> Value {
    json!({"type": kind, "callbackId": 0})
}

#[test]
fn showing_and_hiding_one_controller_is_idempotent() {
    let (mut menus, mut wire) = (Menus::default(), Wire::default());
    let controller = Controller::new();
    let first = rect(0.0, 100.0);
    let next = rect(1.0, 200.0);
    assert!(!menus.is_visible(&controller));
    assert!(wire.0.is_empty());

    for (target, x, width) in [(first, 0.0, 100.0), (next, 1.0, 200.0)] {
        let command = menus
            .show_with_items(&controller, CLIENT, target, vec![Item::Copy])
            .unwrap();
        assert!(menus.is_visible(&controller));
        wire.send(command);
        wire.expect(
            "ContextMenu.showSystemContextMenu",
            show_args(x, width, json!([item("copy")])),
        );
        assert!(
            menus
                .show_with_items(&controller, CLIENT, target, vec![Item::Copy])
                .unwrap()
                .is_none()
        );
        assert!(menus.is_visible(&controller));
        assert!(wire.0.is_empty());
    }

    wire.send(menus.hide(&controller));
    assert!(!menus.is_visible(&controller));
    wire.expect("ContextMenu.hideSystemContextMenu", Value::Null);
    assert!(menus.hide(&controller).is_none());
    assert!(wire.0.is_empty());

    wire.send(
        menus
            .show_with_items(&controller, CLIENT, next, vec![Item::Copy])
            .unwrap(),
    );
    assert!(menus.is_visible(&controller));
    wire.expect(
        "ContextMenu.showSystemContextMenu",
        show_args(1.0, 200.0, json!([item("copy")])),
    );
    wire.send(menus.hide(&controller));
    assert!(!menus.is_visible(&controller));
    wire.expect("ContextMenu.hideSystemContextMenu", Value::Null);
}

#[test]
fn platform_dismissal_clears_visibility_without_echoing_hide() {
    let (mut menus, mut wire) = (Menus::default(), Wire::default());
    let count = Rc::new(Cell::new(0));
    let controller = Controller::with_system_hide_handler({
        let count = count.clone();
        move || count.set(count.get() + 1)
    });
    assert_eq!(count.get(), 0);
    assert!(!menus.is_visible(&controller));

    wire.send(
        menus
            .show_with_items(&controller, CLIENT, rect(0.0, 100.0), vec![Item::Copy])
            .unwrap(),
    );
    wire.expect(
        "ContextMenu.showSystemContextMenu",
        show_args(0.0, 100.0, json!([item("copy")])),
    );
    assert!(menus.is_visible(&controller));
    assert!(
        menus
            .handle_platform_message(br#"{"method":"ContextMenu.onDismissSystemContextMenu"}"#)
            .unwrap()
    );
    assert_eq!(count.get(), 1);
    assert!(!menus.is_visible(&controller));
    assert!(menus.hide(&controller).is_none());
    assert!(wire.0.is_empty());
}

#[test]
fn a_second_controller_cannot_replace_a_visible_distinct_menu() {
    let mut menus = Menus::default();
    let first = Controller::new();
    let second = Controller::new();
    let mut wire = Wire::default();
    assert!(!menus.is_visible(&first));
    let target = rect(0.0, 100.0);
    wire.send(
        menus
            .show_with_items(&first, CLIENT, target, vec![Item::Copy])
            .unwrap(),
    );
    assert!(menus.is_visible(&first));
    assert!(!menus.is_visible(&second));
    assert!(matches!(
        menus.show_with_items(&second, CLIENT, rect(1.0, 200.0), vec![Item::Copy]),
        Err(Error::AlreadyVisible)
    ));
    assert!(menus.is_visible(&first));
    assert!(!menus.is_visible(&second));
    assert!(menus.hide(&first).is_some());
    assert!(!menus.is_visible(&first));
    assert!(!menus.is_visible(&second));
}

#[test]
fn one_controller_cannot_hide_the_other_and_visibility_can_transfer() {
    let (mut menus, mut wire) = (Menus::default(), Wire::default());
    let first = Controller::new();
    let second = Controller::new();
    for (owner, other, x, width) in [(&first, &second, 0.0, 100.0), (&second, &first, 1.0, 200.0)] {
        wire.send(
            menus
                .show_with_items(owner, CLIENT, rect(x, width), vec![Item::Copy])
                .unwrap(),
        );
        assert!(menus.is_visible(owner));
        assert!(!menus.is_visible(other));
        wire.expect(
            "ContextMenu.showSystemContextMenu",
            show_args(x, width, json!([item("copy")])),
        );
        assert!(menus.hide(other).is_none());
        assert!(menus.is_visible(owner));
        assert!(wire.0.is_empty());
        wire.send(menus.hide(owner));
        assert!(!menus.is_visible(owner));
        assert!(!menus.is_visible(other));
        wire.expect("ContextMenu.hideSystemContextMenu", Value::Null);
    }
}

#[test]
fn changing_items_republishes_exact_order_and_localized_title() {
    let (mut menus, mut wire) = (Menus::default(), Wire::default());
    let controller = Controller::new();
    let target = rect(0.0, 100.0);
    let first = vec![
        Item::Cut,
        Item::Copy,
        Item::Paste,
        Item::SelectAll,
        Item::SearchWeb("Special Search".into()),
    ];
    wire.send(
        menus
            .show_with_items(&controller, CLIENT, target, first.clone())
            .unwrap(),
    );
    assert!(menus.is_visible(&controller));
    wire.expect(
        "ContextMenu.showSystemContextMenu",
        show_args(
            0.0,
            100.0,
            json!([
                item("cut"), item("copy"), item("paste"), item("selectAll"),
                {"type": "searchWeb", "callbackId": 0, "title": "Special Search"},
            ]),
        ),
    );
    assert!(
        menus
            .show_with_items(&controller, CLIENT, target, first)
            .unwrap()
            .is_none()
    );
    assert!(menus.is_visible(&controller));
    wire.send(
        menus
            .show_with_items(&controller, CLIENT, target, vec![Item::Cut])
            .unwrap(),
    );
    wire.expect(
        "ContextMenu.showSystemContextMenu",
        show_args(0.0, 100.0, json!([item("cut")])),
    );
    wire.send(menus.hide(&controller));
    assert!(!menus.is_visible(&controller));
}

#[test]
fn empty_menu_and_inactive_text_client_do_not_mutate_visibility() {
    let mut menus = Menus::default();
    let controller = Controller::new();
    let target = rect(0.0, 100.0);
    assert!(!menus.is_visible(&controller));
    assert!(matches!(
        menus.show_with_items(&controller, CLIENT, target, vec![]),
        Err(Error::EmptyItems)
    ));
    assert!(!menus.is_visible(&controller));
    assert!(matches!(
        menus.show_with_items(&controller, None, target, vec![Item::Copy]),
        Err(Error::NoTextInput)
    ));
    assert!(!menus.is_visible(&controller));
    assert!(menus.hide(&controller).is_none());
}

fn custom(title: &str) -> (Item, Rc<Cell<usize>>) {
    let count = Rc::new(Cell::new(0));
    (
        Item::Custom {
            title: title.into(),
            on_pressed: Rc::new({
                let count = count.clone();
                move || count.set(count.get() + 1)
            }),
        },
        count,
    )
}

fn custom_ids(
    menus: &mut Menus,
    owner: &Controller,
    target: Rect,
    items: Vec<Item>,
    titles: &[&str],
) -> Vec<String> {
    let mut wire = Wire::default();
    wire.send(menus.show_with_items(owner, CLIENT, target, items).unwrap());
    let [call] = &wire.0[..] else {
        panic!("expected one platform call to show the menu");
    };
    assert_eq!(call.method, "ContextMenu.showSystemContextMenu");
    let encoded = call.arguments["items"].as_array().unwrap();
    assert_eq!(encoded.len(), titles.len());
    encoded
        .iter()
        .zip(titles)
        .map(|(item, title)| {
            assert_eq!(item["type"], "custom");
            assert_eq!(item["title"], *title);
            item["id"].as_str().unwrap().to_owned()
        })
        .collect()
}

fn click(menus: &mut Menus, id: &str) -> Result<bool, Error> {
    menus.handle_platform_message(
        &serde_json::to_vec(&json!({
            "method": "ContextMenu.onPerformCustomAction",
            "args": [1, id],
        }))
        .unwrap(),
    )
}

#[test]
fn custom_callbacks_route_independently_and_hide_clears_both() {
    let mut menus = Menus::default();
    let owner = Controller::new();
    let (action_one, one) = custom("Action 1");
    let (action_two, two) = custom("Action 2");
    let ids = custom_ids(
        &mut menus,
        &owner,
        rect(0.0, 100.0),
        vec![action_one, action_two],
        &["Action 1", "Action 2"],
    );
    assert!(menus.is_visible(&owner));
    assert_ne!(ids[0], ids[1]);
    assert!(click(&mut menus, &ids[0]).unwrap());
    assert_eq!((one.get(), two.get()), (1, 0));
    assert!(click(&mut menus, &ids[1]).unwrap());
    assert_eq!((one.get(), two.get()), (1, 1));
    let mut wire = Wire::default();
    wire.send(menus.hide(&owner));
    wire.expect("ContextMenu.hideSystemContextMenu", Value::Null);
    assert!(!menus.is_visible(&owner));
    for id in ids {
        assert!(matches!(click(&mut menus, &id), Err(Error::NoVisibleMenu)));
    }
    assert_eq!((one.get(), two.get()), (1, 1));
}

#[test]
fn separate_controllers_never_share_actions() {
    let mut menus = Menus::default();
    let (first, second) = (Controller::new(), Controller::new());
    let (old, old_count) = custom("Controller 1 Action");
    let (new, new_count) = custom("Controller 2 Action");
    let old_id = custom_ids(
        &mut menus,
        &first,
        rect(0.0, 100.0),
        vec![old],
        &["Controller 1 Action"],
    )
    .remove(0);
    assert!(menus.is_visible(&first));
    Wire::default().send(menus.hide(&first));
    assert!(!menus.is_visible(&first));
    let new_id = custom_ids(
        &mut menus,
        &second,
        rect(1.0, 200.0),
        vec![new],
        &["Controller 2 Action"],
    )
    .remove(0);
    assert!(menus.is_visible(&second));
    assert!(click(&mut menus, &new_id).unwrap());
    assert_eq!((old_count.get(), new_count.get()), (0, 1));
    assert!(matches!(click(&mut menus, &old_id), Err(Error::UnknownAction(id)) if id == old_id));
    assert_eq!((old_count.get(), new_count.get()), (0, 1));
}

#[test]
fn dismissed_and_absent_menus_cannot_dispatch_actions() {
    let mut menus = Menus::default();
    assert!(matches!(
        click(&mut menus, "any-id"),
        Err(Error::NoVisibleMenu)
    ));
    let owner = Controller::new();
    let (action, count) = custom("Action");
    let id = custom_ids(
        &mut menus,
        &owner,
        rect(0.0, 100.0),
        vec![action],
        &["Action"],
    )
    .remove(0);
    assert!(menus.is_visible(&owner));
    assert!(matches!(
        click(&mut menus, "non-existent-id"),
        Err(Error::UnknownAction(id)) if id == "non-existent-id"
    ));
    assert_eq!(count.get(), 0);
    assert!(
        menus
            .handle_platform_message(br#"{"method":"ContextMenu.onDismissSystemContextMenu"}"#)
            .unwrap()
    );
    assert!(!menus.is_visible(&owner));
    assert!(matches!(click(&mut menus, &id), Err(Error::NoVisibleMenu)));
    assert_eq!(count.get(), 0);
}

#[test]
fn showing_new_contents_invalidates_ids_but_an_identical_show_does_not_publish() {
    let mut menus = Menus::default();
    let owner = Controller::new();
    let target = rect(0.0, 100.0);
    let (old, old_count) = custom("Old Action");
    let (new, new_count) = custom("New Action");
    let old_id = custom_ids(
        &mut menus,
        &owner,
        target,
        vec![old.clone()],
        &["Old Action"],
    )
    .remove(0);
    assert!(menus.is_visible(&owner));
    assert!(
        menus
            .show_with_items(&owner, CLIENT, target, vec![old])
            .unwrap()
            .is_none()
    );
    let new_id = custom_ids(&mut menus, &owner, target, vec![new], &["New Action"]).remove(0);
    assert!(menus.is_visible(&owner));
    assert!(matches!(click(&mut menus, &old_id), Err(Error::UnknownAction(id)) if id == old_id));
    assert_eq!((old_count.get(), new_count.get()), (0, 0));
    assert!(click(&mut menus, &new_id).unwrap());
    assert_eq!((old_count.get(), new_count.get()), (0, 1));
}
