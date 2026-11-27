use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_rendering::{DisplayList, GaussianBlur, ImageFilter, PaintCommand};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{ColoredBox, CustomPaint, CustomPainter, ImageFiltered, Widget};

struct PaintCounter(Rc<Cell<usize>>);

impl CustomPainter for PaintCounter {
    fn paint(&self, _: Size) -> DisplayList {
        self.0.set(self.0.get() + 1);
        DisplayList::default()
    }

    fn should_repaint(&self, _: &Self) -> bool {
        false
    }
}

#[test]
fn animating_image_filter_does_not_repaint_its_child() {
    let paints = Rc::new(Cell::new(0));
    let child: Widget =
        CustomPaint::from_painter(Size::ZERO, PaintCounter(Rc::clone(&paints))).into();
    let build = |sigma| {
        ColoredBox::new(
            Color::rgba(255, 0, 0, 255),
            ImageFiltered::new(ImageFilter::blur(sigma, sigma), child.clone()).unwrap(),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(5.)).unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    for sigma in [5., 6.] {
        if sigma == 6. {
            tree.update(root, build(sigma)).unwrap();
        }
        tree.layout(constraints).unwrap();
        let scene = tree.paint();
        assert_eq!(paints.get(), 1, "child paints at sigma {sigma}");
        assert!(scene.commands().iter().any(|command| {
            matches!(command, PaintCommand::PushBlur { blur, .. } if *blur == GaussianBlur::uniform(sigma))
        }));
    }
}
