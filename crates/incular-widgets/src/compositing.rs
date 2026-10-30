//! Retained widgets whose behavior is defined by compositor layers rather
//! than by a standalone render object.

use std::{any::Any, rc::Rc};

use incular_config::Alignment;
use incular_core::{Offset, Rect};
use incular_rendering::{
    Annotation, BackdropKey, BlendMode, GaussianBlur, LayerAnchor, LayerLink, Shader,
};

use crate::macros::widget_from_kind;
use crate::tree::{Widget, WidgetKind};

/// The renderer-neutral form of Flutter's `ShaderCallback`.
///
/// A callback runs at resolve time with the widget's local bounds (zero
/// origin, node size): on first paint, on any repaint (including child
/// resize, which relayouts and repaints the mask node), and on mask
/// configuration changes. It never runs per frame: rebuilding the
/// identical mask resolves nothing. Keeping the callback behind an
/// identity-bearing value lets reconciliation distinguish a changed
/// shader source without putting a backend shader object into the
/// widget tree.
#[derive(Clone)]
pub struct ShaderCallback(Rc<dyn Fn(Rect) -> Shader>);

impl ShaderCallback {
    #[must_use]
    pub fn new(callback: impl Fn(Rect) -> Shader + 'static) -> Self {
        Self(Rc::new(callback))
    }

    #[must_use]
    pub fn call(&self, bounds: Rect) -> Shader {
        (self.0)(bounds)
    }
}

impl<F> From<F> for ShaderCallback
where
    F: Fn(Rect) -> Shader + 'static,
{
    fn from(callback: F) -> Self {
        Self::new(callback)
    }
}

impl std::fmt::Debug for ShaderCallback {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ShaderCallback(..)")
    }
}

impl PartialEq for ShaderCallback {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Applies a callback-generated shader to a child using Flutter's default
/// `BlendMode::modulate` behavior.
///
/// The recorded mask size and transform snapshot resolve time; placement
/// of resolved content follows the live transform stack every flatten,
/// while the stage bounds recompute live. Hit testing and semantics pass
/// through the mask untouched.
///
/// Backend compatibility: the WGPU backend does not execute this stage
/// yet — presenting a scene containing it fails the frame with
/// `RendererError::UnsupportedShaderMask` instead of drawing the child
/// unmasked. Retained transport, hit testing, and semantics are
/// unaffected by backend support.
#[derive(Clone, Debug, PartialEq)]
pub struct ShaderMask {
    shader: ShaderCallback,
    blend_mode: BlendMode,
    child: Widget,
}

impl ShaderMask {
    #[must_use]
    pub fn new(shader: impl Into<ShaderCallback>, child: impl Into<Widget>) -> Self {
        Self {
            shader: shader.into(),
            blend_mode: BlendMode::Modulate,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn shader_callback(mut self, shader: impl Into<ShaderCallback>) -> Self {
        self.shader = shader.into();
        self
    }

    #[must_use]
    pub fn blend_mode(mut self, blend_mode: BlendMode) -> Self {
        self.blend_mode = blend_mode;
        self
    }
}

widget_from_kind!(ShaderMask => ShaderMask { shader, blend_mode, child });

/// Filters the already-painted backdrop behind a child: the backdrop is
/// whatever the flattened scene painted before the stage in paint order,
/// and the recorded bounds cover the child only. The filter is disabled
/// without removing the child from layout or paint order, and zero sigma
/// passes through identically. Hit testing and semantics pass through
/// the filter untouched.
///
/// Backend compatibility: the WGPU backend executes only the supported
/// passthroughs (disabled filters and zero sigma). Presenting an enabled
/// nonzero filter fails the frame with
/// `RendererError::UnsupportedBackdropFilter` instead of drawing the
/// child unfiltered. Retained transport, hit testing, and semantics are
/// unaffected by backend support.
#[derive(Clone, Debug, PartialEq)]
pub struct BackdropFilter {
    blur: GaussianBlur,
    blend_mode: BlendMode,
    enabled: bool,
    matrix: Option<incular_core::Transform>,
    backdrop_key: Option<BackdropKey>,
    /// `BackdropFilter.grouped`: take the key of the nearest
    /// [`BackdropGroup`] at build time.
    grouped: bool,
    child: Widget,
}

impl BackdropFilter {
    #[must_use]
    pub fn new(blur: GaussianBlur, child: impl Into<Widget>) -> Self {
        Self {
            blur: GaussianBlur::new(blur.sigma_x, blur.sigma_y),
            blend_mode: BlendMode::SrcOver,
            enabled: true,
            matrix: None,
            backdrop_key: None,
            grouped: false,
            child: child.into(),
        }
    }

    /// Flutter's `BackdropFilter.grouped`: shares the backdrop of the
    /// nearest [`BackdropGroup`], so every grouped filter under it samples
    /// one capture. Without a group it filters on its own.
    #[must_use]
    pub fn grouped(blur: GaussianBlur, child: impl Into<Widget>) -> Self {
        Self {
            grouped: true,
            ..Self::new(blur, child)
        }
    }

    /// Flutter's `backdropKey`: filters with the same key sample one
    /// backdrop, captured where the first of them paints.
    #[must_use]
    pub fn backdrop_key(mut self, key: BackdropKey) -> Self {
        self.backdrop_key = Some(key);
        self.grouped = false;
        self
    }

    /// Flutter's `BackdropFilter` with `ImageFilter.matrix`: the backdrop
    /// transformed by `matrix`, in the filter's coordinates, rather than
    /// blurred.
    #[must_use]
    pub fn matrix(matrix: incular_core::Transform, child: impl Into<Widget>) -> Self {
        Self {
            matrix: Some(matrix),
            ..Self::new(GaussianBlur::uniform(0.0), child)
        }
    }

    #[must_use]
    pub fn blur(sigma: f32, child: impl Into<Widget>) -> Self {
        Self::new(GaussianBlur::uniform(sigma), child)
    }

    #[must_use]
    pub fn asymmetric(sigma_x: f32, sigma_y: f32, child: impl Into<Widget>) -> Self {
        Self::new(GaussianBlur::new(sigma_x, sigma_y), child)
    }

    #[must_use]
    pub fn filter(mut self, blur: GaussianBlur) -> Self {
        self.blur = GaussianBlur::new(blur.sigma_x, blur.sigma_y);
        self
    }

    #[must_use]
    pub fn blend_mode(mut self, blend_mode: BlendMode) -> Self {
        self.blend_mode = blend_mode;
        self
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl From<BackdropFilter> for Widget {
    fn from(value: BackdropFilter) -> Self {
        let kind = |value: BackdropFilter, backdrop_key| {
            Widget::from_kind(WidgetKind::BackdropFilter {
                blur: value.blur,
                blend_mode: value.blend_mode,
                enabled: value.enabled,
                matrix: value.matrix,
                backdrop_key,
                child: value.child,
            })
        };
        if !value.grouped {
            let key = value.backdrop_key;
            return kind(value, key);
        }
        crate::Builder::new(move |context| {
            let key = BackdropGroup::of(context);
            kind(value.clone(), key)
        })
        .into()
    }
}

/// Flutter's `BackdropGroup`: a [`BackdropKey`] for the
/// [`BackdropFilter::grouped`] filters below it. The key lasts as long as
/// the group's element, unless one is given.
#[derive(Clone)]
pub struct BackdropGroup {
    backdrop_key: Option<BackdropKey>,
    child: Widget,
}

impl BackdropGroup {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            backdrop_key: None,
            child: child.into(),
        }
    }

    /// Uses `key` instead of a key of the group's own.
    #[must_use]
    pub fn backdrop_key(mut self, key: BackdropKey) -> Self {
        self.backdrop_key = Some(key);
        self
    }

    /// `BackdropGroup.of(context)?.backdropKey`: the nearest group's key.
    #[must_use]
    pub fn of(context: &crate::BuildContext<'_>) -> Option<BackdropKey> {
        context.depend_on::<GroupKey>().map(|key| key.0)
    }
}

#[derive(Clone, Copy)]
struct GroupKey(BackdropKey);

struct GroupState(BackdropKey);

impl crate::tree::RetainedState for GroupState {
    fn rebind_revision(&self, _revision: Rc<std::cell::Cell<u64>>) {}
}

impl From<BackdropGroup> for Widget {
    fn from(value: BackdropGroup) -> Self {
        Widget::retained_builder(
            |_| GroupState(BackdropKey::new()),
            move |state, _, _| {
                Widget::environment_scope(
                    GroupKey(value.backdrop_key.unwrap_or(state.0)),
                    value.child.clone(),
                )
            },
            false,
        )
    }
}

/// Associates a strongly typed platform value with a painted region. The
/// value is erased only inside the renderer-neutral layer tree and can be
/// recovered with `LayerTree::find_annotation`.
#[derive(Clone)]
pub struct AnnotatedRegion<T: Any + Clone + 'static> {
    value: T,
    sized: bool,
    child: Widget,
}

impl<T: Any + Clone + 'static> AnnotatedRegion<T> {
    #[must_use]
    pub fn new(value: T, child: impl Into<Widget>) -> Self {
        Self {
            value,
            sized: true,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn sized(mut self, sized: bool) -> Self {
        self.sized = sized;
        self
    }
}

impl<T: Any + Clone + 'static> std::fmt::Debug for AnnotatedRegion<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AnnotatedRegion")
            .field("value_type", &std::any::TypeId::of::<T>())
            .field("sized", &self.sized)
            .field("child", &self.child)
            .finish()
    }
}

impl<T: Any + Clone + 'static> From<AnnotatedRegion<T>> for Widget {
    fn from(value: AnnotatedRegion<T>) -> Self {
        Widget::from_kind(WidgetKind::AnnotatedRegion {
            annotation: Annotation::new(value.value),
            sized: value.sized,
            child: value.child,
        })
    }
}

/// Identifies a target that followers can resolve across unrelated retained
/// subtrees. The link is intentionally shared and cheap to clone.
#[derive(Clone, Debug, PartialEq)]
pub struct CompositedTransformTarget {
    link: LayerLink,
    child: Widget,
}

impl CompositedTransformTarget {
    #[must_use]
    pub fn new(link: LayerLink, child: impl Into<Widget>) -> Self {
        Self {
            link,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn link(&self) -> LayerLink {
        self.link.clone()
    }
}

widget_from_kind!(CompositedTransformTarget => CompositedTransformTarget { link, child });

/// Positions a child relative to a [`CompositedTransformTarget`]. Anchors
/// use Flutter's normalized `Alignment` convention; an unlinked follower
/// keeps its normal layout position when `show_when_unlinked` is true.
#[derive(Clone, Debug, PartialEq)]
pub struct CompositedTransformFollower {
    link: LayerLink,
    show_when_unlinked: bool,
    offset: Offset,
    target_anchor: LayerAnchor,
    follower_anchor: LayerAnchor,
    child: Widget,
}

impl CompositedTransformFollower {
    #[must_use]
    pub fn new(link: LayerLink, child: impl Into<Widget>) -> Self {
        Self {
            link,
            show_when_unlinked: true,
            offset: Offset::ZERO,
            target_anchor: LayerAnchor::TOP_LEFT,
            follower_anchor: LayerAnchor::TOP_LEFT,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn show_when_unlinked(mut self, show: bool) -> Self {
        self.show_when_unlinked = show;
        self
    }

    #[must_use]
    pub fn offset(mut self, offset: Offset) -> Self {
        self.offset = Offset::finite_or_zero(offset);
        self
    }

    #[must_use]
    pub fn target_anchor(mut self, anchor: Alignment) -> Self {
        self.target_anchor = LayerAnchor::new(anchor.x, anchor.y);
        self
    }

    #[must_use]
    pub fn follower_anchor(mut self, anchor: Alignment) -> Self {
        self.follower_anchor = LayerAnchor::new(anchor.x, anchor.y);
        self
    }

    #[must_use]
    pub fn target_layer_anchor(mut self, anchor: LayerAnchor) -> Self {
        self.target_anchor = anchor;
        self
    }

    #[must_use]
    pub fn follower_layer_anchor(mut self, anchor: LayerAnchor) -> Self {
        self.follower_anchor = anchor;
        self
    }
}

widget_from_kind!(CompositedTransformFollower => CompositedTransformFollower {
    link,
    show_when_unlinked,
    offset,
    target_anchor,
    follower_anchor,
    child,
});
