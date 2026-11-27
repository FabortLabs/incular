use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::Constraints;
use incular_core::Size;
use incular_widgets::internal::{Widget, WidgetTree};
use incular_widgets::{
    Action, ActionListener, ActionResult, ActionScope, Actions, Builder, Intent, SizedBox,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Edit {
    First,
    Second,
    Third,
}

fn registry(actions: &[&Action<Edit>]) -> Rc<Actions<Edit>> {
    let mut registry = Actions::typed();
    for &action in actions {
        registry.register_action(action.clone());
    }
    Rc::new(registry)
}

#[test]
fn retained_scopes_and_listeners_follow_action_mapping_and_unmount() {
    let make = |command| {
        let hits = Rc::new(Cell::new(0));
        let observed = hits.clone();
        (
            Action::<Edit>::with_value(command, move |_| {
                observed.set(observed.get() + 1);
                true
            }),
            hits,
        )
    };
    let (first, first_hits) = make(Edit::First);
    let (second, second_hits) = make(Edit::Second);
    let (third, third_hits) = make(Edit::Third);
    let observed = |action: &Action<Edit>| {
        let enabled = Rc::new(Cell::new(true));
        let output = enabled.clone();
        let queried = action.clone();
        let subscription = action.add_action_listener(move || output.set(queried.is_enabled()));
        (enabled, subscription)
    };
    let (first_enabled, _first_subscription) = observed(&first);
    let (second_enabled, _second_subscription) = observed(&second);
    let (third_enabled, _third_subscription) = observed(&third);
    first.set_enabled(false);
    second.set_enabled(false);
    third.set_enabled(false);
    assert!(!first_enabled.get() && !second_enabled.get() && !third_enabled.get());

    let leaf: Widget = SizedBox::shrink().into();
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            ActionScope::new(
                registry(&[&first, &second]),
                ActionScope::new(registry(&[&third]), leaf.clone()),
            )
            .into(),
        )
        .unwrap();
    let intent = Intent::<Edit>::new;
    let nested = tree.children(tree.children(root).unwrap()[0]).unwrap()[0];
    let outcome = tree
        .invoke_action_output_at(nested, &intent(Edit::First))
        .unwrap();
    assert_eq!(outcome.result(), ActionResult::Ignored);
    assert_eq!(outcome.into_value::<bool>(), None);
    assert_eq!(first_hits.get(), 0);
    first.set_enabled(true);
    assert!(first_enabled.get());
    let outcome = tree
        .invoke_action_output_at(nested, &intent(Edit::First))
        .unwrap();
    assert_eq!(outcome.result(), ActionResult::Handled);
    assert_eq!(outcome.into_value::<bool>(), Some(true));
    assert_eq!(first_hits.get(), 1);

    let seen = Rc::new(Cell::new(None));
    let recorded = seen.clone();
    tree.update(
        root,
        ActionScope::new(
            registry(&[&first, &second]),
            ActionListener::new(
                second.clone(),
                ActionScope::new(registry(&[&third]), leaf.clone()),
            )
            .listener(move |action| recorded.set(Some(action.is_enabled()))),
        )
        .into(),
    )
    .unwrap();
    let scope = tree.children(tree.children(root).unwrap()[0]).unwrap()[0];
    let nested = tree.children(scope).unwrap()[0];
    let outcome = tree
        .invoke_action_output_at(nested, &intent(Edit::Second))
        .unwrap();
    assert_eq!(outcome.result(), ActionResult::Ignored);
    assert_eq!(outcome.into_value::<bool>(), None);
    assert_eq!(seen.get(), None);
    assert_eq!(second_hits.get(), 0);
    second.set_enabled(true);
    assert_eq!(seen.get(), Some(true));
    assert!(second_enabled.get());
    let outcome = tree
        .invoke_action_output_at(nested, &intent(Edit::Second))
        .unwrap();
    assert_eq!(outcome.result(), ActionResult::Handled);
    assert_eq!(outcome.into_value::<bool>(), Some(true));
    assert_eq!(second_hits.get(), 1);
    assert_eq!(third_hits.get(), 0);
    assert_eq!(
        (
            first.listener_count(),
            second.listener_count(),
            third.listener_count()
        ),
        (2, 3, 2)
    );

    tree.update(
        root,
        ActionScope::new(
            registry(&[&first]),
            ActionScope::new(registry(&[&third]), leaf.clone()),
        )
        .into(),
    )
    .unwrap();
    assert_eq!(
        (
            first.listener_count(),
            second.listener_count(),
            third.listener_count()
        ),
        (2, 1, 2)
    );
    tree.update(
        root,
        ActionScope::new(registry(&[&first, &third]), leaf.clone()).into(),
    )
    .unwrap();
    assert_eq!(
        (
            first.listener_count(),
            second.listener_count(),
            third.listener_count()
        ),
        (2, 1, 2)
    );
    tree.update(
        root,
        ActionScope::new(registry(&[&first]), leaf.clone()).into(),
    )
    .unwrap();
    assert_eq!(
        (
            first.listener_count(),
            second.listener_count(),
            third.listener_count()
        ),
        (2, 1, 1)
    );
    tree.mount(leaf).unwrap();
    assert_eq!(
        (
            first.listener_count(),
            second.listener_count(),
            third.listener_count()
        ),
        (1, 1, 1)
    );
}

#[test]
fn an_inherited_action_state_change_rebuilds_only_its_scope_readers() {
    let first = Action::<Edit>::new(Edit::First, || {});
    let unrelated = Action::<Edit>::new(Edit::Second, || {});
    let trace = Rc::new(RefCell::new(Vec::new()));
    let observed = trace.clone();
    let builds = Rc::new(Cell::new(0));
    let unobserved = builds.clone();
    let reader = Builder::new(move |context| {
        let action = context.maybe_find_action(&Intent::<Edit>::new(Edit::First));
        observed
            .borrow_mut()
            .push(action.map(|value| value.is_enabled()));
        SizedBox::shrink().into()
    });
    let silent = Builder::new(move |_| {
        unobserved.set(unobserved.get() + 1);
        SizedBox::shrink().into()
    });
    let child = incular_widgets::Column::new([reader, silent]);
    let mut tree = WidgetTree::new();
    tree.mount(
        ActionScope::new(
            registry(&[&first]),
            ActionScope::new(registry(&[&unrelated]), child),
        )
        .into(),
    )
    .unwrap();
    let view = Constraints::tight(Size::new(100., 100.));
    tree.layout(view).unwrap();
    assert_eq!(&*trace.borrow(), &[Some(true)]);
    assert_eq!(builds.get(), 1);

    unrelated.set_enabled(false);
    assert!(!tree.has_dirty_dependencies());
    tree.layout(view).unwrap();
    assert_eq!(&*trace.borrow(), &[Some(true)]);
    first.set_enabled(false);
    assert!(tree.has_dirty_dependencies());
    tree.layout(view).unwrap();
    assert_eq!(&*trace.borrow(), &[Some(true), Some(false)]);
    assert_eq!(builds.get(), 1);
}
