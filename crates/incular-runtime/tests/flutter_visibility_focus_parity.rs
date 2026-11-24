//! Focus-policy variants in Flutter 3.50.0 widgets/visibility_test.dart.
//! These exercise real retained keyboard input, not an editable-text model.

use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Code, InputEvent, KeyboardEvent, KeyboardKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Center, FocusNode, KeyboardListener, Offstage, Placeholder, Text, TextDirection, Visibility,
    Widget, internal::Key,
};

#[derive(Clone, Copy)]
enum Policy {
    Replace,
    Maintain,
    PreserveEverythingExceptFocus,
    PreserveFocus,
    Offstage,
}

// (same element, node focus after hiding, node focus after request,
// runtime delivery target after request, total delivered key downs)
fn after_hiding(policy: Policy) -> (bool, bool, bool, bool, u32) {
    let focus = FocusNode::new();
    let presses = Rc::new(Cell::new(0));
    let key = Key::from(1_u64);
    let build = |visible: bool| {
        let child = Widget::from(
            KeyboardListener::new(Text::new("child").text_direction(TextDirection::Ltr))
                .focus_node(focus.clone())
                .on_key_down({
                    let presses = Rc::clone(&presses);
                    move |_| presses.set(presses.get() + 1)
                }),
        )
        .with_key(key.clone());
        if matches!(policy, Policy::Offstage) {
            return Widget::from(Center::new(Offstage::new(child).offstage(!visible)));
        }
        let visibility = match policy {
            Policy::Replace => Visibility::new(child),
            Policy::Maintain => Visibility::maintain(child),
            Policy::PreserveEverythingExceptFocus => Visibility::new(child)
                .maintain_state(true)
                .maintain_animation(true)
                .maintain_interactivity(true)
                .maintain_semantics(true)
                .maintain_size(true),
            Policy::PreserveFocus => Visibility::new(child)
                .maintain_state(true)
                .maintain_focusability(true),
            Policy::Offstage => unreachable!(),
        };
        // Update the stable parent so Visibility may replace its child
        // subtree without replacing the mounted Rust root's widget type.
        Widget::from(Center::new(visibility.visible(visible)))
    };
    let frame = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(build(true)).expect("mount");
    runtime.run_frame(frame).expect("visible layout");
    let root = runtime.tree().root().expect("visibility root");
    let original = runtime
        .tree()
        .element_with_key(&key)
        .expect("mounted listener");
    let press = |runtime: &mut Runtime, value: &str, code| {
        let key = KeyboardKey::Character(value.into());
        let _ = runtime.handle_input(InputEvent::Key(KeyboardEvent::key_down(key.clone(), code)));
        let _ = runtime.handle_input(InputEvent::Key(KeyboardEvent::key_up(key, code)));
    };
    focus.request_focus();
    runtime.run_frame(frame).expect("apply focus");
    assert_eq!(runtime.focused_element(), Some(original));
    assert!(focus.has_focus());
    press(&mut runtime, "a", Code::KeyA);
    assert_eq!(presses.get(), 1, "visible key delivery control");

    runtime.tree_mut().update(root, build(false)).expect("hide");
    runtime.run_frame(frame).expect("hidden layout");
    let focus_after_hiding = focus.has_focus();
    if !matches!(
        policy,
        Policy::Maintain | Policy::PreserveFocus | Policy::Offstage
    ) {
        focus.request_focus();
        runtime
            .run_frame(frame)
            .expect("apply hidden focus request");
    }
    press(&mut runtime, "b", Code::KeyB);
    (
        runtime.tree().element_with_key(&key) == Some(original),
        focus_after_hiding,
        focus.has_focus(),
        runtime.focused_element() == Some(original),
        presses.get(),
    )
}

// visibility_test.dart:406 -- plus actual keyboard delivery.
#[test]
fn default_hidden_visibility_unmounts_and_cannot_route_keys() {
    assert_eq!(
        after_hiding(Policy::Replace),
        (false, false, false, false, 1),
    );
}

// visibility_test.dart:434 -- plus actual keyboard delivery.
#[test]
fn visibility_maintain_keeps_focus_and_key_delivery_while_hidden() {
    assert_eq!(after_hiding(Policy::Maintain), (true, true, true, true, 2));
}

// visibility_test.dart:461 -- all five non-focus preservation options, not
// the maintainState-only EditableText regression.
#[test]
fn preserving_all_non_focus_options_does_not_keep_key_delivery() {
    assert_eq!(
        after_hiding(Policy::PreserveEverythingExceptFocus),
        (true, false, false, false, 1),
    );
}

#[test]
fn hidden_focusability_is_independent_of_preserved_pointer_interactivity() {
    assert_eq!(
        after_hiding(Policy::PreserveFocus),
        (true, true, true, true, 2)
    );
}

#[test]
fn offstage_preserves_focus_and_keyboard_delivery() {
    assert_eq!(after_hiding(Policy::Offstage), (true, true, true, true, 2));
}

// visibility_test.dart:625 -- neutral zero-constraint runtime, not platform
// window resize or the TestWidgetsApp container.
#[test]
fn visibility_placeholder_can_complete_a_zero_area_frame() {
    let key = Key::from(1_u64);
    let visible = Widget::from(Visibility::new(Placeholder::new())).with_key(key.clone());
    let mut runtime = Runtime::new(Center::new(visible).into()).expect("mount");
    runtime
        .run_frame(Constraints::tight(Size::ZERO))
        .expect("zero-area frame");
    let tree = runtime.tree();
    let element = tree.element_with_key(&key).expect("visible child");
    assert_eq!(
        tree.element_bounds(element).expect("bounds").size,
        Size::ZERO
    );
}
