//! Palette and small layout helpers shared by every playground section.
use incular::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

pub const BACKGROUND: Color = Color::rgba(246, 245, 251, 255);
pub const SURFACE: Color = Color::rgba(255, 255, 255, 255);
pub const LINE: Color = Color::rgba(224, 221, 236, 255);
pub const INK: Color = Color::rgba(30, 27, 46, 255);
pub const MUTED: Color = Color::rgba(110, 106, 134, 255);
pub const VIOLET: Color = Color::rgba(128, 0, 255, 255);
pub const VIOLET_SOFT: Color = Color::rgba(240, 229, 255, 255);
pub const TEAL: Color = Color::rgba(14, 165, 143, 255);
pub const AMBER: Color = Color::rgba(245, 158, 11, 255);
pub const ROSE: Color = Color::rgba(236, 72, 120, 255);
pub const SKY: Color = Color::rgba(56, 132, 244, 255);

/// A text run with an explicit size, weight and colour.
pub fn text(value: impl Into<String>, size: f32, weight: FontWeight, color: Color) -> Widget {
    Text::new(value)
        .style(
            TextStyle::new()
                .font_size(size)
                .font_weight(weight)
                .color(color),
        )
        .into()
}

pub fn body(value: impl Into<String>) -> Widget {
    text(value, 14., FontWeight::NORMAL, INK)
}

pub fn muted(value: impl Into<String>) -> Widget {
    text(value, 12.5, FontWeight::NORMAL, MUTED)
}

pub fn gap(height: f32) -> Widget {
    SizedBox::new().height(height).into()
}

pub fn hgap(width: f32) -> Widget {
    SizedBox::new().width(width).into()
}

/// A solid, fixed-size swatch used as a recognisable inspector target.
pub fn swatch(width: f32, height: f32, color: Color) -> Widget {
    Container::new()
        .width(width)
        .height(height)
        .color(color)
        .radius(6.)
        .into()
}

/// A labelled white card of a fixed width.
pub fn card(title: &str, subtitle: &str, width: f32, child: impl Into<Widget>) -> Widget {
    Container::new()
        .width(width)
        .padding(EdgeInsets::all(18.))
        .color(SURFACE)
        .radius(14.)
        .border(RenderBorder::new(1., LINE))
        .child(
            Column::new([
                text(title, 15., FontWeight::W700, INK),
                gap(2.),
                muted(subtitle),
                gap(14.),
                child.into(),
            ])
            .cross_axis_alignment(CrossAxisAlignment::Start),
        )
        .into()
}

/// A page: heading, one-line description, then cards flowing in a wrapping grid.
pub fn page(title: &str, blurb: &str, cards: Vec<Widget>) -> Widget {
    SingleChildScrollView::new(
        Container::new().padding(EdgeInsets::all(28.)).child(
            Column::new([
                text(title, 26., FontWeight::W800, INK),
                gap(4.),
                muted(blurb),
                gap(22.),
                Wrap::new(cards).spacing(18.).run_spacing(18.).into(),
            ])
            .cross_axis_alignment(CrossAxisAlignment::Start),
        ),
    )
    .into()
}

/// Rebuilds `build` on every animation tick of `controller`. The controller
/// repeats forever, so the window keeps presenting frames while this is mounted.
pub fn ticking(
    controller: &AnimationController,
    reverse: bool,
    build: impl Fn(f32) -> Widget + 'static,
) -> Widget {
    let revision = Rc::new(Cell::new(0_u64));
    let reader = controller.clone();
    let body = Widget::stateful_builder(revision.clone(), move |_| build(reader.value()));
    Widget::animation_ticker_repeating_with_retarget(
        controller.clone(),
        reverse,
        revision,
        None,
        body,
    )
}

/// Maps a hue in `[0, 1)` to a saturated colour.
pub fn hue(h: f32) -> Color {
    let h = h.rem_euclid(1.) * 6.;
    let x = 1. - (h % 2. - 1.).abs();
    let (r, g, b) = match h as u32 {
        0 => (1., x, 0.),
        1 => (x, 1., 0.),
        2 => (0., 1., x),
        3 => (0., x, 1.),
        4 => (x, 0., 1.),
        _ => (1., 0., x),
    };
    let lift = |c: f32| (80. + c * 150.) as u8;
    Color::rgba(lift(r), lift(g), lift(b), 255)
}
