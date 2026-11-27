use std::{cell::RefCell, rc::Rc};

use incular_config::{Alignment, Constraints, TextDirection};
use incular_core::{InputEvent, Offset, PointerPhase, Size, Transform, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::{
    Align, Directionality, HitTestBehavior, Listener, ScrollController, SizedBox,
    TransformationController, Widget, widget_previews::ZoomablePreviewArea,
};

struct Fixture {
    runtime: Runtime,
    controller: TransformationController,
    outer: ScrollController,
    inner: ScrollController,
    hits: Rc<RefCell<Vec<Offset>>>,
}

impl Fixture {
    fn new(scale: f32, failed: bool) -> Self {
        let controller = TransformationController::with_value(Transform::scale(scale));
        let outer = ScrollController::new();
        let inner = ScrollController::new();
        let hits = Rc::new(RefCell::new(Vec::new()));
        let content = Widget::from(
            Listener::new(SizedBox::new().width(100.).height(90.))
                .behavior(HitTestBehavior::Opaque)
                .on_pointer_down({
                    let hits = hits.clone();
                    move |sample| hits.borrow_mut().push(sample.local_position)
                }),
        )
        .with_key("preview.content");
        let area = ZoomablePreviewArea::new(content, controller.clone())
            .error_thrown_during_tree_construction(failed)
            .vertical_controller(outer.clone())
            .horizontal_controller(inner.clone());
        let runtime = Runtime::new(
            Directionality::new(TextDirection::Ltr, Align::new(Alignment::TOP_LEFT, area)).into(),
        )
        .unwrap();
        let mut this = Self {
            runtime,
            controller,
            outer,
            inner,
            hits,
        };
        this.frame();
        this
    }

    fn frame(&mut self) {
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(120., 100.)), Instant::now())
            .unwrap();
    }

    fn tap(&mut self, position: Offset) {
        for phase in [PointerPhase::Down, PointerPhase::Up] {
            let _ = self
                .runtime
                .handle_input(InputEvent::Pointer { phase, position });
        }
    }
}

#[test]
fn a_constructed_preview_mounts_both_axes_and_keeps_the_original_child() {
    let mut f = Fixture::new(1., false);
    let content = f
        .runtime
        .tree()
        .element_with_key(&"preview.content".into())
        .unwrap();
    assert!(f.inner.has_clients());
    assert!(f.outer.has_clients());
    assert_eq!(f.inner.metrics().max_scroll_extent, 0.);
    assert_eq!(f.outer.metrics().max_scroll_extent, 0.);

    f.controller.set_value(Transform::scale(2.));
    f.frame();
    assert_eq!(
        f.runtime.tree().element_with_key(&"preview.content".into()),
        Some(content)
    );
    assert_eq!(f.inner.metrics().max_scroll_extent, 80.);
    assert_eq!(f.outer.metrics().max_scroll_extent, 80.);
    f.tap(Offset::new(110., 60.));
    assert_eq!(*f.hits.borrow(), [Offset::new(55., 30.)]);
    assert!(f.inner.jump_to(50.));
    assert!(f.outer.jump_to(30.));
    f.frame();
    f.tap(Offset::new(110., 60.));
    assert_eq!(
        *f.hits.borrow(),
        [Offset::new(55., 30.), Offset::new(80., 45.)]
    );

    f.controller.set_value(Transform::IDENTITY);
    f.frame();
    assert_eq!(f.inner.metrics().max_scroll_extent, 0.);
    assert_eq!(f.outer.metrics().max_scroll_extent, 0.);
    assert_eq!(f.inner.offset(), 0.);
    assert_eq!(f.outer.offset(), 0.);
}

#[test]
fn construction_failure_bypasses_both_viewports_and_ignores_zoom() {
    let mut f = Fixture::new(2., true);
    let tree = f.runtime.tree();
    let content = tree.element_with_key(&"preview.content".into()).unwrap();
    assert_eq!(
        tree.element_bounds(content).unwrap().size,
        Size::new(100., 90.)
    );
    assert!(!f.inner.has_clients());
    assert!(!f.outer.has_clients());
    f.tap(Offset::new(20., 20.));
    f.tap(Offset::new(110., 60.));
    assert_eq!(*f.hits.borrow(), [Offset::new(20., 20.)]);
    f.controller.set_value(Transform::scale(4.));
    f.frame();
    f.tap(Offset::new(30., 40.));
    assert_eq!(
        *f.hits.borrow(),
        [Offset::new(20., 20.), Offset::new(30., 40.)]
    );
    assert!(!f.inner.has_clients() && !f.outer.has_clients());
}
