//! Typed in-place property editors for the inspector's Properties tab.
//!
//! Every editable property edits where it is shown: numbers get a field with
//! steppers, drag-to-scrub on the label and arrow keys; colours a swatch with a
//! palette and hex field; enums a segmented control, menu or 3x3 grid; insets
//! four linked fields; booleans a switch. Overridden rows carry an accent dot
//! and a revert button, and a rejected edit shows the target's message inline.

use super::shared::{gap, input_style, mono, refresh, state_surface, text_style, ui_text};
use super::theme::{
    background, border, control, control_active, danger, on_accent, primary, text_muted,
    text_primary,
};
use crate::{
    inspector::{debug_value, parse_debug_value},
    property_edit::{
        EditThrottle, EditorKind, Offer, OriginalValues, PALETTE, alignment_cell, alignment_name,
        apply_inset_edit, clamp_to, color_of, editor_kind, float_of, float_text, format_color,
        format_float, insets_uniform, parse_color, parse_float, property_key, step_value,
    },
    transport::ClientBridge,
};
use incular::controls::TextField;
use incular::material::RawMaterialButton;
use incular::prelude::*;
use incular::widgets::HitTestBehavior;
use incular::widgets::internal::{TextEditingController, icons};
use incular_devtools_protocol::{DebugProperty, DebugValue, DevWidgetId, RequestMethod};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Instant,
};

const FIELD_HEIGHT: f32 = 26.;
const CONTROL_HEIGHT: f32 = 24.;
/// Width of the dot / revert slots that keep every row's columns aligned.
const MARKER_SLOT: f32 = 14.;
const REVERT_SLOT: f32 = 46.;
const LABEL_WIDTH: f32 = 112.;

struct EditorField {
    controller: TextEditingController,
    /// Text last written from the target's value, so typing in progress is
    /// only replaced when the target's value actually changes.
    synced: String,
}

/// UI-thread state shared by every editor row: text controllers, originals for
/// revert, the open palette/menu, and the edit rate limiter.
#[derive(Clone, Default)]
pub(crate) struct PropertyEditors {
    fields: Rc<RefCell<HashMap<String, EditorField>>>,
    originals: Rc<RefCell<OriginalValues>>,
    throttle: Arc<Mutex<EditThrottle>>,
    open: Rc<RefCell<Option<String>>>,
    links: Rc<RefCell<HashMap<String, bool>>>,
    /// Text updates found while building; applied outside the build because
    /// a controller is reactive state that must not change mid-build.
    pending: Rc<RefCell<Vec<(TextEditingController, String)>>>,
}

impl PropertyEditors {
    /// The persistent controller for `slot`, showing `text` whenever the
    /// target's value changed since the last sync.
    fn field(&self, slot: &str, text: &str) -> TextEditingController {
        let mut fields = self.fields.borrow_mut();
        let field = fields.entry(slot.to_owned()).or_insert_with(|| {
            let controller = TextEditingController::with_text(text.to_owned());
            EditorField {
                controller,
                synced: text.to_owned(),
            }
        });
        if field.synced != text {
            field.synced = text.to_owned();
            self.pending
                .borrow_mut()
                .push((field.controller.clone(), text.to_owned()));
        }
        field.controller.clone()
    }

    /// Applies the text updates queued by the last build (call from an effect).
    pub(crate) fn flush_pending(&self) {
        let pending = std::mem::take(&mut *self.pending.borrow_mut());
        for (controller, text) in pending {
            if controller.text() != text {
                controller.set_text(text);
            }
        }
    }

    /// Whether any property was edited from this session.
    pub(crate) fn has_originals(&self) -> bool {
        !self.originals.borrow().is_empty()
    }

    /// Forgets recorded originals after the target dropped every override.
    pub(crate) fn forget_originals(&self) {
        self.originals.borrow_mut().clear();
    }

    fn is_open(&self, key: &str) -> bool {
        self.open.borrow().as_deref() == Some(key)
    }

    fn toggle_open(&self, key: &str) {
        let mut open = self.open.borrow_mut();
        *open = if open.as_deref() == Some(key) {
            None
        } else {
            Some(key.to_owned())
        };
    }
}

/// What every row needs to talk to the target.
#[derive(Clone)]
pub(crate) struct RowEnv {
    pub(crate) editors: PropertyEditors,
    pub(crate) bridge: ClientBridge,
    pub(crate) tick: Signal<u64>,
}

/// One property's edit session: sends, throttles and reverts.
struct Edit {
    env: RowEnv,
    id: DevWidgetId,
    name: String,
    key: String,
    current: DebugValue,
}

impl Edit {
    fn request(&self, value: DebugValue) -> RequestMethod {
        RequestMethod::EditProperty {
            id: self.id,
            name: self.name.clone(),
            value,
        }
    }

    fn send(&self, value: DebugValue) {
        self.env
            .editors
            .originals
            .borrow_mut()
            .record(&self.key, &self.current);
        let offer = self
            .env
            .editors
            .throttle
            .lock()
            .map(|mut throttle| throttle.offer(&self.key, value, Instant::now()));
        match offer {
            Ok(Offer::SendNow(value)) => self.env.bridge.send_or_report(self.request(value)),
            Ok(Offer::Deferred {
                flush_after: Some(delay),
            }) => {
                let bridge = self.env.bridge.clone();
                let throttle = Arc::clone(&self.env.editors.throttle);
                let key = self.key.clone();
                let (id, name) = (self.id, self.name.clone());
                std::thread::spawn(move || {
                    std::thread::sleep(delay);
                    let value = throttle
                        .lock()
                        .ok()
                        .and_then(|mut throttle| throttle.flush(&key, Instant::now()));
                    if let Some(value) = value {
                        bridge.send_or_report(RequestMethod::EditProperty { id, name, value });
                    }
                });
            }
            _ => {}
        }
        refresh(&self.env.tick);
    }

    fn revert(&self) {
        let value = self.env.editors.originals.borrow().revert_value(&self.key);
        if let Some(value) = value {
            if let Ok(mut throttle) = self.env.editors.throttle.lock() {
                throttle.cancel(&self.key, Instant::now());
            }
            self.env.bridge.send_or_report(self.request(value));
            refresh(&self.env.tick);
        }
    }
}

fn square_button(
    label: impl Into<String>,
    caption: &str,
    width: f32,
    selected: bool,
    enabled: bool,
    callback: impl Fn() + 'static,
) -> Widget {
    state_surface(
        RawMaterialButton::new(label)
            .size(Size::new(width, CONTROL_HEIGHT))
            .padding(EdgeInsets::symmetric(4., 3.))
            .label_style(text_style(
                12.,
                if selected {
                    primary()
                } else if enabled {
                    text_primary()
                } else {
                    super::theme::text_disabled()
                },
            ))
            .content(Align::new(
                Alignment::CENTER,
                ui_text(
                    caption.to_owned(),
                    12.,
                    if selected {
                        primary()
                    } else if enabled {
                        text_primary()
                    } else {
                        super::theme::text_disabled()
                    },
                ),
            ))
            .enabled(enabled)
            .on_press(callback),
        if selected {
            control_active()
        } else {
            control()
        },
        4.,
    )
}

fn field_box(field: impl Into<Widget>) -> Widget {
    SizedBox::new().height(FIELD_HEIGHT).child(field).into()
}

fn text_input(
    label: String,
    controller: TextEditingController,
    on_submit: impl Fn(String) + 'static,
) -> Widget {
    Semantics::new(field_box(
        TextField::new(controller)
            .size(Size::new(0., 14.))
            .style(input_style())
            .on_submit(on_submit),
    ))
    .role(SemanticRole::TextField)
    .label(label)
    .state(SemanticState {
        enabled: true,
        ..SemanticState::default()
    })
    .into()
}

fn stretch_column(children: impl IntoIterator<Item = Widget>) -> Widget {
    Column::new(children)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .main_axis_size(MainAxisSize::Min)
        .into()
}

/// Lays children out in rows of `per_row`; with `fill` each cell shares the
/// row width equally, otherwise cells keep their natural size.
fn chunked(children: Vec<Widget>, per_row: usize, fill: bool) -> Widget {
    let mut rows: Vec<Widget> = Vec::new();
    let mut pending = children.into_iter().peekable();
    while pending.peek().is_some() {
        let mut cells: Vec<Widget> = Vec::new();
        for slot in 0..per_row {
            if slot > 0 {
                cells.push(gap(4., 1.));
            }
            match pending.next() {
                Some(child) if fill => cells.push(expanded(child)),
                Some(child) => cells.push(child),
                None if fill => cells.push(expanded(gap(1., 1.))),
                None => {}
            }
        }
        if !rows.is_empty() {
            rows.push(gap(1., 4.));
        }
        rows.push(Row::new(cells).into());
    }
    stretch_column(rows)
}

fn expanded(widget: Widget) -> Widget {
    Expanded::new(widget).into()
}

fn row_of(children: Vec<Widget>) -> Widget {
    Row::new(children)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .into()
}

/// A rendered row for one property: editor when editable, muted read-only
/// text otherwise.
pub(crate) fn property_row(
    env: &RowEnv,
    id: DevWidgetId,
    property: &DebugProperty,
    error: Option<&str>,
) -> Widget {
    if !property.editable {
        return read_only_row(property);
    }
    let key = property_key(id, &property.name);
    let kind = editor_kind(property);
    let modified =
        env.editors
            .originals
            .borrow()
            .is_modified(&key, property.overridden, &property.value)
            && property.overridden;
    let has_original = env.editors.originals.borrow().original(&key).is_some();
    let edit = Rc::new(Edit {
        env: env.clone(),
        id,
        name: property.name.clone(),
        key: key.clone(),
        current: property.value.clone(),
    });

    let marker: Widget = SizedBox::new()
        .width(MARKER_SLOT)
        .height(FIELD_HEIGHT)
        .child(Align::new(
            Alignment::CENTER_LEFT,
            Container::new()
                .width(6.)
                .height(6.)
                .color(if modified {
                    primary()
                } else {
                    Color::TRANSPARENT
                })
                .radius(3.),
        ))
        .into();
    let revert: Widget = if modified && has_original {
        let revert_edit = edit.clone();
        SizedBox::new()
            .width(REVERT_SLOT)
            .height(FIELD_HEIGHT)
            .child(Align::new(
                Alignment::CENTER_RIGHT,
                square_button(
                    format!("Revert {}", property.name),
                    "Undo",
                    40.,
                    false,
                    true,
                    move || revert_edit.revert(),
                ),
            ))
            .into()
    } else {
        gap(REVERT_SLOT, 1.)
    };

    let scrub: Option<Rc<dyn Fn(f32)>> = match &kind {
        EditorKind::Float { min, max, step } | EditorKind::OptionalFloat { min, max, step } => {
            let (min, max, step) = (*min, *max, *step);
            let edit = edit.clone();
            let slot = format!("{key}#v");
            let optional = matches!(kind, EditorKind::OptionalFloat { .. });
            Some(Rc::new(move |dx: f32| {
                let controller = edit.env.editors.field(&slot, "");
                let base = parse_float(&controller.text()).or_else(|| float_of(&edit.current));
                let Some(base) = base else { return };
                let direction = f64::from(dx.round());
                if direction == 0. {
                    return;
                }
                let next = step_value(base, direction, step, false, min, max);
                controller.set_text(format_float(next));
                edit.send(float_value(optional, next));
            }))
        }
        _ => None,
    };
    let label_color = if modified { primary() } else { text_primary() };
    let label = property_label(&property.name, label_color, scrub);

    let stacked = matches!(kind, EditorKind::Insets { .. });
    let editor = build_editor(env, &edit, property, kind);
    let head: Widget = if stacked {
        row_of(vec![marker, expanded(label), revert])
    } else {
        Row::new(vec![
            marker,
            label,
            expanded(editor.clone()),
            gap(4., 1.),
            revert,
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .into()
    };
    let mut rows = vec![head];
    if stacked {
        rows.push(
            Padding::new(
                EdgeInsets::from_ltrb(MARKER_SLOT, 0., REVERT_SLOT, 0.),
                editor,
            )
            .into(),
        );
    }
    if let Some(message) = error {
        rows.push(
            Padding::new(
                EdgeInsets::from_ltrb(MARKER_SLOT, 2., 0., 0.),
                ui_text(format!("Rejected: {message}"), 11., danger()),
            )
            .into(),
        );
    }
    Padding::new(
        EdgeInsets::symmetric(0., 3.),
        Column::new(rows)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .main_axis_size(MainAxisSize::Min),
    )
    .into()
}

fn property_label(name: &str, color: Color, scrub: Option<Rc<dyn Fn(f32)>>) -> Widget {
    let text: Widget = Text::new(name.to_owned())
        .style(text_style(12., color))
        .max_lines(Some(1))
        .overflow(TextOverflow::Ellipsis)
        .into();
    let body: Widget = SizedBox::new()
        .width(LABEL_WIDTH)
        .height(FIELD_HEIGHT)
        .child(Align::new(Alignment::CENTER_LEFT, text))
        .into();
    match scrub {
        Some(scrub) => GestureDetector::new(body)
            .behavior(HitTestBehavior::Opaque)
            .on_horizontal_drag_update(move |details| scrub(details.delta.x))
            .into(),
        None => body,
    }
}

fn read_only_row(property: &DebugProperty) -> Widget {
    Padding::new(
        EdgeInsets::from_ltrb(MARKER_SLOT, 3., 0., 3.),
        Row::new([
            Widget::from(
                SizedBox::new().width(LABEL_WIDTH).child(
                    Text::new(property.name.clone())
                        .style(text_style(12., text_muted()))
                        .max_lines(Some(1))
                        .overflow(TextOverflow::Ellipsis),
                ),
            ),
            Expanded::new(mono(debug_value(&property.value), 12., text_muted())).into(),
        ]),
    )
    .into()
}

fn float_value(optional: bool, value: f64) -> DebugValue {
    if optional {
        DebugValue::Optional(Some(Box::new(DebugValue::Float(value))))
    } else {
        DebugValue::Float(value)
    }
}

fn build_editor(
    env: &RowEnv,
    edit: &Rc<Edit>,
    property: &DebugProperty,
    kind: EditorKind,
) -> Widget {
    let key = edit.key.clone();
    match kind {
        EditorKind::Float { min, max, step } => {
            float_editor(env, edit, &property.value, min, max, step, false)
        }
        EditorKind::OptionalFloat { min, max, step } => {
            float_editor(env, edit, &property.value, min, max, step, true)
        }
        EditorKind::Color => color_editor(env, edit, &property.value, false),
        EditorKind::OptionalColor => color_editor(env, edit, &property.value, true),
        EditorKind::Segmented(options) => {
            let current = enum_text(&property.value);
            segmented(edit, &property.name, &options, &current)
        }
        EditorKind::Menu(options) => {
            let current = enum_text(&property.value);
            menu(env, edit, &property.name, &options, &current)
        }
        EditorKind::AlignmentGrid(_) => {
            alignment_grid(edit, &property.name, &enum_text(&property.value))
        }
        EditorKind::Insets { min, max, step } => {
            insets_editor(env, edit, &property.value, min, max, step)
        }
        EditorKind::Bool => bool_switch(edit, &property.name, &property.value),
        EditorKind::Text => {
            let text = match &property.value {
                DebugValue::Str(text) => text.clone(),
                other => debug_value(other),
            };
            let controller = env.editors.field(&format!("{key}#v"), &text);
            let edit = edit.clone();
            text_input(
                format!("{} value", property.name),
                controller,
                move |input| {
                    edit.send(DebugValue::Str(input));
                },
            )
        }
        EditorKind::Raw => {
            let text = debug_value(&property.value);
            let controller = env.editors.field(&format!("{key}#v"), &text);
            let edit = edit.clone();
            let template = property.value.clone();
            text_input(
                format!("{} value", property.name),
                controller,
                move |input| {
                    if let Some(value) = parse_debug_value(&template, &input) {
                        edit.send(value);
                    }
                },
            )
        }
    }
}

fn enum_text(value: &DebugValue) -> String {
    match value {
        DebugValue::Enum(text) => text.clone(),
        other => debug_value(other),
    }
}

/// Wraps a field so ArrowUp/ArrowDown step the number (Shift steps by ten).
fn with_arrow_steps(field: Widget, step: impl Fn(f64, bool) + 'static) -> Widget {
    KeyboardListener::new(field)
        .on_key(move |event| {
            if !event.state.is_down() {
                return false;
            }
            let direction = match event.code {
                Code::ArrowUp => 1.,
                Code::ArrowDown => -1.,
                _ => return false,
            };
            step(direction, event.modifiers.contains(Modifiers::SHIFT));
            true
        })
        .into()
}

fn float_editor(
    env: &RowEnv,
    edit: &Rc<Edit>,
    value: &DebugValue,
    min: Option<f64>,
    max: Option<f64>,
    step: f64,
    optional: bool,
) -> Widget {
    let slot = format!("{}#v", edit.key);
    let unbounded = matches!(value, DebugValue::Optional(None));
    if unbounded {
        let set_edit = edit.clone();
        let fallback = clamp_to(100., min, max);
        return row_of(vec![
            Widget::from(
                SizedBox::new()
                    .height(FIELD_HEIGHT)
                    .width(96.)
                    .child(Align::new(
                        Alignment::CENTER_LEFT,
                        mono("∞  unbounded", 12., text_muted()),
                    )),
            ),
            gap(6., 1.),
            square_button(
                format!("Set finite {}", edit.name),
                "Set value",
                70.,
                false,
                true,
                move || set_edit.send(float_value(true, fallback)),
            ),
        ]);
    }
    let text = float_text(value).unwrap_or_default();
    let controller = env.editors.field(&slot, &text);
    let commit = {
        let edit = edit.clone();
        let controller = controller.clone();
        move |input: String| {
            let Some(parsed) = parse_float(&input) else {
                return;
            };
            let next = clamp_to(parsed, min, max);
            controller.set_text(format_float(next));
            edit.send(float_value(optional, next));
        }
    };
    let stepper = {
        let edit = edit.clone();
        let controller = controller.clone();
        let current = float_of(value).unwrap_or(0.);
        Rc::new(move |direction: f64, fast: bool| {
            let base = parse_float(&controller.text()).unwrap_or(current);
            let next = step_value(base, direction, step, fast, min, max);
            controller.set_text(format_float(next));
            edit.send(float_value(optional, next));
        })
    };
    let field = with_arrow_steps(
        text_input(format!("{} value", edit.name), controller, commit),
        {
            let stepper = stepper.clone();
            move |direction, fast| stepper(direction, fast)
        },
    );
    let (minus, plus) = (stepper.clone(), stepper);
    let mut parts = vec![
        square_button(
            format!("Decrease {}", edit.name),
            "−",
            24.,
            false,
            true,
            move || minus(-1., false),
        ),
        gap(4., 1.),
        expanded(field),
        gap(4., 1.),
        square_button(
            format!("Increase {}", edit.name),
            "+",
            24.,
            false,
            true,
            move || plus(1., false),
        ),
    ];
    if optional {
        let infinite_edit = edit.clone();
        parts.push(gap(4., 1.));
        parts.push(square_button(
            format!("Unbound {}", edit.name),
            "∞",
            24.,
            false,
            true,
            move || infinite_edit.send(DebugValue::Optional(None)),
        ));
    }
    let controls = row_of(parts);
    match (min, max) {
        (Some(min), Some(max)) if max - min > 0. && max - min <= 10. => {
            let slider_edit = edit.clone();
            let controller = env.editors.field(&slot, &text);
            let current = float_of(value).unwrap_or(min);
            stretch_column([
                controls,
                gap(1., 4.),
                slider(min, max, step, current, move |next| {
                    controller.set_text(format_float(next));
                    slider_edit.send(float_value(optional, next));
                }),
            ])
        }
        _ => controls,
    }
}

/// A horizontal track for bounded ranges: tap or drag to set the value.
fn slider(min: f64, max: f64, step: f64, value: f64, on_change: impl Fn(f64) + 'static) -> Widget {
    let on_change = Rc::new(on_change);
    LayoutBuilder::new(move |_, constraints| {
        let width = constraints.max_width().max(40.);
        let t = ((value - min) / (max - min)).clamp(0., 1.) as f32;
        let mut canvas = Canvas::default();
        canvas.rect(
            incular::core::Rect::from_origin_size(Offset::new(0., 8.), Size::new(width, 4.)),
            border(),
        );
        canvas.rect(
            incular::core::Rect::from_origin_size(Offset::new(0., 8.), Size::new(width * t, 4.)),
            primary(),
        );
        let thumb_x = (width * t - 4.).clamp(0., width - 8.);
        canvas.rect(
            incular::core::Rect::from_origin_size(Offset::new(thumb_x, 3.), Size::new(8., 14.)),
            primary(),
        );
        let set = {
            let on_change = on_change.clone();
            move |x: f32| {
                let t = f64::from((x / width).clamp(0., 1.));
                let raw = min + t * (max - min);
                let snapped = (raw / step).round() * step;
                on_change(clamp_to(
                    (snapped * 1e6).round() / 1e6,
                    Some(min),
                    Some(max),
                ));
            }
        };
        let drag = set.clone();
        GestureDetector::new(CustomPaint::new(Size::new(width, 20.), canvas.finish()))
            .behavior(HitTestBehavior::Opaque)
            .on_tap_down(move |details| set(details.local_position.x))
            .on_horizontal_drag_update(move |details| drag(details.local_position.x))
            .into()
    })
    .into()
}

fn color_editor(env: &RowEnv, edit: &Rc<Edit>, value: &DebugValue, optional: bool) -> Widget {
    let slot = format!("{}#v", edit.key);
    let color = color_of(value);
    let text = color.map_or_else(String::new, |(r, g, b, a)| format_color(r, g, b, a));
    let controller = env.editors.field(&slot, &text);
    let wrap = move |(r, g, b, a): (u8, u8, u8, u8)| {
        let color = DebugValue::Color(r, g, b, a);
        if optional {
            DebugValue::Optional(Some(Box::new(color)))
        } else {
            color
        }
    };
    let commit = {
        let edit = edit.clone();
        let controller = controller.clone();
        move |input: String| {
            if let Some((r, g, b, a)) = parse_color(&input) {
                controller.set_text(format_color(r, g, b, a));
                edit.send(wrap((r, g, b, a)));
            }
        }
    };
    let swatch_fill = color.map_or(Color::TRANSPARENT, |(r, g, b, a)| Color::rgba(r, g, b, a));
    let open = env.editors.is_open(&edit.key);
    let toggle = {
        let editors = env.editors.clone();
        let key = edit.key.clone();
        let tick = env.tick.clone();
        move || {
            editors.toggle_open(&key);
            refresh(&tick);
        }
    };
    let swatch = RawMaterialButton::new(format!("Color palette {}", edit.name))
        .size(Size::new(CONTROL_HEIGHT, CONTROL_HEIGHT))
        .content(Align::new(
            Alignment::CENTER,
            Container::new()
                .width(18.)
                .height(18.)
                .color(swatch_fill)
                .border(Border::new(1., if open { primary() } else { text_muted() }))
                .radius(4.),
        ))
        .on_press(toggle);
    let mut parts = vec![
        state_surface(swatch, Color::TRANSPARENT, 4.),
        gap(6., 1.),
        expanded(text_input(
            format!("{} hex", edit.name),
            controller.clone(),
            commit,
        )),
    ];
    if optional {
        let none_edit = edit.clone();
        parts.push(gap(4., 1.));
        parts.push(square_button(
            format!("No {}", edit.name),
            "None",
            44.,
            color.is_none(),
            true,
            move || none_edit.send(DebugValue::Optional(None)),
        ));
    }
    let head = row_of(parts);
    if !open {
        return head;
    }
    let mut swatches = Vec::new();
    for hex in PALETTE {
        let (r, g, b, a) = parse_color(hex).expect("palette colours are valid");
        let edit = edit.clone();
        let controller = controller.clone();
        swatches.push(state_surface(
            RawMaterialButton::new(format!("Swatch {hex}"))
                .size(Size::new(22., 22.))
                .content(Align::new(
                    Alignment::CENTER,
                    Container::new()
                        .width(16.)
                        .height(16.)
                        .color(Color::rgba(r, g, b, a))
                        .border(Border::new(1., border()))
                        .radius(3.),
                ))
                .on_press(move || {
                    controller.set_text(format_color(r, g, b, a));
                    edit.send(wrap((r, g, b, a)));
                }),
            Color::TRANSPARENT,
            4.,
        ));
    }
    stretch_column([head, gap(1., 4.), chunked(swatches, 6, false)])
}

fn segmented(edit: &Rc<Edit>, name: &str, options: &[String], current: &str) -> Widget {
    let mut children = Vec::new();
    for (index, option) in options.iter().enumerate() {
        if index > 0 {
            children.push(gap(2., 1.));
        }
        let selected = option == current;
        let edit = edit.clone();
        let value = option.clone();
        children.push(state_surface(
            RawMaterialButton::new(format!("{name} {option}"))
                .size(Size::new(
                    option.chars().count() as f32 * 7. + 20.,
                    CONTROL_HEIGHT,
                ))
                .padding(EdgeInsets::symmetric(9., 4.))
                .label_style(text_style(
                    11.,
                    if selected { primary() } else { text_muted() },
                ))
                .content(Align::new(
                    Alignment::CENTER,
                    ui_text(
                        option.clone(),
                        11.,
                        if selected { primary() } else { text_muted() },
                    ),
                ))
                .on_press(move || edit.send(DebugValue::Enum(value.clone()))),
            if selected {
                control_active()
            } else {
                Color::TRANSPARENT
            },
            4.,
        ));
    }
    Align::new(
        Alignment::CENTER_LEFT,
        DecoratedBox::new(Padding::all(
            2.,
            Row::new(children).main_axis_size(MainAxisSize::Min),
        ))
        .background(background())
        .border(Border::new(1., border()))
        .radius(6.),
    )
    .into()
}

fn menu(env: &RowEnv, edit: &Rc<Edit>, name: &str, options: &[String], current: &str) -> Widget {
    let open = env.editors.is_open(&edit.key);
    let toggle = {
        let editors = env.editors.clone();
        let key = edit.key.clone();
        let tick = env.tick.clone();
        move || {
            editors.toggle_open(&key);
            refresh(&tick);
        }
    };
    let header = state_surface(
        RawMaterialButton::new(format!("Choose {name}"))
            .size(Size::new(0., FIELD_HEIGHT))
            .padding(EdgeInsets::symmetric(9., 4.))
            .content(Padding::new(
                EdgeInsets::symmetric(9., 0.),
                Row::new([
                    Widget::from(Expanded::new(ui_text(
                        current.to_owned(),
                        12.,
                        text_primary(),
                    ))),
                    Icon::new(if open {
                        icons::chevron_up()
                    } else {
                        icons::chevron_down()
                    })
                    .size(10.)
                    .brush(text_muted())
                    .into(),
                ])
                .cross_axis_alignment(CrossAxisAlignment::Center),
            ))
            .on_press(toggle),
        background(),
        4.,
    );
    let header: Widget = DecoratedBox::new(header)
        .border(Border::new(1., if open { primary() } else { border() }))
        .radius(4.)
        .into();
    if !open {
        return header;
    }
    let mut chips = Vec::new();
    for option in options {
        let selected = option == current;
        let edit = edit.clone();
        let value = option.clone();
        let editors = env.editors.clone();
        let tick = env.tick.clone();
        chips.push(square_button(
            format!("{name} {option}"),
            option,
            (option.chars().count() as f32 * 6.4 + 18.).max(40.),
            selected,
            true,
            move || {
                edit.send(DebugValue::Enum(value.clone()));
                *editors.open.borrow_mut() = None;
                refresh(&tick);
            },
        ));
    }
    stretch_column([header, gap(1., 4.), chunked(chips, 3, true)])
}

fn alignment_grid(edit: &Rc<Edit>, name: &str, current: &str) -> Widget {
    let selected = alignment_cell(current);
    let mut rows: Vec<Widget> = Vec::new();
    for row in 0..3 {
        let mut cells = Vec::new();
        for column_index in 0..3 {
            let option = alignment_name(column_index, row);
            let active = selected == Some((column_index, row));
            let edit = edit.clone();
            cells.push(state_surface(
                RawMaterialButton::new(format!("{name} {option}"))
                    .size(Size::new(22., 22.))
                    .content(Align::new(
                        Alignment::CENTER,
                        Container::new()
                            .width(if active { 10. } else { 5. })
                            .height(if active { 10. } else { 5. })
                            .color(if active { primary() } else { text_muted() })
                            .radius(if active { 5. } else { 2.5 }),
                    ))
                    .on_press(move || edit.send(DebugValue::Enum(option.to_owned()))),
                Color::TRANSPARENT,
                3.,
            ));
        }
        rows.push(Row::new(cells).main_axis_size(MainAxisSize::Min).into());
    }
    let grid: Widget = DecoratedBox::new(Padding::all(
        3.,
        Column::new(rows).main_axis_size(MainAxisSize::Min),
    ))
    .background(background())
    .border(Border::new(1., border()))
    .radius(6.)
    .into();
    row_of(vec![
        grid,
        gap(10., 1.),
        mono(current.to_owned(), 12., text_muted()),
    ])
}

fn bool_switch(edit: &Rc<Edit>, name: &str, value: &DebugValue) -> Widget {
    let on = matches!(value, DebugValue::Bool(true));
    let toggle_edit = edit.clone();
    let track = DecoratedBox::new(
        SizedBox::new().width(32.).height(18.).child(Align::new(
            if on {
                Alignment::CENTER_RIGHT
            } else {
                Alignment::CENTER_LEFT
            },
            Padding::all(
                2.,
                Container::new()
                    .width(14.)
                    .height(14.)
                    .color(if on { on_accent() } else { text_muted() })
                    .radius(7.),
            ),
        )),
    )
    .background(if on { primary() } else { border() })
    .radius(9.);
    let button = state_surface(
        RawMaterialButton::new(format!("Toggle {name}"))
            .size(Size::new(0., FIELD_HEIGHT))
            .padding(EdgeInsets::symmetric(2., 2.))
            .content(
                Row::new([
                    Widget::from(track),
                    gap(8., 1.),
                    mono(on.to_string(), 12., text_muted()),
                ])
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Center),
            )
            .on_press(move || toggle_edit.send(DebugValue::Bool(!on))),
        Color::TRANSPARENT,
        4.,
    );
    Align::new(Alignment::CENTER_LEFT, button).into()
}

fn insets_editor(
    env: &RowEnv,
    edit: &Rc<Edit>,
    value: &DebugValue,
    min: Option<f64>,
    max: Option<f64>,
    step: f64,
) -> Widget {
    let current = match value {
        DebugValue::Insets(insets) => *insets,
        _ => [0.; 4],
    };
    let linked = env
        .editors
        .links
        .borrow()
        .get(&edit.key)
        .copied()
        .unwrap_or_else(|| insets_uniform(current));
    let controllers: Vec<TextEditingController> = (0..4)
        .map(|side| {
            env.editors.field(
                &format!("{}#{side}", edit.key),
                &format_float(f64::from(current[side])),
            )
        })
        .collect();
    let controllers = Rc::new(controllers);
    let apply = {
        let edit = edit.clone();
        let controllers = controllers.clone();
        Rc::new(move |side: usize, next: f32| {
            let links = edit.env.editors.links.borrow();
            let linked = links
                .get(&edit.key)
                .copied()
                .unwrap_or_else(|| insets_uniform(current));
            drop(links);
            let mut now = current;
            for (index, controller) in controllers.iter().enumerate() {
                if let Some(parsed) = parse_float(&controller.text()) {
                    now[index] = parsed as f32;
                }
            }
            let result = apply_inset_edit(now, side, next, linked, min, max);
            for (controller, side_value) in controllers.iter().zip(result) {
                controller.set_text(format_float(f64::from(side_value)));
            }
            edit.send(DebugValue::Insets(result));
        })
    };
    let mut fields = Vec::new();
    for (side, caption) in ["L", "T", "R", "B"].into_iter().enumerate() {
        if side > 0 {
            fields.push(gap(4., 1.));
        }
        let submit_apply = apply.clone();
        let controller = controllers[side].clone();
        let stepper = {
            let apply = apply.clone();
            let controller = controller.clone();
            move |direction: f64, fast: bool| {
                let base = parse_float(&controller.text()).unwrap_or(0.);
                apply(
                    side,
                    step_value(base, direction, step, fast, min, max) as f32,
                );
            }
        };
        let field = with_arrow_steps(
            text_input(
                format!("{} {caption}", edit.name),
                controller,
                move |input| {
                    if let Some(parsed) = parse_float(&input) {
                        submit_apply(side, parsed as f32);
                    }
                },
            ),
            stepper,
        );
        fields.push(expanded(row_of(vec![
            Widget::from(
                SizedBox::new()
                    .width(11.)
                    .child(ui_text(caption, 10., text_muted())),
            ),
            expanded(field),
        ])));
    }
    let link_editors = env.editors.clone();
    let link_key = edit.key.clone();
    let tick = env.tick.clone();
    fields.push(gap(6., 1.));
    fields.push(square_button(
        format!("Link all {}", edit.name),
        "Link",
        44.,
        linked,
        true,
        move || {
            link_editors
                .links
                .borrow_mut()
                .insert(link_key.clone(), !linked);
            refresh(&tick);
        },
    ));
    row_of(fields)
}
