//! Callback checkpoints from Flutter 3.50.0 implicit_animations_test.dart.
use incular_config::{Alignment, Constraints, EdgeInsets};
use incular_core::{Offset, Size};
use incular_widgets::{
    AnimatedAlign, AnimatedContainer, AnimatedDefaultTextStyle, AnimatedOpacity, AnimatedPadding,
    BoxDecoration, Center, Directionality, SizedBox, Text, TextDirection, TextStyle, Widget,
    internal::WidgetTree,
};
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_widgets::{
    AnimatedFractionallySizedBox, AnimatedRotation, AnimatedScale, AnimatedSlide, CustomScrollView,
    Sliver, SliverAnimatedOpacity, SliverToBoxAdapter,
};

const RUN: Duration = Duration::from_secs(1);
enum Property {
    Width,
    Padding,
    Align,
    Opacity,
    TextStyle,
    Slide,
    Scale,
    Rotation,
    Fraction,
    SliverOpacity,
}
fn target(property: &Property, next: bool, count: &Rc<Cell<u32>>) -> Widget {
    let calls = count.clone();
    let end = move || calls.set(calls.get() + 1);
    let box_ = || SizedBox::new().width(32.).height(32.);
    let animated = match property {
        Property::Width => {
            let container = AnimatedContainer::new(RUN)
                .width(if next { 10. } else { 20. })
                .child(box_())
                .on_end(end);
            if next {
                Widget::from(container.foreground_decoration(BoxDecoration::new()))
            } else {
                container.into()
            }
        }
        Property::Padding => {
            AnimatedPadding::new(EdgeInsets::all(if next { 8. } else { 16. }), RUN, box_())
                .on_end(end)
                .into()
        }
        Property::Align => AnimatedAlign::new(
            if next {
                Alignment::TOP_LEFT
            } else {
                Alignment::BOTTOM_RIGHT
            },
            RUN,
            box_(),
        )
        .on_end(end)
        .into(),
        Property::Opacity => AnimatedOpacity::new(if next { 1. } else { 0. }, RUN, box_())
            .on_end(end)
            .into(),
        Property::Slide => AnimatedSlide::new(
            if next {
                Offset::new(1., 1.)
            } else {
                Offset::ZERO
            },
            RUN,
            box_(),
        )
        .on_end(end)
        .into(),
        Property::Scale => AnimatedScale::new(if next { 2. } else { 1. }, RUN, box_())
            .on_end(end)
            .into(),
        Property::Rotation => AnimatedRotation::new(if next { 1.5 } else { 0. }, RUN, box_())
            .on_end(end)
            .into(),
        Property::Fraction => {
            let fraction = if next { 0.25 } else { 0.75 };
            AnimatedFractionallySizedBox::new(RUN, box_())
                .width_factor(fraction)
                .height_factor(fraction)
                .on_end(end)
                .into()
        }
        Property::SliverOpacity => {
            let sliver: Box<dyn Sliver> = Box::new(
                SliverAnimatedOpacity::new(
                    if next { 1. } else { 0. },
                    RUN,
                    SliverToBoxAdapter::new(box_()),
                )
                .on_end(end),
            );
            CustomScrollView::new([sliver]).into()
        }
        Property::TextStyle => {
            let mut style = TextStyle::default();
            if next {
                style = style.italic();
            }
            AnimatedDefaultTextStyle::new(style, RUN, Text::new("tap"))
                .on_end(end)
                .into()
        }
    };
    Directionality::new(TextDirection::Ltr, Center::new(animated)).into()
}

fn check(property: Property) {
    let calls = Rc::new(Cell::new(0));
    let mut tree = WidgetTree::new();
    let root = tree.mount(target(&property, false, &calls)).unwrap();
    let anchor = Instant::now();
    let frame = |tree: &mut WidgetTree, ms| {
        tree.update_compositor(anchor + Duration::from_millis(ms))
            .unwrap();
        tree.layout(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
        let _ = tree.paint();
    };
    frame(&mut tree, 0);
    let mut snapshots = vec![calls.get()];
    for (toggle, ms) in [(true, 10), (false, 1020), (true, 2030)] {
        tree.update(root, target(&property, toggle, &calls))
            .unwrap();
        frame(&mut tree, ms);
        frame(&mut tree, ms);
        if ms == 10 {
            snapshots.push(calls.get());
            frame(&mut tree, ms + 1000);
            snapshots.push(calls.get());
        }
        frame(&mut tree, ms + 1001);
        snapshots.push(calls.get());
    }
    assert_eq!(snapshots, [0, 0, 0, 1, 2, 3]);
}

#[test]
fn container_completion_timing() {
    check(Property::Width);
}
#[test]
fn padding_completion_timing() {
    check(Property::Padding);
}
#[test]
fn alignment_completion_timing() {
    check(Property::Align);
}
#[test]
fn opacity_completion_timing() {
    check(Property::Opacity);
}
#[test]
fn default_text_style_completion_timing() {
    check(Property::TextStyle);
}

#[test]
fn slide_completion_timing() {
    check(Property::Slide);
}
#[test]
fn scale_completion_timing() {
    check(Property::Scale);
}
#[test]
fn rotation_completion_timing() {
    check(Property::Rotation);
}
#[test]
fn fractional_box_completion_timing() {
    check(Property::Fraction);
}
#[test]
fn sliver_opacity_completion_timing() {
    check(Property::SliverOpacity);
}
