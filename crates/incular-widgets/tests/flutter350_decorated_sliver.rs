//! Whole-sliver paint geometry, order, and real retained child identity.
use std::time::Instant;

use incular_config::{Axis, Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Brush, Decoration, DisplayList, PaintCommand};
use incular_scroll::ScrollController;
use incular_widgets::{
    BoxDecoration, CustomScrollView, DecoratedSliver, DecorationPosition, Directionality,
    GlobalKey, Sliver, SliverCrossAxisGroup, SliverMainAxisGroup, SliverToBoxAdapter, Widget,
    internal::WidgetTree,
};

const EDGE: Color = Color::rgba(10, 19, 111, 255);
const BACKGROUND: Color = Color::rgba(33, 150, 243, 255);
const CHILD: Color = Color::rgba(244, 67, 54, 255);

fn native_decoration() -> Decoration {
    Decoration {
        border: Some(incular_rendering::Border::new(1., EDGE)),
        ..Decoration::default()
    }
}

fn mounted(
    sliver: impl Sliver + 'static,
    axis: Axis,
    reversed: bool,
    controller: ScrollController,
    size: Size,
) -> WidgetTree {
    let viewport = CustomScrollView::new(vec![Box::new(sliver) as Box<dyn Sliver>])
        .scroll_direction(axis)
        .reverse(reversed)
        .controller(controller);
    let scene = Directionality::new(TextDirection::Ltr, viewport);
    let mut tree = WidgetTree::new();
    tree.mount(scene.into()).unwrap();
    tree.layout(Constraints::tight(size)).unwrap();
    tree.update_compositor(Instant::now()).unwrap();
    tree
}

fn border_rects(list: &DisplayList) -> Vec<Rect> {
    let mut transforms = vec![Offset::ZERO];
    let mut result = Vec::new();
    for cmd in list.commands() {
        match cmd {
            PaintCommand::PushTransform { transform } => {
                transforms
                    .push(*transforms.last().unwrap() + transform.transform_point(Offset::ZERO));
            }
            PaintCommand::PopTransform => {
                transforms.pop();
            }
            PaintCommand::Border { rrect, border } if border.color == EDGE => {
                let half = border.width * 0.5;
                result.push(Rect::from_origin_size(
                    rrect.rect.origin + *transforms.last().unwrap() + Offset::new(half, half),
                    Size::new(
                        rrect.rect.size.width - border.width,
                        rrect.rect.size.height - border.width,
                    ),
                ));
            }
            _ => {}
        }
    }
    result
}

#[test]
fn one_full_outline_survives_scrolling_on_each_axis_and_direction() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for reverse in [false, true] {
            let (view_size, child_size) = if axis.is_vertical() {
                (Size::new(100., 300.), Size::new(100., 500.))
            } else {
                (Size::new(300., 100.), Size::new(500., 100.))
            };
            let key = GlobalKey::new();
            let child = Widget::box_(child_size, CHILD).with_key(key);
            let sliver = DecoratedSliver::new(native_decoration(), SliverToBoxAdapter::new(child));
            let controller = ScrollController::new();
            let mut tree = mounted(sliver, axis, reverse, controller.clone(), view_size);
            let before = tree.element_with_key(&key.into()).unwrap();
            controller.jump_to(200.);
            tree.layout(Constraints::tight(view_size)).unwrap();
            tree.update_compositor(Instant::now()).unwrap();
            assert_eq!(tree.element_with_key(&key.into()), Some(before));
            assert_eq!(controller.offset(), 200.);
            let local = if reverse { 0.5 } else { -199.5 };
            let (origin, size) = if axis.is_vertical() {
                (Offset::new(0.5, local), Size::new(99., 499.))
            } else {
                (Offset::new(local, 0.5), Size::new(499., 99.))
            };
            assert_eq!(
                border_rects(&tree.paint()),
                [Rect::from_origin_size(origin, size)],
                "{axis:?}, reverse={reverse}",
            );
        }
    }
}

fn leaf() -> Box<dyn Sliver> {
    Box::new(SliverToBoxAdapter::new(Widget::box_(
        Size::new(100., 100.),
        CHILD,
    )))
}

#[test]
fn main_and_cross_axis_groups_get_one_outline_instead_of_a_border_per_child() {
    for main_axis in [true, false] {
        let decorated = if main_axis {
            DecoratedSliver::new(
                native_decoration(),
                SliverMainAxisGroup::new(vec![leaf(), leaf()]),
            )
        } else {
            DecoratedSliver::new(
                native_decoration(),
                SliverCrossAxisGroup::new(vec![leaf(), leaf()]),
            )
        };
        let controller = ScrollController::new();
        let mut tree = mounted(
            decorated,
            Axis::Vertical,
            false,
            controller,
            Size::new(300., 100.),
        );
        assert_eq!(
            border_rects(&tree.paint()),
            [Rect::from_origin_size(
                Offset::new(0.5, 0.5),
                Size::new(299., if main_axis { 199. } else { 99. })
            )]
        );
    }
}

#[test]
fn box_decoration_paints_in_front_or_behind_without_changing_the_hit_target() {
    let key = GlobalKey::new();
    let view = Size::new(100., 100.);
    let controller = ScrollController::new();
    let scene = |position| {
        let child = Widget::box_(view, CHILD).with_key(key);
        let decorated = DecoratedSliver::new_box(
            BoxDecoration::new().color(BACKGROUND),
            SliverToBoxAdapter::new(child),
        )
        .position(position);
        Widget::from(Directionality::new(
            TextDirection::Ltr,
            CustomScrollView::new(vec![Box::new(decorated) as Box<dyn Sliver>])
                .controller(controller.clone()),
        ))
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(DecorationPosition::Background)).unwrap();
    let mut retained_child = None;
    for position in [
        DecorationPosition::Background,
        DecorationPosition::Foreground,
    ] {
        if retained_child.is_some() {
            tree.update(root, scene(position)).unwrap();
        }
        tree.layout(Constraints::tight(view)).unwrap();
        tree.update_compositor(Instant::now()).unwrap();
        let layer_order: Vec<_> = tree
            .paint()
            .commands()
            .iter()
            .filter_map(|command| {
                let color = match command {
                    PaintCommand::Rect { color, .. }
                    | PaintCommand::RRect {
                        brush: Brush::Solid(color),
                        ..
                    } => *color,
                    _ => return None,
                };
                matches!(color, BACKGROUND | CHILD).then_some(color)
            })
            .collect();
        let expected = match position {
            DecorationPosition::Background => [BACKGROUND, CHILD],
            DecorationPosition::Foreground => [CHILD, BACKGROUND],
        };
        assert_eq!(layer_order, expected);
        let child = tree.element_with_key(&key.into()).unwrap();
        if let Some(previous) = retained_child {
            assert_eq!(child, previous);
        }
        retained_child = Some(child);
        let hit = tree.hit_test(Offset::new(30., 30.)).unwrap();
        assert_eq!(tree.element_for_render(hit), Some(child));
    }
}

#[test]
fn cached_decoration_enters_the_view_with_the_sliver_on_scroll() {
    let view = Size::new(100., 100.);
    let controller = ScrollController::new();
    let prefix = SliverToBoxAdapter::new(Widget::box_(Size::new(100., 150.), CHILD));
    let decorated = DecoratedSliver::new(
        native_decoration(),
        SliverToBoxAdapter::new(Widget::box_(Size::new(100., 200.), BACKGROUND)),
    );
    let viewport = CustomScrollView::new(vec![
        Box::new(prefix) as Box<dyn Sliver>,
        Box::new(decorated),
    ])
    .controller(controller.clone());
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, viewport).into())
        .unwrap();
    tree.layout(Constraints::tight(view)).unwrap();
    tree.update_compositor(Instant::now()).unwrap();
    let count = tree.diagnostics().layouts;

    controller.jump_to(60.);
    tree.update_compositor(Instant::now()).unwrap();
    assert_eq!(tree.diagnostics().layouts, count);
    assert_eq!(
        border_rects(&tree.paint()),
        [Rect::from_origin_size(
            Offset::new(0.5, 90.5),
            Size::new(99., 199.)
        )]
    );
}

#[test]
fn decorated_pinned_header_keeps_its_outline_at_the_viewport_edge() {
    let controller = ScrollController::new();
    let view = Size::new(300., 100.);
    let header = incular_widgets::internal::PinnedHeaderSliver::new(
        incular_widgets::SizedBox::new().width(300.).height(50.),
    );
    let decorated = DecoratedSliver::new(native_decoration(), header);
    let content = SliverToBoxAdapter::new(Widget::box_(Size::new(300., 1000.), CHILD));
    let viewport = CustomScrollView::new(vec![
        Box::new(decorated) as Box<dyn Sliver>,
        Box::new(content),
    ])
    .controller(controller.clone());
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, viewport).into())
        .unwrap();
    for scroll in [0., 200.] {
        if scroll > 0. {
            assert!(controller.jump_to(scroll));
        }
        tree.layout(Constraints::tight(view)).unwrap();
        tree.update_compositor(Instant::now()).unwrap();
        let list = tree.paint();
        assert_eq!(
            border_rects(&list),
            [Rect::from_origin_size(
                Offset::new(0.5, 0.5),
                Size::new(299., 49.)
            )],
        );
        let border = list.commands().iter().position(|command| {
            matches!(command, PaintCommand::Border { border, .. } if border.color == EDGE)
        }).unwrap();
        let content = list
            .commands()
            .iter()
            .position(
                |command| matches!(command, PaintCommand::Rect { color, .. } if *color == CHILD),
            )
            .unwrap();
        assert!(
            border > content,
            "the scrolling content must not cover the pinned outline"
        );
    }
}
