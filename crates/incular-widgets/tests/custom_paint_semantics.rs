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
            vec![
                CustomPainterSemantics::new(
                    Rect::from_origin_size(Offset::ZERO, Size::new(size.width / 2., 20.)),
                    Semantics::builder().label(label).build(),
                )
                .key(41_u64),
            ]
        }))
    }
}

#[test]
fn painter_semantic_rebuilds_are_independent_of_repainting_and_preserve_keys() {
    let paints = Rc::new(std::cell::RefCell::new(Vec::new()));
    let regions = Rc::new(std::cell::RefCell::new(Vec::new()));
    let widget = |label, width, repaint, rebuild_semantics| {
        Widget::from(CustomPaint::from_painter(
            Size::new(width, 40.),
            IndependentPolicy {
                label,
                repaint,
                rebuild_semantics,
                paints: paints.clone(),
                regions: regions.clone(),
            },
        ))
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget("old", 100., true, false)).unwrap();
    let frame = |tree: &mut WidgetTree| {
        tree.layout(Constraints::loose(Size::new(600., 400.)))
            .unwrap();
        let _ = tree.paint();
        tree.update_semantics();
    };
    frame(&mut tree);
    let id = labelled(&tree, "old").unwrap().id;
    assert_eq!(*regions.borrow(), [Size::new(100., 40.)]);

    tree.update(root, widget("pending", 100., true, false))
        .unwrap();
    frame(&mut tree);
    assert_eq!(labelled(&tree, "old").unwrap().id, id);
    assert_eq!(paints.borrow().last(), Some(&"pending"));
    assert_eq!(*regions.borrow(), [Size::new(100., 40.)]);

    tree.update(root, widget("pending", 120., true, false))
        .unwrap();
    frame(&mut tree);
    let resized = labelled(&tree, "pending").unwrap();
    assert_eq!(resized.id, id);
    assert_eq!(resized.bounds.size, Size::new(60., 20.));
    assert_eq!(
        *regions.borrow(),
        [Size::new(100., 40.), Size::new(120., 40.)]
    );

    let paint_count = paints.borrow().len();
    tree.update(root, widget("fresh", 120., false, true))
        .unwrap();
    frame(&mut tree);
    assert_eq!(labelled(&tree, "fresh").unwrap().id, id);
    assert_eq!(paints.borrow().len(), paint_count);
    assert_eq!(
        *regions.borrow(),
        [
            Size::new(100., 40.),
            Size::new(120., 40.),
            Size::new(120., 40.)
        ]
    );
}

struct FixedRegion(&'static str);

impl CustomPainter for FixedRegion {
    fn paint(&self, _: Size) -> DisplayList {
        DisplayList::default()
    }

    fn should_repaint(&self, _: &Self) -> bool {
        false
    }

    fn semantics_builder(&self) -> Option<SemanticsBuilder> {
        let label = self.0;
        Some(Rc::new(move |_| {
            vec![CustomPainterSemantics::new(
                Rect::from_origin_size(Offset::new(1., 1.), Size::new(1., 1.)),
                Semantics::builder()
                    .label(label)
                    .build()
                    .text_direction(incular_config::TextDirection::Rtl),
            )]
        }))
    }
}

fn custom_paint_semantic_frame(widget: impl Into<Widget>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(
        incular_widgets::Directionality::new(incular_config::TextDirection::Ltr, widget).into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    tree.update_semantics();
    tree
}

#[test]
fn one_automatic_semantic_boundary_contains_either_painters_region() {
    for foreground in [false, true] {
        let name = if foreground {
            "foreground"
        } else {
            "background"
        };
        let paint = CustomPaint::new(Size::ZERO, DisplayList::default());
        let paint = if foreground {
            paint.foreground_painter(FixedRegion(name))
        } else {
            paint.painter(FixedRegion(name))
        };
        let tree = custom_paint_semantic_frame(paint);
        assert_eq!(tree.semantics().roots().len(), 1);
        let boundary = tree
            .semantics()
            .node(tree.semantics().root().unwrap())
            .unwrap();
        assert_eq!(boundary.label, None);
        assert_eq!(
            boundary.bounds,
            Rect::from_origin_size(Offset::ZERO, Size::new(800., 600.))
        );
        assert_eq!(boundary.children.len(), 1);
        let region = tree.semantics().node(boundary.children[0]).unwrap();
        assert_eq!(region.label.as_deref(), Some(name));
        assert_eq!(
            region.bounds,
            Rect::from_origin_size(Offset::new(1., 1.), Size::new(1., 1.))
        );
        assert_eq!(
            region.state.text_direction,
            Some(incular_config::TextDirection::Rtl)
        );
        assert_eq!(tree.semantics().iter().count(), 2);
    }
}

#[test]
fn automatic_boundary_orders_background_child_foreground() {
    let tree = custom_paint_semantic_frame(
        CustomPaint::new(Size::ZERO, DisplayList::default())
            .painter(FixedRegion("background"))
            .foreground_painter(FixedRegion("foreground"))
            .child(Semantics::new(incular_widgets::Text::new("Hello")).container(true)),
    );
    let boundary = tree
        .semantics()
        .node(tree.semantics().root().unwrap())
        .unwrap();
    assert_eq!(boundary.children.len(), 3);
    let child_labels: Vec<_> = boundary
        .children
        .iter()
        .map(|id| {
            tree.semantics()
                .node(*id)
                .unwrap()
                .label
                .as_deref()
                .unwrap()
        })
        .collect();
    assert_eq!(child_labels, ["background", "Hello", "foreground"]);
    assert_eq!(tree.semantics().iter().count(), 4);
}

#[derive(Clone)]
struct SuppliedRegion(Semantics);

impl CustomPainter for SuppliedRegion {
    fn paint(&self, _: Size) -> DisplayList {
        DisplayList::default()
    }

    fn should_repaint(&self, _: &Self) -> bool {
        true
    }

    fn semantics_builder(&self) -> Option<SemanticsBuilder> {
        let semantics = self.0.clone();
        Some(Rc::new(move |_| {
            vec![
                CustomPainterSemantics::new(
                    Rect::from_origin_size(Offset::new(1., 1.), Size::new(1., 1.)),
                    semantics.clone(),
                )
                .key(1_u64),
            ]
        }))
    }
}

#[test]
fn painter_region_preserves_role_and_validation_properties() {
    use incular_semantics::{SemanticRole, SemanticValidationResult};
    let table = Semantics::builder()
        .label("this is a table")
        .build()
        .role(SemanticRole::Table)
        .text_direction(incular_config::TextDirection::Rtl);
    let tree = custom_paint_semantic_frame(
        CustomPaint::new(Size::ZERO, DisplayList::default())
            .foreground_painter(SuppliedRegion(table)),
    );
    let table = labelled(&tree, "this is a table").unwrap();
    assert_eq!(table.role, SemanticRole::Table);
    assert_eq!(
        table.bounds,
        Rect::from_origin_size(Offset::new(1., 1.), Size::new(1., 1.))
    );
    assert_eq!(
        table.state.text_direction,
        Some(incular_config::TextDirection::Rtl)
    );

    let input = Semantics::builder()
        .label("email address")
        .build()
        .text_field(true)
        .text_direction(incular_config::TextDirection::Ltr)
        .validation_result(SemanticValidationResult::Invalid);
    let tree = custom_paint_semantic_frame(
        CustomPaint::new(Size::ZERO, DisplayList::default())
            .foreground_painter(SuppliedRegion(input)),
    );
    let input = labelled(&tree, "email address").unwrap();
    assert_eq!(
        input
            .state
            .annotation_flags
            .get(incular_semantics::SemanticAnnotationFlag::TextField),
        Some(true)
    );
    assert_eq!(
        input.state.effective_validation_result(),
        SemanticValidationResult::Invalid
    );
    assert_eq!(
        input.bounds,
        Rect::from_origin_size(Offset::new(1., 1.), Size::new(1., 1.))
    );
}

#[test]
fn semantic_painter_regions_can_enable_disable_and_reenable() {
    let painter = SuppliedRegion(
        Semantics::builder()
            .label("label-before")
            .build()
            .value("value-before")
            .hint("hint-before")
            .text_direction(incular_config::TextDirection::Rtl),
    );
    let mut tree = WidgetTree::new();
    tree.set_semantics_enabled(false);
    let painted = CustomPaint::from_painter(Size::new(100., 100.), painter);
    tree.mount(painted.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    assert_eq!(tree.semantics().iter().count(), 0);
    for enabled in [true, false, true] {
        tree.set_semantics_enabled(enabled);
        tree.update_semantics();
        assert_eq!(labelled(&tree, "label-before").is_some(), enabled);
        if enabled {
            let node = labelled(&tree, "label-before").unwrap();
            assert_eq!(node.value.as_deref(), Some("value-before"));
        } else {
            assert!(tree.semantics().root().is_none());
        }
    }
}

#[derive(Clone)]
struct RegionList(&'static [&'static str]);
impl CustomPainter for RegionList {
    fn paint(&self, _: Size) -> DisplayList {
        DisplayList::default()
    }
    fn should_repaint(&self, _: &Self) -> bool {
        true
    }
    fn semantics_builder(&self) -> Option<SemanticsBuilder> {
        let labels = self.0;
        Some(Rc::new(move |_| {
            labels
                .iter()
                .map(|label| {
                    let region = CustomPainterSemantics::new(
                        Rect::from_origin_size(Offset::new(1., 1.), Size::new(1., 1.)),
                        Semantics::builder().label(*label).build(),
                    );
                    if label.ends_with("-k") {
                        region.key(incular_widgets::internal::Key::String((*label).into()))
                    } else {
                        region
                    }
                })
                .collect()
        }))
    }
}

#[test]
fn painter_regions_diff_all_the_upstream_keyed_and_unkeyed_shapes() {
    let cases: [(&[&str], &[&str]); 7] = [
        (&[], &["a"]),
        (&["a"], &[]),
        (&["a"], &["a", "b"]),
        (&["b"], &["a", "b"]),
        (&["a-k", "c-k"], &["a-k", "b-k", "c-k"]),
        (&["a-k", "b-k", "c-k"], &["a-k", "c-k"]),
        (&["a", "b"], &["b", "a"]),
    ];
    for (from, to) in cases
        .into_iter()
        .chain([(&["a-k", "b", "c"][..], &["b", "c", "a-k"][..])])
    {
        let make = |labels| {
            Widget::from(CustomPaint::from_painter(
                Size::new(100., 100.),
                RegionList(labels),
            ))
        };
        let mut tree = WidgetTree::new();
        let root = tree.mount(make(from)).unwrap();
        let frame = |tree: &mut WidgetTree| {
            tree.layout(Constraints::tight(Size::new(100., 100.)))
                .unwrap();
            tree.update_semantics();
        };
        frame(&mut tree);
        assert_eq!(labels(&tree), from);
        let ids: Vec<_> = from
            .iter()
            .filter(|label| label.ends_with("-k"))
            .map(|label| (*label, labelled(&tree, label).unwrap().id))
            .collect();
        tree.update(root, make(to)).unwrap();
        frame(&mut tree);
        assert_eq!(labels(&tree), to);
        for (label, id) in ids {
            if to.contains(&label) {
                assert_eq!(labelled(&tree, label).unwrap().id, id);
            }
        }
    }
}

#[test]
fn painter_semantics_reject_duplicate_keys() {
    let mut tree = WidgetTree::new();
    tree.mount(
        CustomPaint::from_painter(Size::new(100., 100.), RegionList(&["a-k", "a-k"])).into(),
    )
    .unwrap();
    let error = tree
        .layout(Constraints::tight(Size::new(100., 100.)))
        .unwrap_err();
    use incular_widgets::internal::{Key, TreeError};
    assert!(
        matches!(&error, TreeError::InvalidGeneratedChild { source, .. }
            if matches!(source.as_ref(), TreeError::DuplicateKey { key: Key::String(value), .. }
                if value == "a-k")),
        "{error:?}"
    );
}

struct PositionedProperties {
    rect: Rect,
    semantics: Semantics,
}

impl CustomPainter for PositionedProperties {
    fn paint(&self, _: Size) -> DisplayList {
        DisplayList::default()
    }
    fn should_repaint(&self, _: &Self) -> bool {
        true
    }
    fn semantics_builder(&self) -> Option<SemanticsBuilder> {
        let rect = self.rect;
        let props = self.semantics.clone();
        Some(Rc::new(move |_| {
            vec![CustomPainterSemantics::new(rect, props.clone()).key(1_u64)]
        }))
    }
}

#[test]
fn painted_accessibility_properties_update_in_place_with_the_region_key() {
    use incular_config::TextDirection;
    use incular_semantics::{CheckedState, SemanticActionKind as Action, SemanticAnnotationFlag};
    let build = |after: bool| {
        let rect = Rect::from_origin_size(
            if after {
                Offset::new(5., 6.)
            } else {
                Offset::new(1., 2.)
            },
            Size::new(2., 2.),
        );
        let semantics = if after {
            Semantics::default()
                .checked(CheckedState::Checked)
                .selected(true)
                .button(true)
                .label("label-after")
                .value("value-after")
                .increased_value("increase-after")
                .decreased_value("decrease-after")
                .hint("hint-after")
                .text_direction(TextDirection::Ltr)
                .on_scroll_down(|| {})
                .on_long_press(|| {})
                .on_decrease(|| {})
                .on_increase(|| {})
                .on_scroll_left(|| {})
                .on_scroll_right(|| {})
                .on_scroll_up(|| {})
                .on_tap(|| {})
        } else {
            Semantics::default()
                .checked(CheckedState::Unchecked)
                .selected(false)
                .button(false)
                .label("label-before")
                .value("value-before")
                .increased_value("increase-before")
                .decreased_value("decrease-before")
                .hint("hint-before")
                .text_direction(TextDirection::Rtl)
        };
        Widget::from(CustomPaint::from_painter(
            Size::ZERO,
            PositionedProperties { rect, semantics },
        ))
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(false)).unwrap();
    let check = |tree: &mut WidgetTree, after| {
        tree.layout(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
        tree.update_semantics();
        let (label, value, hint, increase, decrease, origin, direction) = if after {
            (
                "label-after",
                "value-after",
                "hint-after",
                "increase-after",
                "decrease-after",
                Offset::new(5., 6.),
                TextDirection::Ltr,
            )
        } else {
            (
                "label-before",
                "value-before",
                "hint-before",
                "increase-before",
                "decrease-before",
                Offset::new(1., 2.),
                TextDirection::Rtl,
            )
        };
        let node = labelled(tree, label).unwrap();
        assert_eq!(
            node.bounds,
            Rect::from_origin_size(origin, Size::new(2., 2.))
        );
        assert_eq!(node.value.as_deref(), Some(value));
        assert_eq!(node.description.as_deref(), Some(hint));
        assert_eq!(node.state.increased_value.as_deref(), Some(increase));
        assert_eq!(node.state.decreased_value.as_deref(), Some(decrease));
        assert_eq!(node.state.text_direction, Some(direction));
        assert_eq!(node.state.selected, after);
        assert_eq!(node.state.declared_selected(), Some(after));
        assert_eq!(
            node.state.checked,
            Some(if after {
                CheckedState::Checked
            } else {
                CheckedState::Unchecked
            })
        );
        assert_eq!(
            node.state
                .annotation_flags
                .get(SemanticAnnotationFlag::Button),
            Some(after)
        );
        let wants = [
            Action::ScrollDown,
            Action::LongPress,
            Action::Decrement,
            Action::Increment,
            Action::ScrollLeft,
            Action::ScrollRight,
            Action::ScrollUp,
            Action::Activate,
        ];
        assert_eq!(node.actions.len(), if after { wants.len() } else { 0 });
        if after {
            assert!(wants.iter().all(|action| node.actions.contains(action)));
        }
        node.id
    };
    let id = check(&mut tree, false);
    tree.update(root, build(true)).unwrap();
    assert_eq!(check(&mut tree, true), id);
}

#[test]
fn painted_region_exports_field_lengths_tags_and_optional_metadata() {
    use incular_config::TextDirection;
    use incular_semantics::{
        CustomSemanticsAction, SemanticActionKind, SemanticAnnotationFlag, SemanticRole,
        SemanticValidationResult, SemanticsInputType, SemanticsTag,
    };
    let tag = SemanticsTag::new("tag");
    let properties = Semantics::default()
        .label("label")
        .value("value")
        .increased_value("increasedValue")
        .decreased_value("decreasedValue")
        .hint("hint")
        .link(true)
        .link_url("http://google.com")
        .heading_level(1)
        .max_value_length(3)
        .current_value_length(1)
        .identifier("id")
        .tooltip("tooltip")
        .on_long_press_hint("long")
        .on_tap_hint("tap")
        .text_direction(TextDirection::Rtl)
        .tag_for_children(tag.clone())
        .role(SemanticRole::AlertDialog)
        .controls_nodes(["abc"])
        .input_type(SemanticsInputType::Phone)
        .validation_result(SemanticValidationResult::Invalid);
    let tree = custom_paint_semantic_frame(CustomPaint::from_painter(
        Size::ZERO,
        PositionedProperties {
            rect: Rect::from_origin_size(Offset::new(1., 2.), Size::new(2., 2.)),
            semantics: properties,
        },
    ));
    let node = labelled(&tree, "label").unwrap();
    assert_eq!(
        node.bounds,
        Rect::from_origin_size(Offset::new(1., 2.), Size::new(2., 2.))
    );
    assert_eq!(node.value.as_deref(), Some("value"));
    assert_eq!(node.description.as_deref(), Some("hint"));
    assert_eq!(
        node.state.increased_value.as_deref(),
        Some("increasedValue")
    );
    assert_eq!(
        node.state.decreased_value.as_deref(),
        Some("decreasedValue")
    );
    assert_eq!(node.state.link_url.as_deref(), Some("http://google.com"));
    assert_eq!(
        node.state
            .annotation_flags
            .get(SemanticAnnotationFlag::Link),
        Some(true)
    );
    assert_eq!(node.state.heading_level, Some(1));
    assert_eq!(node.state.max_value_length, Some(3));
    assert_eq!(node.state.current_value_length, Some(1));
    assert_eq!(node.state.identifier.as_deref(), Some("id"));
    assert_eq!(node.state.tooltip.as_deref(), Some("tooltip"));
    assert_eq!(node.state.text_direction, Some(TextDirection::Rtl));
    assert_eq!(node.tags(), &[tag]);
    assert_eq!(node.role, SemanticRole::AlertDialog);
    assert_eq!(node.state.controls_nodes.as_deref().unwrap(), &["abc"]);
    assert_eq!(node.state.input_type, SemanticsInputType::Phone);
    assert_eq!(
        node.state.effective_validation_result(),
        SemanticValidationResult::Invalid
    );
    assert!(node.actions.is_empty(), "hints don't create actions");
    let hints: Vec<_> = node
        .state
        .custom_actions
        .iter()
        .filter_map(|id| CustomSemanticsAction::get_action(*id))
        .collect();
    assert!(
        hints
            .iter()
            .any(|hint| hint.action() == Some(SemanticActionKind::Activate)
                && hint.hint() == Some("tap"))
    );
    assert!(
        hints
            .iter()
            .any(|hint| hint.action() == Some(SemanticActionKind::LongPress)
                && hint.hint() == Some("long"))
    );
}

#[test]
fn painted_regions_preserve_the_full_raw_flutter_flag_sets() {
    use incular_semantics::{CheckedState, SemanticAnnotationFlag as Flag};
    let build = |mixed| {
        let semantics = Semantics::default()
            .explicit_child_nodes(true)
            .enabled(true)
            .checked(if mixed {
                CheckedState::Indeterminate
            } else {
                CheckedState::Checked
            })
            .selected(true)
            .hidden(true)
            .button(true)
            .slider(true)
            .keyboard_key(true)
            .link(true)
            .text_field(true)
            .read_only(true)
            .focused(true)
            .focusable(true)
            .in_mutually_exclusive_group(true)
            .header(true)
            .obscured(true)
            .multiline(true)
            .scopes_route(true)
            .names_route(true)
            .image(true)
            .live_region(true)
            .toggled(true)
            .expanded(true)
            .required(true);
        Widget::from(CustomPaint::from_painter(
            Size::ZERO,
            PositionedProperties {
                rect: Rect::from_origin_size(Offset::new(1., 2.), Size::new(2., 2.)),
                semantics,
            },
        ))
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(false)).unwrap();
    let mut original = None;
    for mixed in [false, true] {
        if mixed {
            tree.update(root, build(true)).unwrap();
        }
        tree.layout(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
        tree.update_semantics();
        let sem = tree.semantics();
        assert_eq!(sem.iter().count(), 2);
        let region = sem
            .node(sem.node(sem.root().unwrap()).unwrap().children[0])
            .unwrap();
        if let Some(id) = original {
            assert_eq!(region.id, id);
        }
        original = Some(region.id);
        let s = &region.state;
        assert!(s.enabled && s.has_enabled_state);
        assert!(s.hidden && s.selected && s.focused && s.focusable);
        assert_eq!(s.declared_selected(), Some(true));
        assert_eq!(
            s.checked,
            Some(if mixed {
                CheckedState::Indeterminate
            } else {
                CheckedState::Checked
            })
        );
        assert_eq!(s.toggled, Some(true));
        assert_eq!(s.expanded, Some(true));
        assert!(s.read_only && s.obscured && s.multiline && s.live_region && s.required);
        assert!(!s.has_implicit_scrolling && region.actions.is_empty());
        for flag in [
            Flag::Button,
            Flag::Slider,
            Flag::KeyboardKey,
            Flag::Link,
            Flag::TextField,
            Flag::MutuallyExclusiveGroup,
            Flag::Header,
            Flag::ScopesRoute,
            Flag::NamesRoute,
            Flag::Image,
        ] {
            assert_eq!(s.annotation_flags.get(flag), Some(true), "{flag:?}");
        }
        assert!(tree.focused_keyboard_element().is_none());
    }
}
