# Reproducing the six behavior probes

These are expected-parity assertions for the **Incular** side of
[the behavior audit](behavior-audit.md). At the audited revision they fail at
the stated assertions. Flutter expectations were derived by reading the pinned
source; these are not cross-executable tests or a reason to run the full suite.

Use temporary files with the exact names below, only if the paths do not
already exist. For snippets based on an existing test, start with a **copy**
of that test and append the snippet. Remove only the temporary files when done.
These historical scratch recipes recorded the original evidence. The
[backfill ledger](test-backfill.tsv) now points to permanent, deliberately
red regression targets.

## B02 and B03: border command and cache range

Temporary file: `crates/incular-widgets/tests/audit_f350_behavior_tmp.rs`.

```rust
use incular_config::{Axis, Constraints};
use incular_core::{Color, Size};
use incular_rendering::PaintCommand;
use incular_scroll::{ScrollController, SliverConstraints};
use incular_widgets::{
    Border, BorderSide, BorderStyle, BoxDecoration, Container, Sliver, SliverFixedExtentList,
    Widget, internal::WidgetTree,
};

fn paints_border(style: Option<BorderStyle>) -> bool {
    let mut tree = WidgetTree::new();
    let tint = Color::rgba(217, 11, 79, 255);
    let size = Size::new(40., 40.);
    let mut decoration = BoxDecoration::new();
    if let Some(style) = style {
        decoration = decoration.border(Border::all(BorderSide::new(tint, 2., style)));
    }
    tree.mount(Container::new().width(40.).height(40.).decoration(decoration)
        .child(Widget::box_(Size::new(10.,10.), Color::WHITE)).into()).unwrap();
    tree.layout(Constraints::tight(size)).unwrap();
    tree.paint().commands().iter().any(|cmd| matches!(cmd, PaintCommand::Border { border, .. }
        if border.width == 2. && border.color == tint))
}

#[test]
fn none_border_in_container_does_not_paint() {
    assert!(!paints_border(None), "no border is a negative control");
    assert!(paints_border(Some(BorderStyle::Solid)), "solid border is a positive control");
    assert!(!paints_border(Some(BorderStyle::None)), "Container painted an explicitly invisible border");
}

fn sliver_indices(extent: f32, cache_end: f32) -> Vec<usize> {
    use std::{cell::RefCell, rc::Rc};
    let built = Rc::new(RefCell::new(Vec::new()));
    let observed = built.clone();
    let sliver = SliverFixedExtentList::new(10, extent, move |idx| {
        observed.borrow_mut().push(idx);
        Widget::box_(Size::new(40., extent), Color::WHITE)
    });
    let mut render = sliver.create_render_sliver(&ScrollController::new(), Axis::Vertical, false);
    let _ = render.perform_layout(SliverConstraints::new(
        Axis::Vertical, false, 0., 0., 0., cache_end, 40., cache_end, cache_end, 0.,
    ));
    let indices = built.borrow().clone();
    indices
}

#[test]
fn fractional_fixed_extent_does_not_build_next_row() {
    assert_eq!(sliver_indices(20., 60.), [0, 1, 2], "integer edge control");
    assert_eq!(sliver_indices(20.3, 60.9), [0, 1, 2], "row exactly at the end is outside the cache range");
}
```

`cargo test --locked -p incular-widgets --test audit_f350_behavior_tmp`

## B01: cancellation retains the active editor

Copy `crates/incular-runtime/tests/text_field_touch_focus.rs` to
`crates/incular-runtime/tests/audit_f350_focus_tmp.rs` and append:

```rust
#[test]
fn cancelled_touch_does_not_retarget_editor() {
    let (mut runtime, a, b, mut now) = fields(TextShortcutPlatform::Android);
    tap(&mut runtime, Offset::new(100.,20.), &mut now);
    for (phase, buttons) in [
        (PointerPhase::Down, PRIMARY_POINTER_BUTTON),
        (PointerPhase::Cancel, 0),
    ] {
        let _ = runtime.handle_input_at(InputEvent::PointerWithMetadata {
            pointer: 0, device: 0, kind: PointerDeviceKind::Touch,
            buttons, button: Some(PRIMARY_POINTER_BUTTON),
            sample: PointerSampleMetadata::default(), phase,
            position: Offset::new(100.,60.),
        }, now);
        now += Duration::from_millis(50);
    }
    type_text(&mut runtime, now);
    assert_eq!((a.text(), b.text()), ("ax".into(), "b".into()),
               "a cancelled tap must not move the editing target");
}
```

`cargo test --locked -p incular-runtime --test audit_f350_focus_tmp cancelled_touch_does_not_retarget_editor -- --exact`

## B04 and B05: native editing

Copy `crates/incular-runtime/tests/platform_text_input.rs` to
`crates/incular-runtime/tests/audit_f350_edit_tmp.rs` and append:

```rust
#[test]
fn flutter350_single_line_native_edit_strips_lf() {
    let controller = TextEditingController::new();
    let mut runtime = editor(&controller);
    tap(&mut runtime);
    runtime.run_frame(constraints()).unwrap();
    assert!(runtime.focused_element().is_some());
    let _ = runtime.handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
        text: "a\nb".into(), selection_start: 3, selection_end: 3, composing: None,
    }));
    assert_eq!(controller.text(), "ab");
    assert_eq!(controller.selection().extent, 2);
}
#[test]
fn flutter350_native_selection_only_does_not_run_formatter() {
    struct Count(std::rc::Rc<std::cell::Cell<u32>>);
    impl incular_widgets::TextInputFormatter for Count {
        fn format_edit_update(&self, _: &incular_text::TextEditingValue, new: &incular_text::TextEditingValue)
            -> incular_text::TextEditingValue {
            self.0.set(self.0.get() + 1);
            new.clone()
        }
    }
    let controller = TextEditingController::with_text("abc");
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let mut runtime = Runtime::new(EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr).input_formatter(Count(calls.clone())).into()).unwrap();
    runtime.run_frame(constraints()).unwrap();
    tap(&mut runtime);
    runtime.run_frame(constraints()).unwrap();
    let pick = if controller.selection().extent == 0 {1} else {0};
    calls.set(0);
    let _ = runtime.handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
        text: "abc".into(), selection_start: pick, selection_end: pick, composing: None,
    }));
    assert_eq!(controller.selection().extent, pick);
    assert_eq!(calls.get(), 0, "a selection-only update must not invoke text formatters");
}
```

`cargo test --locked -p incular-runtime --test audit_f350_edit_tmp flutter350_`

## B06: cancellation within a detached scheduler batch

Copy `crates/incular-runtime/tests/frame_scheduler.rs` to
`crates/incular-runtime/tests/audit_f350_scheduler_tmp.rs` and append:

```rust
#[test]
fn flutter350_a_transient_callback_can_cancel_later_callback_in_same_batch() {
    let mut runtime = runtime();
    let id = Rc::new(Cell::new(None));
    let seen = Rc::new(RefCell::new(Vec::new()));
    schedule_frame_callback({
        let id = id.clone();
        let seen = seen.clone();
        move |_| {
            seen.borrow_mut().push('A');
            let _ = cancel_frame_callback(id.get().unwrap());
        }
    });
    id.set(Some(schedule_frame_callback({
        let seen = seen.clone();
        move |_| seen.borrow_mut().push('B')
    })));
    runtime.run_frame_at(bounds(), Instant::now()).unwrap();
    assert_eq!(seen.borrow().as_slice(), &['A']);
}
```

`cargo test --locked -p incular-runtime --test audit_f350_scheduler_tmp flutter350_`
