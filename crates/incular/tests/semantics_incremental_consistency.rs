//! The incremental semantics pass must always describe the same graph as a
//! collection of the whole tree.
//!
//! Two identical applications receive the same pseudo-random sequence of
//! hovers, presses, scrolls, resizes, state changes, structural rebuilds and
//! animations. One keeps its retained semantic graph; the other releases it
//! before every comparison, so its graph is always a fresh full collection.

use std::time::{Duration, Instant};

use incular::prelude::*;
use incular::widgets::internal::TranslationController;
use incular_config::Constraints;
use incular_core::{InputEvent, Offset, PointerPhase, Rect, Size};
use incular_runtime::Runtime;
use incular_semantics::{Role, SemanticActionKind, SemanticNodeId, SemanticState, SemanticsTree};

#[derive(Clone)]
struct Model {
    title: Signal<u32>,
    shown: Signal<bool>,
    index: Signal<usize>,
    reversed: Signal<bool>,
    count: Signal<usize>,
    hidden: Signal<bool>,
    scroll: ScrollController,
    list: ScrollController,
    translate: TranslationController,
}

impl Model {
    fn new() -> Self {
        Self {
            title: Signal::new(0),
            shown: Signal::new(true),
            index: Signal::new(0),
            reversed: Signal::new(false),
            count: Signal::new(3),
            hidden: Signal::new(false),
            scroll: ScrollController::new(),
            list: ScrollController::new(),
            translate: TranslationController::new(),
        }
    }
}

fn button(label: impl Into<String>) -> Widget {
    incular_controls::Button::new(label.into())
        .on_click(|| {})
        .into()
}

fn sized(height: f32, child: impl Into<Widget>) -> Widget {
    ConstrainedBox::new(Constraints::new(0., f32::INFINITY, height, height), child).into()
}

fn content(model: &Model) -> Widget {
    let title = model.title.clone();
    let shown = model.shown.clone();
    let index = model.index.clone();
    let (reversed, count) = (model.reversed.clone(), model.count.clone());
    let hidden = model.hidden.clone();
    let scrolled = Column::new((0..40).map(|row| {
        match row % 3 {
            0 => button(format!("Scrolled {row}")),
            1 => Text::new(format!("Scrolled text {row}")).into(),
            _ => incular_controls::TextField::new(TextEditingController::with_text(format!(
                "field {row}"
            )))
            .size(Size::new(200., 30.))
            .into(),
        }
    }));
    Column::new(vec![
        Builder::new(move |_| Text::new(format!("Title {}", title.get())).into()).into(),
        Row::new(vec![button("One"), button("Two"), button("Three")]).into(),
        // A node that appears, disappears and changes role.
        Builder::new(move |_| {
            if shown.get() {
                button("Shown")
            } else {
                Text::new("Replaced by text").into()
            }
        })
        .into(),
        Builder::new(move |_| {
            Visibility::new(Row::new(vec![button("Maybe"), Text::new("visible").into()]))
                .visible(!hidden.get())
                .into()
        })
        .into(),
        Builder::new(move |_| {
            IndexedStack::new(vec![
                Text::new("First page").into(),
                button("Second page"),
                Widget::from(Column::new(vec![
                    Text::new("Third page").into(),
                    button("Third action"),
                ])),
            ])
            .index(index.get())
            .into()
        })
        .into(),
        // Children that are added, removed and reordered.
        Builder::new(move |_| {
            let mut rows: Vec<Widget> = (0..count.get())
                .map(|row| {
                    if row % 2 == 0 {
                        button(format!("Row {row}"))
                    } else {
                        Text::new(format!("Row text {row}")).into()
                    }
                })
                .collect();
            if reversed.get() {
                rows.reverse();
            }
            Row::new(rows).into()
        })
        .into(),
        Semantics::new(Row::new(vec![
            Widget::from(Text::new("Grouped")),
            Widget::from(Text::new("label")),
        ]))
        .label("Explicit group")
        .button(true)
        .into(),
        Widget::translate(model.translate.clone(), button("Moving")),
        sized(
            140.,
            SingleChildScrollView::new(scrolled).controller(model.scroll.clone()),
        ),
        sized(
            140.,
            ListView::builder(300, |row| {
                incular_controls::Button::new(format!("Lazy {row}")).on_click(|| {})
            })
            .item_extent_builder(|_| 36.)
            .controller(model.list.clone()),
        ),
    ])
    .into()
}

struct Harness {
    runtime: Runtime,
    model: Model,
    width: f32,
    now: Instant,
}

impl Harness {
    fn new(now: Instant) -> Self {
        let model = Model::new();
        let build = model.clone();
        let app = Application::new(move |_| {
            Directionality::new(TextDirection::Ltr, content(&build)).into()
        })
        .expect("application");
        let mut harness = Self {
            runtime: app.into_runtime(),
            model,
            width: 800.,
            now,
        };
        for _ in 0..3 {
            harness.frame();
        }
        harness
    }

    fn frame(&mut self) {
        self.now += Duration::from_millis(16);
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(self.width, 700.)), self.now)
            .expect("frame");
    }

    fn pointer(&mut self, phase: PointerPhase, position: Offset) {
        let _ = self
            .runtime
            .handle_input(InputEvent::Pointer { phase, position });
    }

    fn apply(&mut self, step: Step) {
        match step {
            Step::Hover(position) => self.pointer(PointerPhase::Move, position),
            Step::Click(position) => {
                self.pointer(PointerPhase::Move, position);
                self.pointer(PointerPhase::Down, position);
                self.pointer(PointerPhase::Up, position);
            }
            Step::Wheel(position, delta) => {
                self.pointer(PointerPhase::Move, position);
                let _ = self.runtime.handle_input(InputEvent::Scroll {
                    delta: Offset::new(0., delta),
                });
            }
            Step::ScrollTo(offset) => {
                let _ = self.model.scroll.jump_to(offset);
            }
            Step::ListTo(offset) => {
                let _ = self.model.list.jump_to(offset);
            }
            Step::Resize(width) => self.width = width,
            Step::Title => self.model.title.update(|value| *value += 1),
            Step::ToggleShown => self.model.shown.update(|value| *value = !*value),
            Step::ToggleHidden => self.model.hidden.update(|value| *value = !*value),
            Step::Page(index) => {
                let _ = self.model.index.set(index);
            }
            Step::Reverse => self.model.reversed.update(|value| *value = !*value),
            Step::Count(count) => {
                let _ = self.model.count.set(count);
            }
            Step::Translate(offset) => {
                self.model
                    .translate
                    .animate_to(offset, Duration::from_millis(40), self.now)
            }
            Step::Type => {
                let _ = self.runtime.handle_input(InputEvent::Text("x".into()));
            }
            Step::Idle => {}
        }
        self.frame();
    }

    /// Releases the retained graph, so the next pass collects everything.
    fn collect_from_scratch(&mut self) {
        let tree = self.runtime.tree_mut();
        tree.set_semantics_enabled(false);
        tree.set_semantics_enabled(true);
        tree.update_semantics();
    }

    fn dump(&self) -> Option<Dumped> {
        let tree = self.runtime.tree().semantics();
        tree.root().map(|root| dump(tree, root))
    }
}

#[derive(Clone, Copy, Debug)]
enum Step {
    Hover(Offset),
    Click(Offset),
    Wheel(Offset, f32),
    ScrollTo(f32),
    ListTo(f32),
    Resize(f32),
    Title,
    ToggleShown,
    ToggleHidden,
    Page(usize),
    Reverse,
    Count(usize),
    Translate(Offset),
    Type,
    Idle,
}

#[derive(Debug, PartialEq)]
struct Dumped {
    role: Role,
    label: Option<String>,
    value: Option<String>,
    description: Option<String>,
    bounds: Rect,
    state: SemanticState,
    actions: Vec<SemanticActionKind>,
    children: Vec<Dumped>,
}

fn dump(tree: &SemanticsTree, id: SemanticNodeId) -> Dumped {
    let node = tree.node(id).expect("live node");
    Dumped {
        role: node.role,
        label: node.label.clone(),
        value: node.value.clone(),
        description: node.description.clone(),
        bounds: node.bounds,
        state: node.state.clone(),
        actions: node.actions.clone(),
        children: node
            .children
            .iter()
            .map(|&child| dump(tree, child))
            .collect(),
    }
}

/// Describes the first difference between two graphs.
fn difference(path: &str, incremental: &Dumped, full: &Dumped) -> Option<String> {
    let here = format!(
        "{path}/{:?}({})",
        full.role,
        full.label.as_deref().unwrap_or("")
    );
    macro_rules! field {
        ($name:ident) => {
            if incremental.$name != full.$name {
                return Some(format!(
                    "{here}: {} is {:?}, a full collection gives {:?}",
                    stringify!($name),
                    incremental.$name,
                    full.$name
                ));
            }
        };
    }
    field!(role);
    field!(label);
    field!(value);
    field!(description);
    field!(bounds);
    field!(state);
    field!(actions);
    if incremental.children.len() != full.children.len() {
        return Some(format!(
            "{here}: {} children, a full collection gives {}",
            incremental.children.len(),
            full.children.len()
        ));
    }
    incremental
        .children
        .iter()
        .zip(&full.children)
        .find_map(|(a, b)| difference(&here, a, b))
}

struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }
    fn unit(&mut self) -> f32 {
        self.below(10_000) as f32 / 10_000.
    }
    fn step(&mut self) -> Step {
        let position = Offset::new(self.unit() * 800., self.unit() * 700.);
        match self.below(20) {
            0..=3 => Step::Hover(position),
            4..=5 => Step::Click(position),
            6..=7 => Step::Wheel(position, self.unit() * 120. - 60.),
            8 => Step::ScrollTo(self.unit() * 900.),
            9 => Step::ListTo(self.unit() * 9000.),
            10 => Step::Resize(500. + (self.unit() * 500.).round()),
            11 => Step::Title,
            12 => Step::ToggleShown,
            13 => Step::ToggleHidden,
            14 => Step::Page(self.below(3) as usize),
            15 => Step::Reverse,
            16 => Step::Count(self.below(6) as usize),
            17 => Step::Translate(Offset::new(self.unit() * 200., self.unit() * 40.)),
            18 => Step::Type,
            _ => Step::Idle,
        }
    }
}

fn run(seed: u64, steps: usize) {
    let start = Instant::now();
    let mut incremental = Harness::new(start);
    let mut full = Harness::new(start);
    let mut random = Random(seed);
    let mut history = Vec::new();
    for step in 0..steps {
        let action = random.step();
        history.push(action);
        incremental.apply(action);
        full.apply(action);
        full.collect_from_scratch();
        let (incremental, full) = (incremental.dump(), full.dump());
        let mismatch = match (&incremental, &full) {
            (Some(incremental), Some(full)) => difference("", incremental, full),
            (None, None) => None,
            _ => Some("only one graph has a root".to_owned()),
        };
        if let Some(mismatch) = mismatch {
            let recent = &history[history.len().saturating_sub(6)..];
            panic!("seed {seed}, step {step} ({action:?}): {mismatch}\nrecent steps: {recent:?}");
        }
    }
}

#[test]
fn incremental_semantics_match_a_full_collection_after_every_frame() {
    for seed in [1, 2, 3, 4] {
        run(seed, 250);
    }
}

#[test]
fn the_graph_holds_every_kind_of_node_the_steps_exercise() {
    // Guards the test itself: the comparison is only meaningful while the
    // application really exposes buttons, text, fields and scroll views.
    let harness = Harness::new(Instant::now());
    let tree = harness.runtime.tree().semantics();
    let count = |role: Role| tree.iter().filter(|(_, node)| node.role == role).count();
    assert!(count(Role::Button) > 20, "{} buttons", count(Role::Button));
    assert!(count(Role::Text) > 10);
    assert!(count(Role::TextField) > 5);
    assert_eq!(count(Role::ScrollView), 2);
}
