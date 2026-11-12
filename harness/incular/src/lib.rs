//! Frame-by-frame parity harness: the Incular side.
//!
//! The Flutter side is `harness/flutter/test/frames/recorder.dart`. Both
//! sides run the same scenario (same widgets, same probes, same timeline)
//! against a fake clock and write the same trace format; [`compare`] diffs
//! the two traces frame by frame. See `harness/README.md` for the workflow
//! and the rules for writing a scenario.
//!
//! A scenario mounts a phase-driven scene and advances the clock frame by
//! frame. After every frame each probe (a subtree marked with [`probe`])
//! records its global rect, full 2D world transform, size, accumulated
//! opacity and first solid fill, and every named value the scenario samples
//! ([`FrameHarness::value`]). Input carries the frame clock's time stamp, as
//! Flutter's `TestPointer` events do.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Once;
use std::time::Duration;

use incular_config::Constraints;
use incular_core::time::Instant;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Rect, Size,
};
use incular_rendering::Brush;
use incular_runtime::Runtime;
use incular_text::{FontFamily, TextStyle, register_application_font};
use incular_widgets::internal::{ElementId, Key, RenderKind, WidgetTree};
use incular_widgets::{DefaultTextStyle, Directionality, KeyedSubtree, TextDirection, Widget};
use serde_json::{Value, json};

/// The logical surface every scenario runs in (device pixel ratio 1).
pub const SURFACE: Size = Size {
    width: 800.,
    height: 600.,
};

/// The frame interval scenarios advance by unless they pump explicitly.
pub const INTERVAL: Duration = Duration::from_millis(16);

/// The key prefix that marks a probe.
pub const PROBE_PREFIX: &str = "probe:";

/// The text style at the root of every scene: Inter, loaded on both sides
/// from `harness/flutter/fonts`, so glyph metrics match.
#[must_use]
pub fn root_text_style() -> TextStyle {
    TextStyle::new()
        .family(FontFamily::named("Inter"))
        .size(14.)
        .color(Color::BLACK)
}

/// The directory traces and reports are written under: `$INCULAR_PARITY_DIR`
/// or `harness/out`. Traces go to `<dir>/frames/{flutter,incular}`.
#[must_use]
pub fn output_dir() -> PathBuf {
    std::env::var_os("INCULAR_PARITY_DIR").map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("harness/incular has a parent")
                .join("out")
        },
        PathBuf::from,
    )
}

/// Registers the Inter font once per process.
pub fn load_fonts() {
    static FONTS: Once = Once::new();
    FONTS.call_once(|| {
        let font = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../flutter/fonts/Inter-Regular.ttf"
        ))
        .expect("read harness/flutter/fonts/Inter-Regular.ttf");
        register_application_font(font);
    });
}

thread_local! {
    static PROBES: RefCell<BTreeSet<String>> = const { RefCell::new(BTreeSet::new()) };
}

/// Marks `child` as the probe `name`. Like the Flutter side's
/// `KeyedSubtree(key: ValueKey('probe:<name>'))`, the probe keys the
/// subtree, so reconciliation matches on both sides. It replaces a key the
/// child already had: probe inside, not around, a widget that needs its own
/// key (an `AnimatedSwitcher` child, a list item reordered by key).
pub fn probe(name: &str, child: impl Into<Widget>) -> Widget {
    PROBES.with(|probes| probes.borrow_mut().insert(name.to_owned()));
    KeyedSubtree::new(probe_key(name), child).into()
}

fn probe_key(name: &str) -> Key {
    Key::from(format!("{PROBE_PREFIX}{name}").as_str())
}

/// A scenario: its name (the trace file name, identical on the Flutter
/// side) and the body that mounts and drives it.
pub type Scenario = (&'static str, fn(&mut FrameHarness));

type Build = Rc<dyn Fn(u32) -> Widget>;

/// Drives one scenario and records its frames.
pub struct FrameHarness {
    name: String,
    runtime: Option<Runtime>,
    start: Instant,
    elapsed: Duration,
    phase: Rc<Cell<u32>>,
    revision: Rc<Cell<u64>>,
    values: Vec<(String, Box<dyn Fn() -> f64>)>,
    frames: Vec<Value>,
    touch_down: bool,
    touch_position: Offset,
    mouse_added: bool,
}

const TOUCH_POINTER: u64 = 1;
const TOUCH_DEVICE: u64 = 1;
const MOUSE_POINTER: u64 = 2;
const MOUSE_DEVICE: u64 = 2;

impl FrameHarness {
    fn new(name: &str) -> Self {
        PROBES.with(|probes| probes.borrow_mut().clear());
        Self {
            name: name.to_owned(),
            runtime: None,
            start: Instant::now(),
            elapsed: Duration::ZERO,
            phase: Rc::new(Cell::new(0)),
            revision: Rc::new(Cell::new(0)),
            values: Vec::new(),
            frames: Vec::new(),
            touch_down: false,
            touch_position: Offset::ZERO,
            mouse_added: false,
        }
    }

    /// The scenario name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The current phase; the scene's builder sees it and rebuilds when it
    /// changes (Flutter side: a `ValueListenableBuilder`).
    #[must_use]
    pub fn phase(&self) -> u32 {
        self.phase.get()
    }

    /// The time since mount.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// The runtime, for scenario-specific inspection.
    ///
    /// # Panics
    /// Before [`Self::mount`].
    pub fn runtime(&mut self) -> &mut Runtime {
        self.runtime.as_mut().expect("mount the scene first")
    }

    /// Samples `read` after every frame as the value `name`.
    pub fn value(&mut self, name: &str, read: impl Fn() -> f64 + 'static) {
        self.values.push((name.to_owned(), Box::new(read)));
    }

    /// Mounts the scene at time zero and records frame 0.
    ///
    /// # Panics
    /// If the scene fails to mount or lay out.
    pub fn mount(&mut self, build: impl Fn(u32) -> Widget + 'static) {
        load_fonts();
        let build: Build = Rc::new(build);
        let phase = Rc::clone(&self.phase);
        let scene =
            Widget::stateful_builder(Rc::clone(&self.revision), move |_| build(phase.get()));
        let root = Directionality::new(
            TextDirection::Ltr,
            DefaultTextStyle::new(root_text_style(), scene),
        );
        self.runtime = Some(Runtime::new(root.into()).expect("mount scene"));
        self.start = Instant::now();
        self.elapsed = Duration::ZERO;
        self.frame();
    }

    /// Changes the phase. The scene rebuilds in the next frame.
    pub fn set_phase(&mut self, phase: u32) {
        self.phase.set(phase);
        self.revision.set(self.revision.get().wrapping_add(1));
    }

    /// Advances `count` frames of [`INTERVAL`], recording each.
    pub fn frames(&mut self, count: usize) {
        self.frames_every(count, INTERVAL);
    }

    /// Advances `count` frames of `interval`, recording each.
    pub fn frames_every(&mut self, count: usize, interval: Duration) {
        for _ in 0..count {
            self.pump(interval);
        }
    }

    /// Advances one frame of `duration` and records it.
    pub fn pump(&mut self, duration: Duration) {
        self.elapsed += duration;
        self.frame();
    }

    fn frame(&mut self) {
        let now = self.start + self.elapsed;
        self.runtime()
            .run_frame_at(Constraints::tight(SURFACE), now)
            .expect("frame");
        self.record();
    }

    /// The center of a probe's global rect, for input.
    ///
    /// # Panics
    /// If the probe is not mounted.
    pub fn center(&mut self, probe: &str) -> Offset {
        let tree = self.runtime().tree();
        let element = tree
            .element_with_key(&probe_key(probe))
            .unwrap_or_else(|| panic!("no probe {probe}"));
        let (transform, size) = tree.element_geometry(element).expect("probe geometry");
        let rect = transform.transform_rect_bbox(Rect::from_origin_size(Offset::ZERO, size));
        Offset::new(
            rect.origin.x + rect.size.width / 2.,
            rect.origin.y + rect.size.height / 2.,
        )
    }

    fn input(&mut self, event: InputEvent) {
        let now = self.start + self.elapsed;
        let _ = self.runtime().handle_input_at(event, now);
    }

    fn touch(&mut self, phase: PointerPhase, position: Offset) {
        self.touch_position = position;
        let (buttons, button) = match phase {
            PointerPhase::Down => (PRIMARY_POINTER_BUTTON, Some(PRIMARY_POINTER_BUTTON)),
            PointerPhase::Move => (PRIMARY_POINTER_BUTTON, None),
            PointerPhase::Up => (0, Some(PRIMARY_POINTER_BUTTON)),
            _ => (0, None),
        };
        self.input(InputEvent::PointerWithMetadata {
            pointer: TOUCH_POINTER,
            device: TOUCH_DEVICE,
            kind: PointerDeviceKind::Touch,
            buttons,
            button,
            sample: PointerSampleMetadata::EMPTY,
            phase,
            position,
        });
    }

    /// A touch pointer goes down at `position` (Flutter's default test
    /// pointer kind).
    pub fn down(&mut self, position: Offset) {
        self.touch_down = true;
        self.touch(PointerPhase::Down, position);
    }

    /// The touch pointer moves to `position`.
    pub fn move_to(&mut self, position: Offset) {
        assert!(self.touch_down, "move_to without down");
        self.touch(PointerPhase::Move, position);
    }

    /// The touch pointer lifts where it last was.
    pub fn up(&mut self) {
        assert!(self.touch_down, "up without down");
        self.touch_down = false;
        self.touch(PointerPhase::Up, self.touch_position);
    }

    /// The touch pointer is canceled.
    pub fn cancel(&mut self) {
        assert!(self.touch_down, "cancel without down");
        self.touch_down = false;
        self.touch(PointerPhase::Cancel, self.touch_position);
    }

    /// A mouse pointer hovers at `position`.
    pub fn hover(&mut self, position: Offset) {
        if !self.mouse_added {
            self.mouse_added = true;
            self.mouse(PointerPhase::Enter, position);
        }
        self.mouse(PointerPhase::Move, position);
    }

    fn mouse(&mut self, phase: PointerPhase, position: Offset) {
        self.input(InputEvent::PointerWithMetadata {
            pointer: MOUSE_POINTER,
            device: MOUSE_DEVICE,
            kind: PointerDeviceKind::Mouse,
            buttons: 0,
            button: None,
            sample: PointerSampleMetadata::EMPTY,
            phase,
            position,
        });
    }

    /// A mouse wheel scroll of `delta` logical pixels at `position`
    /// (positive y scrolls content up, as Flutter's `scrollDelta`).
    pub fn wheel(&mut self, position: Offset, delta: Offset) {
        self.hover(position);
        self.input(InputEvent::Scroll { delta });
    }

    /// Sends any other input event, stamped with the frame clock.
    pub fn send(&mut self, event: InputEvent) {
        self.input(event);
    }

    fn record(&mut self) {
        let names: Vec<String> = PROBES.with(|probes| probes.borrow().iter().cloned().collect());
        let tree = self.runtime.as_ref().expect("mounted").tree();
        let mut probes = serde_json::Map::new();
        for name in names {
            let Some(element) = tree.element_with_key(&probe_key(&name)) else {
                continue;
            };
            let Some((transform, size)) = tree.element_geometry(element) else {
                continue;
            };
            let rect = transform.transform_rect_bbox(Rect::from_origin_size(Offset::ZERO, size));
            let [a, b, c, d, e, f] = transform.coefficients();
            probes.insert(
                name,
                json!({
                    "rect": [
                        rect.origin.x,
                        rect.origin.y,
                        rect.origin.x + rect.size.width,
                        rect.origin.y + rect.size.height,
                    ],
                    "transform": [a, b, c, d, e, f],
                    "size": [size.width, size.height],
                    "opacity": opacity(tree, element),
                    "color": fill(tree, element),
                }),
            );
        }
        let values: serde_json::Map<String, Value> = self
            .values
            .iter()
            .map(|(name, read)| (name.clone(), json!(read())))
            .collect();
        self.frames.push(json!({
            "t_us": u64::try_from(self.elapsed.as_micros()).unwrap_or(u64::MAX),
            "probes": probes,
            "values": values,
        }));
    }

    fn write(&self) -> PathBuf {
        let dir = output_dir().join("frames").join("incular");
        std::fs::create_dir_all(&dir).expect("create trace dir");
        let path = dir.join(format!("{}.json", self.name));
        std::fs::write(&path, trace_text(&self.name, &self.frames)).expect("write trace");
        path
    }
}

/// The product of every opacity between the probe and the root.
fn opacity(tree: &WidgetTree, element: ElementId) -> f64 {
    let mut opacity = 1.0_f64;
    let mut last_render = None;
    let mut current = Some(element);
    while let Some(id) = current {
        let render = tree.render_id(id);
        if render != last_render {
            last_render = render;
            match render.and_then(|render| tree.render_object_kind(render)) {
                Some(RenderKind::Opacity { alpha, controller }) => {
                    let alpha = controller.as_ref().map_or(*alpha, |c| c.opacity());
                    opacity *= f64::from(alpha);
                }
                Some(RenderKind::Visibility { visible: false, .. }) => opacity = 0.,
                _ => {}
            }
        }
        current = tree.parent(id);
    }
    opacity
}

/// The first solid fill at or below the probe, as `[a, r, g, b]`.
fn fill(tree: &WidgetTree, element: ElementId) -> Value {
    let mut stack = vec![element];
    while let Some(id) = stack.pop() {
        let color = match tree
            .render_id(id)
            .and_then(|render| tree.render_object_kind(render))
        {
            Some(RenderKind::Box { color, .. }) => Some(*color),
            Some(RenderKind::Decorated {
                background: Some(Brush::Solid(color)),
                ..
            }) => Some(*color),
            _ => None,
        };
        if let Some(color) = color {
            return json!([color.alpha, color.red, color.green, color.blue]);
        }
        if let Some(children) = tree.children(id) {
            stack.extend(children.iter().rev().copied());
        }
    }
    Value::Null
}

/// One frame per line, so traces diff and grep by frame; the same layout
/// the Flutter side writes.
fn trace_text(name: &str, frames: &[Value]) -> String {
    let frames: Vec<String> = frames.iter().map(Value::to_string).collect();
    format!(
        "{{\"scenario\": {}, \"frames\": [\n{}\n]}}\n",
        Value::from(name),
        frames.join(",\n")
    )
}

/// Runs `scenario` and writes its trace; returns the trace path.
pub fn run(scenario: &Scenario) -> PathBuf {
    let (name, body) = scenario;
    let mut harness = FrameHarness::new(name);
    body(&mut harness);
    harness.write()
}

/// Comparison tolerances. Each can be overridden from the environment
/// (`PARITY_TOL_PX`, `PARITY_TOL_TRANSFORM`, `PARITY_TOL_OPACITY`,
/// `PARITY_TOL_COLOR`, `PARITY_TOL_VALUE`).
#[derive(Clone, Copy, Debug)]
pub struct Tolerances {
    /// Logical pixels, for rects, sizes and transform translations.
    pub px: f64,
    /// Transform scale/rotation/skew coefficients.
    pub transform: f64,
    /// Accumulated opacity.
    pub opacity: f64,
    /// Color channel steps (0-255).
    pub color: f64,
    /// Scenario values.
    pub value: f64,
}

impl Default for Tolerances {
    fn default() -> Self {
        Self {
            px: 0.01,
            transform: 1e-4,
            opacity: 2e-3,
            color: 1.,
            value: 1e-3,
        }
    }
}

impl Tolerances {
    /// The defaults with any environment overrides applied.
    #[must_use]
    pub fn from_env() -> Self {
        let read = |name: &str, default: f64| {
            std::env::var(name)
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(default)
        };
        let base = Self::default();
        Self {
            px: read("PARITY_TOL_PX", base.px),
            transform: read("PARITY_TOL_TRANSFORM", base.transform),
            opacity: read("PARITY_TOL_OPACITY", base.opacity),
            color: read("PARITY_TOL_COLOR", base.color),
            value: read("PARITY_TOL_VALUE", base.value),
        }
    }
}

/// The first frame where one probe field or value diverges, and how far
/// it diverges over the whole trace.
#[derive(Clone, Debug)]
pub struct Divergence {
    /// `probe.<name>.<field>` or `value.<name>`.
    pub what: String,
    /// The index of the first diverging frame.
    pub frame: usize,
    /// That frame's time in microseconds.
    pub t_us: u64,
    /// Flutter's value at that frame (JSON).
    pub flutter: Value,
    /// Incular's value at that frame (JSON).
    pub incular: Value,
    /// How many frames diverge.
    pub frames: usize,
    /// The largest difference over all frames (for numeric fields).
    pub max_diff: f64,
}

/// One scenario's comparison.
#[derive(Clone, Debug, Default)]
pub struct Comparison {
    /// The scenario name.
    pub scenario: String,
    /// Problems that make a frame-by-frame comparison meaningless (frame
    /// count or timestamp mismatch, unreadable trace).
    pub structural: Vec<String>,
    /// Per-field divergences, in field order.
    pub divergences: Vec<Divergence>,
    /// Frames compared.
    pub frames: usize,
}

impl Comparison {
    /// Whether the traces match within tolerance.
    #[must_use]
    pub fn matches(&self) -> bool {
        self.structural.is_empty() && self.divergences.is_empty()
    }
}

fn read_trace(path: &Path) -> Result<Vec<Value>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let trace: Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    trace["frames"]
        .as_array()
        .cloned()
        .ok_or_else(|| format!("{}: no frames", path.display()))
}

fn numbers(value: &Value) -> Option<Vec<f64>> {
    match value {
        Value::Number(n) => n.as_f64().map(|n| vec![n]),
        Value::Array(items) => items.iter().map(Value::as_f64).collect(),
        _ => None,
    }
}

/// The largest difference between two field values scaled by its
/// tolerance (a result above 1 diverges), and the raw largest difference.
fn field_diff(field: &str, flutter: &Value, incular: &Value, tol: &Tolerances) -> (f64, f64) {
    let (Some(a), Some(b)) = (numbers(flutter), numbers(incular)) else {
        return if flutter == incular {
            (0., 0.)
        } else {
            (f64::INFINITY, f64::INFINITY)
        };
    };
    if a.len() != b.len() {
        return (f64::INFINITY, f64::INFINITY);
    }
    let mut scaled = 0.0_f64;
    let mut raw = 0.0_f64;
    for (index, (x, y)) in a.iter().zip(&b).enumerate() {
        let diff = (x - y).abs();
        let tolerance = match field {
            "transform" if index < 4 => tol.transform,
            "rect" | "size" | "transform" => tol.px,
            "opacity" => tol.opacity,
            "color" => tol.color,
            _ => tol.value,
        };
        raw = raw.max(diff);
        scaled = scaled.max(diff / tolerance.max(f64::MIN_POSITIVE));
    }
    (scaled, raw)
}

/// Compares the Flutter and Incular traces of `scenario` frame by frame.
#[must_use]
pub fn compare(scenario: &str, flutter: &Path, incular: &Path, tol: &Tolerances) -> Comparison {
    let mut result = Comparison {
        scenario: scenario.to_owned(),
        ..Comparison::default()
    };
    let (theirs, ours) = match (read_trace(flutter), read_trace(incular)) {
        (Ok(theirs), Ok(ours)) => (theirs, ours),
        (theirs, ours) => {
            result.structural.extend(theirs.err());
            result.structural.extend(ours.err());
            return result;
        }
    };
    if theirs.len() != ours.len() {
        result.structural.push(format!(
            "frame count: Flutter {} vs Incular {}",
            theirs.len(),
            ours.len()
        ));
    }
    let frames = theirs.len().min(ours.len());
    result.frames = frames;
    for (index, (a, b)) in theirs.iter().zip(&ours).enumerate() {
        if a["t_us"] != b["t_us"] {
            result.structural.push(format!(
                "frame {index}: time {} vs {}",
                a["t_us"], b["t_us"]
            ));
            break;
        }
    }
    let mut divergences: Vec<Divergence> = Vec::new();
    let mut note =
        |what: String, index: usize, t_us: u64, flutter: &Value, incular: &Value, raw: f64| {
            if let Some(existing) = divergences.iter_mut().find(|d| d.what == what) {
                existing.frames += 1;
                existing.max_diff = existing.max_diff.max(raw);
            } else {
                divergences.push(Divergence {
                    what,
                    frame: index,
                    t_us,
                    flutter: flutter.clone(),
                    incular: incular.clone(),
                    frames: 1,
                    max_diff: raw,
                });
            }
        };
    for index in 0..frames {
        let (a, b) = (&theirs[index], &ours[index]);
        let t_us = a["t_us"].as_u64().unwrap_or_default();
        let empty = serde_json::Map::new();
        let probes_a = a["probes"].as_object().unwrap_or(&empty);
        let probes_b = b["probes"].as_object().unwrap_or(&empty);
        let names: BTreeSet<&String> = probes_a.keys().chain(probes_b.keys()).collect();
        for name in names {
            match (probes_a.get(name), probes_b.get(name)) {
                (Some(pa), Some(pb)) => {
                    for field in ["rect", "transform", "size", "opacity", "color"] {
                        let (scaled, raw) = field_diff(field, &pa[field], &pb[field], tol);
                        if scaled > 1. {
                            note(
                                format!("probe.{name}.{field}"),
                                index,
                                t_us,
                                &pa[field],
                                &pb[field],
                                raw,
                            );
                        }
                    }
                }
                (pa, pb) => note(
                    format!("probe.{name}.present"),
                    index,
                    t_us,
                    &Value::Bool(pa.is_some()),
                    &Value::Bool(pb.is_some()),
                    f64::INFINITY,
                ),
            }
        }
        let values_a = a["values"].as_object().unwrap_or(&empty);
        let values_b = b["values"].as_object().unwrap_or(&empty);
        let names: BTreeSet<&String> = values_a.keys().chain(values_b.keys()).collect();
        for name in names {
            let va = values_a.get(name).unwrap_or(&Value::Null);
            let vb = values_b.get(name).unwrap_or(&Value::Null);
            let (scaled, raw) = field_diff("value", va, vb, tol);
            if scaled > 1. {
                note(format!("value.{name}"), index, t_us, va, vb, raw);
            }
        }
    }
    result.divergences = divergences;
    result
}

/// A Markdown report of `comparisons`.
#[must_use]
pub fn report(comparisons: &[Comparison], tol: &Tolerances) -> String {
    let matched = comparisons.iter().filter(|c| c.matches()).count();
    let mut out = format!(
        "# Frame parity report\n\n{matched}/{} scenarios match.\n\nTolerances: px {}, \
         transform {}, opacity {}, color {}, value {}.\n\n",
        comparisons.len(),
        tol.px,
        tol.transform,
        tol.opacity,
        tol.color,
        tol.value
    );
    for comparison in comparisons {
        let status = if comparison.matches() {
            "match"
        } else {
            "DIVERGES"
        };
        let _ = writeln!(
            out,
            "## {} — {status} ({} frames)\n",
            comparison.scenario, comparison.frames
        );
        for problem in &comparison.structural {
            let _ = writeln!(out, "- structural: {problem}");
        }
        if !comparison.divergences.is_empty() {
            out.push_str(
                "\n| field | first frame | t (ms) | Flutter | Incular | frames | max diff |\n\
                 | --- | --- | --- | --- | --- | --- | --- |\n",
            );
            for d in &comparison.divergences {
                let _ = writeln!(
                    out,
                    "| {} | {} | {:.1} | `{}` | `{}` | {} | {:.4} |",
                    d.what,
                    d.frame,
                    d.t_us as f64 / 1000.,
                    d.flutter,
                    d.incular,
                    d.frames,
                    d.max_diff
                );
            }
        }
        out.push('\n');
    }
    out
}
