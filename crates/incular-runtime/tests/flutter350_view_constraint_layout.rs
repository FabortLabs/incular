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

#[test]
fn fractional_view_scale_preserves_the_exact_physical_size_of_the_retained_root() {
    use incular_platform::{PhysicalSize, WindowMetrics};
    let metrics = WindowMetrics::new(PhysicalSize::new(1008, 2198), 1.912500023841858);
    let view = metrics.view_configuration().unwrap();
    let mut runtime = Runtime::new(SizedBox::new().into()).unwrap();
    let root = runtime.tree().root().unwrap();
    runtime.run_frame(view.logical_constraints()).unwrap();
    let logical = runtime.tree().element_bounds(root).unwrap().size;
    assert_eq!(logical, metrics.logical_size());
    assert_eq!(
        view.to_physical_size(logical),
        Some(Size::new(1008., 2198.))
    );
}
