use incular_config::Clip;
use incular_core::{Color, Offset, Rect, Size, Transform as CoreTransform};
use incular_rendering::{
    Brush, CornerRadii, DisplayList, DropShadowEffect, FillRule, FollowerLayer, GaussianBlur,
    LayerId, LayerTree, Path, RRect,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

use crate::layout::ClipDelegate;

use crate::layout::ClipShape;
use crate::painting_effects::RasterPaintSources;
use crate::tree::{RenderKind, WidgetKind};

enum BoxPaintClip<'a> {
    LayoutBounds,
    Custom(&'a ClipDelegate),
}

/// Whether a widget clips raster output through a compositor clip layer.
/// `Clip::None` disables clipping; every other behavior clips.
fn widget_clips(widget: &WidgetKind) -> bool {
    match widget {
        WidgetKind::ClipOval { clip_behavior, .. } => *clip_behavior != Clip::None,
        WidgetKind::ClipRect {
            clip_behavior,
            child,
            ..
        }
        | WidgetKind::ClipRRect {
            clip_behavior,
            child,
            ..
        }
        | WidgetKind::ClipPath {
            clip_behavior,
            child,
            ..
        }
        | WidgetKind::FittedBox {
            clip_behavior,
            child,
            ..
        }
        | WidgetKind::Unconstrained {
            clip_behavior,
            child,
            ..
        } => child.is_some() && *clip_behavior != Clip::None,
        WidgetKind::Flex { clip_behavior, .. }
        | WidgetKind::Wrap { clip_behavior, .. }
        | WidgetKind::Stack { clip_behavior, .. }
        | WidgetKind::IndexedStack { clip_behavior, .. }
        | WidgetKind::Scroll { clip_behavior, .. } => *clip_behavior != Clip::None,
        WidgetKind::SliverViewport { config } => config.clip_behavior != Clip::None,
        WidgetKind::CustomLayout { delegate, .. } => delegate
            .overflow_clip()
            .is_some_and(|clip| clip != Clip::None),
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AttachmentSpec {
    Direct,
    Transform,
    Clip,
    Opacity,
    Blur,
    DropShadow,
    ColorFilter,
    Blend,
    RasterEffects(u8),
    ShaderMask,
    BackdropFilter,
    Annotation,
    Leader,
    Follower,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LayerSpec {
    picture: bool,
    foreground_picture: bool,
    attachment: AttachmentSpec,
}

impl LayerSpec {
    fn for_widget(widget: &WidgetKind) -> Self {
        let foreground_picture = matches!(
            widget,
            WidgetKind::Button(spec) if spec.color.alpha == 0 && spec.focused_color.is_some()
        ) || matches!(widget, WidgetKind::Table { border: Some(border), .. } if border.has_paint());
        let attachment = match widget {
            WidgetKind::Scroll { .. } | WidgetKind::SliverViewport { .. } => {
                if widget_clips(widget) {
                    AttachmentSpec::Clip
                } else {
                    AttachmentSpec::Transform
                }
            }
            WidgetKind::ClipRect { .. }
            | WidgetKind::ClipRRect { .. }
            | WidgetKind::ClipOval { .. }
            | WidgetKind::ClipPath { .. }
            | WidgetKind::FittedBox { .. }
            | WidgetKind::Unconstrained { .. }
            | WidgetKind::Flex { .. }
            | WidgetKind::Wrap { .. }
            | WidgetKind::Stack { .. }
            | WidgetKind::IndexedStack { .. }
            | WidgetKind::CustomLayout { .. }
                if widget_clips(widget) =>
            {
                AttachmentSpec::Clip
            }
            WidgetKind::PersistentHeader { .. }
            | WidgetKind::Translate { .. }
            | WidgetKind::Transform { .. }
            | WidgetKind::Scale { .. }
            | WidgetKind::Affine { .. }
            | WidgetKind::Rotation { .. }
            | WidgetKind::FittedBox { .. } => AttachmentSpec::Transform,
            WidgetKind::Opacity { .. } => AttachmentSpec::Opacity,
            WidgetKind::Blur { .. } => AttachmentSpec::Blur,
            WidgetKind::DropShadow { .. } | WidgetKind::Banner { .. } => AttachmentSpec::DropShadow,
            WidgetKind::Text { style, .. }
                if crate::utilities::layered_text_shadow(&style.shadows).is_some() =>
            {
                AttachmentSpec::DropShadow
            }
            WidgetKind::ColorFiltered { .. } => AttachmentSpec::ColorFilter,
            WidgetKind::Blend { .. } => AttachmentSpec::Blend,
            WidgetKind::Image {
                image: Some(_),
                blend_mode,
                source_effects,
                ..
            } => raster_attachment_spec(source_effects.raster_paint_sources(*blend_mode)),
            WidgetKind::ShaderMask { child: Some(_), .. } => AttachmentSpec::ShaderMask,
            WidgetKind::BackdropFilter { .. } => AttachmentSpec::BackdropFilter,
            WidgetKind::AnnotatedRegion { .. } => AttachmentSpec::Annotation,
            WidgetKind::CompositedTransformTarget { .. } => AttachmentSpec::Leader,
            WidgetKind::CompositedTransformFollower { .. } => AttachmentSpec::Follower,
            _ => AttachmentSpec::Direct,
        };
        Self {
            picture: needs_picture(widget),
            foreground_picture,
            attachment,
        }
    }
}

fn raster_attachment_spec(source: RasterPaintSources) -> AttachmentSpec {
    let stages = raster_stage_bits(
        source.filter.is_some(),
        source.opacity.is_some(),
        source.blend.is_some(),
    );
    if stages == 0 {
        AttachmentSpec::Direct
    } else {
        AttachmentSpec::RasterEffects(stages)
    }
}

fn raster_stage_bits(filter: bool, opacity: bool, blend: bool) -> u8 {
    u8::from(filter) | (u8::from(opacity) << 1) | (u8::from(blend) << 2)
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RasterEffectLayers {
    filter: Option<LayerId>,
    opacity: Option<LayerId>,
    blend: Option<LayerId>,
}

impl RasterEffectLayers {
    fn ids(&self) -> [Option<LayerId>; 3] {
        [self.filter, self.opacity, self.blend]
    }

    fn flags(&self) -> u8 {
        raster_stage_bits(
            self.filter.is_some(),
            self.opacity.is_some(),
            self.blend.is_some(),
        )
    }

    fn attach(&self, compositor: &mut LayerTree, root: LayerId, picture: Option<LayerId>) {
        let mut child = picture;
        for stage in self.ids().into_iter().flatten() {
            compositor.set_children(stage, child.into_iter().collect());
            child = Some(stage);
        }
        compositor.set_children(root, child.into_iter().collect());
    }

    fn reconcile(&mut self, compositor: &mut LayerTree, source: RasterPaintSources) {
        fn stage<T>(
            id: &mut Option<LayerId>,
            desired: Option<T>,
            compositor: &mut LayerTree,
            create: impl FnOnce(&mut LayerTree, T) -> LayerId,
        ) {
            match (*id, desired) {
                (None, Some(value)) => *id = Some(create(compositor, value)),
                (Some(old), None) => {
                    compositor.remove(old);
                    *id = None;
                }
                _ => {}
            }
        }
        stage(
            &mut self.filter,
            source.filter,
            compositor,
            LayerTree::create_color_filter,
        );
        stage(
            &mut self.opacity,
            source.opacity,
            compositor,
            LayerTree::create_opacity,
        );
        stage(
            &mut self.blend,
            source.blend,
            compositor,
            LayerTree::create_blend,
        );
    }

    fn update(&self, compositor: &mut LayerTree, source: RasterPaintSources) {
        if let (Some(layer), Some(filter)) = (self.filter, source.filter) {
            compositor.update_color_filter(layer, filter);
        }
        if let (Some(layer), Some(opacity)) = (self.opacity, source.opacity) {
            compositor.update_opacity(layer, opacity);
        }
        if let (Some(layer), Some(blend)) = (self.blend, source.blend) {
            compositor.update_blend(layer, blend);
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum LayerAttachment {
    Direct,
    Transform {
        layer: LayerId,
    },
    Clip {
        clip: LayerId,
        content: LayerId,
        overflow_active: Cell<Option<bool>>,
    },
    Opacity {
        layer: LayerId,
    },
    Blur {
        layer: LayerId,
    },
    DropShadow {
        layer: LayerId,
    },
    ColorFilter {
        layer: LayerId,
    },
    Blend {
        layer: LayerId,
    },
    RasterEffects(Rc<RasterEffectLayers>),
    ShaderMask {
        layer: LayerId,
    },
    BackdropFilter {
        layer: LayerId,
    },
    Annotation {
        layer: LayerId,
    },
    Leader {
        layer: LayerId,
    },
    Follower {
        layer: LayerId,
    },
}

impl LayerAttachment {
    fn primary_layer(&self) -> Option<LayerId> {
        match self {
            Self::Direct => None,
            Self::RasterEffects(layers) => layers.blend.or(layers.opacity).or(layers.filter),
            Self::Transform { layer }
            | Self::Opacity { layer }
            | Self::Blur { layer }
            | Self::DropShadow { layer }
            | Self::ColorFilter { layer }
            | Self::Blend { layer }
            | Self::ShaderMask { layer }
            | Self::BackdropFilter { layer }
            | Self::Annotation { layer }
            | Self::Leader { layer }
            | Self::Follower { layer } => Some(*layer),
            Self::Clip { clip, .. } => Some(*clip),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RenderLayers {
    pub(crate) root: LayerId,
    pub(crate) picture: Option<LayerId>,
    pub(crate) foreground_picture: Option<LayerId>,
    #[cfg(debug_assertions)]
    debug_foreground_picture: Option<LayerId>,
    pub(crate) attachment: LayerAttachment,
    /// Retained even through Clip::None so clipping-only updates do not
    /// need to remeasure the positioned children.
    layout_overflow: Cell<bool>,
    /// Shared by painted clipping and hit testing even if the paint stage
    /// is disabled. Only clip objects with resolved geometry allocate it.
    clip_geometry: Option<Rc<RefCell<Option<CachedClip>>>>,
}

impl RenderLayers {
    pub(crate) fn create(compositor: &mut LayerTree, widget: &WidgetKind) -> Self {
        let spec = LayerSpec::for_widget(widget);
        let caches_clip_geometry = matches!(
            widget,
            WidgetKind::ClipRect { .. }
                | WidgetKind::ClipRRect { .. }
                | WidgetKind::ClipOval { .. }
                | WidgetKind::ClipPath { .. }
                | WidgetKind::SliverViewport { .. }
        );
        let root = compositor.create_transform(CoreTransform::translation(Offset::ZERO));
        let picture = spec.picture.then(|| {
            compositor.create_picture(
                DisplayList::new(),
                Rect::from_origin_size(Offset::ZERO, Size::ZERO),
            )
        });
        let foreground_picture = spec.foreground_picture.then(|| {
            compositor.create_picture(
                DisplayList::new(),
                Rect::from_origin_size(Offset::ZERO, Size::ZERO),
            )
        });

        let attachment = match widget {
            WidgetKind::Scroll { .. } | WidgetKind::SliverViewport { .. }
                if widget_clips(widget) =>
            {
                let clip =
                    compositor.create_clip_rect(Rect::from_origin_size(Offset::ZERO, Size::ZERO));
                let content = compositor.create_transform(CoreTransform::translation(Offset::ZERO));
                compositor.set_children(root, std::iter::once(clip).chain(picture).collect());
                compositor.set_children(clip, vec![content]);
                LayerAttachment::Clip {
                    clip,
                    content,
                    overflow_active: Cell::new(None),
                }
            }
            // `Clip.none`: the scrolled content keeps its offset transform
            // but paints past the viewport bounds.
            WidgetKind::Scroll { .. } | WidgetKind::SliverViewport { .. } => {
                let layer = compositor.create_transform(CoreTransform::translation(Offset::ZERO));
                compositor.set_children(root, std::iter::once(layer).chain(picture).collect());
                LayerAttachment::Transform { layer }
            }
            // Clip widgets start with zero geometry like scrolling; layout
            // and the update path fill the retained shape from the node
            // size before any flatten can observe it.
            WidgetKind::ClipRect { clip_behavior, .. }
            | WidgetKind::Unconstrained { clip_behavior, .. }
            | WidgetKind::Flex { clip_behavior, .. }
            | WidgetKind::Wrap { clip_behavior, .. }
            | WidgetKind::FittedBox { clip_behavior, .. }
                if widget_clips(widget) =>
            {
                let clip = compositor.create_clip_rect_with_coverage(
                    Rect::from_origin_size(Offset::ZERO, Size::ZERO),
                    shape_clip_coverage(*clip_behavior),
                );
                Self::create_clip_stage(compositor, root, picture, clip)
            }
            WidgetKind::CustomLayout { delegate, .. } if widget_clips(widget) => {
                let behavior = delegate.overflow_clip().expect("clip is enabled");
                let clip = compositor.create_clip_rect_with_coverage(
                    Rect::from_origin_size(Offset::ZERO, Size::ZERO),
                    shape_clip_coverage(behavior),
                );
                Self::create_clip_stage(compositor, root, picture, clip)
            }
            WidgetKind::ClipRRect { clip_behavior, .. } if widget_clips(widget) => {
                let clip = compositor.create_clip_rrect_with_coverage(
                    RRect::new(
                        Rect::from_origin_size(Offset::ZERO, Size::ZERO),
                        CornerRadii::ZERO,
                    ),
                    shape_clip_coverage(*clip_behavior),
                );
                Self::create_clip_stage(compositor, root, picture, clip)
            }
            WidgetKind::ClipOval { clip_behavior, .. } if widget_clips(widget) => {
                let clip = compositor.create_clip_oval_with_coverage(
                    Rect::from_origin_size(Offset::ZERO, Size::ZERO),
                    shape_clip_coverage(*clip_behavior),
                );
                Self::create_clip_stage(compositor, root, picture, clip)
            }
            WidgetKind::ClipPath { clip_behavior, .. } if widget_clips(widget) => {
                let clip = compositor.create_clip_path_with_coverage(
                    Arc::new(Path::builder().build()),
                    FillRule::NonZero,
                    shape_clip_coverage(*clip_behavior),
                );
                Self::create_clip_stage(compositor, root, picture, clip)
            }
            WidgetKind::Stack { .. } | WidgetKind::IndexedStack { .. } if widget_clips(widget) => {
                let clip =
                    compositor.create_clip_rect(Rect::from_origin_size(Offset::ZERO, Size::ZERO));
                Self::create_clip_stage(compositor, root, picture, clip)
            }
            WidgetKind::PersistentHeader { .. } | WidgetKind::Translate { .. } => {
                let layer = compositor.create_transform(CoreTransform::translation(Offset::ZERO));
                compositor.set_children(root, vec![layer]);
                LayerAttachment::Transform { layer }
            }
            WidgetKind::Transform { .. }
            | WidgetKind::Scale { .. }
            | WidgetKind::Affine { .. }
            | WidgetKind::Rotation { .. }
            | WidgetKind::FittedBox { .. } => {
                let layer = compositor.create_transform(CoreTransform::IDENTITY);
                if let WidgetKind::Transform { filter_quality, .. } = widget {
                    compositor.update_transform_sampling(
                        layer,
                        filter_quality.map(incular_rendering::FilterQuality::sampling),
                    );
                }
                compositor.set_children(root, vec![layer]);
                LayerAttachment::Transform { layer }
            }
            WidgetKind::Opacity { alpha, .. } => {
                let layer = compositor.create_opacity(incular_rendering::flutter_layer_opacity(*alpha));
                compositor.set_children(root, vec![layer]);
                LayerAttachment::Opacity { layer }
            }
            WidgetKind::Blur {
                sigma_x, sigma_y, ..
            } => {
                let layer = compositor.create_blur(GaussianBlur::new(*sigma_x, *sigma_y));
                compositor.set_children(root, vec![layer]);
                LayerAttachment::Blur { layer }
            }
            WidgetKind::DropShadow {
                offset,
                sigma_x,
                sigma_y,
                color,
                ..
            } => {
                let layer = compositor.create_drop_shadow(DropShadowEffect::asymmetric(
                    *offset, *sigma_x, *sigma_y, *color,
                ));
                compositor.set_children(root, vec![layer]);
                LayerAttachment::DropShadow { layer }
            }
            WidgetKind::Banner { shadow, .. } => {
                let sigma = crate::utilities::banner_shadow_sigma(shadow.blur_radius);
                let layer = compositor.create_drop_shadow(DropShadowEffect::asymmetric(
                    shadow.offset,
                    sigma,
                    sigma,
                    shadow.color,
                ));
                compositor.set_children(root, vec![layer]);
                LayerAttachment::DropShadow { layer }
            }
            // A blurred text shadow is the glyphs' own alpha, blurred and
            // tinted behind them: the same retained drop-shadow stage.
            WidgetKind::Text { style, .. }
                if let Some(shadow) = crate::utilities::layered_text_shadow(&style.shadows) =>
            {
                let layer = compositor.create_drop_shadow(text_shadow_effect(shadow));
                compositor.set_children(root, vec![layer]);
                LayerAttachment::DropShadow { layer }
            }
            WidgetKind::ColorFiltered { filter, .. } => {
                let layer = compositor.create_color_filter(**filter);
                compositor.set_children(root, vec![layer]);
                LayerAttachment::ColorFilter { layer }
            }
            WidgetKind::Image {
                blend_mode,
                source_effects,
                ..
            } if matches!(spec.attachment, AttachmentSpec::RasterEffects(_)) => {
                let mut raster = RasterEffectLayers::default();
                raster.reconcile(compositor, source_effects.raster_paint_sources(*blend_mode));
                raster.attach(compositor, root, picture);
                LayerAttachment::RasterEffects(Rc::new(raster))
            }
            WidgetKind::Blend { mode, .. } => {
                let layer = compositor.create_blend(*mode);
                compositor.set_children(root, vec![layer]);
                LayerAttachment::Blend { layer }
            }
            WidgetKind::ShaderMask { blend_mode, child: Some(_), .. } => {
                let layer = compositor.create_shader_mask(
                    Brush::Solid(Color::WHITE),
                    *blend_mode,
                    Size::ZERO,
                    CoreTransform::IDENTITY,
                );
                compositor.set_children(root, vec![layer]);
                LayerAttachment::ShaderMask { layer }
            }
            WidgetKind::BackdropFilter {
                blur,
                blend_mode,
                enabled,
                matrix,
                image_filter,
                backdrop_key,
                ..
            } => {
                let layer = compositor.create_backdrop_filter(*blur, *blend_mode, *enabled);
                compositor.update_backdrop_matrix(layer, *matrix);
                compositor.update_shared_backdrop_image_filter(layer, image_filter.clone());
                compositor.update_backdrop_key(layer, *backdrop_key);
                compositor.set_children(root, vec![layer]);
                LayerAttachment::BackdropFilter { layer }
            }
            WidgetKind::AnnotatedRegion {
                annotation, sized, ..
            } => {
                let layer =
                    compositor.create_annotated_region(annotation.clone(), *sized, Size::ZERO);
                compositor.set_children(root, vec![layer]);
                LayerAttachment::Annotation { layer }
            }
            WidgetKind::CompositedTransformTarget { link, .. } => {
                let layer = compositor.create_leader(link.clone(), Size::ZERO);
                compositor.set_children(root, vec![layer]);
                LayerAttachment::Leader { layer }
            }
            WidgetKind::CompositedTransformFollower {
                link,
                show_when_unlinked,
                offset,
                target_anchor,
                follower_anchor,
                ..
            } => {
                let layer = compositor.create_follower(FollowerLayer {
                    link: link.clone(),
                    show_when_unlinked: *show_when_unlinked,
                    offset: *offset,
                    target_anchor: *target_anchor,
                    follower_anchor: *follower_anchor,
                    size: Size::ZERO,
                });
                compositor.set_children(root, vec![layer]);
                LayerAttachment::Follower { layer }
            }
            _ => {
                compositor.set_children(
                    root,
                    picture.into_iter().chain(foreground_picture).collect(),
                );
                LayerAttachment::Direct
            }
        };
        Self {
            root,
            picture,
            foreground_picture,
            #[cfg(debug_assertions)]
            debug_foreground_picture: None,
            attachment,
            layout_overflow: Cell::new(false),
            clip_geometry: caches_clip_geometry.then(|| Rc::new(RefCell::new(None))),
        }
    }

    /// Attaches a fresh clip stage under `root`: the clip layer owns the
    /// shape, the content transform positions children inside it.
    fn create_clip_stage(
        compositor: &mut LayerTree,
        root: LayerId,
        picture: Option<LayerId>,
        clip: LayerId,
    ) -> LayerAttachment {
        let content = compositor.create_transform(CoreTransform::translation(Offset::ZERO));
        compositor.set_children(root, std::iter::once(clip).chain(picture).collect());
        compositor.set_children(clip, vec![content]);
        LayerAttachment::Clip {
            clip,
            content,
            overflow_active: Cell::new(None),
        }
    }

    fn spec(&self) -> LayerSpec {
        let attachment = match self.attachment {
            LayerAttachment::Direct => AttachmentSpec::Direct,
            LayerAttachment::Transform { .. } => AttachmentSpec::Transform,
            LayerAttachment::Clip { .. } => AttachmentSpec::Clip,
            LayerAttachment::Opacity { .. } => AttachmentSpec::Opacity,
            LayerAttachment::Blur { .. } => AttachmentSpec::Blur,
            LayerAttachment::DropShadow { .. } => AttachmentSpec::DropShadow,
            LayerAttachment::ColorFilter { .. } => AttachmentSpec::ColorFilter,
            LayerAttachment::Blend { .. } => AttachmentSpec::Blend,
            LayerAttachment::RasterEffects(ref layers) => {
                AttachmentSpec::RasterEffects(layers.flags())
            }
            LayerAttachment::ShaderMask { .. } => AttachmentSpec::ShaderMask,
            LayerAttachment::BackdropFilter { .. } => AttachmentSpec::BackdropFilter,
            LayerAttachment::Annotation { .. } => AttachmentSpec::Annotation,
            LayerAttachment::Leader { .. } => AttachmentSpec::Leader,
            LayerAttachment::Follower { .. } => AttachmentSpec::Follower,
        };
        LayerSpec {
            picture: self.picture.is_some(),
            foreground_picture: self.foreground_picture.is_some(),
            attachment,
        }
    }

    pub(crate) fn structure_matches(&self, widget: &WidgetKind) -> bool {
        self.spec() == LayerSpec::for_widget(widget)
    }

    pub(crate) fn owned_ids(&self) -> [Option<LayerId>; 5] {
        if let LayerAttachment::RasterEffects(layers) = &self.attachment {
            return [
                Some(self.root),
                self.picture,
                layers.filter,
                layers.opacity,
                layers.blend,
            ];
        }
        let (attachment_a, attachment_b) = match self.attachment {
            LayerAttachment::Direct => (None, None),
            LayerAttachment::Clip { clip, content, .. } => (Some(clip), Some(content)),
            ref attachment => (attachment.primary_layer(), None),
        };
        [
            Some(self.root),
            self.picture,
            self.foreground_picture,
            attachment_a,
            attachment_b,
        ]
    }

    /// Rebuilds only this render node's owned layer structure when the
    /// declarative compositor spec changes. The root transform ID remains
    /// stable so parent topology and render identity never need rewriting.
    pub(crate) fn reconcile_structure(
        &mut self,
        compositor: &mut LayerTree,
        widget: &WidgetKind,
    ) -> bool {
        if self.structure_matches(widget) {
            return false;
        }

        if let WidgetKind::Image {
            image,
            blend_mode,
            source_effects,
            ..
        } = widget
        {
            // Keep the picture and unchanged stages by identity even when
            // a new tint, opacity, or blend stage must be inserted.
            let mut raster = match std::mem::replace(&mut self.attachment, LayerAttachment::Direct)
            {
                LayerAttachment::RasterEffects(layers) => layers,
                _ => Rc::new(RasterEffectLayers::default()),
            };
            let source = if image.is_some() {
                source_effects.raster_paint_sources(*blend_mode)
            } else {
                RasterPaintSources {
                    blend: None,
                    filter: None,
                    opacity: None,
                }
            };
            Rc::make_mut(&mut raster).reconcile(compositor, source);
            if raster.flags() != 0 {
                self.attachment = LayerAttachment::RasterEffects(raster);
            }
            return true;
        }
        self.remove_owned_except_root(compositor);
        let fresh = Self::create(compositor, widget);
        compositor.remove(fresh.root);
        self.picture = fresh.picture;
        self.foreground_picture = fresh.foreground_picture;
        self.attachment = fresh.attachment;
        if let LayerAttachment::Clip { clip, .. } | LayerAttachment::Transform { layer: clip } =
            self.attachment
        {
            // The temporary root is gone. A clip or transform stage gets
            // its children on the content layer, so bind the fresh stage
            // to the retained root here as well.
            compositor.set_children(
                self.root,
                std::iter::once(clip).chain(self.picture).collect(),
            );
        }
        true
    }

    fn remove_owned_except_root(&mut self, compositor: &mut LayerTree) {
        #[cfg(debug_assertions)]
        if let Some(picture) = self.debug_foreground_picture.take() {
            compositor.remove(picture);
        }
        if let Some(picture) = self.picture.take() {
            compositor.remove(picture);
        }
        if let Some(picture) = self.foreground_picture.take() {
            compositor.remove(picture);
        }
        match std::mem::replace(&mut self.attachment, LayerAttachment::Direct) {
            LayerAttachment::Direct => {}
            LayerAttachment::Clip { clip, content, .. } => {
                compositor.remove(content);
                compositor.remove(clip);
            }
            LayerAttachment::RasterEffects(layers) => {
                for stage in layers.ids().into_iter().flatten() {
                    compositor.remove(stage);
                }
            }
            attachment => {
                compositor.remove(attachment.primary_layer().expect("attachment layer"));
            }
        }
    }

    #[cfg(feature = "devtools")]
    pub(crate) fn clip(&self) -> Option<LayerId> {
        match self.attachment {
            LayerAttachment::Clip { clip, .. } => Some(clip),
            _ => None,
        }
    }
    pub(crate) fn content(&self) -> Option<LayerId> {
        match self.attachment {
            LayerAttachment::Clip { content, .. } => Some(content),
            LayerAttachment::Transform { layer } => Some(layer),
            _ => None,
        }
    }
    pub(crate) fn opacity(&self) -> Option<LayerId> {
        match self.attachment {
            LayerAttachment::Opacity { layer } => Some(layer),
            _ => None,
        }
    }
    pub(crate) fn blur(&self) -> Option<LayerId> {
        match self.attachment {
            LayerAttachment::Blur { layer } => Some(layer),
            _ => None,
        }
    }
    pub(crate) fn shadow(&self) -> Option<LayerId> {
        match self.attachment {
            LayerAttachment::DropShadow { layer } => Some(layer),
            _ => None,
        }
    }
    pub(crate) fn color_filter(&self) -> Option<LayerId> {
        match self.attachment {
            LayerAttachment::ColorFilter { layer } => Some(layer),
            _ => None,
        }
    }
    pub(crate) fn blend(&self) -> Option<LayerId> {
        match self.attachment {
            LayerAttachment::Blend { layer } => Some(layer),
            _ => None,
        }
    }
    pub(crate) fn shader_mask(&self) -> Option<LayerId> {
        match self.attachment {
            LayerAttachment::ShaderMask { layer } => Some(layer),
            _ => None,
        }
    }
    pub(crate) fn backdrop_filter(&self) -> Option<LayerId> {
        match self.attachment {
            LayerAttachment::BackdropFilter { layer } => Some(layer),
            _ => None,
        }
    }
    pub(crate) fn update_from_kind(
        &self,
        compositor: &mut LayerTree,
        kind: &RenderKind,
        size: Size,
        content_transform: Option<CoreTransform>,
    ) {
        if matches!(self.attachment, LayerAttachment::Clip { .. }) {
            self.apply_clip(compositor, kind, size);
            if let (Some(layer), Some(transform)) = (self.content(), content_transform) {
                compositor.update_transform(layer, transform);
            }
            return;
        }
        match (&self.attachment, kind) {
            (LayerAttachment::Opacity { layer }, RenderKind::Opacity { alpha, .. }) => {
                compositor.update_opacity(*layer, *alpha);
            }
            (
                LayerAttachment::Blur { layer },
                RenderKind::Blur {
                    sigma_x, sigma_y, ..
                },
            ) => {
                compositor.update_blur(*layer, GaussianBlur::new(*sigma_x, *sigma_y));
            }
            (
                LayerAttachment::DropShadow { layer },
                RenderKind::DropShadow {
                    offset,
                    sigma_x,
                    sigma_y,
                    color,
                    ..
                },
            ) => {
                compositor.update_drop_shadow(
                    *layer,
                    DropShadowEffect::asymmetric(*offset, *sigma_x, *sigma_y, *color),
                );
            }
            (LayerAttachment::DropShadow { layer }, RenderKind::Text { style, .. }) => {
                // The resolved style may have dropped the blurred shadow the
                // widget declared; a transparent effect then draws nothing.
                compositor.update_drop_shadow(
                    *layer,
                    crate::utilities::layered_text_shadow(&style.shadows).map_or_else(
                        || DropShadowEffect::asymmetric(Offset::ZERO, 0.0, 0.0, Color::TRANSPARENT),
                        text_shadow_effect,
                    ),
                );
            }
            (LayerAttachment::DropShadow { layer }, RenderKind::Banner { shadow, .. }) => {
                let sigma = crate::utilities::banner_shadow_sigma(shadow.blur_radius);
                compositor.update_drop_shadow(
                    *layer,
                    DropShadowEffect::asymmetric(shadow.offset, sigma, sigma, shadow.color),
                );
            }
            (LayerAttachment::ColorFilter { layer }, RenderKind::ColorFiltered { filter, .. }) => {
                compositor.update_color_filter(*layer, **filter);
            }
            (LayerAttachment::Blend { layer }, RenderKind::Blend { mode }) => {
                compositor.update_blend(*layer, *mode);
            }
            (
                LayerAttachment::RasterEffects(layers),
                RenderKind::Image {
                    blend_mode,
                    source_effects,
                    ..
                },
            ) => layers.update(compositor, source_effects.raster_paint_sources(*blend_mode)),
            (
                LayerAttachment::ShaderMask { layer },
                RenderKind::ShaderMask { blend_mode, .. },
            ) => {
                compositor.update_shader_mask_blend(*layer, *blend_mode);
            }
            (
                LayerAttachment::BackdropFilter { layer },
                RenderKind::BackdropFilter {
                    blur,
                    blend_mode,
                    enabled,
                    matrix,
                    image_filter,
                    backdrop_key,
                },
            ) => {
                compositor.update_backdrop_filter(*layer, *blur, *blend_mode, *enabled);
                compositor.update_backdrop_matrix(*layer, *matrix);
                compositor.update_shared_backdrop_image_filter(*layer, image_filter.clone());
                compositor.update_backdrop_key(*layer, *backdrop_key);
            }
            (
                LayerAttachment::Annotation { layer },
                RenderKind::AnnotatedRegion { annotation, sized },
            ) => {
                compositor.update_annotated_region(*layer, annotation.clone(), *sized, size);
            }
            (LayerAttachment::Leader { layer }, RenderKind::Leader { link }) => {
                compositor.update_leader(*layer, link.clone(), size);
            }
            (
                LayerAttachment::Follower { layer },
                RenderKind::Follower {
                    link,
                    show_when_unlinked,
                    offset,
                    target_anchor,
                    follower_anchor,
                },
            ) => {
                compositor.update_follower(
                    *layer,
                    FollowerLayer {
                        link: link.clone(),
                        show_when_unlinked: *show_when_unlinked,
                        offset: *offset,
                        target_anchor: *target_anchor,
                        follower_anchor: *follower_anchor,
                        size,
                    },
                );
            }
            (LayerAttachment::Transform { layer }, _) => {
                if let Some(transform) = content_transform {
                    compositor.update_transform(*layer, transform);
                }
                // Flutter `Transform.filterQuality`. Scale and rotation
                // transitions apply theirs only while animating, which the
                // compositor update decides.
                if let RenderKind::Transform { filter_quality, .. } = kind {
                    compositor.update_transform_sampling(
                        *layer,
                        filter_quality.map(incular_rendering::FilterQuality::sampling),
                    );
                }
            }
            _ => {}
        }
    }

    pub(crate) fn update_layout_geometry(
        &self,
        compositor: &mut LayerTree,
        kind: &RenderKind,
        size: Size,
        content_transform: Option<CoreTransform>,
    ) {
        // Clip geometry follows the measured size here; option changes
        // take the same path through `update_from_kind`, so neither needs
        // layout beyond measuring.
        self.apply_clip(compositor, kind, size);
        if let Some(content) = self.content()
            && let Some(transform) = content_transform
        {
            compositor.update_transform(content, transform);
        }
        match (&self.attachment, kind) {
            (
                LayerAttachment::Annotation { layer },
                RenderKind::AnnotatedRegion { annotation, sized },
            ) => {
                compositor.update_annotated_region(*layer, annotation.clone(), *sized, size);
            }
            (LayerAttachment::Leader { layer }, RenderKind::Leader { .. }) => {
                compositor.update_leader_size(*layer, size);
            }
            (
                LayerAttachment::Follower { layer },
                RenderKind::Follower {
                    link,
                    show_when_unlinked,
                    offset,
                    target_anchor,
                    follower_anchor,
                },
            ) => {
                compositor.update_follower(
                    *layer,
                    FollowerLayer {
                        link: link.clone(),
                        show_when_unlinked: *show_when_unlinked,
                        offset: *offset,
                        target_anchor: *target_anchor,
                        follower_anchor: *follower_anchor,
                        size,
                    },
                );
            }
            _ => {}
        }
    }

    pub(crate) fn set_render_children(
        &self,
        compositor: &mut LayerTree,
        kind: &RenderKind,
        mut child_layers: Vec<LayerId>,
    ) {
        if matches!(kind, RenderKind::Visibility { visible: false, .. }) {
            // Disconnect cached child pictures as well as skipping new paint.
            // The retained layers remain owned and reconnect when shown.
            compositor.set_children(self.root, Vec::new());
            return;
        }
        if matches!(kind, RenderKind::Banner { .. } | RenderKind::Text { .. }) {
            let mut layers = child_layers;
            if let (Some(shadow), Some(picture)) = (self.shadow(), self.picture) {
                compositor.set_children(shadow, vec![picture]);
                layers.push(shadow);
            } else {
                layers.extend(self.picture);
            }
            compositor.set_children(self.root, layers);
            return;
        }
        match self.attachment {
            LayerAttachment::Direct => {
                let mut layers = Vec::with_capacity(
                    child_layers.len() + 1 + usize::from(self.foreground_picture.is_some()),
                );
                layers.extend(self.picture);
                layers.append(&mut child_layers);
                layers.extend(self.foreground_picture);
                layers.extend(self.debug_foreground_picture());
                compositor.set_children(self.root, layers);
            }
            LayerAttachment::Clip { content, .. }
            | LayerAttachment::Transform { layer: content } => {
                compositor.set_children(content, child_layers);
            }
            LayerAttachment::RasterEffects(ref layers) => {
                debug_assert!(child_layers.is_empty(), "retained image is a leaf");
                layers.attach(compositor, self.root, self.picture);
            }
            _ => {
                let layer = self
                    .attachment
                    .primary_layer()
                    .expect("non-direct attachment has a layer");
                compositor.set_children(layer, child_layers);
                compositor.set_children(self.root, vec![layer]);
            }
        }
    }

    fn debug_foreground_picture(&self) -> Option<LayerId> {
        #[cfg(debug_assertions)]
        {
            self.debug_foreground_picture
        }
        #[cfg(not(debug_assertions))]
        {
            None
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn has_debug_foreground_picture(&self) -> bool {
        self.debug_foreground_picture.is_some()
    }

    /// Lazily retains a diagnostic picture above the child. Returns
    /// whether the direct child's layer attachment must be synchronized.
    #[cfg(debug_assertions)]
    pub(crate) fn update_debug_foreground_picture(
        &mut self,
        compositor: &mut LayerTree,
        picture: &DisplayList,
        bounds: Rect,
    ) -> bool {
        match (picture.commands().is_empty(), self.debug_foreground_picture) {
            (true, None) => return false,
            (false, Some(id)) => {
                compositor.update_picture_if_changed(id, picture, bounds);
                return false;
            }
            (true, Some(id)) => {
                self.debug_foreground_picture = None;
                compositor.remove(id);
            }
            (false, None) => {
                let id = compositor.create_picture(DisplayList::new(), bounds);
                compositor.update_picture_if_changed(id, picture, bounds);
                self.debug_foreground_picture = Some(id);
            }
        }
        if let LayerAttachment::Clip { clip, content, overflow_active, .. } = &self.attachment {
            let stage = if overflow_active.get().unwrap_or(true) { *clip } else { *content };
            compositor.set_children(
                self.root,
                std::iter::once(stage)
                    .chain(self.picture)
                    .chain(self.debug_foreground_picture)
                    .collect(),
            );
            false
        } else {
            true
        }
    }

    pub(crate) fn remove(mut self, compositor: &mut LayerTree) {
        let root = self.root;
        self.remove_owned_except_root(compositor);
        compositor.remove(root);
    }
}

fn needs_picture(widget: &WidgetKind) -> bool {
    if let WidgetKind::Decorated { background, border, .. } = widget {
        // A hit-only decoration uses its child's paint layers.
        return background.is_some() || border.is_some();
    }
    matches!(
        widget,
        WidgetKind::Box { .. }
            | WidgetKind::Shape { .. }
            | WidgetKind::CustomPaint { .. }
            | WidgetKind::RepaintBoundary { .. }
            | WidgetKind::Colored { .. }
            | WidgetKind::Banner { .. }
            | WidgetKind::Button(_)
            | WidgetKind::Text { .. }
            | WidgetKind::SelectableText { .. }
            | WidgetKind::TextField(_)
            | WidgetKind::Image { .. }
            | WidgetKind::Scroll { .. }
            | WidgetKind::SliverViewport { .. }
    )
}

fn text_shadow_effect(shadow: incular_text::TextShadow) -> DropShadowEffect {
    let sigma = crate::utilities::banner_shadow_sigma(shadow.blur_radius);
    DropShadowEffect::asymmetric(shadow.offset, sigma, sigma, shadow.color)
}

/// Clip geometry a clip render object resolves for its laid-out size.
#[derive(Clone, Debug)]
enum ClipGeometry {
    Rect(Rect),
    RRect(RRect),
    Oval(Rect),
    Path(Arc<Path>),
}

impl ClipGeometry {
    fn accepts_hit(&self, point: Offset) -> bool {
        let contains_rect = |rect: Rect| {
            point.x >= rect.origin.x
                && point.y >= rect.origin.y
                && point.x < rect.origin.x + rect.size.width
                && point.y < rect.origin.y + rect.size.height
        };
        match self {
            Self::Rect(rect) => contains_rect(*rect),
            Self::RRect(rrect) => contains_rect(rrect.rect) && rrect.contains(point),
            Self::Oval(rect) => {
                let x = (point.x - rect.origin.x - rect.size.width * 0.5) / rect.size.width;
                let y = (point.y - rect.origin.y - rect.size.height * 0.5) / rect.size.height;
                (x * x + y * y).partial_cmp(&0.25) != Some(std::cmp::Ordering::Greater)
            }
            Self::Path(path) => path.contains(point, FillRule::NonZero),
        }
    }
}

fn rect_path(rect: Rect) -> Arc<Path> {
    let mut lines = Path::builder();
    lines
        .move_to(rect.origin)
        .line_to(Offset::new(rect.origin.x + rect.size.width, rect.origin.y))
        .line_to(Offset::new(
            rect.origin.x + rect.size.width,
            rect.origin.y + rect.size.height,
        ))
        .line_to(Offset::new(rect.origin.x, rect.origin.y + rect.size.height))
        .close();
    Arc::new(lines.build())
}

/// Resolves a clip node's geometry from its kind and laid-out size. A
/// [`crate::layout::ClipDelegate`] derives the shape from the size; without one
/// the widget's own rectangle, radii or path apply.
fn clip_geometry(kind: &RenderKind, size: Size) -> Option<ClipGeometry> {
    let bounds = Rect::from_origin_size(Offset::ZERO, size);
    Some(match kind {
        RenderKind::ClipRect { clipper, .. } => ClipGeometry::Rect(
            clipper
                .as_ref()
                .map_or(bounds, |clipper| clipper.clip(size).bounds()),
        ),
        RenderKind::ClipRRect {
            radius, clipper, ..
        } => ClipGeometry::RRect(match clipper.as_ref().map(|clipper| clipper.clip(size)) {
            None => RRect::new(bounds, *radius),
            Some(ClipShape::RRect(rrect)) => rrect,
            Some(shape) => RRect::new(shape.bounds(), *radius),
        }),
        RenderKind::ClipOval { clipper, .. } => ClipGeometry::Oval(
            clipper
                .as_ref()
                .map_or(bounds, |clipper| clipper.clip(size).bounds()),
        ),
        RenderKind::ClipPath { path, clipper, .. } => {
            ClipGeometry::Path(match clipper.as_ref().map(|clipper| clipper.clip(size)) {
                None => path.clone().unwrap_or_else(|| rect_path(bounds)),
                Some(ClipShape::Path(path)) => path,
                Some(shape) => rect_path(shape.bounds()),
            })
        }
        // Viewports and other clipping kinds clip to their own bounds.
        _ => ClipGeometry::Rect(bounds),
    })
}

fn apply_clip_geometry(compositor: &mut LayerTree, clip: LayerId, geometry: &ClipGeometry) {
    match geometry {
        ClipGeometry::Rect(rect) => {
            compositor.update_clip(clip, *rect);
        }
        ClipGeometry::RRect(rrect) => {
            compositor.update_clip_rrect(clip, *rrect);
        }
        ClipGeometry::Oval(rect) => {
            compositor.update_clip_oval(clip, *rect);
        }
        ClipGeometry::Path(path) => {
            compositor.update_clip_path(clip, path.clone(), FillRule::NonZero);
        }
    }
}

fn shape_clip_coverage(mode: Clip) -> incular_rendering::ClipCoverage {
    use incular_rendering::ClipCoverage;
    match mode {
        Clip::None | Clip::HardEdge => ClipCoverage::HardEdge,
        Clip::AntiAlias => ClipCoverage::AntiAlias,
        Clip::AntiAliasWithSaveLayer => ClipCoverage::AntiAliasWithSaveLayer,
    }
}

/// One resolved shape, used by raster and hit testing.
#[derive(Clone, Debug)]
struct CachedClip {
    size: Size,
    kind: RenderKind,
    geometry: ClipGeometry,
}

/// Whether two clip kinds share the non-delegate inputs (corner radii, path)
/// that shape the clip geometry.
fn same_clip_inputs(old: &RenderKind, new: &RenderKind) -> bool {
    match (old, new) {
        (RenderKind::ClipRect { .. }, RenderKind::ClipRect { .. })
        | (RenderKind::ClipOval { .. }, RenderKind::ClipOval { .. }) => true,
        (RenderKind::ClipRRect { radius: a, .. }, RenderKind::ClipRRect { radius: b, .. }) => {
            a == b
        }
        (RenderKind::ClipPath { path: a, .. }, RenderKind::ClipPath { path: b, .. }) => a == b,
        _ => false,
    }
}

fn clip_delegate(kind: &RenderKind) -> Option<&ClipDelegate> {
    match kind {
        RenderKind::ClipRect { clipper, .. }
        | RenderKind::ClipRRect { clipper, .. }
        | RenderKind::ClipOval { clipper, .. }
        | RenderKind::ClipPath { clipper, .. } => clipper.as_ref(),
        _ => None,
    }
}

fn cached_clip<'a>(
    cache: &'a mut Option<CachedClip>,
    kind: &RenderKind,
    size: Size,
) -> Option<&'a ClipGeometry> {
    let reuse = cache.as_ref().is_some_and(|cached| {
        cached.size == size
            && (cached.kind == *kind
                || (same_clip_inputs(&cached.kind, kind)
                    && match (clip_delegate(&cached.kind), clip_delegate(kind)) {
                        (Some(old), Some(new)) => old == new || !new.reclips_from(old),
                        (None, None) => true,
                        _ => false,
                    }))
    });
    if reuse {
        let cached = cache.as_mut().expect("existing clip cache");
        if cached.kind != *kind {
            cached.kind = kind.clone();
        }
    } else {
        *cache = Some(CachedClip {
            size,
            kind: kind.clone(),
            geometry: clip_geometry(kind, size)?,
        });
    }
    cache.as_ref().map(|cached| &cached.geometry)
}

impl RenderLayers {
    pub(crate) fn accepts_clip_hit(&self, kind: &RenderKind, size: Size, local: Offset) -> bool {
        if clip_delegate(kind).is_some_and(|delegate| !delegate.clips_hit_tests()) {
            return true;
        }
        // RenderClipRect and RenderClipRRect only narrow their normal box
        // with an explicit clipper. Oval always tests its ellipse; Incular's
        // explicit ClipPath shape corresponds to Flutter's path clipper.
        if !matches!(
            kind,
            RenderKind::ClipRect {
                clipper: Some(_),
                ..
            } | RenderKind::ClipRRect {
                clipper: Some(_),
                ..
            } | RenderKind::ClipOval { .. }
                | RenderKind::ClipPath {
                    clipper: Some(_),
                    ..
                }
                | RenderKind::ClipPath { path: Some(_), .. }
        ) {
            return true;
        }
        self.clip_geometry.as_ref().is_none_or(|cache| {
            cached_clip(&mut cache.borrow_mut(), kind, size)
                .is_none_or(|geometry| geometry.accepts_hit(local))
        })
    }

    pub(crate) fn set_layout_overflow(&self, overflow: bool) {
        self.layout_overflow.set(overflow);
    }

    fn box_paint_clip<'a>(&self, kind: &'a RenderKind) -> Option<BoxPaintClip<'a>> {
        match kind {
            RenderKind::ClipRect {
                clip_behavior,
                clipper,
            }
            | RenderKind::ClipRRect {
                clip_behavior,
                clipper,
                ..
            }
            | RenderKind::ClipOval {
                clip_behavior,
                clipper,
            }
            | RenderKind::ClipPath {
                clip_behavior,
                clipper,
                ..
            } if *clip_behavior != Clip::None => Some(
                clipper
                    .as_ref()
                    .map_or(BoxPaintClip::LayoutBounds, BoxPaintClip::Custom),
            ),
            RenderKind::Stack { stack }
                if stack.clip_behavior != Clip::None && self.layout_overflow.get() =>
            {
                Some(BoxPaintClip::LayoutBounds)
            }
            RenderKind::Flex { clip_behavior, .. }
            | RenderKind::IndexedStack { clip_behavior, .. }
                if *clip_behavior != Clip::None && self.layout_overflow.get() =>
            {
                Some(BoxPaintClip::LayoutBounds)
            }
            _ => None,
        }
    }

    #[must_use]
    pub(crate) fn has_box_semantic_clip(&self, kind: &RenderKind) -> bool {
        self.box_paint_clip(kind).is_some()
    }

    /// The clip bounds for accessibility are independent of the paint
    /// and hit-test shape, as in Flutter's describeApproximatePaintClip.
    pub(crate) fn approximate_box_paint_clip(&self, kind: &RenderKind, size: Size) -> Option<Rect> {
        Some(match self.box_paint_clip(kind)? {
            BoxPaintClip::LayoutBounds => Rect::from_origin_size(Offset::ZERO, size),
            BoxPaintClip::Custom(clipper) => clipper.approximate_clip_rect(size),
        })
    }

    /// Keep the child's content and clip stage available, but bypass the
    /// stage when the layout reports no visual overflow.
    pub(crate) fn apply_overflow_rect_clip(
        &self,
        compositor: &mut LayerTree,
        size: Size,
        enabled: bool,
    ) {
        let LayerAttachment::Clip {
            clip,
            content,
            overflow_active,
            ..
        } = &self.attachment
        else {
            return;
        };
        if overflow_active.get() != Some(enabled) {
            if enabled {
                compositor.set_children(*clip, vec![*content]);
            } else {
                compositor.set_children(*clip, Vec::new());
            }
            let stage = if enabled { *clip } else { *content };
            compositor.set_children(
                self.root,
                std::iter::once(stage)
                    .chain(self.picture)
                    .chain(self.debug_foreground_picture())
                    .collect(),
            );
            overflow_active.set(Some(enabled));
        }
        if enabled {
            compositor.update_clip(*clip, Rect::from_origin_size(Offset::ZERO, size));
        }
    }

    pub(crate) fn apply_overflow_clip_with_coverage(
        &self,
        compositor: &mut LayerTree,
        size: Size,
        behavior: Clip,
        overflows: bool,
    ) {
        if let LayerAttachment::Clip { clip, .. } = &self.attachment {
            compositor.update_clip_rect_coverage(*clip, shape_clip_coverage(behavior));
            self.apply_overflow_rect_clip(compositor, size, overflows);
        }
    }

    /// Rebuilds the clip geometry from the kind and size. The compositor keeps
    /// its geometry when the size and clip inputs (radius, path) are unchanged
    /// and the current delegate does not reclip against the one that produced it
    /// (`CustomClipper::should_reclip`), as Flutter's `_RenderCustomClip` does.
    fn apply_clip(&self, compositor: &mut LayerTree, kind: &RenderKind, size: Size) {
        if let RenderKind::CustomLayout { delegate } = kind
            && let Some(behavior) = delegate.overflow_clip()
        {
            let always_clip = matches!(
                delegate,
                crate::custom_layout::CustomLayoutDelegate::Flow(_)
            );
            self.apply_overflow_clip_with_coverage(
                compositor,
                size,
                behavior,
                always_clip || self.layout_overflow.get(),
            );
            return;
        }
        if let RenderKind::Flex { clip_behavior, .. }
        | RenderKind::Wrap { clip_behavior, .. }
        | RenderKind::Unconstrained { clip_behavior, .. } = *kind
        {
            self.apply_overflow_clip_with_coverage(
                compositor,
                size,
                clip_behavior,
                self.layout_overflow.get(),
            );
            return;
        }
        if let RenderKind::SliverViewport { config } = kind {
            self.apply_overflow_clip_with_coverage(
                compositor,
                size,
                config.clip_behavior,
                self.layout_overflow.get(),
            );
            return;
        }
        let stack_clip = match kind {
            RenderKind::Stack { stack } => Some(stack.clip_behavior),
            RenderKind::IndexedStack { clip_behavior, .. } => Some(*clip_behavior),
            _ => None,
        };
        if let Some(behavior) = stack_clip {
            self.apply_overflow_clip_with_coverage(
                compositor,
                size,
                behavior,
                self.layout_overflow.get(),
            );
            return;
        }
        if matches!(
            kind,
            RenderKind::Scroll { .. } | RenderKind::FittedBox { .. }
        ) {
            // The viewport supplies its laid-out child's translated bounds.
            return;
        }
        let LayerAttachment::Clip { clip, .. } = &self.attachment else {
            return;
        };
        // A coverage change does not recalculate cached clipper geometry.
        match kind {
            RenderKind::ClipRect { clip_behavior, .. } => {
                compositor.update_clip_rect_coverage(*clip, shape_clip_coverage(*clip_behavior));
            }
            RenderKind::ClipRRect { clip_behavior, .. } => {
                compositor.update_clip_rrect_coverage(*clip, shape_clip_coverage(*clip_behavior));
            }
            RenderKind::ClipOval { clip_behavior, .. } => {
                compositor.update_clip_oval_coverage(*clip, shape_clip_coverage(*clip_behavior));
            }
            RenderKind::ClipPath { clip_behavior, .. } => {
                compositor.update_clip_path_coverage(*clip, shape_clip_coverage(*clip_behavior));
            }
            _ => {}
        }
        if let Some(cache) = &self.clip_geometry
            && let Some(geometry) = cached_clip(&mut cache.borrow_mut(), kind, size)
        {
            apply_clip_geometry(compositor, *clip, geometry);
        }
    }
}
