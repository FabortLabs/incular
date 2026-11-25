use incular_config::Constraints;
use incular_core::{Offset, Size};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{Positioned, SizedBox, Stack, Widget};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};

#[test]
fn empty_positioned_data_participates_in_retained_stack_sizing() {
    let frame = |left| {
        let mut positioned = Positioned::new(SizedBox::new().width(40.).height(20.));
        if let Some(left) = left {
            positioned = positioned.left(left);
        }
        Widget::from(Stack::new([positioned]))
    };
    let mut tree = WidgetTree::new();
    let stack = tree.mount(frame(None)).unwrap();
    tree.layout(Constraints::loose(VIEW)).unwrap();
    let wrapper = tree.children(stack).unwrap()[0];
    let child = tree.children(wrapper).unwrap()[0];
    assert_eq!(
        tree.element_bounds(stack).unwrap().size,
        Size::new(40., 20.)
    );

    tree.update(stack, frame(Some(5.))).unwrap();
    tree.layout(Constraints::loose(VIEW)).unwrap();
    assert_eq!(tree.element_bounds(stack).unwrap().size, VIEW);
    assert_eq!(
        tree.element_bounds(child).unwrap().origin,
        Offset::new(5., 0.)
    );
    assert_eq!(tree.children(wrapper).unwrap()[0], child);

    tree.update(stack, frame(None)).unwrap();
    tree.layout(Constraints::loose(VIEW)).unwrap();
    assert_eq!(
        tree.element_bounds(stack).unwrap().size,
        Size::new(40., 20.)
    );
    assert_eq!(tree.children(wrapper).unwrap()[0], child);
}

use incular_config::{Alignment, AlignmentDirectional};
use incular_core::Rect;
use incular_widgets::internal::Key;
use incular_widgets::{
    Center, Directionality, IndexedStack, Placeholder, PositionedDirectional, RelativeRect,
    TextDirection::{self, Ltr, Rtl},
};

const STACK: Key = Key::Value(100);
const FIRST: Key = Key::Value(101);
const SECOND: Key = Key::Value(102);

fn named(widget: impl Into<Widget>, key: Key) -> Widget {
    widget.into().with_key(key)
}

fn square(size: f32, key: Key) -> Widget {
    named(SizedBox::square(size), key)
}

fn bounds(tree: &WidgetTree, key: Key) -> Rect {
    tree.element_bounds(tree.element_with_key(&key).unwrap())
        .unwrap()
}

fn mount(widget: impl Into<Widget>) -> (WidgetTree, incular_widgets::internal::ElementId) {
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget.into()).unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    (tree, root)
}

fn directional_alignment_case(direction: TextDirection, second_offset: Offset) {
    let frame = |align: incular_widgets::AlignmentGeometry| {
        Directionality::new(
            direction,
            Center::new(named(
                Stack::new([square(20., FIRST), square(10., SECOND)]).alignment(align),
                STACK,
            )),
        )
    };
    let (mut tree, root) = mount(frame(Alignment::CENTER.into()));
    let first = tree.element_with_key(&FIRST).unwrap();
    let second = tree.element_with_key(&SECOND).unwrap();
    let local = |tree: &WidgetTree, key| bounds(tree, key).origin - bounds(tree, STACK).origin;
    assert_eq!(local(&tree, FIRST), Offset::ZERO);
    assert_eq!(local(&tree, SECOND), Offset::new(5., 5.));

    tree.update(root, frame(AlignmentDirectional::BOTTOM_END.into()).into())
        .unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(tree.element_with_key(&FIRST), Some(first));
    assert_eq!(tree.element_with_key(&SECOND), Some(second));
    assert_eq!(local(&tree, FIRST), Offset::ZERO);
    assert_eq!(local(&tree, SECOND), second_offset);
}

#[test]
fn nonpositioned_bottom_end_tracks_ltr() {
    directional_alignment_case(Ltr, Offset::new(10., 10.));
}

#[test]
fn nonpositioned_bottom_end_tracks_rtl() {
    directional_alignment_case(Rtl, Offset::new(0., 10.));
}

#[test]
fn explicit_positioned_direction_ignores_the_ambient_direction() {
    let frame = |direction| {
        Directionality::new(
            Ltr,
            Stack::new([Positioned::directional(
                direction,
                PositionedDirectional::new(named(SizedBox::new().width(75.).height(175.), FIRST))
                    .start(50.),
            )]),
        )
    };
    let (mut tree, root) = mount(frame(Rtl));
    assert_eq!(bounds(&tree, FIRST).origin, Offset::new(675., 0.));
    tree.update(root, frame(Ltr).into()).unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(bounds(&tree, FIRST).origin, Offset::new(50., 0.));
}

#[test]
fn ambient_positioned_direction_updates_a_retained_child() {
    let frame = |direction| {
        Directionality::new(
            direction,
            Stack::new([PositionedDirectional::new(named(
                SizedBox::new().width(75.).height(175.),
                FIRST,
            ))
            .start(50.)]),
        )
    };
    let (mut tree, root) = mount(frame(Rtl));
    assert_eq!(bounds(&tree, FIRST).origin, Offset::new(675., 0.));
    let child = tree.element_with_key(&FIRST).unwrap();
    tree.update(root, frame(Ltr).into()).unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(bounds(&tree, FIRST).origin, Offset::new(50., 0.));
    assert_eq!(tree.element_with_key(&FIRST), Some(child));
}

#[test]
fn a_stack_can_change_its_direction_and_reset_to_physical_alignment() {
    let (mut tree, stack) = mount(Stack::new(Vec::<Widget>::new()).alignment(Alignment::CENTER));
    tree.update(
        stack,
        Stack::new(Vec::<Widget>::new()).text_direction(Rtl).into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    tree.update(
        stack,
        Stack::new(Vec::<Widget>::new())
            .alignment(Alignment::CENTER)
            .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();

    let frame = |direction| {
        Directionality::new(
            Ltr,
            IndexedStack::new([square(20., FIRST), square(10., SECOND)]).text_direction(direction),
        )
    };
    let (mut tree, root) = mount(frame(Rtl));
    assert_eq!(bounds(&tree, SECOND).origin.x, 790.);
    tree.update(root, frame(Ltr).into()).unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(bounds(&tree, SECOND).origin.x, 0.);
}

#[test]
fn partial_positioning_retains_fallback_alignment_on_the_unspecified_axis() {
    let frame = |direction, alignment| {
        let kids = [
            square(100., Key::Value(200)),
            Positioned::new(square(100., Key::Value(201)))
                .left(0.)
                .into(),
            Positioned::new(square(100., Key::Value(202)))
                .right(0.)
                .into(),
            Positioned::new(square(100., Key::Value(203)))
                .top(0.)
                .into(),
            Positioned::new(square(100., Key::Value(204)))
                .bottom(0.)
                .into(),
            PositionedDirectional::new(square(100., Key::Value(205)))
                .start(0.)
                .into(),
            PositionedDirectional::new(square(100., Key::Value(206)))
                .end(0.)
                .into(),
            PositionedDirectional::new(square(100., Key::Value(207)))
                .top(0.)
                .into(),
            PositionedDirectional::new(square(100., Key::Value(208)))
                .bottom(0.)
                .into(),
        ];
        Directionality::new(direction, Stack::new(kids).alignment(alignment))
    };
    let cases = [
        (
            Rtl,
            Alignment::CENTER,
            [
                (350., 250.),
                (0., 250.),
                (700., 250.),
                (350., 0.),
                (350., 500.),
                (700., 250.),
                (0., 250.),
                (350., 0.),
                (350., 500.),
            ],
        ),
        (
            Ltr,
            Alignment::CENTER,
            [
                (350., 250.),
                (0., 250.),
                (700., 250.),
                (350., 0.),
                (350., 500.),
                (0., 250.),
                (700., 250.),
                (350., 0.),
                (350., 500.),
            ],
        ),
        (
            Ltr,
            Alignment::BOTTOM_RIGHT,
            [
                (700., 500.),
                (0., 500.),
                (700., 500.),
                (700., 0.),
                (700., 500.),
                (0., 500.),
                (700., 500.),
                (700., 0.),
                (700., 500.),
            ],
        ),
        (
            Ltr,
            Alignment::TOP_LEFT,
            [
                (0., 0.),
                (0., 0.),
                (700., 0.),
                (0., 0.),
                (0., 500.),
                (0., 0.),
                (700., 0.),
                (0., 0.),
                (0., 500.),
            ],
        ),
    ];
    let mut tree = WidgetTree::new();
    let mut root = None;
    for (direction, alignment, expected) in cases {
        let frame = frame(direction, alignment).into();
        if let Some(root) = root {
            tree.update(root, frame).unwrap();
        } else {
            root = Some(tree.mount(frame).unwrap());
        }
        tree.layout(Constraints::tight(VIEW)).unwrap();
        for (i, (x, y)) in expected.into_iter().enumerate() {
            assert_eq!(
                bounds(&tree, Key::Value(200 + i as u64)),
                Rect::from_origin_size(Offset::new(x, y), Size::new(100., 100.)),
                "case {direction:?} {alignment:?}, child {i}"
            );
        }
    }
}

#[test]
fn positioned_constructors_set_only_the_specified_edges_and_extents() {
    let elements = |p: Positioned| {
        [
            p.get_left(),
            p.get_top(),
            p.get_right(),
            p.get_bottom(),
            p.get_width(),
            p.get_height(),
        ]
    };
    let child = || SizedBox::shrink();
    assert_eq!(
        elements(
            Positioned::new(child())
                .left(101.)
                .right(201.)
                .top(301.)
                .bottom(401.)
        ),
        [Some(101.), Some(301.), Some(201.), Some(401.), None, None]
    );
    let rect = Rect::from_origin_size(Offset::new(102., 302.), Size::new(100., 200.));
    assert_eq!(
        elements(Positioned::from_rect(rect, child())),
        [Some(102.), Some(302.), None, None, Some(100.), Some(200.)]
    );
    assert_eq!(
        elements(Positioned::from_relative_rect(
            RelativeRect::from_ltrb(103., 303., 203., 403.),
            child()
        )),
        [Some(103.), Some(303.), Some(203.), Some(403.), None, None]
    );
}

#[test]
fn positioned_placeholder_does_not_crash_in_a_zero_area_host() {
    let mut tree = WidgetTree::new();
    tree.mount(
        Center::new(named(
            Stack::new([Positioned::new(Placeholder::new()).top(0.).left(0.)]),
            STACK,
        ))
        .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    assert_eq!(bounds(&tree, STACK).size, Size::ZERO);
}
