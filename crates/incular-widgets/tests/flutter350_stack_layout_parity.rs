//! Observable probes from the pinned Flutter widgets/stack_test.dart.
use std::{cell::RefCell, rc::Rc};

use incular_config::{Alignment, Clip, Constraints, StackFit};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::PaintCommand;
use incular_widgets::{
    Center, ConstrainedBox, Directionality, LayoutBuilder, Placeholder, Positioned, SizedBox,
    Stack, TextDirection, Widget,
    internal::{Key, RenderKind, WidgetTree},
};

const DIM: Size = Size {
    width: 800.,
    height: 600.,
};
const CHILD: u64 = 10;
const POSITIONER: u64 = 11;
const STACK: u64 = 12;
fn scene(w: impl Into<Widget>) -> Widget {
    direction(TextDirection::Ltr, w)
}
fn direction(d: TextDirection, w: impl Into<Widget>) -> Widget {
    Directionality::new(d, w).into()
}
fn mount(w: impl Into<Widget>) -> WidgetTree {
    let mut t = WidgetTree::new();
    t.mount(w.into()).unwrap();
    t.layout(Constraints::tight(DIM)).unwrap();
    t
}
fn update(t: &mut WidgetTree, w: impl Into<Widget>) {
    t.update(t.root().unwrap(), w.into()).unwrap();
    t.layout(Constraints::tight(DIM)).unwrap();
}
fn key(w: impl Into<Widget>, i: u64) -> Widget {
    w.into().with_key(Key::Value(i))
}
fn bounds(t: &WidgetTree, i: u64) -> Rect {
    t.element_bounds(t.element_with_key(&Key::Value(i)).unwrap())
        .unwrap()
}
fn fixed(i: u64, width: f32, height: f32) -> Widget {
    key(SizedBox::new().width(width).height(height), i)
}
fn position_data(t: &WidgetTree) -> [Option<f32>; 6] {
    let id = t
        .render_id(t.element_with_key(&Key::Value(POSITIONER)).unwrap())
        .unwrap();
    let RenderKind::Positioned {
        left,
        top,
        right,
        bottom,
        width,
        height,
    } = t.render_object_kind(id).unwrap()
    else {
        panic!("expected Positioned render object")
    };
    [*left, *top, *right, *bottom, *width, *height]
}

#[test]
fn empty_stack_accepts_normal_and_loose_ancestors() {
    let empty = || Stack::new(Vec::<Widget>::new());
    mount(scene(empty()));
    mount(scene(Center::new(empty())));
}
#[test]
fn retained_position_switch_clears_old_insets_and_moves_child() {
    let build = |left| {
        let p = Positioned::new(fixed(CHILD, 10., 10.));
        scene(Stack::new([key(
            if left { p.left(10.) } else { p.right(10.) },
            POSITIONER,
        )]))
    };
    let mut t = mount(build(true));
    assert_eq!(position_data(&t), [Some(10.), None, None, None, None, None]);
    assert_eq!(bounds(&t, CHILD).origin, Offset::new(10., 0.));
    update(&mut t, build(false));
    assert_eq!(position_data(&t), [None, None, Some(10.), None, None, None]);
    assert_eq!(bounds(&t, CHILD).origin, Offset::new(780., 0.));
}
#[test]
fn removing_positioned_parent_returns_child_to_stack_alignment() {
    let mut t = mount(scene(Stack::new([key(
        Positioned::new(fixed(CHILD, 10., 10.)).left(10.),
        POSITIONER,
    )])));
    assert_eq!(position_data(&t), [Some(10.), None, None, None, None, None]);
    assert_eq!(bounds(&t, CHILD).origin, Offset::new(10., 0.));
    update(&mut t, scene(Stack::new([fixed(CHILD, 10., 10.)])));
    assert!(t.element_with_key(&Key::Value(POSITIONER)).is_none());
    assert_eq!(bounds(&t, CHILD).origin, Offset::ZERO);
}
#[test]
fn sized_positioned_child_honors_both_explicit_dimensions_and_edges() {
    let build = |left| {
        let p = Positioned::new(key(Widget::box_(Size::ZERO, Color::WHITE), CHILD))
            .width(11.)
            .height(12.);
        scene(Stack::new([key(
            if left { p.left(10.) } else { p.right(10.) },
            POSITIONER,
        )]))
    };
    let mut t = mount(build(true));
    assert_eq!(
        position_data(&t),
        [Some(10.), None, None, None, Some(11.), Some(12.)]
    );
    assert_eq!(
        bounds(&t, CHILD),
        Rect::from_origin_size(Offset::new(10., 0.), Size::new(11., 12.))
    );
    update(&mut t, build(false));
    assert_eq!(
        position_data(&t),
        [None, None, Some(10.), None, Some(11.), Some(12.)]
    );
    assert_eq!(
        bounds(&t, CHILD),
        Rect::from_origin_size(Offset::new(779., 0.), Size::new(11., 12.))
    );
}
#[test]
fn center_alignment_of_nonpositioned_children_matches_in_both_directions() {
    for d in [TextDirection::Ltr, TextDirection::Rtl] {
        let w = Stack::aligned(Alignment::CENTER, [fixed(0, 20., 20.), fixed(1, 10., 10.)]);
        let t = mount(direction(d, Center::new(key(w, STACK))));
        let origin = bounds(&t, STACK).origin;
        assert_eq!(
            [bounds(&t, 0).origin - origin, bounds(&t, 1).origin - origin],
            [Offset::ZERO, Offset::new(5., 5.)]
        );
    }
}
#[test]
fn default_alignment_uses_directional_start_not_a_physical_left() {
    let input = [fixed(0, 20., 20.), fixed(1, 10., 10.)];
    let build = |d| direction(d, Center::new(key(Stack::new(input.clone()), STACK)));
    let mut tree = mount(build(TextDirection::Ltr));
    let mut actual = Vec::new();
    for (i, d) in [TextDirection::Ltr, TextDirection::Rtl, TextDirection::Ltr]
        .into_iter()
        .enumerate()
    {
        if i != 0 {
            update(&mut tree, build(d));
        }
        assert_eq!(bounds(&tree, STACK).size, Size::new(20., 20.));
        assert_eq!(
            bounds(&tree, 0).origin - bounds(&tree, STACK).origin,
            Offset::ZERO
        );
        actual.push(bounds(&tree, 1).origin - bounds(&tree, STACK).origin);
    }
    assert_eq!(actual, [Offset::ZERO, Offset::new(10., 0.), Offset::ZERO]);
}
#[test]
fn clip_setting_changes_that_for_overflowing_positioned_child() {
    let build = |clip| {
        let w = Stack::new([
            fixed(0, 100., 100.),
            Widget::from(
                Positioned::new(Widget::box_(Size::new(200., 200.), Color::WHITE))
                    .left(0.)
                    .top(0.),
            ),
        ])
        .clip_behavior(clip);
        scene(Center::new(w))
    };
    let mut t = mount(build(Clip::HardEdge));
    let clipped = |t: &mut WidgetTree| {
        t.paint()
            .commands()
            .iter()
            .any(|cmd| matches!(cmd, PaintCommand::PushClip { .. }))
    };
    let clipped_before = clipped(&mut t);
    update(&mut t, build(Clip::None));
    assert_eq!([clipped_before, clipped(&mut t)], [true, false]);
}
#[test]
fn stack_default_fit_and_explicit_fit_send_expected_constraints() {
    let outer = Constraints::new(2., 3., 5., 7.);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let build = |fit: Option<StackFit>| {
        let child = LayoutBuilder::new({
            let seen = seen.clone();
            move |_, c| {
                seen.borrow_mut().push(c);
                Placeholder::new().into()
            }
        });
        let w = Stack::new([child]);
        scene(Center::new(ConstrainedBox::new(
            outer,
            fit.map_or(w.clone(), |f| w.fit(f)),
        )))
    };
    let mut t = mount(build(None::<StackFit>));
    assert_eq!(*seen.borrow(), [Constraints::new(0., 3., 0., 7.)]);
    seen.borrow_mut().clear();
    // Separate new descriptor for each fit, as in the original Flutter test.
    update(&mut t, build(Some(StackFit::Loose)));
    update(&mut t, build(Some(StackFit::Expand)));
    update(&mut t, build(Some(StackFit::Passthrough)));
    assert_eq!(
        *seen.borrow(),
        [
            Constraints::new(0., 3., 0., 7.),
            Constraints::tight(Size::new(3., 7.)),
            outer
        ]
    );
}
#[test]
fn absolute_partial_positioning_only_overrides_the_specified_axis() {
    let expected = [
        (
            TextDirection::Rtl,
            Alignment::CENTER,
            [
                [350., 250.],
                [0., 250.],
                [700., 250.],
                [350., 0.],
                [350., 500.],
            ],
        ),
        (
            TextDirection::Ltr,
            Alignment::CENTER,
            [
                [350., 250.],
                [0., 250.],
                [700., 250.],
                [350., 0.],
                [350., 500.],
            ],
        ),
        (
            TextDirection::Ltr,
            Alignment::BOTTOM_RIGHT,
            [
                [700., 500.],
                [0., 500.],
                [700., 500.],
                [700., 0.],
                [700., 500.],
            ],
        ),
        (
            TextDirection::Ltr,
            Alignment::TOP_LEFT,
            [[0., 0.], [0., 0.], [700., 0.], [0., 0.], [0., 500.]],
        ),
    ];
    for (d, alignment, offsets) in expected {
        let child = |i| fixed(i, 100., 100.);
        let w = Stack::aligned(
            alignment,
            [
                child(0),
                Positioned::new(child(1)).left(0.).into(),
                Positioned::new(child(2)).right(0.).into(),
                Positioned::new(child(3)).top(0.).into(),
                Positioned::new(child(4)).bottom(0.).into(),
            ],
        );
        let t = mount(direction(d, w));
        for (i, [x, y]) in offsets.into_iter().enumerate() {
            assert_eq!(
                bounds(&t, i as u64),
                Rect::from_origin_size(Offset::new(x, y), Size::new(100., 100.))
            );
        }
    }
}
#[test]
fn missing_directionality_rejects_default_but_accepts_absolute_alignment() {
    mount(Stack::aligned(Alignment::CENTER, Vec::<Widget>::new()));
    let attempt = std::panic::catch_unwind(|| {
        let mut t = WidgetTree::new();
        t.mount(Stack::new(Vec::<Widget>::new()).into())?;
        t.layout(Constraints::tight(DIM))
    });
    assert!(
        !matches!(attempt, Ok(Ok(()))),
        "default Stack accepted missing Directionality"
    );
}
#[test]
fn none_of_the_children_force_a_nonzero_stack_at_zero_area() {
    let mut t = WidgetTree::new();
    t.mount(scene(Center::new(key(
        Stack::new([Placeholder::new()]),
        STACK,
    ))))
    .unwrap();
    t.layout(Constraints::tight(Size::ZERO)).unwrap();
    assert_eq!(bounds(&t, STACK).size, Size::ZERO);
}
