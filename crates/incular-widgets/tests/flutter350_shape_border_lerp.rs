use std::{cell::RefCell, rc::Rc};

use incular_config::TextDirection;
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Canvas, FillRule, PaintCommand, Path};
use incular_widgets::{
    BorderDirectional, CircleBorder, RoundedRectangleBorder, ShapeBorder, ShapeDecoration,
    lerp_shape_border,
};

fn frame() -> Rect {
    Rect::from_origin_size(Offset::ZERO, Size::new(100., 60.))
}

struct Fraction(f32);

impl ShapeBorder for Fraction {
    fn outer_path(&self, rect: Rect) -> Path {
        RoundedRectangleBorder::default().outer_path(Rect::from_origin_size(
            rect.origin,
            Size::new(rect.size.width * self.0, rect.size.height),
        ))
    }

    fn inner_path(&self, rect: Rect) -> Path {
        self.outer_path(rect)
    }

    fn paint(&self, _: &mut Canvas, _: Rect) {}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hook {
    From,
    To,
}

type Calls = Rc<RefCell<Vec<(char, Hook, f32)>>>;

struct Custom {
    name: char,
    supported: Option<Hook>,
    calls: Calls,
}

impl Custom {
    fn invoke(
        &self,
        hook: Hook,
        other: Option<&dyn ShapeBorder>,
        t: f32,
    ) -> Option<Rc<dyn ShapeBorder>> {
        self.calls.borrow_mut().push((self.name, hook, t));
        (other.is_some() && self.supported == Some(hook))
            .then(|| Rc::new(Fraction(t)) as Rc<dyn ShapeBorder>)
    }
}

impl ShapeBorder for Custom {
    fn outer_path(&self, rect: Rect) -> Path {
        Fraction(1.).outer_path(rect)
    }

    fn inner_path(&self, rect: Rect) -> Path {
        self.outer_path(rect)
    }

    fn lerp_from(&self, other: Option<&dyn ShapeBorder>, t: f32) -> Option<Rc<dyn ShapeBorder>> {
        self.invoke(Hook::From, other, t)
    }

    fn lerp_to(&self, other: Option<&dyn ShapeBorder>, t: f32) -> Option<Rc<dyn ShapeBorder>> {
        self.invoke(Hook::To, other, t)
    }

    fn paint(&self, _: &mut Canvas, _: Rect) {}
}

#[test]
fn absent_and_identical_shared_shapes_do_not_create_an_intermediate_border() {
    assert!(lerp_shape_border(None, None, 0.).is_none());
    let circle: Rc<dyn ShapeBorder> = Rc::new(CircleBorder::default());
    let directional: Rc<dyn ShapeBorder> = Rc::new(BorderDirectional::default());
    assert_eq!(BorderDirectional::lerp_optional(None, None, 0.), None);
    for shape in [circle, directional] {
        let lerped = lerp_shape_border(Some(&shape), Some(&shape), 0.5).unwrap();
        assert!(Rc::ptr_eq(&shape, &lerped));
        for direction in [TextDirection::Ltr, TextDirection::Rtl] {
            assert_eq!(
                shape.outer_path_with_direction(frame(), Some(direction)),
                lerped.outer_path_with_direction(frame(), Some(direction)),
            );
        }
    }

    let calls = Calls::default();
    let custom: Rc<dyn ShapeBorder> = Rc::new(Custom {
        name: 'a',
        supported: Some(Hook::From),
        calls: calls.clone(),
    });
    let lerped = lerp_shape_border(Some(&custom), Some(&custom), 0.5).unwrap();
    assert!(Rc::ptr_eq(&custom, &lerped));
    assert!(calls.borrow().is_empty());
}

#[test]
fn forward_dispatch_falls_back_to_reverse_to_then_reverse_from_with_inverted_time() {
    use Hook::{From, To};
    for (source, target, expected) in [
        (
            None,
            Some(To),
            vec![('b', From, 0.25), ('a', To, 0.25), ('b', To, 0.75)],
        ),
        (
            Some(From),
            None,
            vec![
                ('b', From, 0.25),
                ('a', To, 0.25),
                ('b', To, 0.75),
                ('a', From, 0.75),
            ],
        ),
    ] {
        let calls = Calls::default();
        let a: Rc<dyn ShapeBorder> = Rc::new(Custom {
            name: 'a',
            supported: source,
            calls: calls.clone(),
        });
        let b: Rc<dyn ShapeBorder> = Rc::new(Custom {
            name: 'b',
            supported: target,
            calls: calls.clone(),
        });
        let value = lerp_shape_border(Some(&a), Some(&b), 0.25).unwrap();
        assert_eq!(*calls.borrow(), expected);
        assert_eq!(
            value.outer_path(frame()).bounds().unwrap().size,
            Size::new(75., 60.)
        );
    }
}

#[test]
fn a_custom_reverse_interpolation_reaches_both_hit_testing_and_paint() {
    let trace = Calls::default();
    let a = ShapeDecoration::new(Custom {
        name: 'a',
        supported: None,
        calls: trace.clone(),
    })
    .color(Color::WHITE);
    let b = ShapeDecoration::new(Custom {
        name: 'b',
        supported: Some(Hook::To),
        calls: trace.clone(),
    })
    .color(Color::WHITE);
    let value = ShapeDecoration::lerp_optional(Some(&a), Some(&b), 0.25).unwrap();
    let rect = frame();
    let inside = Offset::new(74., 30.);
    let outside = Offset::new(76., 30.);
    assert!(value.hit_test(rect.size, inside, None));
    assert!(!value.hit_test(rect.size, outside, None));
    let clip = value.get_clip_path(rect, None);
    let list = value.paint_fill(rect.size);
    let [
        PaintCommand::FillPath {
            path, fill_rule, ..
        },
    ] = list.commands()
    else {
        panic!("interpolated outline must fill only its own path");
    };
    assert_eq!(*fill_rule, FillRule::NonZero);
    assert_eq!(path.as_ref(), &clip);
    assert!(!path.contains(outside, *fill_rule));
}

#[test]
fn a_successful_forward_hook_prevents_any_other_interpolation_call() {
    let calls = Calls::default();
    let a: Rc<dyn ShapeBorder> = Rc::new(Custom {
        name: 'a',
        supported: Some(Hook::To),
        calls: calls.clone(),
    });
    let b: Rc<dyn ShapeBorder> = Rc::new(Custom {
        name: 'b',
        supported: Some(Hook::From),
        calls: calls.clone(),
    });
    let value = lerp_shape_border(Some(&a), Some(&b), 0.25).unwrap();
    assert_eq!(*calls.borrow(), [('b', Hook::From, 0.25)]);
    assert_eq!(
        value.outer_path(frame()).bounds().unwrap().size,
        Size::new(25., 60.)
    );
}
