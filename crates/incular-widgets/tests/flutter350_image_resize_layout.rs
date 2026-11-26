use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_rendering::{BlendMode, PaintCommand};
use incular_widgets::{Image, MediaQuery, MediaQueryData, Widget, internal::WidgetTree};

const UNIT: &[u8] = include_bytes!(
    "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/resize_source_1x1.png"
);
const SQUARE: &[u8] = include_bytes!(
    "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/resize_source_50x50.png"
);

fn assert_image(
    tree: &mut WidgetTree,
    root: incular_widgets::internal::ElementId,
    layout: (f32, f32),
    decoded: (u32, u32),
) {
    let limits = Constraints::loose(Size::new(250.0, 250.0));
    tree.layout(limits).unwrap();
    assert_eq!(
        tree.element_bounds(root).unwrap().size,
        Size::new(layout.0, layout.1)
    );
    let display = tree.paint();
    let image = display
        .commands()
        .iter()
        .find_map(|command| match command {
            PaintCommand::Image { image, .. } => Some(image),
            _ => None,
        })
        .unwrap();
    assert_eq!((image.decoded().width(), image.decoded().height()), decoded);
}

#[test]
fn cache_resize_never_replaces_the_three_pinned_explicit_display_sizes() {
    for (display, cache) in [(5.0, 20), (50.0, 20), (5.0, 1)] {
        let widget = Image::memory(UNIT)
            .width(display)
            .height(display)
            .cache_width(cache)
            .cache_height(cache)
            .exclude_from_semantics(true)
            .color(Color::rgba(0, 255, 255, 255))
            .color_blend_mode(BlendMode::Plus);
        let mut tree = WidgetTree::new();
        let root = tree.mount(widget.into()).unwrap();
        assert_image(&mut tree, root, (display, display), (1, 1));
    }
}

#[test]
fn logical_cache_size_follows_inherited_dpr_and_reconfiguring_only_cache_replaces_the_stream() {
    let widget = |dpr, logical, w: u32, h: u32| -> Widget {
        let image = Image::memory(SQUARE)
            .width(75.0)
            .height(60.0)
            .cache_width(w)
            .cache_height(h)
            .use_logical_cache_size(logical);
        MediaQuery::new(
            MediaQueryData {
                scale_factor: dpr,
                ..Default::default()
            },
            image,
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget(2.0, false, 12, 15)).unwrap();
    assert_image(&mut tree, root, (75.0, 60.0), (12, 15));
    tree.update(root, widget(2.0, true, 12, 15)).unwrap();
    assert_image(&mut tree, root, (75.0, 60.0), (24, 30));
    tree.update(root, widget(1.25, true, 12, 15)).unwrap();
    assert_image(&mut tree, root, (75.0, 60.0), (15, 19));
    tree.update(root, widget(1.25, true, 10, 12)).unwrap();
    assert_image(&mut tree, root, (75.0, 60.0), (13, 15));
}
