use incular_config::{Clip, Constraints};
use incular_core::{Color, Offset, Size, time::Instant};
use incular_rendering::{FillRule, PaintCommand};
use incular_widgets::{BoxShape, PhysicalModel, SizedBox, Widget, internal::WidgetTree};

const COLOR: Color = Color::rgba(18, 52, 86, 255);
const SIZE: Size = Size {
    width: 100.,
    height: 40.,
};

fn oval(clip: Clip, child: impl Into<Widget>) -> Widget {
    PhysicalModel::new(COLOR, child)
        .shape(BoxShape::Circle)
        .clip_behavior(clip)
        .into()
}

#[test]
fn a_non_square_physical_circle_paints_a_full_ellipse() {
    let mut tree = WidgetTree::new();
    tree.mount(oval(Clip::None, SizedBox::expand())).unwrap();
    tree.layout(Constraints::tight(SIZE)).unwrap();
    tree.update_compositor(Instant::now()).unwrap();
    let picture = tree.paint();
    let contains = |position| {
        picture
            .commands()
            .iter()
            .find_map(|command| match command {
                PaintCommand::FillPath { path, .. } => {
                    Some(path.contains(position, FillRule::NonZero))
                }
                PaintCommand::RRect { rrect, .. } => Some(rrect.contains(position)),
                _ => None,
            })
            .expect("physical surface paints a path or rounded rectangle")
    };
    for (position, expected) in [
        (Offset::new(2., 20.), true),
        (Offset::new(98., 20.), true),
        (Offset::new(50., 2.), true),
        (Offset::new(1., 1.), false),
    ] {
        assert_eq!(contains(position), expected, "{position:?}");
    }
}

#[test]
fn physical_oval_paint_does_not_claim_a_hit_from_an_empty_child() {
    let mut tree = WidgetTree::new();
    tree.mount(oval(Clip::None, SizedBox::expand())).unwrap();
    tree.layout(Constraints::tight(SIZE)).unwrap();
    assert!(tree.hit_test(Offset::new(50., 20.)).is_none());
}

#[test]
fn physical_model_can_lay_out_and_paint_at_zero_area() {
    let mut tree = WidgetTree::new();
    tree.mount(PhysicalModel::new(COLOR, SizedBox::expand()).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    tree.update_compositor(Instant::now()).unwrap();
    assert_eq!(tree.root_layout_size(), Some(Size::ZERO));
    let _ = tree.paint();
}

#[test]
fn physical_shape_can_lay_out_and_paint_at_zero_area() {
    let mut tree = WidgetTree::new();
    let empty_oval = incular_widgets::ShapeBorder::outer_path(
        &incular_widgets::OvalBorder::new(incular_widgets::BorderSide::NONE),
        incular_core::Rect::from_origin_size(Offset::ZERO, Size::ZERO),
    );
    tree.mount(incular_widgets::PhysicalShape::new(empty_oval, COLOR, SizedBox::expand()).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    tree.update_compositor(Instant::now()).unwrap();
    assert_eq!(tree.root_layout_size(), Some(Size::ZERO));
    let _ = tree.paint();
}
