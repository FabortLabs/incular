//! Declarative render configuration updates and their phase invalidation.

use super::{RenderFeatureState, RenderKind, RenderObjectPayload};

use incular_core::Invalidation;

impl RenderObjectPayload {
    /// Applies new declarative configuration and returns the minimum phase
    /// work required to make the retained object observable as that value.
    pub(crate) fn update_kind(&mut self, kind: RenderKind) -> Invalidation {
        if *self.kind == kind {
            return Invalidation::NONE;
        }
        if let (
            RenderKind::CustomLayout {
                delegate: crate::custom_layout::CustomLayoutDelegate::Flow(old),
            },
            RenderKind::CustomLayout {
                delegate: crate::custom_layout::CustomLayoutDelegate::Flow(new),
            },
        ) = (&*self.kind, &kind)
        {
            let invalidation = if new.needs_layout_from(old) {
                Invalidation::LAYOUT
                    | Invalidation::PAINT
                    | Invalidation::SEMANTICS
                    | Invalidation::HIT_TEST
            } else if new.needs_paint_from(old) {
                Invalidation::PAINT
            } else {
                Invalidation::NONE
            };
            self.replace_kind(kind);
            return invalidation;
        }
        if delegate_keeps_output(&self.kind, &kind) {
            // A replacement delegate can preserve layout or paint and
            // still receive the next update as the retained old delegate.
            self.replace_kind(kind);
            return Invalidation::NONE;
        }
        let invalidation = invalidation_for_change(&self.kind, &kind);
        if invalidation == Invalidation::PAINT
            && let (
                RenderKind::Text { spans: before, .. },
                RenderKind::Text { spans: Some(after), .. },
                RenderFeatureState::Text(state),
            ) = (&*self.kind, &kind, &mut self.feature)
            && before.as_deref() != Some(after.as_ref())
            && let Some(layout) = &mut state.layout
        {
            incular_text::TextLayout::refresh_span_foregrounds(layout, after);
        }
        self.replace_kind(kind);
        invalidation
    }
}

fn invalidation_for_change(old: &RenderKind, new: &RenderKind) -> Invalidation {
    use Invalidation as I;

    if image_composite_only_change(old, new) {
        return I::COMPOSITE;
    }
    if let (
        RenderKind::ShaderMask { shader: old_shader, has_child: had_child, .. },
        RenderKind::ShaderMask { shader: new_shader, has_child, .. },
    ) = (old, new)
    {
        if had_child != has_child {
            return I::LAYOUT | I::PAINT | I::SEMANTICS | I::HIT_TEST;
        }
        // Resolve a different application callback only when the mask
        // actually paints. Merely changing the blend keeps the old shader.
        return if old_shader == new_shader { I::COMPOSITE } else { I::PAINT };
    }
    if same_compositor_family(old, new) {
        // Geometry consumers read the current retained transform directly; they
        // do not require measurement or picture recording to observe movement.
        return if matches!(
            new,
            RenderKind::Transform { .. }
                | RenderKind::Scale { .. }
                | RenderKind::Affine { .. }
                | RenderKind::Rotation { .. }
                | RenderKind::Leader { .. }
                | RenderKind::Follower { .. }
        ) {
            I::COMPOSITE | I::SEMANTICS | I::HIT_TEST
        } else {
            I::COMPOSITE
        };
    }
    if paint_only_change(old, new) {
        return I::PAINT;
    }

    // Layout changes also invalidate local painting and semantic geometry.
    I::LAYOUT | I::PAINT | I::SEMANTICS | I::HIT_TEST
}

fn paint_only_change(old: &RenderKind, new: &RenderKind) -> bool {
    text_paint_only_change(old, new)
        || custom_paint_only_change(old, new)
        || visual_paint_only_change(old, new)
        || image_paint_only_change(old, new)
}

fn visual_paint_only_change(old: &RenderKind, new: &RenderKind) -> bool {
    match (old, new) {
        (RenderKind::Box { desired: a, .. }, RenderKind::Box { desired: b, .. })
        | (RenderKind::Shape { desired: a, .. }, RenderKind::Shape { desired: b, .. }) => a == b,
        (RenderKind::Colored { .. }, RenderKind::Colored { .. }) => true,
        (RenderKind::Decorated { desired: a, .. }, RenderKind::Decorated { desired: b, .. }) => {
            a == b
        }
        (
            RenderKind::Button {
                desired: old_size,
                enabled: old_enabled,
                focusable_when_disabled: old_focusable,
                ..
            },
            RenderKind::Button {
                desired: new_size,
                enabled: new_enabled,
                focusable_when_disabled: new_focusable,
                ..
            },
        ) => old_size == new_size && old_enabled == new_enabled && old_focusable == new_focusable,
        _ => false,
    }
}

fn image_composite_only_change(old: &RenderKind, new: &RenderKind) -> bool {
    let (
        RenderKind::Image {
            blend_mode: old_blend,
            source_effects: old_sources,
            ..
        },
        RenderKind::Image {
            blend_mode: new_blend,
            source_effects: new_sources,
            ..
        },
    ) = (old, new)
    else {
        return false;
    };
    if old_blend == new_blend && old_sources == new_sources {
        return false;
    }
    let mut normalized = new.clone();
    if let RenderKind::Image {
        blend_mode,
        source_effects,
        ..
    } = &mut normalized
    {
        *blend_mode = *old_blend;
        *source_effects = *old_sources;
    }
    *old == normalized
}

fn image_paint_only_change(old: &RenderKind, new: &RenderKind) -> bool {
    fn dimensions(image: &Option<incular_image::ImageHandle>) -> Option<(u32, u32)> {
        image.as_ref().map(|handle| {
            let decoded = handle.decoded();
            (decoded.width(), decoded.height())
        })
    }
    matches!(
        (old, new),
        (
            RenderKind::Image {
                image: old_image,
                width: old_width,
                height: old_height,
                scale: old_scale,
                ..
            },
            RenderKind::Image {
                image: new_image,
                width: new_width,
                height: new_height,
                scale: new_scale,
                ..
            }
        ) if old_width == new_width
            && old_height == new_height
            && old_scale == new_scale
            && (old_width.is_some() && old_height.is_some()
                || dimensions(old_image) == dimensions(new_image))
    )
}

fn text_paint_only_change(old: &RenderKind, new: &RenderKind) -> bool {
    let (
        RenderKind::Text {
            text: old_text,
            style: old_style,
            align: old_align,
            text_direction: old_direction,
            soft_wrap: old_soft_wrap,
            max_lines: old_max_lines,
            overflow: old_overflow,
            height_behavior: old_height_behavior,
            spans: old_spans,
            inline: old_inline,
            strut: old_strut,
        },
        RenderKind::Text {
            text: new_text,
            style: new_style,
            align: new_align,
            text_direction: new_direction,
            soft_wrap: new_soft_wrap,
            max_lines: new_max_lines,
            overflow: new_overflow,
            height_behavior: new_height_behavior,
            spans: new_spans,
            inline: new_inline,
            strut: new_strut,
        },
    ) = (old, new)
    else {
        return false;
    };
    old_text == new_text
        && old_align == new_align
        && old_direction == new_direction
        && old_soft_wrap == new_soft_wrap
        && old_max_lines == new_max_lines
        && old_overflow == new_overflow
        && old_height_behavior == new_height_behavior
        && (old_spans == new_spans
            || old_spans
                .as_deref()
                .zip(new_spans.as_deref())
                .is_some_and(|(before, after)| text_runs_change_only_foreground(before, after)))
        && old_inline == new_inline
        && old_strut == new_strut
        && old_style.change_impact(new_style) != incular_core::ChangeImpact::Layout
}

// Only foreground is detached from shaping. For example, a span's
// background rectangles are built during layout and must not go stale.
fn text_runs_change_only_foreground(
    before: &[incular_text::StyledTextRun],
    after: &[incular_text::StyledTextRun],
) -> bool {
    before.len() == after.len()
        && before.iter().zip(after).all(|(before, after)| {
            if before == after {
                return true;
            }
            if before.range != after.range || before.text != after.text {
                return false;
            }
            let mut baseline = before.style.clone();
            baseline.color = after.style.color;
            baseline.has_explicit_color = after.style.has_explicit_color;
            baseline.foreground = after.style.foreground;
            baseline == after.style
        })
}

fn custom_paint_only_change(old: &RenderKind, new: &RenderKind) -> bool {
    matches!(
        (old, new),
        (
            RenderKind::CustomPaint {
                desired: old_size,
                ..
            },
            RenderKind::CustomPaint {
                desired: new_size,
                ..
            }
        ) if old_size == new_size
    )
}

fn same_compositor_family(old: &RenderKind, new: &RenderKind) -> bool {
    if let (
        RenderKind::FittedBox {
            fit: old_fit,
            alignment: old_alignment,
            ..
        },
        RenderKind::FittedBox {
            fit: new_fit,
            alignment: new_alignment,
            ..
        },
    ) = (old, new)
    {
        return old_fit == new_fit && old_alignment == new_alignment;
    }
    // Stack alignment, fit, and text direction change measurement, so a
    // stack pair is compositor-only only for a pure behavior flip.
    if let (RenderKind::Stack { stack: old_stack }, RenderKind::Stack { stack: new_stack }) =
        (old, new)
    {
        return stack_clip_only_change(old_stack, new_stack);
    }
    if let (
        RenderKind::IndexedStack {
            alignment: old_alignment,
            index: old_index,
            fit: old_fit,
            clip_behavior: old_clip,
        },
        RenderKind::IndexedStack {
            alignment: new_alignment,
            index: new_index,
            fit: new_fit,
            clip_behavior: new_clip,
        },
    ) = (old, new)
    {
        return old_clip != new_clip
            && old_alignment == new_alignment
            && old_index == new_index
            && old_fit == new_fit;
    }
    matches!(
        (old, new),
        (RenderKind::Opacity { .. }, RenderKind::Opacity { .. })
            | (RenderKind::Blur { .. }, RenderKind::Blur { .. })
            | (RenderKind::DropShadow { .. }, RenderKind::DropShadow { .. })
            | (
                RenderKind::ColorFiltered { .. },
                RenderKind::ColorFiltered { .. }
            )
            | (RenderKind::Blend { .. }, RenderKind::Blend { .. })
            | (
                RenderKind::BackdropFilter { .. },
                RenderKind::BackdropFilter { .. }
            )
            | (
                RenderKind::AnnotatedRegion { .. },
                RenderKind::AnnotatedRegion { .. }
            )
            | (RenderKind::Leader { .. }, RenderKind::Leader { .. })
            | (RenderKind::Follower { .. }, RenderKind::Follower { .. })
            | (RenderKind::Transform { .. }, RenderKind::Transform { .. })
            | (RenderKind::Scale { .. }, RenderKind::Scale { .. })
            | (RenderKind::Affine { .. }, RenderKind::Affine { .. })
            | (RenderKind::Rotation { .. }, RenderKind::Rotation { .. })
            // Clip options never affect measurement: the shape resolves
            // from the retained size, so same-kind changes only need the
            // compositor layer rewrite in `update_from_kind`.
            | (RenderKind::ClipRect { .. }, RenderKind::ClipRect { .. })
            | (RenderKind::ClipRRect { .. }, RenderKind::ClipRRect { .. })
            | (RenderKind::ClipOval { .. }, RenderKind::ClipOval { .. })
            | (RenderKind::ClipPath { .. }, RenderKind::ClipPath { .. })
    )
}

/// True when two stack descriptors differ only in clipping: alignment
/// and fit both change measurement, so only a pure behavior flip may
/// skip layout.
fn stack_clip_only_change(old: &incular_layout::Stack, new: &incular_layout::Stack) -> bool {
    old.clip_behavior != new.clip_behavior && old.alignment == new.alignment && old.fit == new.fit
}

/// True when a replacement delegate reports no relayout, reclip or
/// repaint against the old delegate, and other phase inputs are unchanged.
fn delegate_keeps_output(old: &RenderKind, new: &RenderKind) -> bool {
    match (old, new) {
        (
            RenderKind::CustomLayout { delegate: old },
            RenderKind::CustomLayout { delegate: new },
        ) => !new.changes_from(old),
        (
            RenderKind::CustomPaint {
                desired: old_desired,
                display_list: old_list,
                painter: Some(old_painter),
            },
            RenderKind::CustomPaint {
                desired: new_desired,
                display_list: new_list,
                painter: Some(new_painter),
            },
        ) => {
            old_desired == new_desired
                && old_list == new_list
                && !new_painter.should_repaint_over(old_painter)
        }
        (
            RenderKind::ClipRect {
                clip_behavior: old_behavior,
                clipper: Some(old_clipper),
            },
            RenderKind::ClipRect {
                clip_behavior: new_behavior,
                clipper: Some(new_clipper),
            },
        ) => old_behavior == new_behavior && !new_clipper.reclips_from(old_clipper),
        (
            RenderKind::ClipRRect {
                radius: old_radius,
                text_direction: old_direction,
                clip_behavior: old_behavior,
                clipper: Some(old_clipper),
            },
            RenderKind::ClipRRect {
                radius: new_radius,
                text_direction: new_direction,
                clip_behavior: new_behavior,
                clipper: Some(new_clipper),
            },
        ) => {
            old_radius == new_radius
                && old_direction == new_direction
                && old_behavior == new_behavior
                && !new_clipper.reclips_from(old_clipper)
        }
        (
            RenderKind::ClipOval {
                clip_behavior: old_behavior,
                clipper: Some(old_clipper),
            },
            RenderKind::ClipOval {
                clip_behavior: new_behavior,
                clipper: Some(new_clipper),
            },
        ) => old_behavior == new_behavior && !new_clipper.reclips_from(old_clipper),
        (
            RenderKind::ClipPath {
                path: old_path,
                clip_behavior: old_behavior,
                clipper: Some(old_clipper),
            },
            RenderKind::ClipPath {
                path: new_path,
                clip_behavior: new_behavior,
                clipper: Some(new_clipper),
            },
        ) => {
            old_path == new_path
                && old_behavior == new_behavior
                && !new_clipper.reclips_from(old_clipper)
        }
        _ => false,
    }
}
