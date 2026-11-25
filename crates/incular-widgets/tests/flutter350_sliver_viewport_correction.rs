//! Flutter Viewport restarts layout immediately when a sliver asks for a
//! scroll offset correction rather than returning its regular geometry.

use std::{cell::RefCell, rc::Rc};

use incular_config::{Axis, Constraints, EdgeInsets};
use incular_core::{Color, Offset, Size};
use incular_scroll::{ScrollController, SliverConstraints, SliverGeometry};
use incular_widgets::internal::{
    Key, RenderSliver, SliverChildId, SliverChildLayout, SliverChildPlacement, SliverLayout,
    SliverScrollDependency, WidgetTree,
};
use incular_widgets::{CustomScrollView, Sliver, SliverPadding, Widget};

#[derive(Clone)]
struct CorrectionSliver(Rc<RefCell<Vec<f32>>>);

impl Sliver for CorrectionSliver {
    fn build(&self, _: &ScrollController) -> Widget {
        Widget::box_(Size::new(800., 400.), Color::WHITE)
    }

    fn create_render_sliver(
        &self,
        _: &ScrollController,
        _: Axis,
        _: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(self.clone())
    }
}

impl RenderSliver for CorrectionSliver {
    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        SliverScrollDependency::ScrollOffset
    }

    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        self.0.borrow_mut().push(constraints.scroll_offset);
        if constraints.scroll_offset != 200. {
            return SliverLayout {
                geometry: SliverGeometry {
                    scroll_offset_correction: Some(200. - constraints.scroll_offset),
                    ..SliverGeometry::ZERO
                },
                children: Vec::new(),
                absorbed_overlap: 0.,
            };
        }
        SliverLayout {
            geometry: SliverGeometry::from_scroll_extent(constraints, 1000.),
            children: vec![SliverChildLayout {
                id: SliverChildId(0),
                widget: Widget::box_(Size::new(800., 400.), Color::WHITE)
                    .with_key(Key::from("corrected")),
                semantic_index: None,
                offset: 0.,
                cross_offset: 0.,
                constraints: Constraints::tight(Size::new(800., 400.)),
                extent: 400.,
                placement: SliverChildPlacement::Flow,
            }],
            absorbed_overlap: 0.,
        }
    }
}

#[test]
fn viewport_applies_a_forward_correction_before_publishing_first_frame_geometry() {
    let controller = ScrollController::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    let sliver = SliverPadding::new(EdgeInsets::ZERO, CorrectionSliver(log.clone()));
    let slivers: Vec<Box<dyn Sliver>> = vec![Box::new(sliver)];
    let mut tree = WidgetTree::new();
    tree.mount(
        CustomScrollView::new(slivers)
            .controller(controller.clone())
            .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    assert_eq!(
        controller.offset(),
        200.,
        "attempted offsets {:?}",
        log.borrow()
    );
    assert_eq!(log.borrow().first(), Some(&0.));
    assert!(log.borrow().iter().skip(1).all(|offset| *offset == 200.));

    let child = tree
        .element_with_key(&Key::from("corrected"))
        .expect("present in the first layout");
    assert_eq!(
        tree.element_bounds(child).unwrap().origin,
        Offset::new(0., -200.)
    );

    // A negative correction must also settle in this same layout.
    log.borrow_mut().clear();
    assert!(controller.jump_to(350.));
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    assert_eq!(controller.offset(), 200.);
    assert_eq!(log.borrow().first(), Some(&350.));
    assert!(log.borrow().iter().skip(1).all(|offset| *offset == 200.));
    assert_eq!(
        tree.element_bounds(child).unwrap().origin,
        Offset::new(0., -200.)
    );
}
