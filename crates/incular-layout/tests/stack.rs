//! Flutter's `RenderStack._computeSize`.

use incular_layout::{Constraints, Positioned, Size, Stack, StackChild, layout_stack};

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
