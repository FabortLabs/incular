//! Mounted comparison with Flutter's rendering/sliver_cache_test.dart.

use std::{cell::Cell, rc::Rc};

use incular_config::{Axis, Constraints, EdgeInsets};
use incular_core::{Color, Size};
use incular_runtime::Runtime;
use incular_scroll::{SliverConstraints, SliverGeometry};
use incular_widgets::{
    CustomScrollView, Directionality, ScrollController, Sliver, SliverFixedExtentList, SliverGrid,
    SliverGridDelegate, SliverList, SliverPadding, SliverToBoxAdapter, TextDirection, Widget,
    internal::{RenderSliver, SliverChildId, SliverLayout, SliverScrollDependency},
};

type Sample = (SliverConstraints, SliverGeometry);
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
        self.latest.set(Some((constraints, result.geometry)));
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
    let (c, g) = sample.get().expect("sliver ran during the mounted layout");
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
