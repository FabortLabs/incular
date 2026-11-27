use std::{
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{Builder, Directionality, SizedBox, Widget};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(160., 100.)))
        .unwrap();
}

#[test]
fn the_same_child_rebuilds_only_when_its_inherited_direction_changes() {
    use TextDirection::{Ltr, Rtl};
    let log = Rc::new(RefCell::new(Vec::new()));
    let child: Widget = Builder::new({
        let log = log.clone();
        move |context| {
            log.borrow_mut().push(Directionality::of(context));
            SizedBox::square(20.).into()
        }
    })
    .into();
    let build = |direction| Directionality::new(direction, child.clone()).into();
    let mut runtime = Runtime::new(build(Ltr)).unwrap();
    frame(&mut runtime);
    assert_eq!(*log.borrow(), [Ltr]);
    for (direction, expected) in [
        (Ltr, vec![Ltr]),
        (Rtl, vec![Ltr, Rtl]),
        (Rtl, vec![Ltr, Rtl]),
        (Ltr, vec![Ltr, Rtl, Ltr]),
    ] {
        runtime
            .schedule_update(runtime.tree().root().unwrap(), build(direction))
            .unwrap();
        frame(&mut runtime);
        assert_eq!(*log.borrow(), expected);
    }
}

#[test]
fn there_is_no_implicit_direction_without_a_provider() {
    let checked = Rc::new(Cell::new(false));
    let root = Builder::new({
        let checked = checked.clone();
        move |context| {
            assert_eq!(Directionality::maybe_of(context), None);
            checked.set(true);
            SizedBox::square(20.).into()
        }
    });
    let mut runtime = Runtime::new(root.into()).unwrap();
    frame(&mut runtime);
    assert!(checked.get());
}

#[test]
fn maybe_of_only_reads_ancestors_not_descendants_or_siblings() {
    use TextDirection::Rtl;
    let count = Rc::new(Cell::new(0));
    let leaf = Builder::new({
        let count = count.clone();
        move |context| {
            assert_eq!(Directionality::maybe_of(context), Some(Rtl));
            count.set(count.get() + 1);
            SizedBox::square(20.).into()
        }
    });
    let root = Builder::new({
        let count = count.clone();
        move |context| {
            assert_eq!(Directionality::maybe_of(context), None);
            count.set(count.get() + 1);
            Directionality::new(Rtl, leaf.clone()).into()
        }
    });
    let mut runtime = Runtime::new(root.into()).unwrap();
    frame(&mut runtime);
    assert_eq!(count.get(), 2);
}

#[test]
fn of_requires_a_provider_and_returns_the_nearest_direction_when_present() {
    let found = Rc::new(Cell::new(false));
    let leaf = Builder::new({
        let found = found.clone();
        move |context| {
            assert_eq!(Directionality::of(context), TextDirection::Rtl);
            found.set(true);
            SizedBox::square(20.).into()
        }
    });
    let root = Builder::new(move |context| {
        let panic = catch_unwind(AssertUnwindSafe(|| Directionality::of(context))).unwrap_err();
        let message = panic
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| panic.downcast_ref::<String>().map(String::as_str));
        assert!(message.is_some_and(|message| message.contains("No Directionality widget found.")));
        Directionality::new(
            TextDirection::Ltr,
            Directionality::new(TextDirection::Rtl, leaf.clone()),
        )
        .into()
    });
    let mut runtime = Runtime::new(root.into()).unwrap();
    frame(&mut runtime);
    assert!(found.get());
}
