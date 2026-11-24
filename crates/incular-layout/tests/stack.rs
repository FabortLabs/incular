//! Flutter's `RenderStack._computeSize`.

use incular_layout::{
    Alignment, Constraints, Offset, Positioned, Size, Stack, StackChild, layout_stack,
};

#[test]
fn a_stack_of_only_positioned_children_takes_the_biggest_size() {
    let child = StackChild::positioned(
        Size::new(20., 20.),
        Positioned {
            left: Some(10.),
            top: Some(10.),
            ..Positioned::default()
        },
    );
    let result = layout_stack(
        Constraints::loose(Size::new(200., 100.)),
        &[child],
        Stack::new(),
    );
    assert_eq!(result.size, Size::new(200., 100.));
}

#[test]
fn a_non_positioned_child_still_sizes_a_loose_stack() {
    let result = layout_stack(
        Constraints::loose(Size::new(200., 100.)),
        &[StackChild::new(Size::new(30., 40.))],
        Stack::new(),
    );
    assert_eq!(result.size, Size::new(30., 40.));
}

#[test]
fn positioned_unanchored_axes_use_alignment_after_explicit_sizing() {
    let positioned = |position| StackChild::positioned(Size::new(20., 20.), position);
    let result = layout_stack(
        Constraints::tight(Size::new(150., 100.)),
        &[
            positioned(Positioned {
                left: Some(3.),
                height: Some(36.),
                ..Positioned::default()
            }),
            positioned(Positioned {
                bottom: Some(4.),
                width: Some(54.),
                ..Positioned::default()
            }),
            positioned(Positioned {
                width: Some(54.),
                height: Some(36.),
                ..Positioned::default()
            }),
        ],
        Stack::new().with_alignment(Alignment::new(0.5, -0.5)),
    );
    assert_eq!(result.children[0].offset, Offset::new(3., 16.));
    assert_eq!(result.children[1].offset, Offset::new(72., 76.));
    assert_eq!(result.children[2].offset, Offset::new(72., 16.));
}
