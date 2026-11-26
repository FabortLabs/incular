//! Flutter 3.50.0 custom BoxBorder dispatch, identity and mounted painting.
use std::borrow::Cow;

use incular_config::{Constraints, EdgeInsets, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Canvas, PaintCommand};
use incular_widgets::{
    Border, BorderRadius, BorderSide, BoxBorderValue, BoxDecoration, BoxShape, Container,
    CustomBoxBorder, CustomBoxBorderHandle, Directionality, GlobalKey, ShapeDecoration, SizedBox,
    Widget, internal::WidgetTree,
};

const INK: Color = Color::rgba(209, 13, 119, 255);

#[derive(Clone, Copy, Debug)]
struct LeadingBorder {
    width: f32,
    logical: bool,
}

impl LeadingBorder {
    fn new(width: f32) -> Self {
        Self {
            width,
            logical: false,
        }
    }

    fn resolve_right(self, direction: Option<TextDirection>) -> bool {
        self.logical && direction == Some(TextDirection::Rtl)
    }
}

impl CustomBoxBorder for LeadingBorder {
    fn top(&self) -> BorderSide {
        BorderSide::solid(INK, self.width)
    }
    fn bottom(&self) -> BorderSide {
        self.top()
    }
    fn is_uniform(&self) -> bool {
        !self.logical
    }
    fn needs_text_direction(&self) -> bool {
        self.logical
    }

    fn dimensions(&self, direction: Option<TextDirection>) -> EdgeInsets {
        if !self.logical {
            EdgeInsets::all(self.width)
        } else if self.resolve_right(direction) {
            EdgeInsets::only(0., 0., self.width, 0.)
        } else {
            EdgeInsets::only(self.width, 0., 0., 0.)
        }
    }

    fn paint(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        direction: Option<TextDirection>,
        shape: BoxShape,
        radius: Option<BorderRadius>,
    ) -> bool {
        assert_eq!(shape, BoxShape::Rectangle);
        assert!(radius.is_none());
        if self.width <= 0. {
            return false;
        }
        let x = if self.resolve_right(direction) {
            rect.origin.x + rect.size.width - self.width
        } else {
            rect.origin.x
        };
        canvas.rect(
            Rect::from_origin_size(
                Offset::new(x, rect.origin.y),
                Size::new(self.width, rect.size.height),
            ),
            INK,
        );
        true
    }

    fn scale(&self, factor: f32) -> CustomBoxBorderHandle {
        CustomBoxBorderHandle::new(Self {
            width: self.width * factor,
            ..*self
        })
    }

    fn lerp_from(
        &self,
        other: Option<BoxBorderValue<'_>>,
        t: f32,
    ) -> Option<CustomBoxBorderHandle> {
        let Some(other) = other else {
            return Some(self.scale(t));
        };
        let a = other.downcast_ref::<Self>()?;
        Some(CustomBoxBorderHandle::new(Self {
            width: a.width + (self.width - a.width) * t,
            ..*self
        }))
    }

    fn lerp_to(&self, other: Option<BoxBorderValue<'_>>, t: f32) -> Option<CustomBoxBorderHandle> {
        let Some(other) = other else {
            return Some(self.scale(1. - t));
        };
        let BoxBorderValue::Physical(b) = other else {
            return None;
        };
        Some(CustomBoxBorderHandle::new(Self {
            width: self.width + (b.top.width - self.width) * t,
            ..*self
        }))
    }

    fn same_value(&self, other: &dyn CustomBoxBorder) -> bool {
        BoxBorderValue::Custom(other)
            .downcast_ref::<Self>()
            .is_some_and(|other| self.width == other.width && self.logical == other.logical)
    }
}

fn custom(width: f32) -> BoxDecoration {
    BoxDecoration::new().custom_border(LeadingBorder::new(width))
}

fn width(decoration: &BoxDecoration) -> f32 {
    decoration
        .custom_border
        .as_ref()
        .unwrap()
        .downcast_ref::<LeadingBorder>()
        .unwrap()
        .width
}

#[test]
fn box_decoration_optional_lerp_retains_the_identical_input() {
    assert!(BoxDecoration::lerp_optional(None, None, 0.).is_none());
    let decoration = BoxDecoration::new();
    let Some(Cow::Borrowed(borrowed)) =
        BoxDecoration::lerp_optional(Some(&decoration), Some(&decoration), 0.5)
    else {
        panic!("the same decoration should keep its identity");
    };
    assert!(std::ptr::eq(&decoration, borrowed));
}

#[test]
fn box_decoration_invokes_the_custom_border_lerp_hooks() {
    let a = custom(2.);
    let b = custom(6.);
    let mixed = BoxDecoration::lerp_optional(Some(&a), Some(&b), 0.25).unwrap();
    assert_eq!(width(&mixed), 3.);
    assert_eq!(mixed.as_ref(), &custom(3.));
    let builtin = BoxDecoration::new().border(Border::all(BorderSide::solid(INK, 6.)));
    let mixed = BoxDecoration::lerp_optional(Some(&a), Some(&builtin), 0.25).unwrap();
    assert_eq!(width(&mixed), 3.);
    assert_eq!(
        width(&BoxDecoration::lerp_optional(None, Some(&b), 0.25).unwrap()),
        1.5
    );
}

#[test]
fn box_decoration_scale_keeps_the_custom_border_contract() {
    let scaled = custom(8.).scale(0.25);
    assert_eq!(width(&scaled), 2.);
    let size = Size::new(20., 10.);
    let display = scaled.paint_border(size).unwrap();
    assert_eq!(
        display.commands(),
        &[PaintCommand::Rect {
            rect: Rect::from_origin_size(Offset::ZERO, Size::new(2., 10.)),
            color: INK,
        }]
    );
    let shape = ShapeDecoration::from_box_decoration(&scaled);
    assert_eq!(shape.shape.dimensions(None), EdgeInsets::all(2.));
    assert!(custom(8.).scale(0.).paint_border(size).is_none());
}

#[test]
fn mounted_custom_border_resolves_rtl_padding_and_never_draws_an_opposite_edge() {
    let key = GlobalKey::new();
    let decoration = BoxDecoration::new()
        .color(Color::WHITE)
        .custom_border(LeadingBorder {
            width: 3.,
            logical: true,
        });
    let scene = |direction| {
        let child = Widget::from(SizedBox::new().width(10.).height(8.)).with_key(key);
        Widget::from(Directionality::new(
            direction,
            Container::new().child(child).decoration(decoration.clone()),
        ))
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(TextDirection::Ltr)).unwrap();
    let mut retained_child = None;
    for direction in [TextDirection::Ltr, TextDirection::Rtl, TextDirection::Ltr] {
        if retained_child.is_some() {
            tree.update(root, scene(direction)).unwrap();
        }
        tree.layout(Constraints::loose(Size::new(40., 40.)))
            .unwrap();
        let child = tree.element_with_key(&key.into()).unwrap();
        if let Some(previous) = retained_child {
            assert_eq!(child, previous);
        }
        retained_child = Some(child);
        let bounds = tree.element_bounds(child).unwrap();
        assert_eq!(
            bounds.origin,
            if direction == TextDirection::Ltr {
                Offset::new(3., 0.)
            } else {
                Offset::ZERO
            },
        );
        assert_eq!(bounds.size, Size::new(10., 8.));
        let paint = tree.paint();
        let found: Vec<_> = paint
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::Rect { rect, color } if *color == INK => Some(*rect),
                _ => None,
            })
            .collect();
        let x = if direction == TextDirection::Ltr {
            0.
        } else {
            10.
        };
        assert_eq!(
            found,
            [Rect::from_origin_size(
                Offset::new(x, 0.),
                Size::new(3., 8.)
            )]
        );
    }
}
