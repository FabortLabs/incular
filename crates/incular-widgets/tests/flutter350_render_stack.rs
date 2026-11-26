use incular_config::{Clip, Constraints, MainAxisAlignment, TextDirection};
use incular_core::{Color, Offset, Size};
use incular_layout::Positioned as ParentData;
use incular_rendering::{ClipCoverage, PaintCommand};
use incular_widgets::{
    Positioned, Row, Stack, Widget,
    internal::{Key, WidgetTree},
};

fn rect(size: Size, key: Key) -> Widget {
    Widget::box_(size, Color::WHITE).with_key(key)
}

fn stack(children: impl IntoIterator<Item = Widget>) -> Stack {
    Stack::new(children).text_direction(TextDirection::Ltr)
}

fn mount(widget: impl Into<Widget>, constraints: Constraints) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).unwrap();
    tree.layout(constraints).unwrap();
    tree
}

#[test]
fn positioned_parent_data_prefers_opposing_edges_and_clamps_negative_sizes() {
    let size = Size::new(800., 600.);
    let mut data = ParentData::new();
    assert!(!data.is_positioned());
    data.width = Some(-100.);
    assert!(data.is_positioned());
    assert_eq!(
        data.positioned_child_constraints(size),
        Constraints::tight_for(Some(0.), None)
    );
    data.width = Some(100.);
    assert_eq!(
        data.positioned_child_constraints(size),
        Constraints::tight_for(Some(100.), None)
    );
    data.left = Some(0.);
    data.right = Some(0.);
    assert_eq!(
        data.positioned_child_constraints(size),
        Constraints::tight_for(Some(800.), None)
    );
    data.height = Some(-100.);
    assert_eq!(
        data.positioned_child_constraints(size),
        Constraints::tight(Size::new(800., 0.))
    );
    data.height = Some(100.);
    assert_eq!(
        data.positioned_child_constraints(size),
        Constraints::tight(Size::new(800., 100.))
    );
    data.top = Some(0.);
    data.bottom = Some(0.);
    assert_eq!(
        data.positioned_child_constraints(size),
        Constraints::tight(Size::new(800., 600.))
    );
    data.bottom = Some(1000.);
    assert_eq!(
        data.positioned_child_constraints(size),
        Constraints::tight(Size::new(800., 0.))
    );
}

#[test]
fn four_zero_offsets_make_a_positioned_child_fill_the_nonpositioned_extent() {
    let size = Size::new(100., 100.);
    let red = Key::Value(10);
    let green = Key::Value(11);
    let tree = mount(
        stack([
            rect(size, red.clone()),
            Positioned::fill(rect(Size::ZERO, green.clone())).into(),
        ]),
        Constraints::unbounded(),
    );
    assert_eq!(tree.root_layout_size(), Some(size));
    for key in [&red, &green] {
        let bounds = tree
            .element_bounds(tree.element_with_key(key).unwrap())
            .unwrap();
        assert_eq!(bounds.origin, Offset::ZERO);
        assert_eq!(bounds.size, size);
    }
}

#[test]
fn empty_stack_fills_a_bounded_box_and_uses_minimums_when_one_axis_is_unbounded() {
    let tight = Constraints::tight(Size::new(100., 100.));
    assert_eq!(
        mount(stack([]), tight).root_layout_size(),
        Some(Size::new(100., 100.))
    );
    for (constraints, expected) in [
        (Constraints::new(5., 100., 8., 200.), Size::new(100., 200.)),
        (
            Constraints::new(5., f32::INFINITY, 8., 200.),
            Size::new(5., 8.),
        ),
        (
            Constraints::new(5., 100., 8., f32::INFINITY),
            Size::new(5., 8.),
        ),
    ] {
        let primitive =
            incular_layout::layout_stack(constraints, &[], incular_layout::Stack::default());
        assert_eq!(primitive.size, expected);
        assert_eq!(
            mount(stack([]), constraints).root_layout_size(),
            Some(expected)
        );
    }
}

#[test]
fn an_overflowing_stack_uses_hard_edge_by_default_and_honors_all_explicit_clips() {
    let positioned = Positioned::new(rect(Size::new(200., 200.), Key::Value(1)))
        .left(0.)
        .right(0.);
    for (configured, expected) in [
        (None, Some(ClipCoverage::HardEdge)),
        (Some(Clip::None), None),
        (Some(Clip::HardEdge), Some(ClipCoverage::HardEdge)),
        (Some(Clip::AntiAlias), Some(ClipCoverage::AntiAlias)),
        (
            Some(Clip::AntiAliasWithSaveLayer),
            Some(ClipCoverage::AntiAliasWithSaveLayer),
        ),
    ] {
        let mut view = stack([positioned.clone().into()]);
        if let Some(clip) = configured {
            view = view.clip_behavior(clip);
        }
        let mut tree = mount(view, Constraints::at_most(100., 100.));
        let list = tree.paint();
        let actual: Vec<_> = list
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::PushClip { .. } => Some(ClipCoverage::HardEdge),
                PaintCommand::PushClipRRect { coverage, .. } => Some(*coverage),
                _ => None,
            })
            .collect();
        assert_eq!(
            actual,
            expected.into_iter().collect::<Vec<_>>(),
            "{configured:?}"
        );
    }
}

#[test]
fn an_empty_stack_inside_a_tight_flex_can_lay_out_without_error() {
    let wrapper = Row::new([stack([])]).main_axis_alignment(MainAxisAlignment::SpaceEvenly);
    let tree = mount(wrapper, Constraints::tight(Size::new(100., 100.)));
    assert_eq!(tree.root_layout_size(), Some(Size::new(100., 100.)));
}
