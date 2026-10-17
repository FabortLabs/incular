use super::shared::{column, heading, key_value, mono};
use super::{border, control_active, gap, primary, surface, text_muted, text_primary, ui_text};
use incular::prelude::*;
use incular_devtools_protocol::LayoutInspection;

pub(crate) fn box_model(layout: &LayoutInspection) -> Widget {
    let content = layout.content_bounds;
    let content_size = format!("{:.1} × {:.1}", content[2], content[3]);
    let padding = layout
        .padding
        .map(|insets| insets.map(|value| format!("{value:.1}")))
        .unwrap_or_else(|| std::array::from_fn(|_| "—".into()));
    let inner = Container::with_child(Align::new(
        Alignment::CENTER,
        column([
            ui_text("content bounds", 10., text_muted()),
            gap(1., 6.),
            mono(content_size, 16., text_primary()),
        ]),
    ))
    .alignment(Alignment::TOP_LEFT)
    .height(66.)
    .color(surface())
    .border(Border::new(1., primary()))
    .radius(3.);
    let diagram = Container::with_child(Padding::all(
        12.,
        column([
            ui_text("PADDING", 10., primary()),
            gap(1., 6.),
            Align::new(Alignment::CENTER, mono(padding[1].clone(), 11., primary())).into(),
            gap(1., 6.),
            Row::new([
                Widget::from(SizedBox::new().width(38.).child(mono(
                    padding[0].clone(),
                    11.,
                    primary(),
                ))),
                Expanded::new(inner).into(),
                SizedBox::new()
                    .width(38.)
                    .child(Align::new(
                        Alignment::CENTER_RIGHT,
                        mono(padding[2].clone(), 11., primary()),
                    ))
                    .into(),
            ])
            .into(),
            gap(1., 6.),
            Align::new(Alignment::CENTER, mono(padding[3].clone(), 11., primary())).into(),
        ]),
    ))
    .alignment(Alignment::TOP_LEFT)
    .color(control_active())
    .border(Border::new(1., border()))
    .radius(5.);
    column([
        heading("Retained box model", 14.),
        gap(1., 6.),
        ui_text("Last completed layout · logical pixels", 11., text_muted()),
        gap(1., 12.),
        diagram.into(),
        gap(1., 12.),
        key_value(
            "Resolved size",
            format!(
                "{:.1} × {:.1}",
                layout.resolved_size[0], layout.resolved_size[1]
            ),
        ),
        key_value(
            "Local offset",
            format!(
                "{:.1}, {:.1}",
                layout.local_offset[0], layout.local_offset[1]
            ),
        ),
        key_value(
            "World bounds",
            format!(
                "({:.1}, {:.1})  {:.1} × {:.1}",
                layout.world_bounds[0],
                layout.world_bounds[1],
                layout.world_bounds[2],
                layout.world_bounds[3]
            ),
        ),
    ])
}
