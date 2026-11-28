use std::time::Duration;

use incular_config::{Constraints, TextDirection};
use incular_core::{
    Color, DragStartBehavior, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind,
    PointerPhase, PointerSampleMetadata, Size, time::Instant,
};
use incular_platform::TextInputCommand;
use incular_runtime::Runtime;
use incular_text::TextEditingController;
use incular_widgets::{
    Column, CustomScrollView, Directionality, EditableText, Expanded, FocusNode, FocusScope,
    GridView, ListView, ScrollBehavior, ScrollConfiguration, ScrollController,
    ScrollViewKeyboardDismissBehavior as Dismiss, SingleChildScrollView, SizedBox, Sliver,
    SliverChildBuilderDelegate, SliverGridDelegate, SliverList, Widget,
};

fn editor(node: &FocusNode) -> Widget {
    EditableText::new(TextEditingController::with_text("still editing"))
        .focus_node(node.clone())
        // Isolate scroll dismissal from the independent outside-press policy.
        .on_tap_outside(|_| {})
        .size(Size::new(300., 50.))
        .into()
}

fn scene(
    focus: &FocusNode,
    scroll: &ScrollController,
    policy: Option<Dismiss>,
    ambient: Dismiss,
    other_scope: bool,
) -> Widget {
    let bars = || Widget::box_(Size::new(300., 1000.), Color::WHITE);
    let child = if other_scope {
        bars()
    } else {
        Column::new([editor(focus), bars()]).into()
    };
    let view = SingleChildScrollView::new(child).controller(scroll.clone());
    let view = if let Some(policy) = policy {
        view.keyboard_dismiss_behavior(policy)
    } else {
        view
    };
    let view = FocusScope::new(view);
    let scene: Widget = if other_scope {
        Column::new([
            Widget::from(FocusScope::new(editor(focus))),
            Expanded::new(view).into(),
        ])
        .into()
    } else {
        view.into()
    };
    Directionality::new(
        TextDirection::Ltr,
        ScrollConfiguration::new(scene)
            .behavior(ScrollBehavior::new().keyboard_dismiss_behavior(ambient)),
    )
    .into()
}

struct Host {
    runtime: Runtime,
    focus: FocusNode,
    scroll: ScrollController,
    time: Instant,
}

impl Host {
    fn new(policy: Option<Dismiss>, ambient: Dismiss, other_scope: bool) -> Self {
        Self::mount(|focus, scroll| scene(focus, scroll, policy, ambient, other_scope))
    }

    fn mount(build: impl FnOnce(&FocusNode, &ScrollController) -> Widget) -> Self {
        let focus = FocusNode::new();
        let scroll = ScrollController::new();
        let runtime = Runtime::new(build(&focus, &scroll)).unwrap();
        let mut host = Self {
            runtime,
            focus,
            scroll,
            time: Instant::now(),
        };
        host.frame();
        host.focus.request_focus();
        host.frame();
        assert!(host.focus.has_primary_focus());
        assert!(
            host.runtime
                .take_text_input_commands()
                .iter()
                .any(|command| matches!(command, TextInputCommand::SetClient { .. }))
        );
        host
    }

    fn frame(&mut self) {
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(300., 220.)), self.time)
            .unwrap();
        self.time += Duration::from_millis(20);
    }

    fn contact(&mut self, phase: PointerPhase, y: f32) {
        let _ = self.runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 7,
                device: 1,
                kind: PointerDeviceKind::Touch,
                buttons: if phase == PointerPhase::Up {
                    0
                } else {
                    PRIMARY_POINTER_BUTTON
                },
                button: (phase != PointerPhase::Move).then_some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::EMPTY,
                phase,
                position: Offset::new(120., y),
            },
            self.time,
        );
        self.frame();
    }

    fn drag(&mut self, travel: f32) {
        self.drag_from(150., travel);
    }

    fn drag_from(&mut self, y: f32, travel: f32) {
        self.contact(PointerPhase::Down, y);
        self.contact(PointerPhase::Move, y + travel.signum() * 20.);
        self.contact(PointerPhase::Move, y + travel);
        self.contact(PointerPhase::Up, y + travel);
    }
}

#[test]
fn only_drag_updates_end_editing_for_the_effective_policy() {
    for (policy, ambient, dismissed) in [
        (Some(Dismiss::OnDrag), Dismiss::Manual, true),
        (Some(Dismiss::Manual), Dismiss::OnDrag, false),
        (None, Dismiss::Manual, false),
        (None, Dismiss::OnDrag, true),
    ] {
        let mut host = Host::new(policy, ambient, false);
        // A jump and a wheel move generate updates, but neither has drag details.
        assert!(host.scroll.jump_to(35.));
        host.frame();
        let _ = host.runtime.handle_input_at(
            InputEvent::ScrollAt {
                position: Offset::new(120., 140.),
                delta: Offset::new(0., 30.),
            },
            host.time,
        );
        host.frame();
        assert!(host.focus.has_primary_focus());
        let before = host.scroll.offset();
        host.drag(-70.);
        assert!(host.scroll.offset() > before);
        assert_eq!(host.focus.has_primary_focus(), !dismissed);
        assert_eq!(
            host.runtime
                .take_text_input_commands()
                .iter()
                .any(|command| matches!(command, TextInputCommand::Hide { .. })),
            dismissed
        );
    }
}

#[test]
fn an_overscroll_without_a_scroll_update_does_not_end_editing() {
    let mut host = Host::new(Some(Dismiss::OnDrag), Dismiss::Manual, false);
    assert_eq!(host.scroll.offset(), 0.);
    host.drag(60.);
    assert_eq!(host.scroll.offset(), 0.);
    assert!(host.focus.has_primary_focus());
    host.drag(-65.);
    assert!(!host.focus.has_primary_focus());
}

#[test]
fn a_drag_does_not_unfocus_an_editor_in_another_focus_scope() {
    let mut host = Host::new(Some(Dismiss::OnDrag), Dismiss::Manual, true);
    host.drag(-65.);
    assert!(host.scroll.offset() > 0.);
    assert!(host.focus.has_primary_focus());
}

#[test]
fn dragging_from_the_focused_editor_itself_obeys_the_policy() {
    for policy in [Dismiss::OnDrag, Dismiss::Manual] {
        let mut host = Host::new(Some(policy), Dismiss::Manual, false);
        host.drag_from(25., -40.);
        assert!(host.scroll.offset() > 0.);
        assert_eq!(host.focus.has_primary_focus(), policy == Dismiss::Manual);
    }
}

#[test]
fn updating_ambient_policy_rebinds_the_same_retained_scroller() {
    let mut host = Host::new(None, Dismiss::Manual, false);
    host.drag_from(25., -40.);
    assert!(host.focus.has_primary_focus());

    let root = host.runtime.tree().root().unwrap();
    host.runtime
        .schedule_update(
            root,
            scene(&host.focus, &host.scroll, None, Dismiss::OnDrag, false),
        )
        .unwrap();
    host.frame();
    assert!(host.focus.has_primary_focus());
    host.drag(-50.);
    assert!(!host.focus.has_primary_focus());

    host.focus.request_focus();
    host.frame();
    assert!(host.focus.has_primary_focus());
    host.runtime
        .schedule_update(
            root,
            scene(
                &host.focus,
                &host.scroll,
                Some(Dismiss::Manual),
                Dismiss::OnDrag,
                false,
            ),
        )
        .unwrap();
    host.frame();
    host.drag(-50.);
    assert!(host.focus.has_primary_focus());
}

#[derive(Clone, Copy, Debug)]
enum SliverView {
    List,
    ListBuilder,
    ListSeparated,
    Grid,
    GridBuilder,
    GridCount,
    GridExtent,
    ListCustom,
    GridCustom,
    Custom,
}

fn sliver_scene(
    kind: SliverView,
    policy: Option<Dismiss>,
    ambient: Dismiss,
    focus: &FocusNode,
    scroll: &ScrollController,
) -> Widget {
    let nodes: Vec<_> = std::iter::once(focus.clone())
        .chain((1..50).map(|_| FocusNode::new()))
        .collect();
    let rows = || nodes.iter().map(editor).collect::<Vec<_>>();
    let nth = {
        let nodes = nodes.clone();
        move |index| editor(&nodes[index])
    };
    let list = |descriptor: ListView| -> Widget {
        let descriptor = descriptor.controller(scroll.clone());
        policy.map_or_else(
            || descriptor.clone().into(),
            |behavior| {
                descriptor
                    .clone()
                    .keyboard_dismiss_behavior(behavior)
                    .into()
            },
        )
    };
    let grid = |descriptor: GridView| -> Widget {
        let descriptor = descriptor.controller(scroll.clone());
        policy.map_or_else(
            || descriptor.clone().into(),
            |behavior| {
                descriptor
                    .clone()
                    .keyboard_dismiss_behavior(behavior)
                    .into()
            },
        )
    };
    let layout = || SliverGridDelegate::fixed_cross_axis_count(2);
    let view = match kind {
        SliverView::List => list(ListView::new(rows())),
        SliverView::ListBuilder => list(ListView::builder(50, nth)),
        SliverView::ListSeparated => list(ListView::separated(50, nth, |_| SizedBox::shrink())),
        SliverView::Grid => grid(GridView::new(layout(), rows())),
        SliverView::GridBuilder => grid(GridView::builder(50, layout(), nth)),
        SliverView::GridCount => grid(GridView::count(2, rows())),
        SliverView::GridExtent => grid(GridView::extent(300., rows())),
        SliverView::ListCustom => list(ListView::custom(
            SliverChildBuilderDelegate::new(move |index| Some(nth(index))).child_count(50),
        )),
        SliverView::GridCustom => grid(GridView::custom(
            layout(),
            SliverChildBuilderDelegate::new(move |index| Some(nth(index))).child_count(50),
        )),
        SliverView::Custom => {
            let sliver: Box<dyn Sliver> = Box::new(SliverList::builder(50, nth));
            let view = CustomScrollView::new([sliver])
                .controller(scroll.clone())
                .drag_start_behavior(DragStartBehavior::Down);
            match policy {
                Some(behavior) => view.keyboard_dismiss_behavior(behavior).into(),
                None => view.into(),
            }
        }
    };
    Directionality::new(
        TextDirection::Ltr,
        ScrollConfiguration::new(FocusScope::new(view))
            .behavior(ScrollBehavior::new().keyboard_dismiss_behavior(ambient)),
    )
    .into()
}

fn assert_sliver_policy(kind: SliverView, explicit: Option<Dismiss>, ambient: Dismiss) {
    let mut host =
        Host::mount(|focus, scroll| sliver_scene(kind, explicit, ambient, focus, scroll));
    let focus = host.runtime.focused_element().unwrap();
    let bounds = host.runtime.tree().element_bounds(focus).unwrap();
    host.drag_from(bounds.origin.y + bounds.size.height * 0.5, -40.);
    assert!(host.scroll.offset() > 0., "{kind:?} really scrolled");
    let should_dismiss = explicit.unwrap_or(ambient) == Dismiss::OnDrag;
    assert_eq!(host.focus.has_primary_focus(), !should_dismiss, "{kind:?}");
}

#[test]
fn each_list_constructor_applies_the_explicit_keyboard_dismissal_policy() {
    for kind in [
        SliverView::List,
        SliverView::ListBuilder,
        SliverView::ListSeparated,
    ] {
        for policy in [None, Some(Dismiss::Manual), Some(Dismiss::OnDrag)] {
            assert_sliver_policy(kind, policy, Dismiss::Manual);
        }
    }
}

#[test]
fn each_grid_constructor_applies_the_explicit_keyboard_dismissal_policy() {
    for kind in [
        SliverView::Grid,
        SliverView::GridBuilder,
        SliverView::GridCount,
        SliverView::GridExtent,
    ] {
        for policy in [None, Some(Dismiss::Manual), Some(Dismiss::OnDrag)] {
            assert_sliver_policy(kind, policy, Dismiss::Manual);
        }
    }
}

#[test]
fn custom_scroll_view_applies_the_explicit_keyboard_dismissal_policy() {
    assert_sliver_policy(SliverView::Custom, Some(Dismiss::OnDrag), Dismiss::Manual);
}

#[test]
fn sliver_scroll_widgets_inherit_policy_unless_explicitly_overridden() {
    for view in [SliverView::List, SliverView::Grid, SliverView::Custom] {
        assert_sliver_policy(view, None, Dismiss::OnDrag);
        assert_sliver_policy(view, Some(Dismiss::Manual), Dismiss::OnDrag);
    }
}

#[test]
fn custom_child_delegates_preserve_the_scroll_views_keyboard_policy() {
    for kind in [SliverView::ListCustom, SliverView::GridCustom] {
        for policy in [None, Some(Dismiss::OnDrag)] {
            assert_sliver_policy(kind, policy, Dismiss::Manual);
        }
    }
}

#[test]
fn finite_custom_child_delegates_build_only_the_mounted_window() {
    use std::{cell::Cell, rc::Rc};
    for grid in [false, true] {
        let builds = Rc::new(Cell::new(0));
        let delegate = SliverChildBuilderDelegate::new({
            let builds = builds.clone();
            move |index| {
                builds.set(builds.get() + 1);
                Some(Widget::from(SizedBox::from_size(Size::new(300., 50.))).with_key(index as u64))
            }
        })
        .child_count(10_000);
        let widget: Widget = if grid {
            GridView::custom(SliverGridDelegate::fixed_cross_axis_count(2), delegate).into()
        } else {
            ListView::custom(delegate).into()
        };
        assert_eq!(builds.get(), 0);
        let mut runtime =
            Runtime::new(Directionality::new(TextDirection::Ltr, widget).into()).unwrap();
        runtime
            .run_frame(Constraints::tight(Size::new(300., 220.)))
            .unwrap();
        assert!((1..40).contains(&builds.get()), "builds={}", builds.get());
        assert!(runtime.tree().element_with_key(&0u64.into()).is_some());
        assert!(runtime.tree().element_with_key(&1000u64.into()).is_none());
    }
}
