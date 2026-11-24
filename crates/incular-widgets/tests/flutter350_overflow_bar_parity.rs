//! Behavior probes for the pinned Flutter OverflowBar.
use incular_config::{Alignment, Constraints, WrapCrossAlignment};
use incular_core::{Color, Rect, Size};
use incular_widgets::{
    Align, Directionality, IntrinsicHeight, IntrinsicWidth, OverflowBar, SizedBox, Text,
    TextDirection, Widget,
    internal::{Key, WidgetTree},
};

const BAR: u64 = 90;
const HOST: Size = Size {
    width: 800.,
    height: 600.,
};
const LTR: TextDirection = TextDirection::Ltr;
const RTL: TextDirection = TextDirection::Rtl;
const SHAPES: [(f32, f32); 3] = [(48., 48.), (64., 64.), (32., 32.)];
fn bar(sizes: &[(f32, f32)]) -> OverflowBar {
    OverflowBar::new(sizes.iter().enumerate().map(|(i, &(w, h))| {
        Widget::box_(Size::new(w, h), Color::WHITE).with_key(Key::Value(i as u64))
    }))
}
fn keyed(b: OverflowBar) -> Widget {
    Widget::from(b).with_key(Key::Value(BAR))
}
fn mount(d: TextDirection, child: impl Into<Widget>, c: Constraints) -> WidgetTree {
    let mut t = WidgetTree::new();
    t.mount(Directionality::new(d, child).into()).unwrap();
    t.layout(c).unwrap();
    t
}
fn bounds(t: &WidgetTree, i: u64) -> Rect {
    t.element_bounds(t.element_with_key(&Key::Value(i)).unwrap())
        .unwrap()
}
fn geometry(t: &WidgetTree) -> [[f32; 4]; 3] {
    std::array::from_fn(|i| {
        let r = bounds(t, i as u64);
        [r.origin.x, r.origin.y, r.size.width, r.size.height]
    })
}
#[test]
fn empty_bar_respects_tight_and_loose_constraints() {
    let s = Size::new(16., 24.);
    let a = mount(LTR, keyed(OverflowBar::default()), Constraints::tight(s));
    let b = mount(LTR, keyed(OverflowBar::default()), Constraints::loose(HOST));
    assert_eq!(
        [bounds(&a, BAR).size, bounds(&b, BAR).size],
        [s, Size::new(0., 0.)]
    );
}
#[test]
fn horizontal_fit_centers_children_vertically_and_honors_direction() {
    let actual = [(LTR, 0.), (LTR, 10.), (RTL, 0.), (RTL, 10.)].map(|(d, s)| {
        geometry(&mount(
            d,
            keyed(bar(&SHAPES).spacing(s)),
            Constraints::loose(HOST),
        ))
    });
    assert_eq!(
        actual,
        [
            [
                [0., 8., 48., 48.],
                [48., 0., 64., 64.],
                [112., 16., 32., 32.]
            ],
            [
                [0., 8., 48., 48.],
                [58., 0., 64., 64.],
                [132., 16., 32., 32.]
            ],
            [
                [96., 8., 48., 48.],
                [32., 0., 64., 64.],
                [0., 16., 32., 32.]
            ],
            [
                [116., 8., 48., 48.],
                [42., 0., 64., 64.],
                [0., 16., 32., 32.]
            ],
        ]
    );
}
#[test]
fn overflow_is_one_column_with_directional_horizontal_alignment() {
    use WrapCrossAlignment::{Center, End, Start};
    let cases = [
        (LTR, Start, 0.),
        (RTL, End, 0.),
        (LTR, Start, 10.),
        (LTR, End, 0.),
        (RTL, Start, 0.),
        (LTR, Center, 0.),
    ];
    let actual = cases.map(|(d, a, s)| {
        geometry(&mount(
            d,
            keyed(bar(&SHAPES).overflow_alignment(a).overflow_spacing(s)),
            Constraints::loose(Size::new(100., 600.)),
        ))
    });
    let xs = [
        [0., 0., 0.],
        [0., 0., 0.],
        [0., 0., 0.],
        [52., 36., 68.],
        [52., 36., 68.],
        [26., 18., 34.],
    ];
    let expected = std::array::from_fn(|i| {
        let [x, y, z] = xs[i];
        let s = cases[i].2;
        [
            [x, 0., 48., 48.],
            [y, 48. + s, 64., 64.],
            [z, 112. + 2. * s, 32., 32.],
        ]
    });
    assert_eq!(actual, expected);
}
#[test]
fn tight_oversized_bar_anchors_children_at_directional_start() {
    let actual = [LTR, RTL].map(|d| {
        let t = mount(
            d,
            keyed(bar(&[(50., 50.), (60., 50.), (70., 50.)]).spacing(10.)),
            Constraints::tight(HOST),
        );
        assert_eq!(bounds(&t, BAR).size, HOST);
        geometry(&t).map(|r| r[0])
    });
    assert_eq!(actual, [[0., 60., 130.], [750., 680., 600.]]);
}
fn intrinsic(width: f32, height: bool) -> [Size; 2] {
    let b = keyed(
        bar(&[(48., 50.), (64., 25.), (32., 75.)])
            .spacing(4.)
            .overflow_spacing(8.),
    );
    let child = if height {
        Widget::from(IntrinsicHeight::new(b))
    } else {
        Widget::from(IntrinsicWidth::new(b))
    };
    let t = mount(
        LTR,
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new()
                .width(width)
                .child(Align::new(Alignment::TOP_LEFT, child.clone())),
        ),
        Constraints::tight(HOST),
    );
    let direct = mount(
        LTR,
        child,
        Constraints::loose(Size::new(width, HOST.height)),
    );
    [bounds(&t, BAR).size, bounds(&direct, BAR).size]
}
#[test]
fn intrinsic_width_includes_gaps_and_clamps_to_available_width() {
    assert_eq!(
        [
            intrinsic(800., false).map(|s| s.width),
            intrinsic(150., false).map(|s| s.width)
        ],
        [[152., 152.], [150., 150.]]
    );
}
#[test]
fn intrinsic_height_switches_from_tallest_child_to_full_column() {
    assert_eq!(
        [
            intrinsic(800., true).map(|s| s.height),
            intrinsic(150., true).map(|s| s.height)
        ],
        [[75., 75.], [166., 166.]]
    );
}
#[test]
fn text_children_at_zero_area_do_not_crash() {
    let z = Size::new(0., 0.);
    let t = mount(
        LTR,
        keyed(OverflowBar::new([Text::new("X"), Text::new("Y")])),
        Constraints::tight(z),
    );
    assert_eq!(bounds(&t, BAR).size, z);
}
