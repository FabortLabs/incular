//! Delegate-driven layouts: Flutter's `CustomSingleChildLayout`,
//! `CustomMultiChildLayout`/`LayoutId` and `Flow`, plus
//! `PositionedDirectional` and `IgnoreBaseline`.

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size, Transform as CoreTransform};
use incular_rendering::{DisplayList, PaintCommand};
use incular_widgets::internal::{ElementId, RenderKind, WidgetTree};
use incular_widgets::{
    CustomMultiChildLayout, CustomSingleChildLayout, Directionality, Flow, FlowDelegate,
    FlowPaintingContext, IgnoreBaseline, LayoutId, MultiChildLayoutContext,
    MultiChildLayoutDelegate, PositionedDirectional, SingleChildLayoutDelegate, Stack, Text,
    Widget,
};

fn painted_rects(list: &DisplayList) -> Vec<Rect> {
    let mut transforms = vec![CoreTransform::IDENTITY];
    let mut rects = Vec::new();
    for command in list.commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                let current = *transforms.last().unwrap();
                transforms.push(current.then(*transform));
            }
            PaintCommand::PopTransform => {
                transforms.pop();
            }
            PaintCommand::Rect { rect, .. } => {
                rects.push(transforms.last().unwrap().transform_rect_bbox(*rect));
            }
            _ => {}
        }
    }
    rects
}

fn mount(widget: Widget) -> (WidgetTree, ElementId) {
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget).expect("mount");
    layout(&mut tree);
    (tree, root)
}

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::loose(Size::new(200., 200.)))
        .expect("layout");
}

fn square(side: f32) -> Widget {
    Widget::box_(Size::new(side, side), Color::WHITE)
}

/// Centers the child in a fixed box under loosened constraints.
struct Centered {
    size: Size,
}

impl SingleChildLayoutDelegate for Centered {
    fn get_size(&self, _: Constraints) -> Size {
        self.size
    }

    fn get_constraints_for_child(&self, constraints: Constraints) -> Constraints {
        constraints.loosen()
    }

    fn get_position_for_child(&self, size: Size, child_size: Size) -> Offset {
        Offset::new(
            (size.width - child_size.width) / 2.,
            (size.height - child_size.height) / 2.,
        )
    }

    fn should_relayout(&self, old: &Self) -> bool {
        self.size != old.size
    }
}

#[test]
fn single_child_layout_sizes_constrains_and_places_its_child() {
    let centered = |width: f32| {
        CustomSingleChildLayout::new(
            Centered {
                size: Size::new(width, 50.),
            },
            square(30.),
        )
        .into()
    };
    let (mut tree, root) = mount(centered(100.));
    assert_eq!(
        tree.element_bounds(root).unwrap().size,
        Size::new(100., 50.)
    );
    assert_eq!(
        painted_rects(&tree.paint()),
        [Rect::from_origin_size(
            Offset::new(35., 10.),
            Size::new(30., 30.)
        )]
    );

    // An equal delegate does not relayout; a changed one does.
    let layouts = tree.diagnostics().layouts;
    tree.update(root, centered(100.)).expect("update");
    layout(&mut tree);
    assert_eq!(tree.diagnostics().layouts, layouts);
    tree.update(root, centered(160.)).expect("update");
    layout(&mut tree);
    assert_eq!(
        painted_rects(&tree.paint()),
        [Rect::from_origin_size(
            Offset::new(65., 10.),
            Size::new(30., 30.)
        )]
    );
}

/// Lays `leading` out at its natural size and `trailing` in the rest of
/// the row, as Flutter's `CustomMultiChildLayout` example does.
struct Row;

impl MultiChildLayoutDelegate for Row {
    fn get_size(&self, _: Constraints) -> Size {
        Size::new(150., 40.)
    }

    fn perform_layout(&self, size: Size, children: &mut MultiChildLayoutContext<'_>) {
        assert!(children.has_child("leading"));
        assert!(!children.has_child("missing"));
        let leading = children.layout_child("leading", Constraints::loose(size));
        let trailing = children.layout_child(
            "trailing",
            Constraints::tight(Size::new(size.width - leading.width, 10.)),
        );
        children.position_child(
            "trailing",
            Offset::new(leading.width, size.height - trailing.height),
        );
    }

    fn should_relayout(&self, _: &Self) -> bool {
        false
    }
}

#[test]
fn multi_child_layout_places_children_by_id() {
    let (mut tree, _) = mount(
        CustomMultiChildLayout::new(
            Row,
            [
                Widget::from(LayoutId::new("trailing", square(0.))),
                LayoutId::new("leading", square(20.)).into(),
            ],
        )
        .into(),
    );
    assert_eq!(
        painted_rects(&tree.paint()),
        [
            Rect::from_origin_size(Offset::new(20., 30.), Size::new(130., 10.)),
            Rect::from_origin_size(Offset::ZERO, Size::new(20., 20.)),
        ]
    );
}

/// Paints the first child moved right, skips the second, and paints the
/// third scaled by two at (100, 100).
struct Menu {
    shift: f32,
}

impl FlowDelegate for Menu {
    fn paint_children(&self, context: &mut FlowPaintingContext<'_>) {
        assert_eq!(context.child_count(), 3);
        assert_eq!(context.get_child_size(0), Some(Size::new(20., 20.)));
        context.paint_child(
            0,
            CoreTransform::translation(Offset::new(self.shift, 0.)),
            1.,
        );
        context.paint_child(
            2,
            CoreTransform::translation(Offset::new(100., 100.)).then(CoreTransform::scale(2.)),
            0.5,
        );
    }

    fn should_repaint(&self, old: &Self) -> bool {
        self.shift != old.shift
    }
}

fn menu(shift: f32) -> Widget {
    Flow::new(Menu { shift }, [square(20.), square(20.), square(10.)]).into()
}

#[test]
fn flow_paints_children_with_the_delegates_transforms() {
    let (mut tree, root) = mount(menu(50.));
    // The flow takes the biggest size; every child sits at the origin.
    assert_eq!(
        tree.element_bounds(root).unwrap().size,
        Size::new(200., 200.)
    );
    let expected = [
        Rect::from_origin_size(Offset::new(50., 0.), Size::new(20., 20.)),
        Rect::from_origin_size(Offset::new(100., 100.), Size::new(20., 20.)),
    ];
    assert_eq!(painted_rects(&tree.paint()), expected);
    // Children are hit where they paint; the unpainted second child, at the
    // origin, is not hit at all.
    let hits_box = |tree: &WidgetTree, point| {
        tree.hit_test(point)
            .and_then(|render| tree.render_object_kind(render))
            .is_some_and(|kind| matches!(kind, RenderKind::Box { .. }))
    };
    assert!(hits_box(&tree, Offset::new(60., 10.)));
    assert!(hits_box(&tree, Offset::new(115., 115.)));
    assert!(!hits_box(&tree, Offset::new(5., 5.)));
    // The third child paints at half opacity.
    let alphas: Vec<f32> = tree
        .paint()
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushOpacity { alpha, .. } => Some(*alpha),
            _ => None,
        })
        .collect();
    assert!(alphas.contains(&0.5), "{alphas:?}");

    // An equal delegate keeps the paint; a new shift moves the child.
    tree.update(root, menu(50.)).expect("update");
    layout(&mut tree);
    assert_eq!(painted_rects(&tree.paint()), expected);
    tree.update(root, menu(80.)).expect("update");
    layout(&mut tree);
    assert_eq!(painted_rects(&tree.paint())[0].origin, Offset::new(80., 0.));
}

#[test]
fn positioned_directional_resolves_start_against_the_direction() {
    let positioned = |direction| {
        Directionality::new(
            direction,
            Stack::new([Widget::from(
                PositionedDirectional::new(Widget::box_(Size::ZERO, Color::WHITE))
                    .start(10.)
                    .top(0.)
                    .width(20.)
                    .height(20.),
            )]),
        )
        .into()
    };
    let (mut tree, _) = mount(positioned(TextDirection::Ltr));
    assert_eq!(painted_rects(&tree.paint())[0].origin, Offset::new(10., 0.));
    let (mut tree, _) = mount(positioned(TextDirection::Rtl));
    // 200 - 10 - 20.
    assert_eq!(
        painted_rects(&tree.paint())[0].origin,
        Offset::new(170., 0.)
    );
}

#[test]
fn ignore_baseline_hides_the_child_baseline() {
    /// The render objects from `root` down its first children.
    fn chain(tree: &WidgetTree, root: ElementId) -> Vec<Option<f32>> {
        let mut baselines = Vec::new();
        let mut current = Some(root);
        while let Some(element) = current {
            if let Some(render) = tree.render_id(element) {
                baselines.push(tree.baseline(render));
            }
            current = tree
                .children(element)
                .and_then(|children| children.first().copied());
        }
        baselines
    }
    let text = || Text::new("Baseline");
    let (tree, root) = mount(Directionality::new(TextDirection::Ltr, text()).into());
    // The text reports its baseline.
    assert!(chain(&tree, root).last().unwrap().is_some());
    let (tree, root) =
        mount(IgnoreBaseline::new(Directionality::new(TextDirection::Ltr, text())).into());
    let baselines = chain(&tree, root);
    // The ignoring wrapper reports none while the text inside still has one.
    assert_eq!(baselines[0], None);
    assert!(baselines.last().unwrap().is_some());
}
