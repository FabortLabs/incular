//! Section 5: layout bounds, scroll viewports, clips and transforms.
use super::theme::*;
use incular::config::StackFit;
use incular::prelude::*;

pub fn view() -> Widget {
    let list = card(
        "Scrollable list",
        "200 lazily built rows in a 260 px viewport.",
        420.,
        Container::new()
            .height(260.)
            .color(BACKGROUND)
            .radius(8.)
            .child(
                ListView::builder(200, |index| {
                    Container::new()
                        .height(40.)
                        .padding(EdgeInsets::symmetric(12., 0.))
                        .alignment(Alignment::CENTER_LEFT)
                        .color(if index % 2 == 0 { SURFACE } else { VIOLET_SOFT })
                        .child(text(format!("Row {index}"), 13., FontWeight::W500, INK))
                })
                .item_extent(40.),
            ),
    );

    let stack = card(
        "Stack with Positioned",
        "Absolutely placed children overlap inside a 360 x 170 stack.",
        420.,
        Container::new()
            .width(360.)
            .height(170.)
            .color(BACKGROUND)
            .radius(8.)
            .child(
                Stack::new([
                    Positioned::new(swatch(120., 80., VIOLET))
                        .left(12.)
                        .top(12.),
                    Positioned::new(swatch(120., 80., TEAL)).left(80.).top(50.),
                    Positioned::new(swatch(100., 60., AMBER))
                        .right(14.)
                        .bottom(14.),
                    Positioned::new(text("positioned", 12., FontWeight::W700, INK))
                        .right(18.)
                        .top(14.),
                ])
                .fit(StackFit::Expand),
            ),
    );

    let chip = |label: &str, color: Color| -> Widget {
        Container::new()
            .padding(EdgeInsets::symmetric(12., 7.))
            .radius(14.)
            .color(color)
            .child(text(label, 12., FontWeight::W600, SURFACE))
            .into()
    };
    let wrap = card(
        "Wrap",
        "Chips flow onto new runs as the card narrows.",
        420.,
        Wrap::new([
            chip("layout", VIOLET),
            chip("scroll viewport", TEAL),
            chip("clip", AMBER),
            chip("transform", ROSE),
            chip("overlay", SKY),
            chip("hit regions", VIOLET),
            chip("semantics", TEAL),
            chip("repaint boundary", ROSE),
        ])
        .spacing(8.)
        .run_spacing(8.),
    );

    let flex = card(
        "Flex with Expanded",
        "Flex factors 1 : 2 : 1 with a fixed trailing box.",
        420.,
        Container::new()
            .width(380.)
            .height(70.)
            .color(BACKGROUND)
            .child(
                Row::new([
                    Widget::from(Expanded::flex_factor(1, swatch(4000., 4000., VIOLET))),
                    Widget::from(Expanded::flex_factor(2, swatch(4000., 4000., TEAL))),
                    Widget::from(Expanded::flex_factor(1, swatch(4000., 4000., AMBER))),
                    swatch(48., 70., ROSE),
                ])
                .spacing(6.),
            ),
    );

    let clip = card(
        "ClipRRect",
        "An oversized swatch is cut to a rounded 120 px box.",
        420.,
        ClipRRect::new(
            28.,
            Container::new()
                .width(120.)
                .height(120.)
                .color(SURFACE)
                .child(
                    Stack::new([
                        Positioned::new(swatch(220., 60., VIOLET))
                            .left(-30.)
                            .top(10.),
                        Positioned::new(swatch(60., 220., TEAL)).left(60.).top(-30.),
                    ])
                    .fit(StackFit::Expand),
                ),
        ),
    );

    let transform = card(
        "Transform",
        "A rotated and a scaled card; bounds follow the transform.",
        420.,
        Container::new().height(130.).child(
            Row::new([
                Transform::rotation(
                    0.18,
                    Container::new()
                        .width(110.)
                        .height(70.)
                        .radius(10.)
                        .color(ROSE)
                        .alignment(Alignment::CENTER)
                        .child(text("rotated", 13., FontWeight::W700, SURFACE)),
                )
                .into(),
                hgap(40.),
                Transform::scale(
                    1.3,
                    Container::new()
                        .width(80.)
                        .height(50.)
                        .radius(10.)
                        .color(SKY)
                        .alignment(Alignment::CENTER)
                        .child(text("scaled", 12., FontWeight::W700, SURFACE)),
                )
                .into(),
            ])
            .cross_axis_alignment(CrossAxisAlignment::Center),
        ),
    );

    page(
        "Layout & scroll",
        "Turn on layout bounds, scroll viewports, clips and layers in the DevTools overlays.",
        vec![list, stack, wrap, flex, clip, transform],
    )
}
