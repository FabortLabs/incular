use std::{cell::RefCell, rc::Rc};

use incular_config::{Clip, Constraints, StackFit, TextDirection};
use incular_core::{Color, InputEvent, Offset, PointerPhase, Rect, Size};
use incular_rendering::{Canvas, Path};
use incular_runtime::Runtime;
use incular_widgets::{
    CircleBorder, ColoredBox, DecoratedShapeBox, Directionality, GestureDetector, HitTestBehavior,
    ShapeBorder, ShapeDecoration, SizedBox, Stack, Widget,
};

struct NoHit;

impl ShapeBorder for NoHit {
    fn outer_path(&self, rect: Rect) -> Path {
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
    fn inner_path(&self, rect: Rect) -> Path {
        self.outer_path(rect)
    }
    fn hit_test(&self, _: Rect, _: Offset, _: Option<TextDirection>) -> bool {
        false
    }
    fn paint(&self, _: &mut Canvas, _: Rect) {}
}

fn scene(
    direction: TextDirection,
    decoration: ShapeDecoration,
    child: impl Into<Widget>,
    hits: &Rc<RefCell<Vec<&'static str>>>,
) -> Widget {
    let enter = |name| {
        let hits = hits.clone();
        move || hits.borrow_mut().push(name)
    };
    let base = GestureDetector::new(SizedBox::expand())
        .behavior(HitTestBehavior::Opaque)
        .on_tap(enter("behind"));
    let above =
        GestureDetector::new(DecoratedShapeBox::new(decoration, child)).on_tap(enter("shape"));
    let stack = Stack::new([base, above])
        .fit(StackFit::Expand)
        .clip_behavior(Clip::None);
    Directionality::new(direction, stack).into()
}

fn fixture(
    decoration: ShapeDecoration,
    child: impl Into<Widget>,
) -> (Runtime, Rc<RefCell<Vec<&'static str>>>) {
    let hits = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = Runtime::new(scene(TextDirection::Ltr, decoration, child, &hits)).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(120., 80.)))
        .unwrap();
    (runtime, hits)
}

fn tap(runtime: &mut Runtime, at: Offset) {
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer {
            phase,
            position: at,
        });
    }
}

#[test]
fn an_unpainted_corner_passes_through_the_shape_decoration() {
    let decoration = ShapeDecoration::new(CircleBorder::default()).color(Color::BLACK);
    let (mut runtime, hits) = fixture(decoration, SizedBox::expand());
    tap(&mut runtime, Offset::new(10., 10.));
    tap(&mut runtime, Offset::new(60., 40.));
    assert_eq!(hits.borrow().as_slice(), ["behind", "shape"]);
}

#[test]
fn real_child_content_can_still_hit_outside_the_decoration() {
    let decoration = ShapeDecoration::new(CircleBorder::default()).color(Color::BLACK);
    let (mut runtime, hits) = fixture(decoration, ColoredBox::empty(Color::WHITE));
    tap(&mut runtime, Offset::new(10., 10.));
    assert_eq!(hits.borrow().as_slice(), ["shape"]);
}

#[test]
fn the_mounted_decoration_honors_a_shape_specific_hit_override() {
    let decoration = ShapeDecoration::new(NoHit).color(Color::BLACK);
    let (mut runtime, hits) = fixture(decoration, SizedBox::expand());
    tap(&mut runtime, Offset::new(60., 40.));
    assert_eq!(hits.borrow().as_slice(), ["behind"]);
}

#[test]
fn changing_direction_moves_the_rounded_hit_corner() {
    use incular_widgets::{BorderRadiusDirectional, BorderSide, Radius, RoundedRectangleBorder};
    let shape = RoundedRectangleBorder::new(
        BorderSide::NONE,
        BorderRadiusDirectional {
            top_start: Radius::circular(30.),
            ..BorderRadiusDirectional::ZERO
        },
    );
    let decoration = ShapeDecoration::new(shape).color(Color::BLACK);
    let (mut runtime, hits) = fixture(decoration.clone(), SizedBox::expand());
    tap(&mut runtime, Offset::new(4., 4.));
    assert_eq!(hits.borrow().as_slice(), ["behind"]);

    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(
            root,
            scene(TextDirection::Rtl, decoration, SizedBox::expand(), &hits),
        )
        .unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(120., 80.)))
        .unwrap();
    tap(&mut runtime, Offset::new(4., 4.));
    assert_eq!(hits.borrow().as_slice(), ["behind", "shape"]);
}
