//! Mounted comparison with Flutter's rendering/sliver_cache_test.dart.

use std::{cell::Cell, rc::Rc};

use incular_config::{Axis, Constraints, EdgeInsets};
use incular_core::{Color, Size};
use incular_runtime::Runtime;
use incular_scroll::{ScrollPhysics, SliverConstraints, SliverGeometry};
use incular_widgets::{
    CustomScrollView, Directionality, ScrollController, Sliver, SliverFillRemaining,
    SliverFixedExtentList, SliverGrid, SliverGridDelegate, SliverList, SliverPadding,
    SliverToBoxAdapter, TextDirection, Widget,
    internal::{RenderSliver, SliverChildId, SliverLayout, SliverScrollDependency},
};

type Sample = (SliverConstraints, SliverGeometry, Option<Constraints>);
type Samples = Rc<Cell<Option<Sample>>>;
type Expected = (f32, f32, f32, f32, f32, f32, bool);

struct Record {
    sliver: Box<dyn Sliver>,
    latest: Samples,
}

impl Record {
    fn new(sliver: impl Sliver + 'static) -> (Self, Samples) {
        let latest = Rc::new(Cell::new(None));
        (
            Self {
                sliver: Box::new(sliver),
                latest: latest.clone(),
            },
            latest,
        )
    }
}

impl Sliver for Record {
    fn build(&self, ctl: &ScrollController) -> Widget {
        self.sliver.build(ctl)
    }

    fn create_render_sliver(
        &self,
        ctl: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(RecordRender {
            delegate: self.sliver.create_render_sliver(ctl, axis, reverse),
            latest: self.latest.clone(),
        })
    }
}

struct RecordRender {
    delegate: Box<dyn RenderSliver>,
    latest: Samples,
}

impl RenderSliver for RecordRender {
    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let result = self.delegate.perform_layout(constraints);
        self.latest.set(Some((
            constraints,
            result.geometry,
            result.children.first().map(|child| child.constraints),
        )));
        result
    }

    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        // Capture layout even where a production viewport could reuse the
        // exact same cache window and just translate its layers.
        SliverScrollDependency::ScrollOffset
    }

    fn set_child_extent(&mut self, id: SliverChildId, extent: f32) -> bool {
        self.delegate.set_child_extent(id, extent)
    }

    fn child_count(&self) -> Option<usize> {
        self.delegate.child_count()
    }

    fn revision(&self) -> u64 {
        self.delegate.revision()
    }
}

fn row(id: usize) -> Widget {
    Widget::box_(Size::new(400., 100.), Color::WHITE).with_key(id as u64)
}

fn mount(slivers: Vec<Box<dyn Sliver>>, controller: &ScrollController) -> Runtime {
    let viewport = CustomScrollView::new(slivers)
        .cache_extent(250.)
        .controller(controller.clone());
    Runtime::new(Directionality::new(TextDirection::Ltr, viewport).into()).unwrap()
}

fn at(runtime: &mut Runtime, ctl: &ScrollController, offset: f32) {
    let _ = ctl.jump_to(offset);
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    assert_eq!(ctl.offset(), offset);
}

fn check(sample: &Samples, expected: Expected, context: impl std::fmt::Debug) {
    let (c, g, _) = sample.get().expect("sliver ran during the mounted layout");
    assert_eq!(
        (
            c.cache_origin,
            c.remaining_paint_extent,
            c.remaining_cache_extent,
            c.scroll_offset,
            g.paint_extent,
            g.cache_extent,
            g.visible
        ),
        expected,
        "{context:?}"
    );
}

#[test]
fn sequential_box_slivers_get_the_exact_paint_and_cache_budgets() {
    let ctl = ScrollController::new();
    let mut samples = Vec::new();
    let slivers = (0..30)
        .map(|index| -> Box<dyn Sliver> {
            let (sliver, sample) = Record::new(SliverToBoxAdapter::new(row(index)));
            samples.push(sample);
            Box::new(sliver)
        })
        .collect();
    let mut runtime = mount(slivers, &ctl);
    for (offset, checks) in [
        (
            0.,
            vec![
                (0, (0., 600., 850., 0., 100., 100., true)),
                (5, (0., 100., 350., 0., 100., 100., true)),
                (6, (0., 0., 250., 0., 0., 100., false)),
                (8, (0., 0., 50., 0., 0., 50., false)),
                (9, (0., 0., 0., 0., 0., 0., false)),
            ],
        ),
        (
            50.,
            vec![
                (0, (-50., 600., 900., 50., 50., 100., true)),
                (6, (0., 50., 300., 0., 50., 100., true)),
                (7, (0., 0., 200., 0., 0., 100., false)),
                (8, (0., 0., 100., 0., 0., 100., false)),
                (9, (0., 0., 0., 0., 0., 0., false)),
            ],
        ),
        (
            150.,
            vec![
                (0, (-150., 600., 1000., 150., 0., 100., false)),
                (1, (-50., 600., 900., 50., 50., 100., true)),
            ],
        ),
        (
            1000.,
            vec![
                (0, (-250., 600., 1100., 1000., 0., 0., false)),
                (7, (-250., 600., 1100., 300., 0., 50., false)),
                (9, (-100., 600., 950., 100., 0., 100., false)),
                (10, (0., 600., 850., 0., 100., 100., true)),
            ],
        ),
    ] {
        at(&mut runtime, &ctl, offset);
        for (i, expected) in checks {
            check(&samples[i], expected, (offset, i));
        }
    }
}

fn check_indexed_cache(descriptor: impl Sliver + 'static, lanes: u64) {
    let ctl = ScrollController::new();
    let (delegate, sample) = Record::new(descriptor);
    let mut runtime = mount(vec![Box::new(delegate)], &ctl);
    for (offset, origin, budget, used, first, end) in [
        (0., 0., 850., 850., 0, 9),
        (50., -50., 900., 900., 0, 9),
        (1500., -250., 1100., 1100., 12, 24),
        (2400., -250., 1100., 850., 21, 30),
    ] {
        at(&mut runtime, &ctl, offset);
        check(
            &sample,
            (origin, 600., budget, offset, 600., used, true),
            offset,
        );
        let mounted = (0..30 * lanes)
            .filter(|&id| runtime.tree().element_with_key(&id.into()).is_some())
            .collect::<Vec<_>>();
        assert_eq!(
            mounted,
            (first * lanes..end * lanes).collect::<Vec<_>>(),
            "mounted rows at {offset}"
        );
    }
}

#[test]
fn fixed_extent_list_paint_cache_and_eviction_are_consistent() {
    check_indexed_cache(SliverFixedExtentList::new(30, 100., row), 1);
}

#[test]
fn variable_extent_list_paint_cache_and_eviction_are_consistent() {
    check_indexed_cache(SliverList::builder(30, row), 1);
}

#[test]
fn grid_paint_cache_and_eviction_are_consistent() {
    check_indexed_cache(
        SliverGrid::builder(
            60,
            SliverGridDelegate::fixed_cross_axis_count(2).main_axis_extent(100.),
            row,
        ),
        2,
    );
}

#[test]
fn padding_accounts_for_both_its_own_and_the_child_cache_geometry() {
    let ctl = ScrollController::new();
    let mut samples = Vec::new();
    let slivers = (0..30)
        .map(|i| -> Box<dyn Sliver> {
            let (child, inner) = Record::new(SliverToBoxAdapter::new(row(i)));
            let (padding, outer) = Record::new(SliverPadding::new(
                EdgeInsets::only(0., 50., 0., 50.),
                child,
            ));
            samples.push((outer, inner));
            Box::new(padding)
        })
        .collect();
    let mut runtime = mount(slivers, &ctl);
    for (offset, checks) in [
        (
            0.,
            vec![
                (
                    0,
                    (0., 600., 850., 0., 200., 200., true),
                    (0., 550., 800., 0., 100., 100., true),
                ),
                (
                    2,
                    (0., 200., 450., 0., 200., 200., true),
                    (0., 150., 400., 0., 100., 100., true),
                ),
                (
                    3,
                    (0., 0., 250., 0., 0., 200., false),
                    (0., 0., 200., 0., 0., 100., false),
                ),
                (
                    4,
                    (0., 0., 50., 0., 0., 50., false),
                    (0., 0., 0., 0., 0., 0., false),
                ),
            ],
        ),
        (
            50.,
            vec![(
                0,
                (-50., 600., 900., 50., 150., 200., true),
                (0., 600., 850., 0., 100., 100., true),
            )],
        ),
        (
            5400.,
            vec![
                (
                    0,
                    (-250., 600., 1100., 5400., 0., 0., false),
                    (-200., 600., 1100., 5350., 0., 0., false),
                ),
                (
                    25,
                    (-250., 600., 1100., 400., 0., 50., false),
                    (-200., 600., 1100., 350., 0., 0., false),
                ),
                (
                    26,
                    (-200., 600., 1050., 200., 0., 200., false),
                    (-150., 600., 1000., 150., 0., 100., false),
                ),
                (
                    29,
                    (0., 200., 450., 0., 200., 200., true),
                    (0., 150., 400., 0., 100., 100., true),
                ),
            ],
        ),
    ] {
        at(&mut runtime, &ctl, offset);
        for (i, outer, inner) in checks {
            check(&samples[i].0, outer, (offset, i, "padding"));
            check(&samples[i].1, inner, (offset, i, "box"));
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Fill {
    Static,
    Stretch,
    ScrollBody,
}

/// scroll offset, paint/cache budgets, reported paint/cache extents, child min/max height
type FillCheck = (f32, f32, f32, f32, f32, f32, f32);

fn check_fill(mode: Fill, preceding: f32, expected: &[FillCheck]) {
    let controller = ScrollController::new();
    let (first, first_sample) = Record::new(SliverToBoxAdapter::new(Widget::box_(
        Size::new(400., preceding),
        Color::WHITE,
    )));
    let (fill, fill_sample) = Record::new(
        SliverFillRemaining::new(Widget::box_(Size::new(100., 100.), Color::WHITE))
            .has_scroll_body(matches!(mode, Fill::ScrollBody))
            .fill_overscroll(matches!(mode, Fill::Stretch)),
    );
    let slivers: [Box<dyn Sliver>; 2] = [Box::new(first), Box::new(fill)];
    let physics = ScrollPhysics::BOUNCING.always_scrollable();
    let viewport = CustomScrollView::new(slivers)
        .cache_extent(250.)
        .physics(physics)
        .controller(controller.clone());
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, viewport).into()).unwrap();
    let bounds = Constraints::tight(Size::new(800., 600.));
    for &(offset, paint_budget, cache_budget, paint, cache, min_h, max_h) in expected {
        let furthest = controller.max_offset();
        if offset > furthest {
            let _ = controller.jump_to(furthest);
            let _ = controller.apply_physics(physics, offset - furthest);
        } else {
            let _ = controller.jump_to(offset);
        }
        runtime.run_frame(bounds).unwrap();
        assert_eq!(
            controller.offset(),
            offset,
            "{mode:?} preceding {preceding}"
        );
        if offset == 0. {
            check(
                &first_sample,
                (0., 600., 850., 0., preceding.min(600.), preceding, true),
                (mode, preceding),
            );
        }
        check(
            &fill_sample,
            (0., paint_budget, cache_budget, 0., paint, cache, paint > 0.),
            (mode, preceding, offset),
        );
        let (_, _, actual) = fill_sample.get().unwrap();
        assert_eq!(
            actual,
            Some(Constraints::new(800., 800., min_h, max_h)),
            "{mode:?} preceding={preceding}, offset={offset}"
        );
    }
}

#[test]
fn non_scrollable_fill_keeps_its_intrinsic_size_while_entering_and_overscrolling() {
    check_fill(
        Fill::Static,
        400.,
        &[
            (0., 200., 450., 200., 200., 200., 200.),
            (50., 250., 500., 200., 200., 200., 200.),
        ],
    );
    check_fill(
        Fill::Static,
        850.,
        &[
            (0., 0., 0., 0., 0., 100., 100.),
            (250., 0., 250., 0., 100., 100., 100.),
            (300., 50., 300., 50., 100., 100., 100.),
            (350., 100., 350., 100., 100., 100., 100.),
            (400., 150., 400., 100., 100., 100., 100.),
        ],
    );
}

#[test]
fn stretching_fill_changes_paint_budget_but_not_the_cached_extent() {
    check_fill(
        Fill::Stretch,
        400.,
        &[
            (0., 200., 450., 200., 200., 200., 200.),
            (50., 250., 500., 250., 200., 200., 250.),
        ],
    );
    check_fill(
        Fill::Stretch,
        850.,
        &[
            (0., 0., 0., 0., 0., 100., 100.),
            (250., 0., 250., 0., 100., 100., 100.),
            (300., 50., 300., 50., 100., 100., 100.),
            (350., 100., 350., 100., 100., 100., 100.),
            (400., 150., 400., 150., 100., 100., 150.),
        ],
    );
}

#[test]
fn scrolling_fill_uses_loose_cached_height_and_tight_visible_height() {
    check_fill(
        Fill::ScrollBody,
        400.,
        &[
            (0., 200., 450., 200., 450., 200., 200.),
            (50., 250., 500., 250., 500., 250., 250.),
            (400., 600., 850., 600., 600., 600., 600.),
        ],
    );
    check_fill(
        Fill::ScrollBody,
        850.,
        &[
            (0., 0., 0., 0., 0., 0., 0.),
            (250., 0., 250., 0., 250., 0., 250.),
            (300., 50., 300., 50., 300., 50., 50.),
            (850., 600., 850., 600., 600., 600., 600.),
        ],
    );
}

#[test]
fn zero_height_fill_can_have_paint_budget_without_painting_area_or_exposing_the_last_row_early() {
    use std::time::Duration;

    use incular_core::{
        Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, time::Instant,
    };
    use incular_rendering::{DisplayList, PaintCommand};
    use incular_semantics::SemanticRole;
    use incular_widgets::{RawPointerEvent, internal::ExplicitSemantics};

    for bouncing in [true, false] {
        let ctl = ScrollController::new();
        let fill_color = Color::rgba(0, 255, 0, 255);
        let list = SliverFixedExtentList::new(5, 150., |i| {
            Widget::box_(Size::ZERO, Color::rgba(255, 0, 0, 255)).semantics(
                ExplicitSemantics::new(SemanticRole::GenericContainer).label(i.to_string()),
            )
        });
        let (fill, sample) = Record::new(
            SliverFillRemaining::new(Widget::box_(Size::ZERO, fill_color).with_key("zero fill"))
                .has_scroll_body(false)
                .fill_overscroll(true),
        );
        let slivers: [Box<dyn Sliver>; 2] = [Box::new(list), Box::new(fill)];
        let viewport = CustomScrollView::new(slivers)
            .controller(ctl.clone())
            .physics(if bouncing {
                ScrollPhysics::BOUNCING
            } else {
                ScrollPhysics::CLAMPING
            });
        let mut rt =
            Runtime::new(Directionality::new(TextDirection::Ltr, viewport).into()).unwrap();
        let bounds = Constraints::tight(Size::new(800., 600.));
        let now = Instant::now();
        let check = |rt: &Runtime, display: &DisplayList, pixels, paint, row_visible| {
            assert_eq!(ctl.offset(), pixels, "bounce={bouncing}");
            let (_, geometry, _) = sample.get().unwrap();
            assert_eq!(
                (geometry.paint_extent, geometry.visible),
                (paint, paint > 0.)
            );
            let tree = rt.tree();
            let fill = tree.element_with_key(&"zero fill".into()).unwrap();
            assert_eq!(tree.element_bounds(fill).unwrap().size, Size::new(800., 0.));
            assert!(!display.commands().iter().any(|cmd| matches!(
                cmd,
                PaintCommand::Rect { color, rect } if *color == fill_color && rect.size.height > 0.
            )));
            assert_eq!(
                tree.semantics()
                    .iter()
                    .any(|(_, node)| { node.label.as_deref() == Some("4") && !node.state.hidden }),
                row_visible
            );
        };
        let (display, _) = rt.run_frame_at(bounds, now).unwrap();
        check(&rt, &display, 0., 0., false);
        ctl.jump_to(ctl.max_offset());
        let (display, _) = rt
            .run_frame_at(bounds, now + Duration::from_millis(16))
            .unwrap();
        check(&rt, &display, 150., 0., true);
        let track = |rt: &mut Runtime, phase, y, ms| {
            let _ = rt.handle_raw_pointer_event(RawPointerEvent::new(
                1,
                0,
                PointerDeviceKind::Touch,
                if phase == PointerPhase::Up {
                    0
                } else {
                    PRIMARY_POINTER_BUTTON
                },
                Offset::new(400., y),
                phase,
                now + Duration::from_millis(ms),
            ));
        };
        track(&mut rt, PointerPhase::Down, 300., 20);
        track(&mut rt, PointerPhase::Move, 250., 36);
        let (display, _) = rt
            .run_frame_at(bounds, now + Duration::from_millis(36))
            .unwrap();
        check(
            &rt,
            &display,
            if bouncing { 200. } else { 150. },
            if bouncing { 50. } else { 0. },
            true,
        );
        track(&mut rt, PointerPhase::Up, 250., 180);
        let mut settled = false;
        for n in 0..150 {
            let (display, stats) = rt
                .run_frame_at(bounds, now + Duration::from_millis(180 + n * 16))
                .unwrap();
            if !stats.requested_another_frame && (ctl.offset() - 150.).abs() < f32::EPSILON {
                check(&rt, &display, 150., 0., true);
                settled = true;
                break;
            }
        }
        assert!(settled, "released spring must settle");
    }
}
