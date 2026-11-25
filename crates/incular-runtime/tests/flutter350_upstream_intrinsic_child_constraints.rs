use incular_config::Constraints;
use incular_core::{Offset, Size};
use incular_runtime::Runtime;
use incular_widgets::{IntrinsicWidth, SizedBox, Widget, internal::Key};

#[test]
fn intrinsic_steps_and_parent_limits_tighten_the_child_not_just_the_wrapper() {
    const CHILD: Key = Key::Value(1);
    let build = |width, height| {
        let child = Widget::from(SizedBox::new().width(103.).height(47.)).with_key(CHILD);
        let width = IntrinsicWidth::new(child).step_width(width);
        match height {
            Some(height) => width.step_height(height).into(),
            None => width.into(),
        }
    };
    let mut runtime = Runtime::new(build(10., Some(8.))).unwrap();
    let root = runtime.tree().root().unwrap();
    for (width, height, limits, expected) in [
        (
            10.,
            Some(8.),
            Constraints::at_most(200., 200.),
            Size::new(110., 48.),
        ),
        (
            7.,
            None,
            Constraints::at_most(200., 200.),
            Size::new(105., 47.),
        ),
        (
            10.,
            Some(8.),
            Constraints::at_most(100., 46.),
            Size::new(100., 46.),
        ),
        (
            10.,
            Some(8.),
            Constraints::tight(Size::new(97., 43.)),
            Size::new(97., 43.),
        ),
    ] {
        runtime.schedule_update(root, build(width, height)).unwrap();
        runtime.run_frame(limits).unwrap();
        let tree = runtime.tree();
        let child = tree.element_with_key(&CHILD).unwrap();
        let outer = tree.element_bounds(root).unwrap();
        let inner = tree.element_bounds(child).unwrap();
        assert_eq!((outer.size, inner.size), (expected, expected));
        assert_eq!(inner.origin - outer.origin, Offset::ZERO);
    }
}
