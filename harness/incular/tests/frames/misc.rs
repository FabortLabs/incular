use std::{cell::Cell, collections::HashSet, rc::Rc};

use incular_config::{Alignment, EdgeInsets, TextDirection};
use incular_core::{Color, Size};
use incular_parity_harness::{FrameHarness, Scenario, probe};
use incular_text::TextEditingController;
use incular_widgets::{
    Align, AutovalidateMode, Builder, Center, ColoredBox, Column, ConnectionState, Directionality,
    Expansible, Form, FutureBuilder, InheritedModel, InheritedModelData, KeyedSubtree, MediaQuery,
    MediaQueryData, RotatedBox, Row, SafeArea, SizedBox, Spacer, StreamBuilder, UiFuture,
    UiStreamController, Widget,
};

const BLUE: Color = Color::rgba(0x21, 0x96, 0xF3, 0xFF);
const ORANGE: Color = Color::rgba(0xFF, 0x57, 0x22, 0xFF);
const GREEN: Color = Color::rgba(0x4C, 0xAF, 0x50, 0xFF);
const RED: Color = Color::rgba(0xF4, 0x43, 0x36, 0xFF);

pub fn scenarios() -> Vec<Scenario> {
    vec![
        (
            "misc__expansible_height_animation",
            expansible_height_animation,
        ),
        ("misc__rotated_box_layout_size", rotated_box_layout_size),
        ("misc__spacer_flex_shares", spacer_flex_shares),
        ("misc__inherited_model_aspects", inherited_model_aspects),
        ("misc__media_query_size_aspect", media_query_size_aspect),
        ("misc__form_save_invalid_field", form_save_invalid_field),
        ("misc__form_field_lifecycle", form_field_lifecycle),
        (
            "misc__future_builder_keyed_reorder",
            future_builder_keyed_reorder,
        ),
        ("misc__stream_builder_events", stream_builder_events),
        (
            "misc__safe_area_nested_consumption",
            safe_area_nested_consumption,
        ),
        ("misc__directionality_row_order", directionality_row_order),
    ]
}

fn colored(color: Color) -> Widget {
    ColoredBox::new(color, SizedBox::shrink()).into()
}

fn connection_code(state: ConnectionState) -> u32 {
    match state {
        ConnectionState::None => 0,
        ConnectionState::Waiting => 1,
        ConnectionState::Active => 2,
        ConnectionState::Done => 3,
    }
}

fn box_widget(color: Color, width: f32, height: f32) -> Widget {
    SizedBox::new()
        .width(width)
        .height(height)
        .child(colored(color))
        .into()
}

fn expansible_height_animation(h: &mut FrameHarness) {
    h.mount({
        move |phase| {
            Align::new(
                Alignment::TOP_LEFT,
                SizedBox::new().width(120.).child(probe(
                    "expansible",
                    Expansible::new(box_widget(BLUE, 120., 32.), box_widget(ORANGE, 120., 80.))
                        .expanded(phase != 0),
                )),
            )
            .into()
        }
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(14);
}

fn rotated_box_layout_size(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(probe(
            "rotated_box",
            RotatedBox::new(1, box_widget(GREEN, 80., 40.)),
        ))
        .into()
    });
    h.frames(1);
}

fn spacer_flex_shares(h: &mut FrameHarness) {
    h.mount(|_| {
        SizedBox::new()
            .width(300.)
            .height(32.)
            .child(Row::new([
                probe("spacer_first", box_widget(BLUE, 40., 32.)),
                Spacer::new().into(),
                probe("spacer_middle", box_widget(ORANGE, 40., 32.)),
                Spacer::new().flex(2).into(),
                probe("spacer_last", box_widget(GREEN, 40., 32.)),
            ]))
            .into()
    });
    h.frames(1);
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PairModel {
    leading: f32,
    trailing: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum PairAspect {
    Leading,
    Trailing,
}

impl InheritedModelData for PairModel {
    type Aspect = PairAspect;

    fn update_should_notify(&self, old: &Self) -> bool {
        self != old
    }

    fn update_should_notify_dependent(&self, old: &Self, aspects: &HashSet<PairAspect>) -> bool {
        (aspects.contains(&PairAspect::Leading) && self.leading != old.leading)
            || (aspects.contains(&PairAspect::Trailing) && self.trailing != old.trailing)
    }
}

fn inherited_model_aspects(h: &mut FrameHarness) {
    let leading_builds = Rc::new(Cell::new(0_u32));
    let trailing_builds = Rc::new(Cell::new(0_u32));
    {
        let builds = leading_builds.clone();
        h.value("leading_builds", move || f64::from(builds.get()));
    }
    {
        let builds = trailing_builds.clone();
        h.value("trailing_builds", move || f64::from(builds.get()));
    }
    let children = Column::new([
        {
            let builds = leading_builds.clone();
            Builder::new(move |context| {
                builds.set(builds.get() + 1);
                let model =
                    InheritedModel::<PairModel>::inherit_from(context, Some(PairAspect::Leading))
                        .expect("leading model");
                probe("inherited_leading", box_widget(BLUE, model.leading, 24.))
            })
        },
        {
            let builds = trailing_builds.clone();
            Builder::new(move |context| {
                builds.set(builds.get() + 1);
                let model =
                    InheritedModel::<PairModel>::inherit_from(context, Some(PairAspect::Trailing))
                        .expect("trailing model");
                probe(
                    "inherited_trailing",
                    box_widget(ORANGE, model.trailing, 24.),
                )
            })
        },
    ]);
    h.mount({
        let children = children.clone();
        move |phase| {
            let (leading, trailing) = match phase {
                0 => (60., 70.),
                1 => (90., 70.),
                _ => (90., 120.),
            };
            InheritedModel::new(PairModel { leading, trailing }, children.clone()).into()
        }
    });
    h.frames(1);
    h.set_phase(1);
    h.frames(1);
    h.set_phase(2);
    h.frames(1);
}

fn media_query_size_aspect(h: &mut FrameHarness) {
    let builder_runs = Rc::new(Cell::new(0_u32));
    {
        let builder_runs = builder_runs.clone();
        h.value("builder_runs", move || f64::from(builder_runs.get()));
    }
    let child = Center::new(Builder::new({
        let builder_runs = builder_runs.clone();
        move |context| {
            builder_runs.set(builder_runs.get() + 1);
            let data = context
                .depend_on::<MediaQueryData>()
                .expect("media query data");
            probe(
                "media_size",
                box_widget(GREEN, data.viewport.width / 2., 24.),
            )
        }
    }));
    h.mount({
        let child = child.clone();
        move |phase| {
            let mut data = MediaQueryData::default();
            data.viewport = Size::new(320., 200.);
            data.scale_factor = if phase == 0 { 1. } else { 2. };
            MediaQuery::new(data, child.clone()).into()
        }
    });
    h.frames(1);
    h.set_phase(1);
    h.frames(1);
}

fn form_save_invalid_field(h: &mut FrameHarness) {
    let form = Form::new();
    let controller = TextEditingController::new();
    let saved_count = Rc::new(Cell::new(0_u32));
    let field = form
        .register(controller)
        .validator(|value| value.is_empty().then(|| "required".to_owned()))
        .on_saved({
            let saved_count = saved_count.clone();
            move |_| saved_count.set(saved_count.get() + 1)
        });
    let valid = Rc::new(Cell::new(-1_i32));
    {
        let valid = valid.clone();
        h.value("form_valid", move || f64::from(valid.get()));
    }
    {
        let saved_count = saved_count.clone();
        h.value("saved_count", move || f64::from(saved_count.get()));
    }
    h.mount({
        let form = form.clone();
        move |_| {
            Align::new(
                Alignment::TOP_LEFT,
                form.clone()
                    .child(probe("form", box_widget(BLUE, 160., 32.))),
            )
            .into()
        }
    });
    h.frames(1);
    valid.set(i32::from(form.validate()));
    h.frames(1);
    let _ = form.save();
    h.frames(1);
    drop(field);
}

fn form_field_lifecycle(h: &mut FrameHarness) {
    let form = Form::new();
    let controller = TextEditingController::new();
    let saved_value_length = Rc::new(Cell::new(-1_i32));
    let field = Rc::new(
        form.register(controller.clone())
            .validator(|value| value.is_empty().then(|| "required".to_owned()))
            .autovalidate(AutovalidateMode::OnUserInteraction)
            .on_saved({
                let saved_value_length = saved_value_length.clone();
                move |value| saved_value_length.set(value.len() as i32)
            }),
    );
    {
        let controller = controller.clone();
        h.value("field_value_length", move || controller.text().len() as f64);
    }
    {
        let field = field.clone();
        h.value("field_has_error", move || {
            if field.error().is_some() { 1. } else { 0. }
        });
    }
    {
        let saved_value_length = saved_value_length.clone();
        h.value("saved_value_length", move || {
            f64::from(saved_value_length.get())
        });
    }
    h.mount({
        let form = form.clone();
        let field = field.clone();
        let controller = controller.clone();
        move |_| {
            Align::new(
                Alignment::TOP_LEFT,
                form.clone().child(Builder::new({
                    let field = field.clone();
                    let controller = controller.clone();
                    move |_| {
                        let color = if field.error().is_some() { RED } else { BLUE };
                        probe(
                            "form_field",
                            box_widget(color, 120. + controller.text().len() as f32 * 4., 32.),
                        )
                    }
                })),
            )
            .into()
        }
    });
    h.frames(1);
    field.set_text("");
    h.frames(1);
    field.set_text("ready");
    h.frames(1);
    let _ = field.validate();
    h.frames(1);
    let _ = form.save();
    h.frames(1);
    field.reset();
    h.frames(1);
}

fn future_builder_keyed_reorder(h: &mut FrameHarness) {
    let (first_future, first_completer) = UiFuture::<String, String>::pending();
    let (second_future, second_completer) = UiFuture::<String, String>::pending();
    let first_state = Rc::new(Cell::new(0_u32));
    let first_length = Rc::new(Cell::new(0_u32));
    let second_state = Rc::new(Cell::new(0_u32));
    let second_length = Rc::new(Cell::new(0_u32));
    {
        let value = first_state.clone();
        h.value("first_state", move || f64::from(value.get()));
    }
    {
        let value = first_length.clone();
        h.value("first_length", move || f64::from(value.get()));
    }
    {
        let value = second_state.clone();
        h.value("second_state", move || f64::from(value.get()));
    }
    {
        let value = second_length.clone();
        h.value("second_length", move || f64::from(value.get()));
    }
    h.mount({
        let first_future = first_future.clone();
        let second_future = second_future.clone();
        let first_state = first_state.clone();
        let first_length = first_length.clone();
        let second_state = second_state.clone();
        let second_length = second_length.clone();
        move |phase| {
            let first = KeyedSubtree::new(
                "first",
                FutureBuilder::new(Some(first_future.clone()), {
                    let first_state = first_state.clone();
                    let first_length = first_length.clone();
                    move |_, snapshot| {
                        first_state.set(connection_code(snapshot.connection_state));
                        let len = snapshot.data.as_ref().map_or(0, |data| data.len() as u32);
                        first_length.set(len);
                        probe("future_first", box_widget(BLUE, 40. + len as f32 * 4., 24.))
                    }
                })
                .initial_data("f0".to_owned()),
            );
            let second = KeyedSubtree::new(
                "second",
                FutureBuilder::new(Some(second_future.clone()), {
                    let second_state = second_state.clone();
                    let second_length = second_length.clone();
                    move |_, snapshot| {
                        second_state.set(connection_code(snapshot.connection_state));
                        let len = snapshot.data.as_ref().map_or(0, |data| data.len() as u32);
                        second_length.set(len);
                        probe(
                            "future_second",
                            box_widget(ORANGE, 40. + len as f32 * 4., 24.),
                        )
                    }
                })
                .initial_data("s0".to_owned()),
            );
            let children = if phase == 0 {
                vec![Widget::from(first), Widget::from(second)]
            } else {
                vec![Widget::from(second), Widget::from(first)]
            };
            Column::new(children).into()
        }
    });
    h.frames(1);
    first_completer.complete("alpha".to_owned());
    second_completer.complete("beta".to_owned());
    h.frames(1);
    h.set_phase(1);
    h.frames(1);
}

fn stream_builder_events(h: &mut FrameHarness) {
    let controller = UiStreamController::<String, String>::new();
    let stream = controller.stream();
    let connection = Rc::new(Cell::new(0_u32));
    let data_length = Rc::new(Cell::new(0_u32));
    let has_error = Rc::new(Cell::new(false));
    {
        let value = connection.clone();
        h.value("stream_state", move || f64::from(value.get()));
    }
    {
        let value = data_length.clone();
        h.value("stream_data_length", move || f64::from(value.get()));
    }
    {
        let value = has_error.clone();
        h.value(
            "stream_has_error",
            move || {
                if value.get() { 1. } else { 0. }
            },
        );
    }
    h.mount({
        let stream = stream.clone();
        move |_| {
            Align::new(
                Alignment::TOP_LEFT,
                StreamBuilder::new(Some(stream.clone()), {
                    let connection = connection.clone();
                    let data_length = data_length.clone();
                    let has_error = has_error.clone();
                    move |_, snapshot| {
                        connection.set(connection_code(snapshot.connection_state));
                        let len = snapshot.data.as_ref().map_or(0, |data| data.len() as u32);
                        data_length.set(len);
                        has_error.set(snapshot.has_error());
                        probe(
                            "stream_builder",
                            box_widget(
                                if snapshot.has_error() { RED } else { GREEN },
                                40. + len as f32 * 4.,
                                24.,
                            ),
                        )
                    }
                })
                .initial_data("seed".to_owned()),
            )
            .into()
        }
    });
    h.frames(1);
    controller.add("one".to_owned());
    h.frames(1);
    controller.add_error("bad".to_owned());
    h.frames(1);
    controller.close();
    h.frames(1);
}

fn safe_area_nested_consumption(h: &mut FrameHarness) {
    let child_top_padding = Rc::new(Cell::new(0.));
    {
        let value = child_top_padding.clone();
        h.value("child_top_padding", move || f64::from(value.get()));
    }
    let child = Builder::new({
        let child_top_padding = child_top_padding.clone();
        move |context| {
            child_top_padding.set(context.safe_insets().top);
            box_widget(BLUE, 100., 50.)
        }
    });
    h.mount({
        let child = child.clone();
        move |_| {
            Center::new(probe(
                "safe_area",
                SafeArea::new(SafeArea::new(child.clone())),
            ))
            .into()
        }
    });
    h.frames(1);
    let mut environment = h.runtime().environment();
    environment.safe_insets = EdgeInsets::only(0., 20., 0., 0.);
    environment.view_padding = EdgeInsets::only(0., 20., 0., 0.);
    let _ = h.runtime().set_environment(environment);
    h.frames(1);
}

fn directionality_row_order(h: &mut FrameHarness) {
    h.mount(|phase| {
        Directionality::new(
            if phase == 0 {
                TextDirection::Ltr
            } else {
                TextDirection::Rtl
            },
            Center::new(SizedBox::new().width(300.).height(32.).child(Row::new([
                probe("direction_first", box_widget(BLUE, 40., 32.)),
                probe("direction_second", box_widget(ORANGE, 40., 32.)),
            ]))),
        )
        .into()
    });
    h.frames(1);
    h.set_phase(1);
    h.frames(1);
}
