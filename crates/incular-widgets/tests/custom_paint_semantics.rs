//! Flutter's `CustomPainter.semanticsBuilder` (`rendering/custom_paint.dart`
//! `RenderCustomPaint.assembleSemanticsNode`, semantics-013): a painter
//! describes regions of its painting at its laid-out size, and each region
//! becomes its own semantic node in painter coordinates, ordered background
//! regions, child, foreground regions.

use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Offset, Rect, Size};
use incular_rendering::DisplayList;
use incular_semantics::SemanticNode;
use incular_widgets::{
    CustomPaint, CustomPainter, CustomPainterSemantics, GestureDetector, Semantics,
    SemanticsBuilder, SizedBox, Widget, internal::WidgetTree,
};

/// A painter whose painting has a left and a right half, each its own
/// region.
struct Halves {
    left: &'static str,
    right: &'static str,
}

impl CustomPainter for Halves {
    fn paint(&self, _: Size) -> DisplayList {
        DisplayList::default()
    }

    fn should_repaint(&self, _: &Self) -> bool {
        false
    }

    fn semantics_builder(&self) -> Option<SemanticsBuilder> {
        let (left, right) = (self.left, self.right);
        Some(Rc::new(move |size: Size| {
            let half = Size::new(size.width / 2., size.height);
            vec![
                CustomPainterSemantics::new(
                    Rect::from_origin_size(Offset::ZERO, half),
                    Semantics::builder().label(left).build(),
                ),
                CustomPainterSemantics::new(
                    Rect::from_origin_size(Offset::new(half.width, 0.), half),
                    Semantics::builder().label(right).build(),
                )
                .key(incular_widgets::internal::Key::String(right.into())),
            ]
        }))
    }
}

/// A painter without semantics.
struct Plain;

impl CustomPainter for Plain {
    fn paint(&self, _: Size) -> DisplayList {
        DisplayList::default()
    }

    fn should_repaint(&self, _: &Self) -> bool {
        false
    }
}

fn mounted(widget: impl Into<Widget>, size: Size) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).expect("mount");
    tree.layout(Constraints::loose(size)).expect("layout");
    tree.update_semantics();
    tree
}

fn labelled<'a>(tree: &'a WidgetTree, label: &str) -> Option<&'a SemanticNode> {
    tree.semantics()
        .iter()
        .map(|(_, node)| node)
        .find(|node| node.label.as_deref() == Some(label))
}

/// Labels of the semantic nodes in tree order.
fn labels(tree: &WidgetTree) -> Vec<String> {
    fn visit(tree: &WidgetTree, node: &SemanticNode, out: &mut Vec<String>) {
        out.extend(node.label.clone());
        for child in &node.children {
            if let Some(child) = tree.semantics().node(*child) {
                visit(tree, child, out);
            }
        }
    }
    let mut out = Vec::new();
    if let Some(node) = tree
        .semantics()
        .root()
        .and_then(|root| tree.semantics().node(root))
    {
        visit(tree, node, &mut out);
    }
    out
}

#[test]
fn each_region_is_a_node_at_its_rect() {
    let tree = mounted(
        CustomPaint::from_painter(
            Size::new(100., 40.),
            Halves {
                left: "west",
                right: "east",
            },
        ),
        Size::new(300., 300.),
    );
    let west = labelled(&tree, "west").expect("left region");
    let east = labelled(&tree, "east").expect("right region");
    assert_eq!(
        west.bounds,
        Rect::from_origin_size(Offset::ZERO, Size::new(50., 40.))
    );
    assert_eq!(
        east.bounds,
        Rect::from_origin_size(Offset::new(50., 0.), Size::new(50., 40.))
    );
}

#[test]
fn regions_follow_the_laid_out_size() {
    let mut tree = mounted(
        CustomPaint::from_painter_filling(Halves {
            left: "west",
            right: "east",
        }),
        Size::new(200., 80.),
    );
    assert_eq!(
        labelled(&tree, "east").expect("right region").bounds,
        Rect::from_origin_size(Offset::new(100., 0.), Size::new(100., 80.))
    );
    tree.layout(Constraints::loose(Size::new(120., 60.)))
        .expect("relayout");
    tree.update_semantics();
    assert_eq!(
        labelled(&tree, "east").expect("right region").bounds,
        Rect::from_origin_size(Offset::new(60., 0.), Size::new(60., 60.))
    );
}

#[test]
fn regions_order_background_child_foreground() {
    let child = Semantics::new(SizedBox::new().width(80.).height(20.)).label("child");
    // One container node gives the regions and the child a single parent.
    let tree = mounted(
        Semantics::new(
            CustomPaint::new(Size::ZERO, DisplayList::default())
                .painter(Halves {
                    left: "back left",
                    right: "back right",
                })
                .foreground_painter(Halves {
                    left: "front left",
                    right: "front right",
                })
                .child(child),
        )
        .label("container"),
        Size::new(300., 300.),
    );
    assert_eq!(
        labels(&tree),
        [
            "container",
            "back left",
            "back right",
            "child",
            "front left",
            "front right"
        ]
    );
}

#[test]
fn a_painter_without_semantics_adds_no_nodes() {
    let tree = mounted(
        CustomPaint::from_painter(Size::new(100., 40.), Plain),
        Size::new(300., 300.),
    );
    assert_eq!(tree.semantics().iter().count(), 0);
}

#[test]
fn regions_do_not_take_pointer_hits_from_the_child() {
    let taps = Rc::new(Cell::new(0));
    // A painted child: the detector defers hit testing to it.
    let child = GestureDetector::new(Widget::box_(
        Size::new(100., 40.),
        incular_core::Color::WHITE,
    ))
    .on_tap({
        let taps = taps.clone();
        move || taps.set(taps.get() + 1)
    });
    let tree = mounted(
        CustomPaint::new(Size::ZERO, DisplayList::default())
            .foreground_painter(Halves {
                left: "west",
                right: "east",
            })
            .child(child),
        Size::new(300., 300.),
    );
    // A press and release over a region reach the child's detector.
    let mut tree = tree;
    let base = std::time::Instant::now();
    let position = Offset::new(20., 20.);
    let hit = tree.pointer_hit_test(position);
    for (millis, phase) in [
        (0, incular_core::PointerPhase::Down),
        (50, incular_core::PointerPhase::Up),
    ] {
        let event = incular_gestures::PointerEvent {
            pointer: 3,
            position,
            phase,
            time: base + std::time::Duration::from_millis(millis),
        };
        let _ = tree.dispatch_pointer_gesture_with_hit_buttons(
            0,
            0,
            incular_gestures::PointerDeviceKind::Mouse,
            incular_core::PRIMARY_POINTER_BUTTON,
            event,
            &hit,
        );
    }
    assert_eq!(taps.get(), 1);
}

#[derive(Clone)]
struct IndependentPolicy {
    label: &'static str,
    repaint: bool,
    rebuild_semantics: bool,
    paints: Rc<std::cell::RefCell<Vec<&'static str>>>,
    regions: Rc<std::cell::RefCell<Vec<Size>>>,
}

impl CustomPainter for IndependentPolicy {
    fn paint(&self, _: Size) -> DisplayList {
        self.paints.borrow_mut().push(self.label);
        DisplayList::default()
    }

    fn should_repaint(&self, _: &Self) -> bool {
        self.repaint
    }

    fn should_rebuild_semantics(&self, _: &Self) -> bool {
        self.rebuild_semantics
    }

    fn semantics_builder(&self) -> Option<SemanticsBuilder> {
        let regions = self.regions.clone();
        let label = self.label;
        Some(Rc::new(move |size| {
            regions.borrow_mut().push(size);
            vec![CustomPainterSemantics::new(
                Rect::from_origin_size(Offset::ZERO, Size::new(size.width / 2., 20.)),
                Semantics::builder().label(label).build(),
            )
            .key(41_u64)]
        }))
    }
}

#[test]
fn painter_semantic_rebuilds_are_independent_of_repainting_and_preserve_keys() {
    let paints = Rc::new(std::cell::RefCell::new(Vec::new()));
    let regions = Rc::new(std::cell::RefCell::new(Vec::new()));
    let widget = |label, width, repaint, rebuild_semantics| {
        Widget::from(
            CustomPaint::from_painter(Size::new(width, 40.), IndependentPolicy {
                label, repaint, rebuild_semantics,
                paints: paints.clone(),
                regions: regions.clone(),
            }),
        )
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget("old", 100., true, false)).unwrap();
    let frame = |tree: &mut WidgetTree| {
        tree.layout(Constraints::loose(Size::new(600., 400.))).unwrap();
        let _ = tree.paint();
        tree.update_semantics();
    };
    frame(&mut tree);
    let id = labelled(&tree, "old").unwrap().id;
    assert_eq!(*regions.borrow(), [Size::new(100., 40.)]);

    tree.update(root, widget("pending", 100., true, false)).unwrap();
    frame(&mut tree);
    assert_eq!(labelled(&tree, "old").unwrap().id, id);
    assert_eq!(paints.borrow().last(), Some(&"pending"));
    assert_eq!(*regions.borrow(), [Size::new(100., 40.)]);

    tree.update(root, widget("pending", 120., true, false)).unwrap();
    frame(&mut tree);
    let resized = labelled(&tree, "pending").unwrap();
    assert_eq!(resized.id, id);
    assert_eq!(resized.bounds.size, Size::new(60., 20.));
    assert_eq!(*regions.borrow(), [Size::new(100., 40.), Size::new(120., 40.)]);

    let paint_count = paints.borrow().len();
    tree.update(root, widget("fresh", 120., false, true)).unwrap();
    frame(&mut tree);
    assert_eq!(labelled(&tree, "fresh").unwrap().id, id);
    assert_eq!(paints.borrow().len(), paint_count);
    assert_eq!(*regions.borrow(), [Size::new(100., 40.), Size::new(120., 40.), Size::new(120., 40.)]);
}
