//! `view_transition`: the scene on screen when the update is made keeps
//! painting above the new scene while it fades out (or a circle reveals the
//! new one), then is released. The new scene is live underneath.

use std::time::{Duration, Instant};

use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{DisplayList, FillRule, PaintCommand};
use incular_widgets::internal::{ElementId, WidgetTree};
use incular_widgets::motion::{Reveal, ViewScope, ViewTransitionExt, ms, view_transition};
use incular_widgets::{Column, Widget};

const OLD: Color = Color::rgba(255, 0, 0, 255);
const NEW: Color = Color::rgba(0, 0, 255, 255);

struct Harness {
    tree: WidgetTree,
    root: ElementId,
    now: Instant,
    last: DisplayList,
}

impl Harness {
    fn new(widget: Widget) -> Self {
        let mut tree = WidgetTree::new();
        let root = tree.mount(widget).expect("mount");
        let mut harness = Self {
            tree,
            root,
            now: Instant::now(),
            last: DisplayList::new(),
        };
        harness.frame();
        harness.frame();
        harness
    }

    fn frame(&mut self) {
        self.tree.tick_animations(self.now);
        self.tree
            .layout(Constraints::tight(Size::new(200., 200.)))
            .expect("layout");
        self.tree.update_compositor(self.now).expect("compositor");
        self.last = self.tree.paint();
    }

    fn run(&mut self, duration: Duration) {
        let end = self.now + duration;
        while self.now < end {
            self.now += ms(16);
            self.frame();
        }
    }

    fn update(&mut self, widget: Widget) {
        self.tree.update(self.root, widget).expect("update");
    }

    fn rects(&self, color: Color) -> usize {
        self.last
            .commands()
            .iter()
            .filter(|command| matches!(command, PaintCommand::Rect { color: c, .. } if *c == color))
            .count()
    }

    /// The alpha of the last opacity group (the overlay is painted last).
    fn overlay_alpha(&self) -> Option<f32> {
        self.last
            .commands()
            .iter()
            .rev()
            .find_map(|command| match command {
                PaintCommand::PushOpacity { alpha, .. } => Some(*alpha),
                _ => None,
            })
    }

    fn last_clip(&self) -> Option<Rect> {
        self.last
            .commands()
            .iter()
            .rev()
            .find_map(|command| match command {
                PaintCommand::PushClip { rect } => Some(*rect),
                _ => None,
            })
    }
}

fn page(color: Color) -> Widget {
    Widget::box_(Size::new(200., 200.), color)
}

#[test]
fn the_old_scene_fades_out_over_the_new_one() {
    let mut h = Harness::new(page(OLD));
    assert_eq!(h.rects(OLD), 1);
    let _ = view_transition(|| h.update(page(NEW))).duration(ms(300));
    h.frame();
    assert_eq!(h.rects(NEW), 1, "the new scene is live at once");
    assert_eq!(h.rects(OLD), 1, "the old scene paints above it");
    assert!(h.overlay_alpha().is_some_and(|alpha| alpha > 0.95));
    h.run(ms(150));
    let alpha = h.overlay_alpha().expect("still fading");
    assert!(alpha > 0.05 && alpha < 0.95, "fading: {alpha}");
    h.run(ms(300));
    assert_eq!(h.rects(OLD), 0, "released when done");
    assert_eq!(h.rects(NEW), 1);
}

#[test]
fn a_circle_reveal_clips_the_old_scene_around_a_growing_hole() {
    let mut h = Harness::new(page(OLD));
    let center = Offset::new(50., 50.);
    let _ = view_transition(|| h.update(page(NEW)))
        .reveal(Reveal::circle_from(center))
        .duration(ms(300));
    h.frame();
    h.run(ms(100));
    let hole = h
        .last
        .commands()
        .iter()
        .rev()
        .find_map(|command| match command {
            PaintCommand::PushClipPath { path, fill_rule } => Some((path.clone(), *fill_rule)),
            _ => None,
        })
        .expect("a reveal clip");
    assert!(
        !hole.0.contains(center, hole.1),
        "the new scene shows at the center"
    );
    assert!(
        hole.0.contains(Offset::new(199., 199.), FillRule::NonZero),
        "the far corner still shows the old scene"
    );
    h.run(ms(400));
    assert_eq!(h.rects(OLD), 0);
}

#[test]
fn within_limits_the_transition_to_a_scope() {
    let scope = ViewScope::new();
    let scene = |color: Color| -> Widget {
        Column::new([
            Widget::box_(Size::new(200., 50.), Color::WHITE),
            Widget::box_(Size::new(200., 100.), color).view_transition_scope(&scope),
        ])
        .into()
    };
    let mut h = Harness::new(scene(OLD));
    let _ = view_transition(|| h.update(scene(NEW))).within(&scope);
    h.frame();
    let clip = h.last_clip().expect("scoped clip");
    assert_eq!(
        clip,
        Rect::from_origin_size(Offset::new(0., 50.), Size::new(200., 100.))
    );
}
