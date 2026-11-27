use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use incular_animation::Curve;
use incular_config::Constraints;
use incular_core::{Lerp, Size};
use incular_widgets::{
    Placeholder, SizedBox, TweenAnimationBuilder, Widget,
    internal::{ElementId, Key, WidgetTree},
};

const SECOND: Duration = Duration::from_secs(1);
type Values = Rc<RefCell<Vec<i32>>>;

struct Host {
    tree: WidgetTree,
    root: ElementId,
    start: Instant,
    constraints: Constraints,
}

impl Host {
    fn mount(widget: Widget) -> Self {
        Self::mount_with_constraints(widget, Constraints::loose(Size::new(200., 200.)))
    }

    fn mount_with_constraints(widget: Widget, constraints: Constraints) -> Self {
        let mut tree = WidgetTree::new();
        let root = tree.mount(widget).unwrap();
        let mut host = Self {
            tree,
            root,
            start: Instant::now(),
            constraints,
        };
        host.frame(0);
        host.frame(0);
        host
    }

    fn frame(&mut self, millis: u64) {
        self.tree
            .update_compositor(self.start + Duration::from_millis(millis))
            .unwrap();
        self.tree.layout(self.constraints).unwrap();
    }

    fn update(&mut self, now: u64, widget: Widget) {
        self.tree.update(self.root, widget).unwrap();
        self.frame(now);
        self.frame(now);
    }
}

fn int(
    values: &Values,
    begin: Option<i32>,
    end: i32,
    duration: Duration,
    curve: Curve,
    endings: Option<&Rc<Cell<u32>>>,
) -> Widget {
    let values = values.clone();
    let mut widget = TweenAnimationBuilder::new(end, duration, move |_, value, _| {
        values.borrow_mut().push(value);
        Placeholder::new()
    })
    .curve(curve);
    if let Some(begin) = begin {
        widget = widget.begin(begin);
    }
    if let Some(endings) = endings.cloned() {
        widget = widget.on_end(move || endings.set(endings.get() + 1));
    }
    widget.into()
}

fn linear(values: &Values, begin: i32, end: i32) -> Widget {
    int(values, Some(begin), end, SECOND, Curve::Linear, None)
}

#[test]
fn starts_at_begin_and_completes_exactly_once() {
    let values = Values::default();
    let endings = Rc::new(Cell::new(0));
    let mut host = Host::mount(int(
        &values,
        Some(10),
        110,
        SECOND,
        Curve::Linear,
        Some(&endings),
    ));
    assert_eq!(endings.get(), 0);
    assert_eq!(*values.borrow(), [10]);
    host.frame(500);
    assert_eq!(*values.borrow(), [10, 60]);
    host.frame(1001);
    assert_eq!(endings.get(), 1);
    assert_eq!(*values.borrow(), [10, 60, 110]);
    host.frame(1501);
    assert_eq!(endings.get(), 1);
    assert_eq!(*values.borrow(), [10, 60, 110]);
}

fn check_no_initial_animation(begin: Option<i32>) {
    let values = Values::default();
    let endings = Rc::new(Cell::new(0));
    let mut host = Host::mount(int(
        &values,
        begin,
        100,
        SECOND,
        Curve::Linear,
        Some(&endings),
    ));
    assert_eq!(endings.get(), 0);
    assert_eq!(*values.borrow(), [100]);
    host.frame(2000);
    assert_eq!(endings.get(), 0);
    assert_eq!(*values.borrow(), [100]);
}

#[test]
fn omitted_begin_uses_end_without_scheduling_an_animation() {
    check_no_initial_animation(None);
}

#[test]
fn equal_begin_and_end_does_not_schedule_an_animation() {
    check_no_initial_animation(Some(100));
}

#[test]
fn replacing_a_completed_tween_starts_at_its_previous_value() {
    let values = Values::default();
    let mut host = Host::mount(linear(&values, 0, 100));
    assert_eq!(*values.borrow(), [0]);
    host.frame(2000);
    assert_eq!(*values.borrow(), [0, 100]);
    host.update(2000, linear(&values, 100, 200));
    assert_eq!(*values.borrow(), [0, 100, 100]);
    host.frame(2500);
    assert_eq!(*values.borrow(), [0, 100, 100, 150]);
    host.frame(3000);
    assert_eq!(*values.borrow(), [0, 100, 100, 150, 200]);
}

#[test]
fn new_curve_is_used_for_the_new_target() {
    let values = Values::default();
    let mut host = Host::mount(int(&values, Some(0), 100, SECOND, Curve::EaseInExpo, None));
    assert_eq!(*values.borrow(), [0]);
    host.frame(500);
    assert!((1..50).contains(values.borrow().last().unwrap()));
    host.frame(2500);
    values.borrow_mut().clear();
    host.update(2500, linear(&values, 100, 200));
    assert_eq!(*values.borrow(), [100]);
    host.frame(3000);
    assert_eq!(*values.borrow(), [100, 150]);
}

#[test]
fn subsequent_targets_use_the_new_duration() {
    let values = Values::default();
    let mut host = Host::mount(linear(&values, 0, 100));
    assert_eq!(*values.borrow(), [0]);
    host.frame(500);
    assert_eq!(*values.borrow(), [0, 50]);
    host.frame(2500);
    values.borrow_mut().clear();
    host.update(
        2500,
        int(&values, Some(100), 200, SECOND * 2, Curve::Linear, None),
    );
    assert_eq!(*values.borrow(), [100]);
    host.frame(3000);
    assert_eq!(*values.borrow(), [100, 125]);
}

#[test]
fn optional_child_is_mounted_and_keeps_its_retained_identity_at_the_next_tick() {
    let key = Key::from("Hello World");
    let child = Widget::from(SizedBox::square(10.)).with_key(key.clone());
    let widget = TweenAnimationBuilder::new(100_i32, SECOND, |_, _, child| child.unwrap())
        .begin(0)
        .child(child)
        .into();
    let mut host = Host::mount(widget);
    let mounted = host.tree.element_with_key(&key).unwrap();
    assert_eq!(
        host.tree.element_bounds(mounted).unwrap().size,
        Size::new(10., 10.)
    );
    host.frame(500);
    assert_eq!(host.tree.element_with_key(&key), Some(mounted));
}

fn check_gapless_retarget(initial: Widget, replacement: Widget, values: &Values) {
    let mut host = Host::mount(initial);
    assert_eq!(*values.borrow(), [0]);
    host.frame(500);
    assert_eq!(*values.borrow(), [0, 50]);
    host.update(500, replacement);
    assert_eq!(*values.borrow(), [0, 50, 50]);
    host.frame(1000);
    assert_eq!(*values.borrow(), [0, 50, 50, 175]);
    host.frame(3000);
    assert_eq!(*values.borrow(), [0, 50, 50, 175, 300]);
}

#[test]
fn a_retarget_is_gapless_while_running_forward() {
    let values = Values::default();
    check_gapless_retarget(linear(&values, 0, 100), linear(&values, 200, 300), &values);
}

#[test]
fn preconstructed_tweens_use_the_same_gapless_path() {
    let values = Values::default();
    let first = linear(&values, 0, 100);
    let second = linear(&values, 200, 300);
    check_gapless_retarget(first, second, &values);
}

#[test]
fn successive_retargets_sample_the_current_value_not_the_stale_begin() {
    let values = Values::default();
    let mut host = Host::mount(linear(&values, 0, 100));
    host.frame(500);
    assert_eq!(values.take(), [0, 50]);
    host.update(500, linear(&values, 200, 300));
    host.frame(1000);
    assert_eq!(values.take(), [50, 175]);
    host.update(1000, linear(&values, 400, 501));
    host.frame(1500);
    host.frame(2000);
    assert_eq!(values.take(), [175, 338, 501]);
}

#[test]
fn changing_only_the_curve_after_completion_does_not_restart() {
    let values = Values::default();
    let mut host = Host::mount(linear(&values, 0, 100));
    host.frame(2000);
    assert_eq!(values.take(), [0, 100]);
    host.update(
        2000,
        int(&values, Some(0), 100, SECOND, Curve::EaseInExpo, None),
    );
    assert_eq!(*values.borrow(), [100]);
    host.frame(4000);
    assert_eq!(*values.borrow(), [100]);
}

#[test]
fn setting_the_same_target_after_completion_does_not_restart() {
    let values = Values::default();
    let mut host = Host::mount(linear(&values, 0, 100));
    host.frame(500);
    host.frame(1000);
    assert_eq!(values.take(), [0, 50, 100]);
    host.update(1000, linear(&values, 0, 100));
    host.frame(1500);
    host.frame(2000);
    let observed = values.take();
    assert!(!observed.is_empty());
    assert!(observed.iter().all(|&value| value == 100));
}

#[test]
fn setting_the_same_target_mid_retarget_does_not_restart_its_clock() {
    let values = Values::default();
    let mut host = Host::mount(linear(&values, 0, 100));
    host.frame(500);
    host.update(500, linear(&values, 200, 300));
    host.frame(1000);
    assert_eq!(*values.borrow(), [0, 50, 50, 175]);
    host.update(1000, linear(&values, 200, 300));
    assert_eq!(*values.borrow(), [0, 50, 50, 175, 175]);
    host.frame(1500);
    assert_eq!(*values.borrow(), [0, 50, 50, 175, 175, 300]);
    values.borrow_mut().clear();
    host.frame(3500);
    assert!(values.borrow().iter().all(|&value| value == 300));
}

#[test]
fn nullable_size_with_no_begin_renders_only_the_present_target() {
    let values = Rc::new(RefCell::new(Vec::new()));
    let observed = values.clone();
    let size = Size::new(10., 10.);
    let widget = TweenAnimationBuilder::new(Some(size), SECOND, move |_, value, _| {
        observed.borrow_mut().push(value);
        Placeholder::new()
    })
    .into();
    let mut host = Host::mount(widget);
    assert_eq!(*values.borrow(), [Some(size)]);
    host.frame(2000);
    assert_eq!(*values.borrow(), [Some(size)]);
}

#[test]
fn zero_area_finishes_without_panicking() {
    let widget = TweenAnimationBuilder::new(Some(Size::new(10., 10.)), SECOND, |_, _, _| {
        Placeholder::new()
    })
    .into();
    let mut host = Host::mount_with_constraints(widget, Constraints::tight(Size::ZERO));
    assert_eq!(
        host.tree.element_bounds(host.root).unwrap().size,
        Size::ZERO
    );
    host.frame(2000);
    assert_eq!(
        host.tree.element_bounds(host.root).unwrap().size,
        Size::ZERO
    );
}

#[test]
fn integer_ties_round_away_from_zero_and_nullable_sizes_scale_from_zero() {
    assert_eq!((-2_i32).lerp(&-1, 0.5), -2);
    assert_eq!(1_i32.lerp(&2, 0.5), 2);
    assert_eq!(i32::MIN.lerp(&i32::MAX, 0.5), -1);
    assert_eq!((-2_i64).lerp(&-1, 0.5), -2);
    let target = Some(Size::new(10., 20.));
    assert_eq!(None.lerp(&target, 0.25), Some(Size::new(2.5, 5.)));
    assert_eq!(target.lerp(&None, 0.25), Some(Size::new(7.5, 15.)));
    assert_eq!(None.lerp(&None, 0.25), None);
}

#[test]
fn changing_only_the_curve_mid_flight_re_evaluates_without_restarting_time() {
    let values = Values::default();
    let endings = Rc::new(Cell::new(0));
    let initial = int(&values, Some(0), 100, SECOND, Curve::Linear, Some(&endings));
    let replacement = int(
        &values,
        Some(0),
        100,
        SECOND,
        Curve::EaseInQuad,
        Some(&endings),
    );
    let mut host = Host::mount(initial);
    host.frame(250);
    assert_eq!(*values.borrow(), [0, 25]);
    host.update(250, replacement);
    let at_quarter = (100. * Curve::EaseInQuad.apply(0.25)).round() as i32;
    assert_eq!(*values.borrow(), [0, 25, at_quarter]);
    host.frame(750);
    let at_three_quarters = (100. * Curve::EaseInQuad.apply(0.75)).round() as i32;
    assert_eq!(values.borrow().last(), Some(&at_three_quarters));
    assert_eq!(endings.get(), 0);
    host.frame(1001);
    assert_eq!(values.borrow().last(), Some(&100));
    assert_eq!(endings.get(), 1);
}
