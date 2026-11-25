//! Visibility's inherited TickerMode value and retained child identity.
use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Builder, Directionality, Text, TextDirection, TickerMode, Visibility, Widget, internal::Key,
};
use std::{cell::RefCell, rc::Rc};

fn check_visibility_ticker_mode(maintain_animation: bool) {
    const READER: Key = Key::Value(20);
    let observed = Rc::new(RefCell::new(Vec::new()));
    let child = Widget::from(Builder::new({
        let observed = Rc::clone(&observed);
        move |ctx| {
            let enabled = TickerMode::of(ctx);
            observed.borrow_mut().push(enabled);
            Text::new(if enabled { "enabled" } else { "muted" }).into()
        }
    }))
    .with_key(READER);
    let build = |parent_enabled, visible| {
        Widget::from(Directionality::new(
            TextDirection::Ltr,
            TickerMode::new(
                parent_enabled,
                Visibility::new(child.clone())
                    .visible(visible)
                    .maintain_state(true)
                    .maintain_animation(maintain_animation),
            ),
        ))
    };

    let mut runtime = Runtime::new(build(true, true)).unwrap();
    let frame = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(frame).unwrap();
    let element = runtime.tree().element_with_key(&READER).unwrap();

    let mut latest = vec![*observed.borrow().last().unwrap()];
    for (parent_enabled, visible) in [(true, false), (false, true), (true, true)] {
        let root = runtime.tree().root().unwrap();
        runtime
            .schedule_update(root, build(parent_enabled, visible))
            .unwrap();
        runtime.run_frame(frame).unwrap();
        assert_eq!(runtime.tree().element_with_key(&READER), Some(element));
        latest.push(*observed.borrow().last().unwrap());
    }
    assert_eq!(latest, [true, maintain_animation, false, true]);
    assert_eq!(
        *observed.borrow(),
        [true, false, true],
        "only notify on changes"
    );
}

#[test]
fn hidden_visibility_publishes_the_muted_ticker_mode_to_its_child() {
    check_visibility_ticker_mode(false);
}

#[test]
fn maintain_animation_still_inherits_the_outer_ticker_mode() {
    check_visibility_ticker_mode(true);
}
