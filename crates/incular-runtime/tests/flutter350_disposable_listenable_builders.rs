//! Pinned Flutter transition builder cases driven through retained frames.

use incular_config::Constraints;
use incular_core::{ChangeNotifier, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    AnimatedBuilder, BuildContext, Builder, Center, Column, FocusNode, ListenableBuilder, SizedBox,
    Widget,
};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy)]
enum Kind {
    Animated,
    Listenable,
}

fn frame(runtime: &mut Runtime, size: Size) {
    runtime.run_frame(Constraints::tight(size)).unwrap();
}

fn counter(count: Rc<Cell<u32>>) -> Widget {
    Builder::new(move |_| {
        count.set(count.get() + 1);
        SizedBox::square(20.0).into()
    })
    .into()
}

fn build(
    kind: Kind,
    notifier: &ChangeNotifier,
    count: Rc<Cell<u32>>,
    child: Option<Widget>,
) -> Widget {
    let redraw = move |_: &BuildContext<'_>, child: Option<Widget>| {
        count.set(count.get() + 1);
        child.unwrap_or_else(|| SizedBox::square(20.0).into())
    };
    match (kind, child) {
        (Kind::Animated, None) => AnimatedBuilder::from_listenable(notifier.clone(), redraw).into(),
        (Kind::Animated, Some(child)) => AnimatedBuilder::from_listenable(notifier.clone(), redraw)
            .child(child)
            .into(),
        (Kind::Listenable, None) => ListenableBuilder::new(notifier.clone(), redraw).into(),
        (Kind::Listenable, Some(child)) => ListenableBuilder::new(notifier.clone(), redraw)
            .child(child)
            .into(),
    }
}

fn exercise(kind: Kind, pass_child: bool) {
    let notifier = ChangeNotifier::new();
    let builds = Rc::new(Cell::new(0));
    let child_builds = Rc::new(Cell::new(0));
    let child = pass_child.then(|| counter(child_builds.clone()));
    let mut runtime = Runtime::new(build(kind, &notifier, builds.clone(), child)).unwrap();
    let bounds = Size::new(100.0, 100.0);
    frame(&mut runtime, bounds);
    assert_eq!(builds.get(), 1);
    assert_eq!(child_builds.get(), u32::from(pass_child));
    assert!(notifier.has_listeners());
    frame(&mut runtime, bounds);
    assert_eq!(builds.get(), 1);
    assert_eq!(child_builds.get(), u32::from(pass_child));

    notifier.notify_listeners();
    assert!(runtime.frame_requested());
    frame(&mut runtime, bounds);
    assert_eq!(builds.get(), 2);
    assert_eq!(child_builds.get(), u32::from(pass_child));
    frame(&mut runtime, bounds);
    frame(&mut runtime, bounds);
    assert_eq!(builds.get(), 2);
    assert_eq!(child_builds.get(), u32::from(pass_child));
}

#[test]
fn animated_builder_rebuilds_on_change_notifier() {
    exercise(Kind::Animated, false);
}

#[test]
fn animated_builder_keeps_the_same_pass_through_child() {
    exercise(Kind::Animated, true);
}

#[test]
fn listenable_builder_rebuilds_on_change_notifier() {
    exercise(Kind::Listenable, false);
}

#[test]
fn listenable_builder_keeps_the_same_pass_through_child() {
    exercise(Kind::Listenable, true);
}

#[test]
fn focus_node_listenable_builder_completes_at_zero_area() {
    let node = FocusNode::new();
    let mut runtime = Runtime::new(
        Center::new(ListenableBuilder::new(node, |_, _| {
            SizedBox::square(20.0).into()
        }))
        .into(),
    )
    .unwrap();
    frame(&mut runtime, Size::ZERO);
    assert_eq!(runtime.tree().root_layout_size(), Some(Size::ZERO));
}

#[test]
fn animated_builder_with_a_controller_completes_at_zero_area() {
    use incular_animation::AnimationController;
    use std::time::Duration;

    let controller = AnimationController::new(Duration::from_secs(2));
    controller.set_value(1.0);
    let mut runtime = Runtime::new(
        Center::new(
            AnimatedBuilder::new(controller, |_, _| SizedBox::square(20.0).into())
                .child(SizedBox::square(20.0)),
        )
        .into(),
    )
    .unwrap();
    frame(&mut runtime, Size::ZERO);
    assert_eq!(runtime.tree().root_layout_size(), Some(Size::ZERO));
}

#[test]
fn replacing_and_unmounting_the_builder_releases_the_old_notifier() {
    let before = ChangeNotifier::new();
    let after = ChangeNotifier::new();
    let builds = Rc::new(Cell::new(0));
    let size = Size::new(100.0, 100.0);
    let subtree = |notifier| Column::new([build(Kind::Listenable, notifier, builds.clone(), None)]);
    let mut runtime = Runtime::new(subtree(&before).into()).unwrap();
    frame(&mut runtime, size);
    assert!(before.has_listeners());
    assert!(!after.has_listeners());

    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, subtree(&after).into())
        .unwrap();
    frame(&mut runtime, size);
    assert_eq!(builds.get(), 2);
    assert!(!before.has_listeners());
    assert!(after.has_listeners());
    before.notify_listeners();
    assert!(!runtime.frame_requested());
    after.notify_listeners();
    assert!(runtime.frame_requested());
    frame(&mut runtime, size);
    assert_eq!(builds.get(), 3);
    runtime
        .schedule_update(
            runtime.tree().root().unwrap(),
            Column::new([SizedBox::shrink()]).into(),
        )
        .unwrap();
    frame(&mut runtime, size);
    assert!(!after.has_listeners());
    after.dispose();
}

#[test]
fn a_post_frame_notification_reports_the_next_build_demand() {
    let notifier = ChangeNotifier::new();
    let builds = Rc::new(Cell::new(0));
    let size = Size::new(100.0, 100.0);
    let mut runtime =
        Runtime::new(build(Kind::Listenable, &notifier, builds.clone(), None)).unwrap();
    frame(&mut runtime, size);
    let notifier = notifier.clone();
    incular_animation::add_post_frame_callback(move |_| notifier.notify_listeners());
    let (_, stats) = runtime.run_frame(Constraints::tight(size)).unwrap();
    assert_eq!(builds.get(), 1);
    assert!(stats.requested_another_frame);
    assert!(runtime.frame_requested());
    frame(&mut runtime, size);
    assert_eq!(builds.get(), 2);
    assert!(!runtime.frame_requested());
}
