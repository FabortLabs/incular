use incular_config::{Constraints, MainAxisSize, TextDirection};
use incular_core::{Color, Offset, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Baseline, ConstrainedBox, Flex, OverflowBox, OverflowBoxFit, Stack, Text, TextStyle, Widget,
    internal::Key,
};

const OVERFLOW: Key = Key::Value(1);
const LEAF: Key = Key::Value(2);

fn scene(fit: OverflowBoxFit, height: Option<f32>) -> Widget {
    let overflow = match height {
        Some(height) => OverflowBox::new(
            Flex::vertical([Widget::box_(Size::new(100., height), Color::WHITE).with_key(LEAF)])
                .main_axis_size(MainAxisSize::Min)
                .text_direction(TextDirection::Ltr),
        ),
        None => OverflowBox::empty(),
    };
    let overflow = Widget::from(overflow.fit(fit).max_height(1_000_000.)).with_key(OVERFLOW);
    Stack::new([overflow]).into()
}

#[test]
fn defer_to_child_handles_short_long_and_missing_children() {
    let mut runtime = Runtime::new(scene(OverflowBoxFit::DeferToChild, Some(100.))).unwrap();
    let root = runtime.tree().root().unwrap();
    let mut retained = None;
    for (height, size) in [
        (Some(100.), Size::new(100., 100.)),
        (Some(10_000.), Size::new(100., 600.)),
        (None, Size::ZERO),
    ] {
        runtime
            .schedule_update(root, scene(OverflowBoxFit::DeferToChild, height))
            .unwrap();
        runtime
            .run_frame(Constraints::loose(Size::new(800., 600.)))
            .unwrap();
        let tree = runtime.tree();
        let id = tree.element_with_key(&OVERFLOW).unwrap();
        assert_eq!(id, *retained.get_or_insert(id));
        let outer = tree.element_bounds(id).unwrap();
        assert_eq!(outer.size, size);
        assert_eq!(outer.origin.y + outer.size.height, size.height);
        if let Some(height) = height {
            let inner = tree
                .element_bounds(tree.element_with_key(&LEAF).unwrap())
                .unwrap();
            assert_eq!(inner.size, Size::new(100., height));
        } else {
            assert!(tree.children(id).unwrap().is_empty());
            assert!(tree.element_with_key(&LEAF).is_none());
        }
    }
}

#[test]
fn default_max_fit_reserves_the_full_area_and_retained_fit_changes_relayout() {
    let build = |fit: Option<OverflowBoxFit>, has_child: bool| {
        let overflow = if has_child {
            OverflowBox::new(Widget::box_(Size::new(20., 10.), Color::WHITE).with_key(LEAF))
        } else {
            OverflowBox::empty()
        };
        match fit {
            Some(fit) => overflow.fit(fit).into(),
            None => overflow.into(),
        }
    };
    let mut runtime = Runtime::new(build(None, true)).unwrap();
    let root = runtime.tree().root().unwrap();
    for (fit, child, size, offset) in [
        (
            None,
            true,
            Size::new(100., 80.),
            Some(Offset::new(40., 35.)),
        ),
        (
            Some(OverflowBoxFit::DeferToChild),
            true,
            Size::new(20., 10.),
            Some(Offset::ZERO),
        ),
        (Some(OverflowBoxFit::Max), false, Size::new(100., 80.), None),
        (Some(OverflowBoxFit::DeferToChild), false, Size::ZERO, None),
    ] {
        runtime.schedule_update(root, build(fit, child)).unwrap();
        runtime.run_frame(Constraints::at_most(100., 80.)).unwrap();
        let tree = runtime.tree();
        assert_eq!(tree.root(), Some(root));
        assert_eq!(tree.element_bounds(root).unwrap().size, size);
        assert_eq!(
            tree.element_with_key(&LEAF)
                .map(|id| tree.element_bounds(id).unwrap().origin),
            offset
        );
    }
}

#[test]
fn overflow_alignment_moves_or_clears_the_actual_child_baseline() {
    let child = ConstrainedBox::new(
        Constraints::tight(Size::new(20., 100.)),
        Baseline::new(
            40.,
            Text::new("A")
                .text_direction(TextDirection::Ltr)
                .style(TextStyle::default().font_size(4.)),
        ),
    );
    let mut runtime = Runtime::new(OverflowBox::new(child).max_height(150.).into()).unwrap();
    let root = runtime.tree().root().unwrap();
    let viewport = Constraints::at_most(100., 80.);
    runtime.run_frame(viewport).unwrap();
    let tree = runtime.tree();
    let render = tree.render_id(root).unwrap();
    assert_eq!(tree.baseline(render), Some(30.));

    runtime
        .schedule_update(root, OverflowBox::empty().max_height(150.).into())
        .unwrap();
    runtime.run_frame(viewport).unwrap();
    assert_eq!(runtime.tree().baseline(render), None);
}
