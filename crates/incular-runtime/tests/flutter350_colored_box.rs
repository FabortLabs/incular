use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{DisplayList, PaintCommand};
use incular_runtime::Runtime;
use incular_widgets::{Center, ColoredBox, CustomPaint, CustomPainter, SizedBox, Widget};

const FILL: Color = Color::rgba(0xab, 0xcd, 0xef, 255);
const CHILD: Color = Color::rgba(0x13, 0x25, 0x37, 255);
const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
const AREA: Rect = Rect {
    origin: Offset::ZERO,
    size: VIEW,
};

fn frame(runtime: &mut Runtime, size: Size) -> DisplayList {
    runtime.run_frame(Constraints::tight(size)).unwrap().0
}

fn rectangles(picture: &DisplayList) -> Vec<(Rect, Color, bool)> {
    picture
        .commands()
        .iter()
        .filter_map(|command| match *command {
            PaintCommand::Rect { rect, color } => Some((rect, color, true)),
            PaintCommand::HardEdgeRect { rect, color } => Some((rect, color, false)),
            _ => None,
        })
        .collect()
}

#[derive(Clone)]
struct TracePaint(Rc<RefCell<Vec<Size>>>);

impl CustomPainter for TracePaint {
    fn paint(&self, size: Size) -> DisplayList {
        self.0.borrow_mut().push(size);
        DisplayList::new()
    }

    fn should_repaint(&self, _old: &Self) -> bool {
        false
    }
}

#[test]
fn zero_size_childless_color_does_not_emit_a_fill_or_a_synthetic_child() {
    let pane = Widget::from(SizedBox::shrink().child(ColoredBox::empty(FILL)));
    let mut runtime = Runtime::new(pane).unwrap();
    assert!(rectangles(&frame(&mut runtime, Size::ZERO)).is_empty());
    let tree = runtime.tree();
    let colored = tree.children(tree.root().unwrap()).unwrap()[0];
    assert!(tree.children(colored).unwrap().is_empty());
    assert_eq!(tree.element_bounds(colored).unwrap().size, Size::ZERO);
}

#[test]
fn zero_size_skips_the_background_but_still_paints_the_child() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let child = CustomPaint::from_painter(Size::ZERO, TracePaint(calls.clone()));
    let colored = ColoredBox::new(FILL, child);
    let mut runtime = Runtime::new(SizedBox::shrink().child(colored).into()).unwrap();
    assert!(rectangles(&frame(&mut runtime, Size::ZERO)).is_empty());
    assert_eq!(calls.borrow().as_slice(), [Size::ZERO]);
}

#[test]
fn positive_childless_color_paints_exactly_one_rectangle() {
    let mut runtime = Runtime::new(ColoredBox::empty(FILL).into()).unwrap();
    assert_eq!(rectangles(&frame(&mut runtime, VIEW)), [(AREA, FILL, true)]);
    let tree = runtime.tree();
    assert!(tree.children(tree.root().unwrap()).unwrap().is_empty());
}

#[test]
fn positive_color_paints_under_its_child_at_the_same_origin() {
    let colored = ColoredBox::new(FILL, Widget::box_(Size::ZERO, CHILD));
    let mut runtime = Runtime::new(colored.into()).unwrap();
    assert_eq!(
        rectangles(&frame(&mut runtime, VIEW)),
        [(AREA, FILL, true), (AREA, CHILD, true)]
    );
}

#[test]
fn color_diagnostics_and_anti_alias_default_match_the_descriptor() {
    let colored = ColoredBox::empty(FILL);
    let debug = format!("{colored:?}");
    assert!(debug.contains(&format!("color: {FILL:?}")));
    assert!(debug.contains("is_anti_alias: true"));
    let built = ColoredBox::builder().color(FILL).build();
    assert_eq!(colored, built);
    let lowered = Widget::from(colored);
    assert_eq!(lowered.debug_type_name(), "ColoredBox");
}

#[test]
fn retained_switch_to_hard_edges_updates_paint_and_keeps_geometry() {
    let mut runtime = Runtime::new(ColoredBox::empty(FILL).into()).unwrap();
    let original = runtime.tree().root().unwrap();
    assert_eq!(rectangles(&frame(&mut runtime, VIEW)), [(AREA, FILL, true)]);
    let layouts = runtime.tree().diagnostics().layouts;
    runtime
        .schedule_update(
            original,
            ColoredBox::empty(FILL).is_anti_alias(false).into(),
        )
        .unwrap();
    assert_eq!(
        rectangles(&frame(&mut runtime, VIEW)),
        [(AREA, FILL, false)]
    );
    assert_eq!(runtime.tree().root(), Some(original));
    assert_eq!(runtime.tree().element_bounds(original).unwrap().size, VIEW);
    assert_eq!(runtime.tree().diagnostics().layouts, layouts);
}

#[test]
fn centered_color_box_does_not_crash_in_a_zero_area_view() {
    use incular_widgets::Placeholder;
    let view = Center::new(ColoredBox::new(FILL, Placeholder::new()));
    let mut runtime = Runtime::new(view.into()).unwrap();
    let _ = frame(&mut runtime, Size::ZERO);
    let tree = runtime.tree();
    let color = tree.children(tree.root().unwrap()).unwrap()[0];
    assert_eq!(tree.element_bounds(color).unwrap().size, Size::ZERO);
}

#[test]
fn transparent_color_still_emits_its_fill_with_no_border() {
    let mut runtime = Runtime::new(ColoredBox::empty(Color::TRANSPARENT).into()).unwrap();
    let list = frame(&mut runtime, VIEW);
    assert_eq!(rectangles(&list), [(AREA, Color::TRANSPARENT, true)]);
    assert!(
        !list
            .commands()
            .iter()
            .any(|cmd| matches!(cmd, PaintCommand::Border { .. }))
    );
}

#[test]
fn an_empty_transparent_color_box_claims_only_its_half_open_bounds() {
    let mut runtime = Runtime::new(ColoredBox::empty(Color::TRANSPARENT).into()).unwrap();
    frame(&mut runtime, VIEW);
    let tree = runtime.tree();
    let render = tree.render_id(tree.root().unwrap()).unwrap();
    for hit in [Offset::ZERO, Offset::new(799.5, 599.5)] {
        assert_eq!(tree.hit_test(hit), Some(render));
    }
    for miss in [
        Offset::new(800., 1.),
        Offset::new(1., 600.),
        Offset::new(-1., 0.),
    ] {
        assert!(tree.hit_test(miss).is_none());
    }
}
