//! Section 3: one stable-keyed widget per kind DevTools can edit.
//!
//! Search `key:edit-` in the DevTools widget inspector to find each one.
use super::theme::*;
use incular::prelude::*;

pub fn view() -> Widget {
    let text_card = card(
        "Text",
        "key edit-text: fontSize, color",
        400.,
        Widget::from(
            Text::new("Edit my size and colour").style(
                TextStyle::new()
                    .font_size(24.)
                    .font_weight(FontWeight::W700)
                    .color(VIOLET),
            ),
        )
        .with_key("edit-text"),
    );

    let decorated_card = card(
        "Container / DecoratedBox",
        "keys edit-container, edit-decorated: background, radius, border",
        400.,
        Row::new([
            Widget::from(
                Container::new()
                    .width(150.)
                    .height(80.)
                    .radius(16.)
                    .color(AMBER)
                    .border(RenderBorder::new(3., INK))
                    .alignment(Alignment::CENTER)
                    .child(text("Container", 13., FontWeight::W700, INK)),
            )
            .with_key("edit-container"),
            hgap(14.),
            Widget::from(
                DecoratedBox::new(SizedBox::from_size(Size::new(150., 80.)))
                    .background(SKY)
                    .radius(40.)
                    .border(RenderBorder::new(2., INK)),
            )
            .with_key("edit-decorated"),
        ]),
    );

    let padding_card = card(
        "Padding",
        "key edit-padding: padding",
        400.,
        Container::new().color(LINE).radius(8.).child(
            Widget::from(Padding::all(18., swatch(180., 44., TEAL))).with_key("edit-padding"),
        ),
    );

    let sizing_card = card(
        "SizedBox / ConstrainedBox",
        "keys edit-sizedbox, edit-constrained: width, height",
        400.,
        Row::new([
            Widget::from(
                SizedBox::new()
                    .width(120.)
                    .height(60.)
                    .child(swatch(4000., 4000., ROSE)),
            )
            .with_key("edit-sizedbox"),
            hgap(14.),
            Widget::from(ConstrainedBox::new(
                Constraints::new(60., 160., 40., 80.),
                swatch(4000., 4000., SKY),
            ))
            .with_key("edit-constrained"),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start),
    );

    let flex_card = card(
        "Row / Column",
        "keys edit-row, edit-column: mainAxisAlignment, crossAxisAlignment",
        400.,
        Column::new([
            Container::new()
                .width(360.)
                .height(64.)
                .color(LINE)
                .child(
                    Widget::from(
                        Row::new([
                            swatch(50., 30., VIOLET),
                            swatch(50., 44., TEAL),
                            swatch(50., 22., AMBER),
                        ])
                        .main_axis_alignment(MainAxisAlignment::SpaceBetween)
                        .cross_axis_alignment(CrossAxisAlignment::Center),
                    )
                    .with_key("edit-row"),
                )
                .into(),
            gap(12.),
            Container::new()
                .width(360.)
                .height(120.)
                .color(LINE)
                .child(
                    Widget::from(
                        Column::new([
                            swatch(90., 20., VIOLET),
                            swatch(140., 20., TEAL),
                            swatch(60., 20., AMBER),
                        ])
                        .main_axis_alignment(MainAxisAlignment::Start)
                        .cross_axis_alignment(CrossAxisAlignment::Start),
                    )
                    .with_key("edit-column"),
                )
                .into(),
        ]),
    );

    let align_card = card(
        "Align",
        "key edit-align: alignment",
        400.,
        Container::new()
            .width(360.)
            .height(110.)
            .color(LINE)
            .radius(8.)
            .child(
                Widget::from(Align::new(
                    Alignment::BOTTOM_RIGHT,
                    swatch(70., 40., VIOLET),
                ))
                .with_key("edit-align"),
            ),
    );

    let opacity_card = card(
        "Opacity",
        "key edit-opacity: opacity",
        400.,
        Widget::from(Opacity::new(
            0.6,
            Container::new()
                .width(220.)
                .height(70.)
                .radius(12.)
                .color(VIOLET)
                .alignment(Alignment::CENTER)
                .child(text("60% opaque", 15., FontWeight::W700, SURFACE)),
        ))
        .with_key("edit-opacity"),
    );

    page(
        "Editable widgets",
        "Select a card's widget in the Widget inspector and edit its properties live.",
        vec![
            text_card,
            decorated_card,
            padding_card,
            sizing_card,
            flex_card,
            align_card,
            opacity_card,
        ],
    )
}
