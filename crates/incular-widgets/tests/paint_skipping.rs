//! Paint enters only the subtrees holding a pending repaint.
//!
//! Each scenario compares the incremental frame with a repaint of every
//! render object in the same tree, so a picture the skipped subtrees left
//! stale would show up as a difference.

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_rendering::DisplayList;
use incular_scroll::ScrollController;
use incular_widgets::internal::{ActionSurface, ElementId, WidgetTree};
use incular_widgets::{Column, Directionality, SingleChildScrollView, TextDirection, Widget};
use std::time::{Duration, Instant};

const BUTTONS: usize = 200;

/// Upper bound on render objects entered to repaint one control: the path
/// from the root down to it, with room for the control's own children.
const PATH_VISITS: u64 = 24;

struct Scene {
    tree: WidgetTree,
    root: ElementId,
    buttons: Vec<ElementId>,
    controller: ScrollController,
    now: Instant,
}

impl Scene {
    fn new() -> Self {
        let controller = ScrollController::new();
        let buttons = (0..BUTTONS)
            .map(|index| {
                Widget::from(
                    ActionSurface::new(format!("Button {index}"))
                        .color(Color::WHITE)
                        .hover_color(Color::BLACK)
                        .size(Size::new(180.0, 32.0)),
                )
            })
            .collect::<Vec<_>>();
        let mut tree = WidgetTree::new();
        let root = tree
            .mount(
                Directionality::new(
                    TextDirection::Ltr,
                    SingleChildScrollView::new(Column::new(buttons)).controller(controller.clone()),
                )
                .into(),
            )
            .expect("mount");
        let mut scene = Self {
            tree,
            root,
            buttons: Vec::new(),
            controller,
            now: Instant::now(),
        };
        scene.frame();
        let scroll = scene.tree.children(root).expect("scroll view")[0];
        let column = scene.tree.children(scroll).expect("column")[0];
        scene.buttons = scene.tree.children(column).expect("buttons").to_vec();
        assert_eq!(scene.buttons.len(), BUTTONS);
        scene
    }

    /// Runs the frame phases in the runtime's order.
    fn frame(&mut self) -> DisplayList {
        self.tree.pump_scrollbar_fades(self.now);
        self.tree
            .layout(Constraints::tight(Size::new(200.0, 300.0)))
            .expect("layout");
        self.tree.update_compositor(self.now).expect("compositor");
        self.tree.paint()
    }

    /// Repaints every render object at the current state.
    fn full_repaint(&mut self) -> DisplayList {
        let mut pending = vec![self.root];
        while let Some(id) = pending.pop() {
            self.tree.mark_paint(id).expect("live element");
            pending.extend_from_slice(self.tree.children(id).unwrap_or_default());
        }
        let before = self.tree.diagnostics().paints;
        let output = self.frame();
        assert!(self.tree.diagnostics().paints - before > BUTTONS as u64);
        output
    }

    /// Runs one frame, checks it painted no more than `max_visits` render
    /// objects' worth of traversal, and that it matches a full repaint.
    fn assert_incremental(&mut self, max_visits: u64) -> (u64, u64) {
        let before = self.tree.diagnostics();
        let output = self.frame();
        let after = self.tree.diagnostics();
        let visits = after.paint_visits - before.paint_visits;
        let paints = after.paints - before.paints;
        assert!(
            visits <= max_visits,
            "paint entered {visits} render objects, expected at most {max_visits}"
        );
        assert_eq!(output, self.full_repaint(), "skipped subtrees went stale");
        self.tree.verify_invariants().expect("tree invariants");
        (visits, paints)
    }

    fn settle(&mut self) {
        for _ in 0..4 {
            self.now += Duration::from_secs(1);
            let _ = self.frame();
        }
    }
}

#[test]
fn clean_frame_enters_no_render_object() {
    let mut scene = Scene::new();
    scene.settle();
    let previous = scene.frame();
    let before = scene.tree.diagnostics();

    let output = scene.frame();

    let after = scene.tree.diagnostics();
    assert_eq!(after.paint_visits, before.paint_visits);
    assert_eq!(after.paints, before.paints);
    assert_eq!(
        after.display_lists_reused - before.display_lists_reused,
        1,
        "the whole tree is reused as one clean subtree"
    );
    assert_eq!(output, previous);
    scene.tree.verify_invariants().expect("tree invariants");
}

#[test]
fn hover_repaints_only_the_hovered_control() {
    let mut scene = Scene::new();
    scene.settle();
    let previous = scene.frame();

    let hovered = scene.buttons[2];
    scene
        .tree
        .set_button_interaction(hovered, Some(true), None, None)
        .expect("hover");
    let (visits, paints) = scene.assert_incremental(PATH_VISITS);
    assert!(paints > 0, "hover must repaint the hovered control");
    assert!(visits > 0);
    assert_ne!(scene.frame(), previous, "hover must change the output");

    scene
        .tree
        .set_button_interaction(hovered, Some(false), None, None)
        .expect("exit");
    let _ = scene.assert_incremental(PATH_VISITS);
    scene.settle();
    assert_eq!(scene.frame(), previous, "exit restores the first frame");
}

#[test]
fn scrolling_reuses_content_pictures() {
    let mut scene = Scene::new();
    scene.settle();

    assert!(scene.controller.jump_to(400.0));
    let (visits, _) = scene.assert_incremental(PATH_VISITS);
    assert!(visits > 0, "the viewport repaints its scrollbar");

    assert!(scene.controller.jump_to(4_000.0));
    let _ = scene.assert_incremental(PATH_VISITS);
}

#[test]
fn scrollbar_fade_animation_repaints_only_the_viewport() {
    let mut scene = Scene::new();
    scene.settle();
    assert!(scene.controller.jump_to(200.0));
    let _ = scene.assert_incremental(PATH_VISITS);

    // Flutter's overlay scrollbar fades out after the scroll ends: every
    // fade step repaints the viewport's scrollbar picture alone.
    let mut repainted_frames = 0;
    for _ in 0..30 {
        scene.now += Duration::from_millis(100);
        let (_, paints) = scene.assert_incremental(PATH_VISITS);
        repainted_frames += usize::from(paints > 0);
    }
    assert!(
        repainted_frames > 1,
        "the fade must animate over several frames"
    );
}
