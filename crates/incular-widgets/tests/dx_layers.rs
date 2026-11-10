//! The developer-experience layers: chainable modifiers (`styled_widget`),
//! effect chains (`flutter_animate`) and responsive breakpoints
//! (`responsive_framework`).

use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::{Alignment, Constraints, EdgeInsets};
use incular_core::{Color, Size};
use incular_widgets::extensions::{
    AnimateExt, Breakpoint, Condition, DESKTOP, EffectKind, MOBILE, ResponsiveBreakpoints,
    ResponsiveBreakpointsData, ResponsiveScaledBox, ResponsiveValue, ResponsiveVisibility, TABLET,
    WidgetExt,
};
use incular_widgets::internal::{Widget, WidgetTree};
use incular_widgets::{Builder, Directionality, Text, TextDirection};

fn layout(widget: impl Into<Widget>, size: Size) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, widget.into()).into())
        .expect("mounts");
    tree.layout(Constraints::tight(size)).expect("lays out");
    tree
}

#[test]
fn styled_modifiers_compose_like_hand_written_widgets() {
    let tapped = Rc::new(RefCell::new(0));
    let widget = Widget::box_(Size::new(10., 10.), Color::WHITE)
        .padding_all(8.)
        .padding(EdgeInsets::symmetric(2., 0.))
        .background(Color::BLACK)
        .clip_rounded(4.)
        .aligned(Alignment::TOP_LEFT)
        .on_tap({
            let tapped = tapped.clone();
            move || *tapped.borrow_mut() += 1
        })
        .semantics_label("square");
    let _ = layout(widget, Size::new(100., 100.));
    let _ = layout(
        Text::new("Hi")
            .centered()
            .sized(50., 20.)
            .with_opacity(0.5)
            .rotated(0.25),
        Size::new(100., 100.),
    );
}

#[test]
fn effect_chains_time_effects_like_flutter_animate() {
    let chain = Text::new("Hi")
        .animate()
        .fade_in()
        .slide_y(0.5)
        .then()
        .ms(200)
        .scale_in(0.8);
    let effects = chain.effects();
    assert_eq!(effects.len(), 3);
    // The fade and slide run together; the scale starts after them.
    assert_eq!(effects[0].delay, Duration::ZERO);
    assert_eq!(effects[1].delay, Duration::ZERO);
    assert_eq!(effects[2].delay, Duration::from_millis(300));
    assert_eq!(effects[2].duration, Duration::from_millis(200));
    assert_eq!(chain.total_duration(), Duration::from_millis(500));
    assert!(matches!(
        effects[0].kind,
        EffectKind::Fade { begin: 0., end: 1. }
    ));
    // Every frame of the chain builds.
    for millis in [0, 150, 300, 400, 500] {
        let _ = layout(
            chain.frame(Duration::from_millis(millis)),
            Size::new(100., 40.),
        );
    }
    let _ = layout(chain.clone(), Size::new(100., 40.));
    let _ = layout(
        Text::new("Shake").animate().shake().blur_in(4.),
        Size::new(100., 40.),
    );
}

#[test]
fn breakpoints_resolve_conditions() {
    let data = |width| ResponsiveBreakpointsData::for_width(width, Breakpoint::defaults().into());
    assert!(data(320.).is_mobile());
    assert!(data(600.).is_tablet());
    assert!(data(1200.).is_desktop());
    assert!(data(1200.).larger_than(TABLET));
    assert!(data(320.).smaller_than(TABLET));
    assert!(data(600.).between(MOBILE, TABLET));
    let columns = ResponsiveValue::new(1)
        .when(Condition::larger_than(TABLET), 3)
        .when(Condition::equals(TABLET), 2);
    assert_eq!(columns.resolve(&data(320.)), 1);
    assert_eq!(columns.resolve(&data(700.)), 2);
    assert_eq!(columns.resolve(&data(1000.)), 3);
    assert_eq!(
        ResponsiveScaledBox::new(400., Text::new("x")).scale_for(800.),
        2.
    );
}

#[test]
fn responsive_breakpoints_publish_the_measured_width() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let reader = {
        let seen = seen.clone();
        Builder::new(move |context| {
            let data = ResponsiveBreakpoints::of(context).expect("published");
            seen.borrow_mut()
                .push((data.width, data.name().map(str::to_owned)));
            Widget::box_(Size::new(1., 1.), Color::WHITE)
        })
    };
    let _ = layout(
        ResponsiveBreakpoints::with_defaults(reader),
        Size::new(1024., 600.),
    );
    assert_eq!(
        seen.borrow().last(),
        Some(&(1024., Some(DESKTOP.to_owned())))
    );
    let hidden =
        ResponsiveVisibility::new(Text::new("Sidebar")).hidden_when(Condition::equals(MOBILE));
    let _ = layout(
        ResponsiveBreakpoints::with_defaults(hidden),
        Size::new(320., 600.),
    );
    let _ = layout(
        ResponsiveBreakpoints::with_defaults(ResponsiveScaledBox::new(1200., Text::new("Wide"))),
        Size::new(600., 400.),
    );
}
