//! FocusManager lifecycle comparisons against Flutter 3.50.0. These drive
//! application lifecycle events, not native per-window focus notifications.

use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_platform::{PlatformEvent, PlatformLifecycle};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_widgets::{Column, Focus, FocusNode, FocusScope, SizedBox, Widget};

const DESKTOP: [TextShortcutPlatform; 3] = [
    TextShortcutPlatform::Linux,
    TextShortcutPlatform::MacOs,
    TextShortcutPlatform::Windows,
];

fn tree(nodes: &[&FocusNode]) -> Widget {
    FocusScope::new(Column::new(
        nodes
            .iter()
            .map(|node| Focus::new(SizedBox::shrink()).node((*node).clone()).into())
            .collect::<Vec<Widget>>(),
    ))
    .into()
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(100.0, 100.0)))
        .unwrap();
}

fn lifecycle(runtime: &mut Runtime, event: PlatformLifecycle) {
    let _ = runtime.handle_platform_event(PlatformEvent::Lifecycle(event));
}

fn start(platform: TextShortcutPlatform, nodes: &[&FocusNode]) -> Runtime {
    let mut runtime = Runtime::new(tree(nodes)).unwrap();
    runtime.set_text_shortcut_platform(platform);
    lifecycle(&mut runtime, PlatformLifecycle::Resumed);
    nodes[0].request_focus();
    frame(&mut runtime);
    assert!(nodes[0].has_primary_focus());
    runtime
}

#[test]
fn mobile_retains_focus_through_suspend_and_resume() {
    for platform in [TextShortcutPlatform::Android, TextShortcutPlatform::Ios] {
        let node = FocusNode::new();
        let mut runtime = start(platform, &[&node]);
        lifecycle(&mut runtime, PlatformLifecycle::Suspended);
        assert!(node.has_primary_focus());
        lifecycle(&mut runtime, PlatformLifecycle::Resumed);
        assert!(node.has_primary_focus());
    }
}

#[test]
fn desktop_parks_focus_on_suspend_and_restores_it_on_resume() {
    for platform in DESKTOP {
        let node = FocusNode::new();
        let mut runtime = start(platform, &[&node]);
        lifecycle(&mut runtime, PlatformLifecycle::Suspended);
        assert!(!node.has_primary_focus(), "{platform:?} did not park focus");
        assert!(runtime.focused_element().is_none());
        lifecycle(&mut runtime, PlatformLifecycle::Resumed);
        assert!(node.has_primary_focus());
    }
}

#[test]
fn a_node_detached_while_suspended_is_not_restored() {
    let node = FocusNode::new();
    let mut runtime = start(TextShortcutPlatform::Linux, &[&node]);
    lifecycle(&mut runtime, PlatformLifecycle::Suspended);
    runtime
        .schedule_update(runtime.tree().root().unwrap(), tree(&[]))
        .unwrap();
    frame(&mut runtime);
    assert!(!node.has_primary_focus());
    lifecycle(&mut runtime, PlatformLifecycle::Resumed);
    assert!(!node.has_primary_focus());
    assert!(runtime.focused_element().is_none());
}

#[test]
fn desktop_respects_a_new_focus_request_queued_before_resume() {
    for platform in DESKTOP {
        let a = FocusNode::new();
        let b = FocusNode::new();
        let mut runtime = start(platform, &[&a, &b]);
        lifecycle(&mut runtime, PlatformLifecycle::Inactive);
        assert!(!a.has_primary_focus(), "{platform:?} did not park focus");
        let regained = Rc::new(Cell::new(false));
        let _subscription = a.observe({
            let a = a.clone();
            let regained = regained.clone();
            move || regained.set(regained.get() || a.has_primary_focus())
        });
        b.request_focus();
        lifecycle(&mut runtime, PlatformLifecycle::Resumed);
        frame(&mut runtime);
        assert!(b.has_primary_focus());
        assert!(!a.has_primary_focus());
        assert!(
            !regained.get(),
            "parked focus must not be restored transiently"
        );
    }
}

#[test]
fn a_different_node_on_the_retained_element_does_not_inherit_parked_focus() {
    let old = FocusNode::new();
    let replacement = FocusNode::new();
    let mut runtime = start(TextShortcutPlatform::Linux, &[&old]);
    let retained = runtime.focused_element().unwrap();
    lifecycle(&mut runtime, PlatformLifecycle::Inactive);
    assert!(!old.has_focus());
    runtime
        .schedule_update(runtime.tree().root().unwrap(), tree(&[&replacement]))
        .unwrap();
    frame(&mut runtime);
    assert_eq!(
        runtime.tree().focus_node_at(retained),
        Some(replacement.clone())
    );
    lifecycle(&mut runtime, PlatformLifecycle::Resumed);
    assert!(!old.has_focus() && !replacement.has_focus());
    assert!(runtime.focused_element().is_none());
}
