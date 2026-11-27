use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_widgets::{
    Center, ClipRect, ColoredBox, IgnorePointer, IntrinsicHeight, IntrinsicWidth, MaxIntrinsicSize,
    Opacity, RepaintBoundary, SizedBox, Widget,
    internal::{Key, WidgetTree},
};

type Wrap = fn(Widget) -> Widget;
const WRAPPERS: [(&str, Wrap); 5] = [
    ("color", |child| ColoredBox::new(Color::WHITE, child).into()),
    ("transparent opacity", |child| {
        Opacity::new(0., child).into()
    }),
    ("clip", |child| ClipRect::new(child).into()),
    ("ignore pointer", |child| IgnorePointer::new(child).into()),
    ("repaint boundary", |child| {
        RepaintBoundary::new(child).into()
    }),
];

fn child() -> Widget {
    MaxIntrinsicSize::new(
        Size::new(100., 200.),
        SizedBox::from_size(Size::new(55., 110.)),
    )
    .into()
}

#[test]
fn visual_and_hit_proxies_do_not_change_the_childs_four_intrinsics() {
    for (name, wrap) in WRAPPERS {
        let mut tree = WidgetTree::new();
        let element = tree.mount(wrap(child())).unwrap();
        let render = tree.render_id(element).unwrap();
        assert_eq!(tree.min_intrinsic_width(render, 50.), Some(55.), "{name}");
        assert_eq!(tree.max_intrinsic_width(render, 50.), Some(100.), "{name}");
        assert_eq!(tree.min_intrinsic_height(render, 50.), Some(110.), "{name}");
        assert_eq!(tree.max_intrinsic_height(render, 50.), Some(200.), "{name}");
    }
}

#[test]
fn visual_and_hit_proxies_report_preferred_size_to_intrinsic_parents() {
    for (name, wrap) in WRAPPERS {
        for horizontal in [true, false] {
            let child = wrap(child()).with_key("proxy");
            let parent = if horizontal {
                Widget::from(IntrinsicWidth::new(child))
            } else {
                Widget::from(IntrinsicHeight::new(child))
            };
            let mut tree = WidgetTree::new();
            tree.mount(Center::new(parent).into()).unwrap();
            tree.layout(Constraints::tight(Size::new(800., 600.)))
                .unwrap();
            let proxy = tree.element_with_key(&Key::from("proxy")).unwrap();
            let actual = tree.element_bounds(proxy).unwrap().size;
            let expected = if horizontal {
                Size::new(100., 110.)
            } else {
                Size::new(55., 200.)
            };
            assert_eq!(actual, expected, "{name} horizontal={horizontal}");
        }
    }
}
