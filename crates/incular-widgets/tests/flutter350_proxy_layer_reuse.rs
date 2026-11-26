//! A forced repaint must not churn retained proxy layers.

use incular_config::{Clip, Constraints};
use incular_core::{Color, Offset, Rect, Size, Transform as CoreTransform};
use incular_rendering::{
    Brush, GaussianBlur, GradientStops, LayerId, LayerKind, LayerTree, Path, RRect, RadialGradient,
};
use incular_widgets::{
    BackdropFilter, ClipOval, ClipPath, ClipRRect, ClipRect, ClipShape, CustomClipper,
    PhysicalModel, PhysicalShape, RepaintBoundary, ShaderMask, Transform, Widget,
    internal::WidgetTree,
};

#[derive(Clone)]
struct FixedClip(ClipShape);

impl CustomClipper for FixedClip {
    fn get_clip(&self, _size: Size) -> ClipShape {
        self.0.clone()
    }
}

#[derive(Clone, Copy, Debug)]
enum Proxy {
    Rect,
    RRect,
    Oval,
    Path,
    PhysicalModel,
    PhysicalShape,
    ShaderMask,
    BackdropFilter,
    AffineTransform,
}

impl Proxy {
    fn widget(self) -> Widget {
        let child = RepaintBoundary::new(Widget::box_(Size::new(1., 1.), Color::WHITE));
        let empty = Rect::from_origin_size(Offset::ZERO, Size::ZERO);
        match self {
            Self::Rect => ClipRect::new(child)
                .custom_clipper(FixedClip(ClipShape::Rect(empty)))
                .into(),
            Self::RRect => ClipRRect::new(0., child)
                .custom_clipper(FixedClip(ClipShape::RRect(RRect::uniform(empty, 0.))))
                .into(),
            Self::Oval => ClipOval::new(child)
                .custom_clipper(FixedClip(ClipShape::Rect(empty)))
                .into(),
            Self::Path => ClipPath::new(outside_path(), child).into(),
            Self::PhysicalModel => PhysicalModel::new(Color::BLACK, child)
                .clip_behavior(Clip::HardEdge)
                .into(),
            Self::PhysicalShape => PhysicalShape::new(outside_path(), Color::BLACK, child)
                .clip_behavior(Clip::HardEdge)
                .into(),
            Self::ShaderMask => ShaderMask::new(
                |bounds: Rect| {
                    let center = bounds.origin
                        + Offset::new(bounds.size.width * 0.5, bounds.size.height * 0.5);
                    Brush::RadialGradient(RadialGradient::new(
                        center,
                        bounds.size.width.min(bounds.size.height) * 0.5,
                        GradientStops::even(&[Color::BLACK, Color::WHITE]),
                    ))
                },
                child,
            )
            .into(),
            Self::BackdropFilter => BackdropFilter::new(GaussianBlur::new(0., 0.), child).into(),
            Self::AffineTransform => Transform::new(CoreTransform::rotation(0.1), child).into(),
        }
    }

    fn owns(self, layer: &LayerKind) -> bool {
        if let (Self::AffineTransform, LayerKind::Transform { transform, .. }) = (self, layer) {
            return *transform != CoreTransform::IDENTITY;
        }
        matches!(
            (self, layer),
            (Self::Rect, LayerKind::ClipRect { .. })
                | (
                    Self::RRect | Self::PhysicalModel,
                    LayerKind::ClipRRect { .. }
                )
                | (Self::Oval, LayerKind::ClipOval { .. })
                | (Self::Path | Self::PhysicalShape, LayerKind::ClipPath { .. })
                | (Self::ShaderMask, LayerKind::ShaderMask { .. })
                | (Self::BackdropFilter, LayerKind::BackdropFilter { .. })
        )
    }
}

fn outside_path() -> Path {
    let mut path = Path::builder();
    path.move_to(Offset::new(50., 50.))
        .line_to(Offset::new(150., 50.))
        .line_to(Offset::new(150., 150.))
        .line_to(Offset::new(50., 150.))
        .close();
    path.build()
}

fn proxy_layer(tree: &LayerTree, proxy: Proxy) -> LayerId {
    let mut work = vec![tree.root().expect("mounted layer root")];
    let mut found = None;
    while let Some(id) = work.pop() {
        let (kind, children) = tree.layer(id).expect("connected retained layer");
        if proxy.owns(kind) {
            assert!(
                found.replace(id).is_none(),
                "one effect layer for {proxy:?}"
            );
        }
        work.extend(children.iter().copied());
    }
    found.unwrap_or_else(|| panic!("missing retained layer for {proxy:?}"))
}

#[test]
fn clip_and_effect_proxy_layers_survive_forced_repaint() {
    for proxy in [
        Proxy::Rect,
        Proxy::RRect,
        Proxy::Oval,
        Proxy::Path,
        Proxy::PhysicalModel,
        Proxy::PhysicalShape,
        Proxy::ShaderMask,
        Proxy::BackdropFilter,
        Proxy::AffineTransform,
    ] {
        let mut tree = WidgetTree::new();
        let root = tree.mount(proxy.widget()).unwrap();
        tree.layout(Constraints::tight(Size::new(10., 10.)))
            .unwrap();
        let before = tree.try_paint().unwrap();
        let id = proxy_layer(tree.compositor_layers(), proxy);
        let baseline = tree.diagnostics();

        tree.mark_paint(root).unwrap();
        let after = tree.try_paint().unwrap();
        let diagnostics = tree.diagnostics();
        assert!(
            diagnostics.paint_visits > baseline.paint_visits,
            "{proxy:?}"
        );
        assert_eq!(diagnostics.layouts, baseline.layouts, "{proxy:?}");
        assert_eq!(
            proxy_layer(tree.compositor_layers(), proxy),
            id,
            "{proxy:?}"
        );
        assert_eq!(before, after, "{proxy:?}");
    }
}
