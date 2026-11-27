use std::{cell::RefCell, rc::Rc};

use incular_widgets::internal::{Widget, WidgetTree};
use incular_widgets::{Action, ActionListener, ActionResult, Intent, SizedBox};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Edit {
    Copy,
}

#[test]
fn state_listener_lives_on_the_mounted_element_not_the_widget_value() {
    let first = Action::new(Edit::Copy, || {});
    let second = Action::new(Edit::Copy, || {});
    let trace = Rc::new(RefCell::new(Vec::new()));
    let scene = |name, action: &Action<Edit>| -> Widget {
        let trace = trace.clone();
        ActionListener::new(action.clone(), SizedBox::shrink())
            .listener(move |action| trace.borrow_mut().push((name, action.is_enabled())))
            .into()
    };

    let shared_template = scene("first", &first);
    let mut one = WidgetTree::new();
    let mut two = WidgetTree::new();
    assert_eq!(first.listener_count(), 0);
    let root = one.mount(shared_template.clone()).unwrap();
    two.mount(shared_template.clone()).unwrap();
    assert_eq!(first.listener_count(), 2);
    assert!(trace.borrow().is_empty());

    let intent = Intent::new(Edit::Copy);
    assert_eq!(first.invoke(&intent), ActionResult::Handled);
    assert!(
        trace.borrow().is_empty(),
        "an invocation isn't a state change"
    );
    first.set_enabled(false);
    first.set_enabled(false);
    assert_eq!(first.invoke(&intent), ActionResult::Ignored);
    assert_eq!(&*trace.borrow(), &[("first", false), ("first", false)]);

    one.update(root, scene("second", &second)).unwrap();
    assert_eq!(one.root(), Some(root));
    assert_eq!(first.listener_count(), 1);
    assert_eq!(second.listener_count(), 1);
    first.set_enabled(true);
    second.set_enabled(false);
    assert_eq!(
        &*trace.borrow(),
        &[
            ("first", false),
            ("first", false),
            ("first", true),
            ("second", false)
        ]
    );

    two.mount(SizedBox::shrink().into()).unwrap();
    assert_eq!(
        first.listener_count(),
        0,
        "unmount must detach even if the original widget is kept"
    );
    drop(one);
    assert_eq!(second.listener_count(), 0, "dropping a tree also detaches");
    first.set_enabled(false);
    second.set_enabled(true);
    assert_eq!(trace.borrow().len(), 4);
    drop(shared_template);
}
