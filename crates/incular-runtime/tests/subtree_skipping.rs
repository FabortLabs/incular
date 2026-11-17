//! Runtime work remains independent from ancestor descriptor reuse.

use std::{cell::Cell, rc::Rc};

use incular_config::{Constraints, EdgeInsets};
use incular_core::{Color, Size};
use incular_runtime::{Runtime, Signal};
use incular_widgets::{Column, Padding, Widget};

fn constraints() -> Constraints {
    Constraints::tight(Size::new(100.0, 100.0))
}

fn box_widget(width: u32) -> Widget {
    Widget::box_(Size::new(width as f32, 10.0), Color::WHITE)
}

#[test]
fn independently_dirty_child_rebuilds_when_its_parent_descriptor_is_skipped() {
    let parent_signal = Signal::new(0_u32);
    let child_signal = Signal::new(10_u32);
    let parent_builds = Rc::new(Cell::new(0));
    let child_builds = Rc::new(Cell::new(0));
    let seen = Rc::new(Cell::new(0));
    let shared: Widget = Padding::new(EdgeInsets::all(1.0), box_widget(10)).into();
    let mut runtime = Runtime::new(Column::new([shared.clone()]).into()).expect("runtime");
    let root = runtime.tree().root().expect("root");
    let wrapper = runtime.tree().children(root).expect("wrapper")[0];
    let child = runtime.tree().children(wrapper).expect("child")[0];

    runtime
        .register_builder(root, {
            let signal = parent_signal.clone();
            let builds = parent_builds.clone();
            move || {
                let _ = signal.get();
                builds.set(builds.get() + 1);
                Column::new([shared.clone()]).into()
            }
        })
        .expect("parent builder");
    runtime
        .register_builder(child, {
            let signal = child_signal.clone();
            let builds = child_builds.clone();
            let seen = seen.clone();
            move || {
                let width = signal.get();
                seen.set(width);
                builds.set(builds.get() + 1);
                box_widget(width)
            }
        })
        .expect("child builder");
    runtime.run_frame(constraints()).expect("initial frame");
    let before_parent = parent_builds.get();
    let before_child = child_builds.get();
    let before = runtime.tree().diagnostics();

    // Queue the parent first: its equal descriptor must not consume the
    // descendant's independent signal subscription or pending build.
    parent_signal.set(1);
    child_signal.set(20);
    runtime.run_frame(constraints()).expect("reactive frame");

    assert_eq!(parent_builds.get(), before_parent + 1);
    assert_eq!(child_builds.get(), before_child + 1);
    assert_eq!(seen.get(), 20);
    assert_eq!(
        runtime
            .tree()
            .element_bounds(child)
            .expect("child bounds")
            .size
            .width,
        20.0
    );
    assert_eq!(runtime.tree().diagnostics().rebuilds - before.rebuilds, 1);
    assert!(!runtime.tree().is_build_dirty(root));
    assert!(!runtime.tree().is_build_dirty(child));
    assert_eq!(child_signal.dependent_count(), 1);

    child_signal.set(30);
    runtime.run_frame(constraints()).expect("later child frame");
    assert_eq!(seen.get(), 30);
    assert_eq!(parent_builds.get(), before_parent + 1);
    assert_eq!(child_builds.get(), before_child + 2);
}

#[test]
fn independently_queued_child_rebuilds_after_a_changed_parent_skips_its_descriptor() {
    let parent_signal = Signal::new(0_u32);
    let child_signal = Signal::new(10_u32);
    let parent_builds = Rc::new(Cell::new(0));
    let child_builds = Rc::new(Cell::new(0));
    let shared = box_widget(10);
    let mut runtime = Runtime::new(Column::new([shared.clone()]).into()).expect("runtime");
    let root = runtime.tree().root().expect("root");
    let child = runtime.tree().children(root).expect("shared child")[0];

    runtime
        .register_builder(root, {
            let signal = parent_signal.clone();
            let builds = parent_builds.clone();
            move || {
                builds.set(builds.get() + 1);
                Column::new([shared.clone()])
                    .spacing(signal.get() as f32)
                    .into()
            }
        })
        .expect("parent builder");
    runtime
        .register_builder(child, {
            let signal = child_signal.clone();
            let builds = child_builds.clone();
            move || {
                builds.set(builds.get() + 1);
                box_widget(signal.get())
            }
        })
        .expect("child builder");
    runtime.run_frame(constraints()).expect("initial frame");
    let before_parent = parent_builds.get();
    let before_child = child_builds.get();
    let before = runtime.tree().diagnostics();

    // The changed parent reconciles its direct shared child and consumes
    // that child's BUILD mark. Queue membership must remain independent:
    // the child's already-enqueued builder still runs afterward.
    runtime.tree_mut().mark_build(child).expect("mark child");
    assert!(runtime.tree().is_build_dirty(child));
    parent_signal.set(5);
    child_signal.set(20);
    runtime
        .run_frame(constraints())
        .expect("parent and child frame");

    assert_eq!(parent_builds.get(), before_parent + 1);
    assert_eq!(child_builds.get(), before_child + 1);
    assert_eq!(runtime.tree().diagnostics().rebuilds - before.rebuilds, 2);
    assert!(
        runtime.tree().diagnostics().identical_child_bailouts > before.identical_child_bailouts,
        "the parent encountered the shared child before its queued rebuild"
    );
    assert_eq!(
        runtime
            .tree()
            .element_bounds(child)
            .expect("child bounds")
            .size
            .width,
        20.0
    );
    assert!(!runtime.tree().is_build_dirty(child));
    assert_eq!(child_signal.dependent_count(), 1);

    child_signal.set(30);
    runtime.run_frame(constraints()).expect("later child frame");
    assert_eq!(parent_builds.get(), before_parent + 1);
    assert_eq!(child_builds.get(), before_child + 2);
    assert_eq!(
        runtime
            .tree()
            .element_bounds(child)
            .expect("updated child bounds")
            .size
            .width,
        30.0
    );
    assert_eq!(child_signal.dependent_count(), 1);
}

#[test]
fn scheduled_unchanged_wrapper_consumes_its_build_mark_without_reconciliation() {
    let shared = box_widget(10);
    let mut runtime = Runtime::new(Column::new([shared.clone()]).into()).expect("runtime");
    let root = runtime.tree().root().expect("root");
    runtime.run_frame(constraints()).expect("initial frame");
    let before = runtime.tree().diagnostics();

    runtime
        .schedule_update(root, Column::new([shared]).into())
        .expect("schedule equal wrapper");
    assert!(runtime.tree().is_build_dirty(root));
    runtime.run_frame(constraints()).expect("scheduled frame");

    assert!(!runtime.tree().is_build_dirty(root));
    assert_eq!(runtime.tree().diagnostics().rebuilds, before.rebuilds);
    assert_eq!(
        runtime.tree().diagnostics().child_list_scans,
        before.child_list_scans
    );
}
