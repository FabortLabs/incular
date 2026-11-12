use crate::compositor::LayerId;
use crate::effects::{BlendMode, ColorFilter, DropShadowEffect, GaussianBlur};
use crate::geometry::RRect;
use crate::glyphs::GlyphRun;
use crate::gradients::Brush;
use crate::paint::{Border, FillRule, Paint, PaintStyle, Stroke};
use crate::paths::Path;
use incular_core::{Color, Rect, Size, Transform};
use incular_image::ImageHandle;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

/// Flutter's `BackdropKey`: backdrop filters sharing a key in one frame
/// sample the backdrop once, as it is when the first of them is reached,
/// instead of each sampling what the filters before it drew. A backend may
/// reuse that one capture for all of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BackdropKey(u64);

impl BackdropKey {
    /// A key no other call returns.
    #[must_use]
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for BackdropKey {
    fn default() -> Self {
        Self::new()
    }
}

/// Renderer-neutral identity for a detachable composition subtree.
///
/// The retained widget layer maps its generational transient-surface identity
/// onto this value. Renderers treat partition markers as no-ops unless a host
/// explicitly detaches the marked commands for presentation on another
/// surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SurfacePartitionId {
    index: u32,
    generation: u32,
}

impl SurfacePartitionId {
    #[must_use]
    pub const fn from_parts(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }

    #[must_use]
    pub const fn generation(self) -> u32 {
        self.generation
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PaintCommand {
    /// Begins a retained subtree that may be detached to another presentation
    /// surface. The marker itself has no visual effect.
    PushSurfacePartition {
        id: SurfacePartitionId,
    },
    /// Ends the most recent surface partition.
    PopSurfacePartition,
    Rect {
        rect: Rect,
        color: Color,
    },
    RRect {
        rrect: RRect,
        brush: Brush,
    },
    Border {
        rrect: RRect,
        border: Border,
    },
    FillPath {
        path: Arc<Path>,
        brush: Brush,
        fill_rule: FillRule,
    },
    StrokePath {
        path: Arc<Path>,
        brush: Brush,
        stroke: Stroke,
    },
    /// Source is image pixels; destination is local logical geometry.
    Image {
        image: ImageHandle,
        source: Rect,
        destination: Rect,
        sampling: ImageSampling,
    },
    GlyphRun {
        run: Arc<GlyphRun>,
        color: Color,
    },
    PushClip {
        rect: Rect,
    },
    PushClipRRect {
        rrect: RRect,
    },
    PushClipOval {
        rect: Rect,
    },
    PushClipPath {
        path: Arc<Path>,
        fill_rule: FillRule,
    },
    PopClip,
    PushTransform {
        transform: Transform,
    },
    PopTransform,
    /// Begins an isolated retained opacity group. The compositor renders the
    /// enclosed commands into a transparent target and applies `alpha` once
    /// when that target is composited back into its parent.
    PushOpacity {
        layer: LayerId,
        alpha: f32,
        generation: u64,
        bounds: Rect,
    },
    PopOpacity,
    /// Begins an isolated Gaussian-filtered subtree. The generation and
    /// bounds describe source pixels only; sigma is a compositor property.
    PushBlur {
        layer: LayerId,
        blur: GaussianBlur,
        generation: u64,
        bounds: Rect,
    },
    /// Begins an isolated arbitrary-subtree drop shadow. The child source is
    /// emitted once and the compositor draws the blurred alpha behind it.
    PushDropShadow {
        layer: LayerId,
        shadow: DropShadowEffect,
        generation: u64,
        bounds: Rect,
    },
    /// Begins an isolated straight-RGBA color-matrix stage.
    PushColorFilter {
        layer: LayerId,
        filter: ColorFilter,
        generation: u64,
        bounds: Rect,
    },
    /// Begins an isolated blend group. The source is emitted once and the
    /// compositor combines it with the destination in painter order.
    PushBlend {
        layer: LayerId,
        mode: BlendMode,
        generation: u64,
        bounds: Rect,
    },
    /// Begins an isolated shader-mask stage. The shader is evaluated in the
    /// mask's logical coordinate space and blended over the child source.
    PushShaderMask {
        layer: LayerId,
        shader: Brush,
        blend_mode: BlendMode,
        mask_size: Size,
        mask_transform: Transform,
        generation: u64,
        bounds: Rect,
    },
    /// Begins a backdrop stage. The backend samples the already painted
    /// destination in `bounds`, filters it, and then composites the filtered
    /// pixels before the child source is painted.
    PushBackdropFilter {
        layer: LayerId,
        blur: GaussianBlur,
        blend_mode: BlendMode,
        enabled: bool,
        /// Flutter's `ImageFilter.matrix`: when set, the backdrop is
        /// transformed by it, in the stage's coordinates, instead of blurred.
        matrix: Option<Transform>,
        /// The backdrop this stage shares with others, if any.
        backdrop_key: Option<BackdropKey>,
        generation: u64,
        bounds: Rect,
    },
    PopEffect,
}
/// Renderer-neutral raster filtering policy. Ordinary UI images default to
/// linear sampling; pixel art can opt into nearest-neighbour sampling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ImageSampling {
    #[default]
    Linear,
    Nearest,
}
/// Recorded paint commands. Cloning is a reference-count bump; the command
/// buffer is copied only when a shared list is mutated.
#[derive(Clone, Default)]
pub struct DisplayList {
    commands: Option<std::sync::Arc<Vec<PaintCommand>>>,
}
impl PartialEq for DisplayList {
    fn eq(&self, other: &Self) -> bool {
        self.same_commands(other)
    }
}
impl PaintCommand {
    /// Equality that recognizes shared glyph runs and paths by identity
    /// before comparing their contents. Retained scenes hand the same
    /// reference-counted runs out frame after frame, so the usual
    /// comparison never walks a glyph or a path segment.
    fn same_as(&self, other: &Self) -> bool {
        fn shared<T: PartialEq>(a: &Arc<T>, b: &Arc<T>) -> bool {
            Arc::ptr_eq(a, b) || a == b
        }
        match (self, other) {
            (Self::GlyphRun { run: a, color: ca }, Self::GlyphRun { run: b, color: cb }) => {
                ca == cb && shared(a, b)
            }
            (
                Self::FillPath {
                    path: a,
                    brush: ba,
                    fill_rule: ra,
                },
                Self::FillPath {
                    path: b,
                    brush: bb,
                    fill_rule: rb,
                },
            ) => ra == rb && ba == bb && shared(a, b),
            (
                Self::StrokePath {
                    path: a,
                    brush: ba,
                    stroke: sa,
                },
                Self::StrokePath {
                    path: b,
                    brush: bb,
                    stroke: sb,
                },
            ) => sa == sb && ba == bb && shared(a, b),
            (
                Self::PushClipPath {
                    path: a,
                    fill_rule: ra,
                },
                Self::PushClipPath {
                    path: b,
                    fill_rule: rb,
                },
            ) => ra == rb && shared(a, b),
            _ => self == other,
        }
    }
}
impl std::fmt::Debug for DisplayList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DisplayList")
            .field("commands", &self.commands())
            .finish()
    }
}
impl DisplayList {
    #[must_use]
    pub const fn new() -> Self {
        Self { commands: None }
    }
    fn commands_mut(&mut self) -> &mut Vec<PaintCommand> {
        std::sync::Arc::make_mut(
            self.commands
                .get_or_insert_with(|| std::sync::Arc::new(Vec::new())),
        )
    }
    /// Whether both lists record the same commands. Lists sharing one
    /// buffer are equal without being read.
    pub(crate) fn same_commands(&self, other: &Self) -> bool {
        if self.shares_buffer(other) {
            return true;
        }
        let (a, b) = (self.commands(), other.commands());
        a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.same_as(b))
    }
    /// Whether both lists are views of one command buffer: clones of each
    /// other that neither has recorded into since. Two empty lists share
    /// nothing and report `false`.
    #[must_use]
    pub fn shares_buffer(&self, other: &Self) -> bool {
        matches!((&self.commands, &other.commands), (Some(a), Some(b)) if Arc::ptr_eq(a, b))
    }
    /// Makes room for `total` commands in one allocation, for a recording
    /// whose size is known from the previous one.
    pub(crate) fn reserve_total(&mut self, total: usize) {
        let commands = self.commands_mut();
        commands.reserve_exact(total.saturating_sub(commands.len()));
    }
    pub fn push(&mut self, command: PaintCommand) {
        let commands = self.commands_mut();
        if commands.len() == commands.capacity() {
            // Retained pictures usually hold one or two commands. Doubling
            // from one slot avoids Vec's four-command (576-byte) minimum.
            commands.reserve_exact(commands.len().max(1));
        }
        commands.push(command);
    }
    pub fn extend_from(&mut self, other: &Self) {
        if other.is_empty() {
            return;
        }
        self.commands_mut().extend(other.commands().iter().cloned());
    }

    /// Splits selected retained surface partitions out of this display list.
    ///
    /// Commands inside an active selected partition are omitted from `parent`
    /// and returned under that partition's stable identity. Nested selected
    /// partitions are assigned to the deepest selected surface, so a submenu
    /// can be detached independently from its owning menu. Partition markers
    /// never reach either output.
    #[must_use]
    pub fn detach_surface_partitions(
        &self,
        selected: &std::collections::HashSet<SurfacePartitionId>,
    ) -> (Self, std::collections::HashMap<SurfacePartitionId, Self>) {
        let mut parent = Self::new();
        let mut detached = std::collections::HashMap::<SurfacePartitionId, Self>::new();
        let mut stack = Vec::<SurfacePartitionId>::new();
        for command in self.commands() {
            match command {
                PaintCommand::PushSurfacePartition { id } => stack.push(*id),
                PaintCommand::PopSurfacePartition => {
                    let _ = stack.pop();
                }
                command => {
                    let target = stack.iter().rev().copied().find(|id| selected.contains(id));
                    if let Some(id) = target {
                        detached.entry(id).or_default().push(command.clone());
                    } else {
                        parent.push(command.clone());
                    }
                }
            }
        }
        (parent, detached)
    }

    /// Moves every retained surface partition (an open popup, tooltip or
    /// menu) after the rest of the list, as Flutter's `Overlay` paints its
    /// entries above the routes beneath it: a popup is never covered by the
    /// widgets painted after its anchor.
    ///
    /// A lifted partition keeps the transforms that enclosed it, so it stays
    /// where it was placed, and leaves the clips and effects of its
    /// ancestors behind, so an ancestor's clip does not cut it off. Its
    /// markers are kept, so [`Self::detach_surface_partitions`] can still
    /// move it to a native surface. Nested partitions are lifted within
    /// their parent. A list without partitions is returned unchanged.
    #[must_use]
    pub fn lift_surface_partitions(self) -> Self {
        if !self
            .commands()
            .iter()
            .any(|command| matches!(command, PaintCommand::PushSurfacePartition { .. }))
        {
            return self;
        }
        Self::lift_partitions_in(self.commands())
    }

    fn lift_partitions_in(commands: &[PaintCommand]) -> Self {
        let mut base = Self::new();
        let mut lifted = Vec::<Self>::new();
        let mut transforms = Vec::<Transform>::new();
        let mut index = 0;
        while index < commands.len() {
            match &commands[index] {
                PaintCommand::PushSurfacePartition { .. } => {
                    // The partition runs to its matching pop.
                    let start = index;
                    let mut depth = 0usize;
                    while index < commands.len() {
                        match commands[index] {
                            PaintCommand::PushSurfacePartition { .. } => depth += 1,
                            PaintCommand::PopSurfacePartition => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                        index += 1;
                    }
                    let end = index.min(commands.len() - 1);
                    let mut entry = Self::new();
                    for transform in &transforms {
                        entry.push(PaintCommand::PushTransform {
                            transform: *transform,
                        });
                    }
                    entry.push(commands[start].clone());
                    entry.extend_from(&Self::lift_partitions_in(&commands[start + 1..end]));
                    if matches!(commands[end], PaintCommand::PopSurfacePartition) && end > start {
                        entry.push(PaintCommand::PopSurfacePartition);
                    }
                    for _ in &transforms {
                        entry.push(PaintCommand::PopTransform);
                    }
                    lifted.push(entry);
                }
                command => {
                    match command {
                        PaintCommand::PushTransform { transform } => transforms.push(*transform),
                        PaintCommand::PopTransform => {
                            let _ = transforms.pop();
                        }
                        _ => {}
                    }
                    base.push(command.clone());
                }
            }
            index += 1;
        }
        for entry in &lifted {
            base.extend_from(entry);
        }
        base
    }

    /// Wraps this list in one logical translation without mutating retained
    /// command geometry.
    #[must_use]
    pub fn translated(&self, offset: incular_core::Offset) -> Self {
        if self.is_empty() || offset == incular_core::Offset::ZERO {
            return self.clone();
        }
        let mut output = Self::new();
        output.push(PaintCommand::PushTransform {
            transform: Transform::translation(offset),
        });
        output.extend_from(self);
        output.push(PaintCommand::PopTransform);
        output
    }

    /// Conservatively proves that the first visual operation covers the entire
    /// target with an opaque axis-aligned rectangle.
    ///
    /// This is intentionally stricter than a general coverage analysis. A
    /// backend may use it before presenting through an opaque native surface;
    /// returning `false` means "not proven" and must never be interpreted as
    /// transparency being unnecessary.
    #[must_use]
    pub fn begins_with_opaque_surface_rect(&self, surface_size: Size) -> bool {
        if !surface_size.width.is_finite()
            || !surface_size.height.is_finite()
            || surface_size.width <= 0.0
            || surface_size.height <= 0.0
        {
            return false;
        }
        let mut transforms = vec![Transform::IDENTITY];
        for command in self.commands() {
            match command {
                PaintCommand::PushTransform { transform } => {
                    let next = transforms
                        .last()
                        .copied()
                        .expect("root transform")
                        .then(*transform);
                    if !next.is_translation() {
                        return false;
                    }
                    transforms.push(next);
                }
                PaintCommand::PopTransform => {
                    if transforms.len() <= 1 {
                        return false;
                    }
                    transforms.pop();
                }
                PaintCommand::PushSurfacePartition { .. } | PaintCommand::PopSurfacePartition => {}
                PaintCommand::Rect { rect, color } => {
                    if color.alpha != u8::MAX {
                        return false;
                    }
                    let world = transforms
                        .last()
                        .expect("root transform")
                        .transform_rect_bbox(*rect);
                    const EPSILON: f32 = 0.01;
                    return world.origin.x <= EPSILON
                        && world.origin.y <= EPSILON
                        && world.origin.x + world.size.width >= surface_size.width - EPSILON
                        && world.origin.y + world.size.height >= surface_size.height - EPSILON;
                }
                // Any clip, effect, or visual primitive before a full opaque
                // rectangle makes complete pixel coverage impossible to prove
                // with this intentionally cheap predicate.
                _ => return false,
            }
        }
        false
    }
    #[must_use]
    pub fn commands(&self) -> &[PaintCommand] {
        self.commands.as_deref().map_or(&[], Vec::as_slice)
    }
    #[must_use]
    pub fn len(&self) -> usize {
        self.commands().len()
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands().is_empty()
    }
    /// Removes every command, keeping the allocation for the next recording.
    pub fn clear(&mut self) {
        if let Some(commands) = self.commands.as_mut() {
            match std::sync::Arc::get_mut(commands) {
                Some(unique) => unique.clear(),
                None => self.commands = None,
            }
        }
    }
}
#[derive(Default)]
pub struct Canvas {
    list: DisplayList,
    saves: Vec<SaveKind>,
}
#[derive(Clone, Copy)]
enum SaveKind {
    Transform,
    Clip,
}
impl Canvas {
    pub fn rect(&mut self, rect: Rect, color: Color) {
        self.list.push(PaintCommand::Rect { rect, color });
    }
    pub fn rrect(&mut self, rrect: RRect, brush: impl Into<Brush>) {
        self.list.push(PaintCommand::RRect {
            rrect,
            brush: brush.into(),
        });
    }
    pub fn border(&mut self, rrect: RRect, border: Border) {
        self.list.push(PaintCommand::Border { rrect, border });
    }
    pub fn fill_path(&mut self, path: Arc<Path>, brush: impl Into<Brush>, fill_rule: FillRule) {
        self.list.push(PaintCommand::FillPath {
            path,
            brush: brush.into(),
            fill_rule,
        });
    }
    pub fn stroke_path(&mut self, path: Arc<Path>, brush: impl Into<Brush>, stroke: Stroke) {
        self.list.push(PaintCommand::StrokePath {
            path,
            brush: brush.into(),
            stroke,
        });
    }
    /// Lowers a [`Paint`] to the existing renderer-neutral path command.
    ///
    /// Blend and color-filter metadata remain available on the paint for a
    /// compositor/backend that needs them; the retained path geometry itself
    /// is never duplicated into a second command representation.
    pub fn draw_path(&mut self, path: Arc<Path>, paint: &Paint, fill_rule: FillRule) {
        match paint.style_value() {
            PaintStyle::Fill => self.fill_path(path, paint.brush_value().clone(), fill_rule),
            PaintStyle::Stroke => {
                self.stroke_path(path, paint.brush_value().clone(), paint.stroke_value())
            }
        }
    }
    pub fn image(&mut self, image: ImageHandle, source: Rect, destination: Rect) {
        self.image_with_sampling(image, source, destination, ImageSampling::Linear);
    }
    pub fn image_with_sampling(
        &mut self,
        image: ImageHandle,
        source: Rect,
        destination: Rect,
        sampling: ImageSampling,
    ) {
        self.list.push(PaintCommand::Image {
            image,
            source,
            destination,
            sampling,
        });
    }
    pub fn glyph_run(&mut self, run: Arc<GlyphRun>, color: Color) {
        self.list.push(PaintCommand::GlyphRun { run, color });
    }
    pub fn save_transform(&mut self, transform: Transform) {
        self.saves.push(SaveKind::Transform);
        self.list.push(PaintCommand::PushTransform { transform });
    }
    pub fn save_clip(&mut self, rect: Rect) {
        self.saves.push(SaveKind::Clip);
        self.list.push(PaintCommand::PushClip { rect });
    }
    pub fn save_clip_rrect(&mut self, rrect: RRect) {
        self.saves.push(SaveKind::Clip);
        self.list.push(PaintCommand::PushClipRRect { rrect });
    }
    pub fn save_clip_oval(&mut self, rect: Rect) {
        self.saves.push(SaveKind::Clip);
        self.list.push(PaintCommand::PushClipOval { rect });
    }
    pub fn save_clip_path(&mut self, path: Arc<Path>, fill_rule: FillRule) {
        self.saves.push(SaveKind::Clip);
        self.list
            .push(PaintCommand::PushClipPath { path, fill_rule });
    }
    pub fn restore(&mut self) {
        match self.saves.pop().expect("unbalanced canvas restore") {
            SaveKind::Transform => self.list.push(PaintCommand::PopTransform),
            SaveKind::Clip => self.list.push(PaintCommand::PopClip),
        }
    }
    #[must_use]
    pub fn finish(self) -> DisplayList {
        assert!(self.saves.is_empty(), "unbalanced display list");
        self.list
    }
}
