//! Flutter 3.50.0 Align and SizedBox public layout fixtures.
use incular_config::{Alignment, Constraints, CrossAxisAlignment, MainAxisAlignment};
use incular_core::{Offset, Size};
use incular_widgets::{
    Align, Center, Column, Container, Directionality, LayoutBuilder, Placeholder, Row,
    SingleChildScrollView, SizedBox, TextDirection, Widget,
    internal::{Key, WidgetTree},
};
use std::{cell::Cell, rc::Rc};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
const PROBE: Key = Key::Value(351);
fn keyed(w: impl Into<Widget>) -> Widget {
    w.into().with_key(PROBE)
}
fn mount(w: impl Into<Widget>, sz: Size) -> WidgetTree {
    let mut t = WidgetTree::new();
    t.mount(w.into()).unwrap();
    t.layout(Constraints::tight(sz)).unwrap();
    t
}
fn bounds(t: &WidgetTree) -> incular_core::Rect {
    t.element_bounds(t.element_with_key(&PROBE).unwrap())
        .unwrap()
}

#[test]
fn align_absolute_control_ignores_ambient_direction() {
    for d in [TextDirection::Ltr, TextDirection::Rtl] {
        let t = mount(
            Directionality::new(
                d,
                Align::new(
                    Alignment::TOP_LEFT,
                    keyed(SizedBox::new().width(100.).height(80.)),
                ),
            ),
            VIEW,
        );
        assert_eq!(bounds(&t).origin, Offset::ZERO);
        assert_eq!(bounds(&t).size, Size::new(100., 80.));
    }
}
#[test]
fn align_smoke_for_absolute_values_and_empty_child_analogs() {
    for w in [
        Align::new(Alignment::new(0.5, 0.5), Container::new()),
        Align::builder().child(Container::new()).build(),
        Align::new(Alignment::TOP_LEFT, SizedBox::shrink()),
        Align::new(Alignment::TOP_RIGHT, SizedBox::shrink()),
    ] {
        mount(w, VIEW);
    }
}
#[test]
fn align_shrinks_only_the_unbounded_scroll_axis() {
    let a = keyed(Align::builder().child(SizedBox::square(10.)).build());
    let t = mount(SingleChildScrollView::new(a), VIEW);
    assert_eq!(bounds(&t).size, Size::new(800., 10.));
}
#[test]
fn factors_scale_the_child_in_each_flex_axis() {
    let x = keyed(Align::new(Alignment::CENTER, SizedBox::square(100.)).width_factor(0.5));
    let y = keyed(Align::new(Alignment::CENTER, SizedBox::square(100.)).height_factor(0.5));
    let x_tree = mount(
        Directionality::new(
            TextDirection::Ltr,
            Row::new([x])
                .main_axis_alignment(MainAxisAlignment::Center)
                .cross_axis_alignment(CrossAxisAlignment::Start),
        ),
        VIEW,
    );
    let y_tree = mount(
        Directionality::new(
            TextDirection::Ltr,
            Column::new([y])
                .main_axis_alignment(MainAxisAlignment::Center)
                .cross_axis_alignment(CrossAxisAlignment::Start),
        ),
        VIEW,
    );
    assert_eq!(bounds(&x_tree).size.width, 50.);
    assert_eq!(bounds(&y_tree).size.height, 50.);
}
#[test]
fn align_and_sized_box_complete_zero_area_with_a_placeholder() {
    for w in [
        keyed(Align::builder().child(Placeholder::new()).build()),
        keyed(SizedBox::new().child(Placeholder::new())),
    ] {
        assert_eq!(bounds(&mount(Center::new(w), Size::ZERO)).size, Size::ZERO);
    }
}
fn sized_sequence(with_child: bool) -> Vec<Size> {
    let center = |b: SizedBox| {
        let b = if with_child {
            b.child(Container::new())
        } else {
            b
        };
        Center::new(keyed(b)).into()
    };
    let candidates = [
        SizedBox::new(),
        SizedBox::new().height(0.),
        SizedBox::shrink(),
        SizedBox::square(100.),
        SizedBox::square(1000.),
        SizedBox::expand(),
        SizedBox::shrink(),
    ];
    let mut t = WidgetTree::new();
    let mut root = None;
    let mut sizes = Vec::new();
    for subject in candidates {
        let w = center(subject);
        if let Some(root) = root {
            t.update(root, w).unwrap();
        } else {
            root = Some(t.mount(w).unwrap());
        }
        t.layout(Constraints::tight(VIEW)).unwrap();
        sizes.push(bounds(&t).size);
    }
    sizes
}
#[test]
fn sized_box_empty_child_constrains_the_complete_retained_sequence() {
    assert_eq!(
        sized_sequence(false),
        [
            Size::ZERO,
            Size::ZERO,
            Size::ZERO,
            Size::new(100., 100.),
            VIEW,
            VIEW,
            Size::ZERO,
        ]
    );
}
#[test]
fn sized_box_container_child_constrains_the_complete_retained_sequence() {
    assert_eq!(
        sized_sequence(true),
        [
            VIEW,
            Size::new(800., 0.),
            Size::ZERO,
            Size::new(100., 100.),
            VIEW,
            VIEW,
            Size::ZERO,
        ]
    );
}
#[test]
fn sized_box_square_passes_tight_constraints_to_its_child() {
    let observed = Rc::new(Cell::new(None));
    let spy = LayoutBuilder::new({
        let observed = observed.clone();
        move |_, c| {
            observed.set(Some(c));
            SizedBox::shrink().into()
        }
    });
    mount(Center::new(SizedBox::square(100.).child(spy)), VIEW);
    assert_eq!(
        observed.get(),
        Some(Constraints::tight(Size::new(100., 100.)))
    );
}
#[test]
fn sized_box_constructor_fields_have_their_expected_public_layout_effects() {
    for (w, expected) in [
        (SizedBox::new(), Size::ZERO),
        (SizedBox::new().width(10.), Size::new(10., 0.)),
        (SizedBox::new().width(10.).height(20.), Size::new(10., 20.)),
        (SizedBox::from_size(Size::new(1., 2.)), Size::new(1., 2.)),
        (SizedBox::expand(), VIEW),
        (SizedBox::shrink(), Size::ZERO),
    ] {
        assert_eq!(bounds(&mount(Center::new(keyed(w)), VIEW)).size, expected);
    }
}

#[test]
fn positioned_box_reports_factor_scaled_intrinsics_across_retained_updates() {
    let scene = |factors| {
        let mut align = Align::new(Alignment::CENTER, SizedBox::square(100.));
        if let Some((width, height)) = factors {
            align = align.width_factor(width).height_factor(height);
        }
        keyed(align)
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(Some((1., 0.)))).unwrap();
    let constraints = Constraints::at_most(200., 200.);
    let element = tree.element_with_key(&PROBE).unwrap();
    let render = tree.render_id(element).unwrap();
    for (factors, intrinsic, laid_out) in [
        (Some((1., 0.)), Size::new(100., 0.), Size::new(100., 0.)),
        (Some((0.5, 0.5)), Size::new(50., 50.), Size::new(50., 50.)),
        (None, Size::new(100., 100.), Size::new(200., 200.)),
    ] {
        tree.update(root, scene(factors)).unwrap();
        tree.layout(constraints).unwrap();
        assert_eq!(tree.element_with_key(&PROBE), Some(element));
        assert_eq!(tree.render_id(element), Some(render));
        assert_eq!(bounds(&tree).size, laid_out);
        assert_eq!(
            tree.min_intrinsic_width(render, 200.),
            Some(intrinsic.width)
        );
        assert_eq!(
            tree.max_intrinsic_width(render, 200.),
            Some(intrinsic.width)
        );
        assert_eq!(
            tree.min_intrinsic_height(render, 200.),
            Some(intrinsic.height)
        );
        assert_eq!(
            tree.max_intrinsic_height(render, 200.),
            Some(intrinsic.height)
        );
    }
}

#[test]
fn align_passes_the_original_cross_extent_to_its_child_before_scaling() {
    use incular_widgets::AspectRatio;
    let tree = mount(
        Center::new(keyed(
            Align::new(Alignment::CENTER, AspectRatio::new(2., SizedBox::shrink()))
                .width_factor(0.5)
                .height_factor(3.),
        )),
        VIEW,
    );
    let render = tree
        .render_id(tree.element_with_key(&PROBE).unwrap())
        .unwrap();
    let before = bounds(&tree);
    // Width uses the child's 2x aspect ratio followed by the parent's 0.5x
    // width factor. Height uses the reciprocal and the parent's 3x factor.
    for cross in [10., 20.] {
        assert_eq!(tree.min_intrinsic_width(render, cross), Some(cross));
        assert_eq!(tree.max_intrinsic_width(render, cross), Some(cross));
        assert_eq!(tree.min_intrinsic_height(render, cross), Some(cross * 1.5));
        assert_eq!(tree.max_intrinsic_height(render, cross), Some(cross * 1.5));
    }
    assert_eq!(bounds(&tree), before);
}

#[test]
fn intrinsic_parents_look_through_align_to_the_childs_reported_extent() {
    use incular_widgets::{IntrinsicHeight, IntrinsicWidth, MaxIntrinsicSize};
    let alignment = || {
        keyed(
            Align::new(
                Alignment::CENTER,
                MaxIntrinsicSize::new(
                    Size::new(100., 200.),
                    SizedBox::from_size(Size::new(55., 110.)),
                ),
            )
            .width_factor(0.5)
            .height_factor(0.5),
        )
    };
    let intrinsic_width = mount(Center::new(IntrinsicWidth::new(alignment())), VIEW);
    assert_eq!(bounds(&intrinsic_width).size, Size::new(50., 55.));
    let intrinsic_height = mount(Center::new(IntrinsicHeight::new(alignment())), VIEW);
    assert_eq!(bounds(&intrinsic_height).size, Size::new(27.5, 100.));
}
