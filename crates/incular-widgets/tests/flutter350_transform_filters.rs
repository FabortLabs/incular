use incular_config::Constraints;
use incular_core::{Offset, Size, Transform as Affine};
use incular_rendering::{DisplayList, FilterQuality, ImageSampling, PaintCommand};
use incular_widgets::{Center, SizedBox, Transform, Widget, internal::WidgetTree};
use std::f32::consts::FRAC_PI_4;

fn mount(widget: impl Into<Widget>) -> (WidgetTree, incular_widgets::internal::ElementId) {
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget.into()).unwrap();
    (tree, root)
}

fn paint(tree: &mut WidgetTree) -> DisplayList {
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    tree.paint()
}

fn matrix_stages(list: &DisplayList) -> Vec<([f64; 6], ImageSampling)> {
    list.commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushMatrixFilter {
                matrix,
                stage,
                sampling,
                ..
            } => Some((
                stage
                    .then(*matrix)
                    .then(stage.inverse().unwrap())
                    .to_kurbo()
                    .as_coeffs(),
                *sampling,
            )),
            _ => None,
        })
        .collect()
}

fn assert_matrix(widget: impl Into<Widget>, expected: [f64; 6]) {
    let (mut tree, _) = mount(widget);
    let list = paint(&mut tree);
    let stages = matrix_stages(&list);
    let [(matrix, sampling)] = stages.as_slice() else {
        panic!("one resampling stage: {stages:?}")
    };
    assert_eq!(*sampling, ImageSampling::Linear);
    for (axis, (actual, expected)) in matrix.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() <= 0.00015,
            "coefficient {axis}: {actual} != {expected}"
        );
    }
}

fn sized() -> SizedBox {
    SizedBox::new().width(100.).height(100.)
}

#[test]
fn translate_and_scale_send_the_reference_matrices_to_resampling() {
    assert_matrix(
        Transform::translation(Offset::new(25., 25.), sized()).filter_quality(FilterQuality::Low),
        [1., 0., 0., 1., 25., 25.],
    );
    assert_matrix(
        Transform::scale(3.14159, sized()).filter_quality(FilterQuality::Low),
        [3.14159, 0., 0., 3.14159, -856.636, -642.477],
    );
}

#[test]
fn rotated_filters_resample_about_the_same_world_pivot_when_offset() {
    let angle = std::f64::consts::FRAC_1_SQRT_2;
    let expected = [
        angle,
        angle,
        -angle,
        angle,
        329.28932188134524,
        -194.97474683058329,
    ];
    let filtered = || Transform::rotation(FRAC_PI_4, sized()).filter_quality(FilterQuality::Low);
    assert_matrix(filtered(), expected);
    assert_matrix(
        SizedBox::square(400.).child(Center::new(filtered())),
        expected,
    );
}

#[test]
fn filter_is_removed_and_recreated_when_the_widget_option_changes() {
    let scene = |quality| {
        let widget = Transform::new(Affine::rotation(FRAC_PI_4), sized());
        if quality {
            widget.filter_quality(FilterQuality::Low).into()
        } else {
            widget.into()
        }
    };
    let (mut tree, root) = mount(scene(true));
    assert_eq!(matrix_stages(&paint(&mut tree)).len(), 1);
    tree.update(root, scene(false)).unwrap();
    assert!(matrix_stages(&paint(&mut tree)).is_empty());
    tree.update(root, scene(true)).unwrap();
    assert_eq!(matrix_stages(&paint(&mut tree)).len(), 1);
}
