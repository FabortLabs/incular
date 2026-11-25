use incular_config::{Constraints, ViewConstraints};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::SizedBox;

#[test]
fn converted_view_bounds_constrain_the_retained_root_on_both_axes() {
    let widget = SizedBox::new().width(10.).height(1.);
    let mut runtime = Runtime::new(widget.into()).unwrap();
    let root = runtime.tree().root().unwrap();

    runtime
        .run_frame(Constraints::from_view_constraints(ViewConstraints::new(
            1., 2., 3., 4.,
        )))
        .unwrap();
    assert_eq!(
        runtime.tree().element_bounds(root).unwrap().size,
        Size::new(2., 3.),
    );

    runtime
        .run_frame(Constraints::from_view_constraints(ViewConstraints::tight(
            Size::new(5., 6.),
        )))
        .unwrap();
    assert_eq!(runtime.tree().root(), Some(root));
    assert_eq!(
        runtime.tree().element_bounds(root).unwrap().size,
        Size::new(5., 6.),
    );
}
