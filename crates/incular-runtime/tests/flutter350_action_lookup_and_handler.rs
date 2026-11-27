use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{Action, ActionResult, ActionScope, Actions, Intent, SizedBox, Widget};

const CHILD: u64 = 932;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum Command {
    Run,
    Missing,
}

fn scene(action: Option<Action<Command>>) -> Widget {
    let child = Widget::from(SizedBox::new().width(30.).height(30.)).with_key(CHILD);
    let mut registry = Actions::<Command>::typed();
    if let Some(action) = action {
        registry.register_action(action);
    }
    let inner = ActionScope::new(Rc::new(Actions::<Command>::typed()), child);
    ActionScope::new(Rc::new(registry), inner).into()
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
}

fn action(calls: &Rc<Cell<usize>>) -> Action<Command> {
    let calls = calls.clone();
    Action::<Command>::with_handler(Command::Run, move |intent| {
        assert_eq!(intent.command(), &Command::Run);
        calls.set(calls.get() + 1);
        true
    })
}

fn intent(command: Command) -> Intent<Command> {
    Intent::new(command)
}

#[test]
fn required_and_optional_lookup_read_the_same_ancestor_mapping_over_rebuilds() {
    let calls = Rc::new(Cell::new(0));
    let original = action(&calls);
    let mut runtime = Runtime::new(scene(Some(original.clone()))).unwrap();
    let root = runtime.tree().root().unwrap();
    frame(&mut runtime);
    let child = runtime.tree().element_with_key(&CHILD.into()).unwrap();
    for _ in 0..2 {
        let found = runtime
            .tree()
            .find_action_at(child, &intent(Command::Run))
            .unwrap();
        assert_eq!(found.command(), &Command::Run);
        original.set_enabled(false);
        assert!(!found.is_enabled());
        original.set_enabled(true);
        assert!(found.is_enabled());

        let missing = intent(Command::Missing);
        assert!(
            runtime
                .tree()
                .maybe_find_action_at(child, &missing)
                .is_none()
        );
        let error = runtime.tree().find_action_at(child, &missing).unwrap_err();
        assert_eq!(error.command_type, std::any::type_name::<Command>());
        assert_eq!(calls.get(), 0);
        runtime
            .schedule_update(root, scene(Some(original.clone())))
            .unwrap();
        frame(&mut runtime);
        assert_eq!(runtime.tree().element_with_key(&CHILD.into()), Some(child));
    }
}

#[test]
fn an_action_handler_forwards_the_typed_intent_and_rechecks_enabled() {
    let calls = Rc::new(Cell::new(0));
    let original = action(&calls);
    let mut runtime = Runtime::new(scene(Some(original.clone()))).unwrap();
    frame(&mut runtime);
    let child = runtime.tree().element_with_key(&CHILD.into()).unwrap();
    let callback = runtime
        .tree()
        .action_handler_at(child, &intent(Command::Run))
        .unwrap();
    assert_eq!(calls.get(), 0);
    callback();
    assert_eq!(calls.get(), 1);
    original.set_enabled(false);
    callback();
    assert_eq!(calls.get(), 1);
    assert_eq!(
        original.invoke(&intent(Command::Run)),
        ActionResult::Ignored
    );
}

#[test]
fn disabled_mapping_has_no_action_handler() {
    let original = action(&Rc::new(Cell::new(0)));
    original.set_enabled(false);
    let mut runtime = Runtime::new(scene(Some(original))).unwrap();
    frame(&mut runtime);
    let child = runtime.tree().element_with_key(&CHILD.into()).unwrap();
    assert!(
        runtime
            .tree()
            .action_handler_at(child, &intent(Command::Run))
            .is_none()
    );
}

#[test]
fn absent_mapping_has_no_action_handler() {
    let mut runtime = Runtime::new(scene(None)).unwrap();
    frame(&mut runtime);
    let child = runtime.tree().element_with_key(&CHILD.into()).unwrap();
    assert!(
        runtime
            .tree()
            .action_handler_at(child, &intent(Command::Run))
            .is_none()
    );
}
