//! Pinned Visibility.of and owned Focus behaviors, exercised in the retained runtime.
use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_rendering::{DisplayList, PaintCommand};
use incular_runtime::Runtime;
use incular_widgets::{
    Builder, Center, Directionality, Focus, Offstage, Text, TextDirection, Visibility, Widget,
    internal::Key,
};
use std::{cell::RefCell, rc::Rc};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
const FOCUS: Key = Key::Value(10);
const READER: Key = Key::Value(11);
const REPLACEMENT: Color = Color::rgba(17, 120, 222, 255);

fn scene(child: impl Into<Widget>) -> Widget {
    Directionality::new(TextDirection::Ltr, child).into()
}

fn frame(runtime: &mut Runtime) -> DisplayList {
    runtime.run_frame(Constraints::tight(VIEW)).unwrap().0
}

fn pump(runtime: &mut Runtime, child: impl Into<Widget>) -> DisplayList {
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, scene(child)).unwrap();
    frame(runtime)
}

fn read(log: &Rc<RefCell<Vec<bool>>>) -> Widget {
    let log = Rc::clone(log);
    Widget::from(Builder::new(move |ctx| {
        log.borrow_mut().push(Visibility::of(ctx));
        Widget::box_(Size::new(30., 40.), REPLACEMENT)
    }))
    .with_key(READER)
}

#[test]
fn visibility_of_defaults_visible_and_reports_hidden_replacement_without_hiding_paint() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let child = read(&log);
    let mut runtime = Runtime::new(scene(child.clone())).unwrap();
    frame(&mut runtime);
    assert_eq!(*log.borrow(), [true]);
    pump(
        &mut runtime,
        Visibility::new(child.clone()).maintain_state(true),
    );
    assert_eq!(*log.borrow(), [true, true]);
    pump(
        &mut runtime,
        Visibility::new(child.clone())
            .maintain_state(true)
            .visible(false),
    );
    assert_eq!(*log.borrow(), [true, true, false]);
    log.borrow_mut().clear();
    let mut replacement = Runtime::new(scene(
        Visibility::new(Text::new("original"))
            .visible(false)
            .replacement(child),
    ))
    .unwrap();
    let display = frame(&mut replacement);
    assert_eq!(*log.borrow(), [false]);
    assert!(display.commands().iter().any(|command| matches!(
        command, PaintCommand::Rect { color, .. } if *color == REPLACEMENT
    )));
}

#[test]
fn visibility_of_matches_nested_dependency_changes_with_the_same_retained_reader() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let reader = read(&log);
    let build = |outer, inner| {
        Visibility::new(Center::new(
            Visibility::new(reader.clone())
                .maintain_state(true)
                .visible(inner),
        ))
        .maintain_state(true)
        .visible(outer)
    };

    let mut runtime = Runtime::new(scene(build(true, true))).unwrap();
    frame(&mut runtime);
    let element = runtime.tree().element_with_key(&READER).unwrap();
    for (outer, inner) in [(true, false), (true, false), (false, false), (false, true)] {
        pump(&mut runtime, build(outer, inner));
        assert_eq!(runtime.tree().element_with_key(&READER), Some(element));
    }
    assert_eq!(*log.borrow(), [true, false, false, false]);

    log.borrow_mut().clear();
    let mut runtime = Runtime::new(scene(build(true, false))).unwrap();
    frame(&mut runtime);
    for (outer, inner) in [
        (false, false),
        (true, true),
        (true, false),
        (false, false),
        (false, false),
    ] {
        pump(&mut runtime, build(outer, inner));
    }
    assert_eq!(*log.borrow(), [false, true, false, false]);
}

#[test]
fn offstage_does_not_supply_a_visibility_scope() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = Runtime::new(scene(Offstage::new(read(&log)))).unwrap();
    frame(&mut runtime);
    assert_eq!(*log.borrow(), [true]);
}

fn visibility_focus(preserve: bool, preserve_focus: bool) {
    let child = Widget::from(Focus::new(Text::new("child"))).with_key(FOCUS);
    let build = |visible| {
        let wrapper = if preserve && preserve_focus {
            Visibility::maintain(child.clone())
        } else if preserve {
            Visibility::new(child.clone())
                .maintain_state(true)
                .maintain_animation(true)
                .maintain_size(true)
                .maintain_semantics(true)
                .maintain_interactivity(true)
        } else {
            Visibility::new(child.clone())
        };
        wrapper.visible(visible)
    };
    let mut runtime = Runtime::new(scene(build(true))).unwrap();
    frame(&mut runtime);
    let element = runtime.tree().element_with_key(&FOCUS).unwrap();
    let node = runtime
        .tree()
        .focus_node_at(element)
        .expect("Focus owns a node");
    node.request_focus();
    frame(&mut runtime);
    assert!(node.has_focus());
    pump(&mut runtime, build(false));
    if !preserve_focus {
        node.request_focus();
        assert!(!node.has_focus(), "a request cannot revive hidden focus");
        frame(&mut runtime);
    }
    assert_eq!(node.has_focus(), preserve_focus);
    assert_eq!(runtime.focused_element() == Some(element), preserve_focus);
    assert_eq!(
        runtime.tree().element_with_key(&FOCUS) == Some(element),
        preserve
    );
}

#[test]
fn replacement_clears_and_cannot_revive_an_owned_focus_node() {
    visibility_focus(false, false);
}

#[test]
fn maintaining_everything_except_focus_clears_an_owned_focus_node() {
    visibility_focus(true, false);
}

#[test]
fn visibility_maintain_keeps_an_owned_focus_node() {
    visibility_focus(true, true);
}

fn assert_no_opacity_layer(mut runtime: Runtime) {
    let display = frame(&mut runtime);
    assert_eq!(
        display
            .commands()
            .iter()
            .filter(|paint| matches!(paint, PaintCommand::GlyphRun { .. }))
            .count(),
        1,
        "visible text must paint once",
    );
    assert!(
        !display
            .commands()
            .iter()
            .any(|paint| matches!(paint, PaintCommand::PushOpacity { .. }))
    );
}

#[test]
fn maintaining_size_does_not_composite_visible_box_text() {
    let host = Visibility::new(Text::new("hello").text_direction(TextDirection::Ltr))
        .maintain_state(true)
        .maintain_animation(true)
        .maintain_size(true);
    assert_no_opacity_layer(Runtime::new(host.into()).unwrap());
}

#[test]
fn maintaining_size_does_not_composite_visible_sliver_text() {
    use incular_widgets::{
        CustomScrollView, SliverChildListDelegate, SliverList, SliverVisibility,
    };
    let child = SliverList::delegate(
        SliverChildListDelegate::new([Text::new("hello").into()]).add_repaint_boundaries(false),
    );
    let sliver = SliverVisibility::new(true, child)
        .maintain_state(true)
        .maintain_animation(true)
        .maintain_size(true);
    assert_no_opacity_layer(
        Runtime::new(scene(CustomScrollView::new([
            Box::new(sliver) as Box<dyn incular_widgets::Sliver>
        ])))
        .unwrap(),
    );
}
