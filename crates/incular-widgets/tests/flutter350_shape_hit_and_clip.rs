use std::{cell::Cell, rc::Rc};

use incular_config::TextDirection;
use incular_core::{Offset, Rect, Size};
use incular_rendering::{Canvas, FillRule, Path};
use incular_widgets::{
    CircleBorder, CompoundBorder, OvalBorder, RoundedRectangleBorder, ShapeBorder, ShapeDecoration,
};

struct PathCounter(Rc<Cell<usize>>);
impl ShapeBorder for PathCounter {
    fn outer_path(&self, rect: Rect) -> Path {
        self.0.set(self.0.get() + 1);
        let mut path = Path::builder();
        let min = rect.origin;
        let max = min + Offset::new(rect.size.width, rect.size.height);
        path.move_to(min)
            .line_to(Offset::new(max.x, min.y))
            .line_to(max)
            .line_to(Offset::new(min.x, max.y))
            .close();
        path.build()
    }
    fn inner_path(&self, _: Rect) -> Path {
        panic!("hit testing is about the outer shape")
    }
    fn paint(&self, _: &mut Canvas, _: Rect) {}
}

struct Override(Rc<Cell<usize>>);
impl ShapeBorder for Override {
    fn outer_path(&self, _: Rect) -> Path {
        panic!("the override does not need a path")
    }
    fn inner_path(&self, _: Rect) -> Path {
        panic!("hit testing is about the outer shape")
    }
    fn paint(&self, _: &mut Canvas, _: Rect) {}
    fn hit_test(&self, _: Rect, _: Offset, _: Option<TextDirection>) -> bool {
        self.0.set(self.0.get() + 1);
        false
    }
}

#[test]
fn shape_border_defaults_to_the_outer_path() {
    let calls = Rc::new(Cell::new(0));
    let border = PathCounter(calls.clone());
    let rect = Rect::from_origin_size(Offset::new(10., 20.), Size::new(100., 50.));
    assert!(border.hit_test(rect, Offset::new(20., 30.), None));
    assert!(!border.hit_test(rect, Offset::ZERO, None));
    assert_eq!(calls.get(), 2);
}

#[test]
fn a_shape_decoration_respects_the_shapes_hit_override() {
    let calls = Rc::new(Cell::new(0));
    let decoration = ShapeDecoration::new(Override(calls.clone()));
    assert!(!decoration.hit_test(Size::new(100., 100.), Offset::new(50., 50.), None));
    assert_eq!(calls.get(), 1);
}

#[test]
fn compound_hit_testing_uses_only_the_outer_borders_override() {
    let calls = Rc::new(Cell::new(0));
    let border = CompoundBorder::new(Override(calls.clone()), RoundedRectangleBorder::default());
    let rect = Rect::from_origin_size(Offset::ZERO, Size::new(100., 100.));
    assert!(!border.hit_test(rect, Offset::ZERO, None));
    assert_eq!(calls.get(), 1);
}

fn outline(decoration: ShapeDecoration, height: f32, misses: [Offset; 3]) {
    let rect = Rect::from_origin_size(Offset::ZERO, Size::new(100., height));
    let path = decoration.get_clip_path(rect, Some(TextDirection::Ltr));
    assert!(path.contains(Offset::new(50., 10.), FillRule::NonZero));
    for miss in misses {
        assert!(!path.contains(miss, FillRule::NonZero), "{miss:?}");
    }
}

#[test]
fn a_circle_clip_is_centered_in_the_short_edge() {
    outline(
        ShapeDecoration::new(CircleBorder::default()),
        20.,
        [
            Offset::new(1., 1.),
            Offset::new(30., 10.),
            Offset::new(99., 19.),
        ],
    );
}

#[test]
fn an_oval_clip_uses_the_full_rectangles_aspect_ratio() {
    outline(
        ShapeDecoration::new(OvalBorder::default()),
        50.,
        [
            Offset::new(1., 1.),
            Offset::new(15., 1.),
            Offset::new(99., 19.),
        ],
    );
}
