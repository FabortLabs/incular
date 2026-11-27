use std::{cell::Cell, rc::Rc};

use incular_config::{Constraints, NavigationMode, RuntimeEnvironment};
use incular_core::{Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    ActionResult, ActionScope, Actions, Command, FocusHighlightManager, FocusHighlightStrategy,
    FocusNode, FocusableActionDetector, Intent, LogicalShortcutKey, MediaQuery, Shortcuts,
    SizedBox, Widget,
};

#[test]
fn a_disabled_detector_can_focus_directionally_without_enabling_its_shortcuts() {
    FocusHighlightManager::new().set_strategy(FocusHighlightStrategy::AlwaysTraditional);
    let focus = FocusNode::new();
    let highlighted = Rc::new(Cell::new(false));
    let invoked = Rc::new(Cell::new(false));
    let called = invoked.clone();
    let mut actions = Actions::new();
    actions.register("run", move || called.set(true));
    let actions = Rc::new(actions);
    let parent_invoked = Rc::new(Cell::new(false));
    let parent_called = parent_invoked.clone();
    let mut parent_actions = Actions::new();
    parent_actions.register("run", move || parent_called.set(true));
    let parent_actions = Rc::new(parent_actions);
    let mut shortcuts = Shortcuts::new();
    shortcuts.bind_logical(
        LogicalShortcutKey::new(KeyboardKey::Named(NamedKey::Enter), Modifiers::empty()),
        "run",
    );
    let shortcuts = Rc::new(shortcuts);
    let scene = |enabled, navigation_mode| -> Widget {
        let callback = highlighted.clone();
        let detector = FocusableActionDetector::new(SizedBox::new().width(30.).height(30.))
            .focus_node(focus.clone())
            .enabled(enabled)
            .actions(actions.clone())
            .shortcuts(shortcuts.clone())
            .on_show_focus_highlight(move |visible| callback.set(visible));
        let media = RuntimeEnvironment {
            navigation_mode,
            ..Default::default()
        };
        MediaQuery::new(media, ActionScope::new(parent_actions.clone(), detector)).into()
    };
    let view = Constraints::tight(Size::new(800., 600.));
    let frame = |runtime: &mut Runtime| {
        runtime.run_frame(view).unwrap();
    };
    let mut runtime = Runtime::new(scene(true, NavigationMode::Traditional)).unwrap();
    let root = runtime.tree().root().unwrap();
    frame(&mut runtime);
    assert!(!highlighted.get());

    focus.request_focus();
    frame(&mut runtime);
    assert!(highlighted.get());
    for (enabled, mode, expected) in [
        (false, NavigationMode::Traditional, false),
        (false, NavigationMode::Traditional, false),
        (false, NavigationMode::Directional, true),
    ] {
        runtime.schedule_update(root, scene(enabled, mode)).unwrap();
        frame(&mut runtime);
        focus.request_focus();
        frame(&mut runtime);
        assert_eq!(highlighted.get(), expected);
        assert_eq!(focus.has_primary_focus(), expected);
    }
    runtime.dispatch_keyboard_event(KeyboardEvent::key_down(
        KeyboardKey::Named(NamedKey::Enter),
        Code::Enter,
    ));
    assert!(!parent_invoked.get());
    let detector = runtime
        .tree()
        .focusable_elements()
        .into_iter()
        .find(|&el| runtime.tree().focus_node_at(el).as_ref() == Some(&focus))
        .unwrap();
    assert_eq!(
        runtime
            .tree()
            .invoke_action_at(detector, &Intent::<Command>::new(Command::new("run"))),
        Some(ActionResult::Handled)
    );
    assert!(parent_invoked.get());
    assert!(!invoked.get());
}

#[test]
fn ambient_navigation_changes_update_the_mounted_detector_without_remounting() {
    let focus = FocusNode::new();
    let root = FocusableActionDetector::new(SizedBox::new().width(30.).height(30.))
        .focus_node(focus.clone())
        .enabled(false);
    let mut runtime = Runtime::new(root.into()).unwrap();
    let detector = runtime.tree().root().unwrap();
    let frame = |runtime: &mut Runtime| {
        runtime
            .run_frame(Constraints::tight(Size::new(100., 100.)))
            .unwrap();
    };
    assert!(
        !focus.can_request_focus(),
        "mount attaches the detector immediately"
    );
    frame(&mut runtime);
    assert!(runtime.set_environment(RuntimeEnvironment {
        navigation_mode: NavigationMode::Directional,
        ..Default::default()
    }));
    frame(&mut runtime);
    assert_eq!(runtime.tree().root(), Some(detector));
    assert!(focus.can_request_focus());
    focus.request_focus();
    frame(&mut runtime);
    assert!(focus.has_primary_focus());

    assert!(runtime.set_environment(RuntimeEnvironment::default()));
    frame(&mut runtime);
    assert_eq!(runtime.tree().root(), Some(detector));
    assert!(!focus.can_request_focus());
    assert!(!focus.has_primary_focus());
}
