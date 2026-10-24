//! Parley-backed text shaping, fallback, bidi ordering, and line breaking.
//!
//! Incular deliberately keeps its renderer-facing glyph protocol small. Parley
//! owns all font discovery, fallback selection and shaping; this module only
//! translates its positioned glyph runs into `incular-rendering` commands.

use super::{FontFamily, FontStyle, TextAffinity, TextAlign, TextOverflow, TextStyle};
use icu_segmenter::GraphemeClusterSegmenter;
pub use incular_assets::FontId;
use incular_assets::{FontBytes, FontHandle};
use incular_config::TextDirection;
use incular_core::{Offset, Size};
use incular_rendering::{GlyphPosition, GlyphRun};
use parley::{
    Alignment, AlignmentOptions, BaseDirection, FontContext, FontWeight as ParleyFontWeight,
    Layout, LayoutContext, PositionedLayoutItem, StyleProperty,
};
mod document;
mod pool;
pub use document::TextDocumentLayout;
use pool::{DEFAULT_CACHE_BYTE_BUDGET, LAYOUT_CACHE_CAPACITY, LayoutPool};

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    sync::{Arc, OnceLock},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextMetrics {
    pub size: Size,
    pub baseline: f32,
    pub line_height: f32,
}

/// A shaped cluster's selectable source coverage, in text-area coordinates
/// with `left <= right`. Spans follow visual order, so a logical byte
/// range can cover several disjoint spans in mixed-direction text.
/// Consumers merge spans intersecting a selection instead of assuming
/// the range maps to one rectangle. Zero-advance clusters are kept because
/// they still own caret stops; selection ignores them.
///
/// Byte offsets are `u32`, like [`GlyphPosition::cluster`]: a line keeps
/// one span per cluster, and `usize` offsets made spans cost more than the
/// glyphs they cover.
#[derive(Clone, Copy, Debug)]
pub struct TextClusterSpan {
    pub start: u32,
    pub end: u32,
    pub left: f32,
    pub right: f32,
    /// Whether the cluster belongs to a right-to-left run.
    pub rtl: bool,
}

impl TextClusterSpan {
    /// The covered source bytes.
    #[must_use]
    pub const fn range(&self) -> std::ops::Range<usize> {
        self.start as usize..self.end as usize
    }
}

impl TextLine {
    /// Visual x-intervals covered by a half-open logical byte range on
    /// this line. Collapsed and reversed ranges yield nothing by
    /// policy; callers pass normalized ranges and never rely on
    /// rejection. Caret rendering stays separate: collapsed positions
    /// highlight nothing here and resolve through caret stops instead.
    ///
    /// Four granularities meet here and are not interchangeable: the
    /// range arrives in UTF-8 bytes (clamped to char boundaries
    /// upstream, so mid-char bytes never arrive); shaping clusters are
    /// the coverage granularity (a range splitting a cluster expands
    /// to the whole shaped cluster, since no visual edge exists
    /// inside it); visual order comes from the stored spans, never
    /// from re-sorting bytes or glyphs downstream. Touching spans
    /// merge; genuinely disjoint spans stay separate. An empty line
    /// covered by a nonempty range yields its caret point. Join controls
    /// are excluded from source coverage without adding caret stops inside
    /// the complete shaping cluster.
    #[must_use]
    pub fn selection_spans(&self, start: usize, end: usize) -> Vec<(f32, f32)> {
        if start >= end {
            return Vec::new();
        }
        let mut spans: Vec<(f32, f32)> = self
            .clusters
            .iter()
            .filter(|span| {
                let range = span.range();
                span.left < span.right && range.start < end && start < range.end
            })
            .map(|span| (span.left, span.right))
            .collect();
        if spans.is_empty() && self.start == self.end && start <= self.start && self.end <= end {
            spans.push((self.offset, self.offset));
        }
        spans.sort_by(|left, right| left.0.total_cmp(&right.0));
        let mut merged: Vec<(f32, f32)> = Vec::with_capacity(spans.len());
        for (left, right) in spans {
            if let Some(last) = merged.last_mut()
                && left <= last.1
            {
                last.1 = last.1.max(right);
            } else {
                merged.push((left, right));
            }
        }
        merged
    }
}

#[derive(Clone, Debug)]
pub struct TextLine {
    pub runs: Arc<[Arc<GlyphRun>]>,
    pub glyphs: Arc<[GlyphPosition]>,
    pub clusters: Arc<[TextClusterSpan]>,
    pub width: f32,
    /// Horizontal alignment offset in text-area coordinates. Glyph and
    /// caret x positions already include it; `width` stays the unshifted
    /// advance so extent checks keep working. Consumers mapping absolute
    /// x positions must add this to `width` instead of re-deriving the
    /// alignment.
    pub offset: f32,
    pub baseline: f32,
    pub start: usize,
    /// End of selectable/caret text on this visual line. This excludes a
    /// trailing hard line-break which has no shaped glyph of its own.
    pub caret_end: usize,
    pub end: usize,
}

#[derive(Clone, Debug)]
pub struct FontRunDebug {
    pub range: std::ops::Range<usize>,
    pub font: FontId,
    pub direction: &'static str,
}

/// A visual insertion point emitted by the shaping engine.
///
/// A logical byte offset can have two distinct visual positions at a bidi run
/// boundary. `affinity` keeps those positions separate, matching the concept
/// used by Parley's cursor implementation and preventing an arrow key from
/// jumping through a right-to-left run in logical UTF-8 order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextCaretPosition {
    pub offset: usize,
    pub affinity: TextAffinity,
    pub x: f32,
    pub line: usize,
}

#[derive(Clone, Debug)]
struct ClusterPosition {
    range: std::ops::Range<usize>,
    left: f32,
    right: f32,
    rtl: bool,
    ligature_start: bool,
    ligature_continuation: bool,
}

impl ClusterPosition {
    fn include(&mut self, other: &Self) {
        self.range.start = self.range.start.min(other.range.start);
        self.range.end = self.range.end.max(other.range.end);
        self.left = self.left.min(other.left);
        self.right = self.right.max(other.right);
        self.ligature_start |= other.ligature_start;
        self.ligature_continuation |= other.ligature_continuation;
    }

    fn can_join(&self, other: &Self) -> bool {
        self.rtl == other.rtl
            && if self.rtl {
                other.range.end == self.range.start
            } else {
                self.range.end == other.range.start
            }
    }
}

/// One lazily derived caret-stop list per laid-out line.
type LineCaretStops = Arc<[OnceLock<Arc<[TextCaretPosition]>>]>;

#[derive(Clone, Debug)]
pub struct TextLayout {
    pub lines: Arc<[TextLine]>,
    pub metrics: TextMetrics,
    pub font_runs: Arc<[FontRunDebug]>,
    /// Caret stops of every line, usually derived on first use. Layouts
    /// containing join controls retain the complete shaping boundaries
    /// before selection coverage excludes those controls.
    caret_positions: OnceLock<Arc<[TextCaretPosition]>>,
    /// Per-line caret stops derived on demand when `caret_positions` was not
    /// retained, so locating the caret in a long document derives one line
    /// instead of every stop of every line.
    line_carets: OnceLock<LineCaretStops>,
    /// Whether the source had content outside the retained layout's visible
    /// bounds. `Visible` layouts deliberately leave this false.
    pub overflowed: bool,
}

/// Paragraph-level layout controls. They deliberately describe behavior in
/// Incular terms and do not expose Parley builder types.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextLayoutOptions {
    pub max_width: Option<f32>,
    pub align: TextAlign,
    /// Explicit paragraph base direction; `None` retains automatic detection.
    pub text_direction: Option<TextDirection>,
    pub soft_wrap: bool,
    pub max_lines: Option<usize>,
    pub overflow: TextOverflow,
}

impl TextLayoutOptions {
    #[must_use]
    pub const fn new(max_width: Option<f32>, align: TextAlign) -> Self {
        Self {
            max_width,
            align,
            text_direction: None,
            soft_wrap: true,
            max_lines: None,
            overflow: TextOverflow::Clip,
        }
    }

    #[must_use]
    pub const fn soft_wrap(mut self, soft_wrap: bool) -> Self {
        self.soft_wrap = soft_wrap;
        self
    }

    #[must_use]
    pub const fn text_direction(mut self, direction: TextDirection) -> Self {
        self.text_direction = Some(direction);
        self
    }

    #[must_use]
    pub const fn max_lines(mut self, max_lines: Option<usize>) -> Self {
        self.max_lines = max_lines;
        self
    }

    #[must_use]
    pub const fn overflow(mut self, overflow: TextOverflow) -> Self {
        self.overflow = overflow;
        self
    }
}

impl Default for TextLayoutOptions {
    fn default() -> Self {
        Self::new(None, TextAlign::Start)
    }
}

impl TextLayout {
    #[must_use]
    pub fn glyph_count(&self) -> usize {
        self.lines.iter().map(|line| line.glyphs.len()).sum()
    }

    /// Returns the visual insertion points for one laid-out line in
    /// left-to-right screen order.
    #[must_use]
    pub fn line_caret_positions(&self, line: usize) -> &[TextCaretPosition] {
        if let Some(caret_positions) = self.caret_positions.get() {
            // Retained stops are grouped by ascending line.
            let start = caret_positions.partition_point(|position| position.line < line);
            let end = caret_positions.partition_point(|position| position.line <= line);
            return &caret_positions[start..end];
        }
        if line >= self.lines.len() {
            return &[];
        }
        let lines = self.line_carets.get_or_init(|| {
            (0..self.lines.len())
                .map(|_| OnceLock::new())
                .collect::<Vec<_>>()
                .into()
        });
        lines[line].get_or_init(|| {
            let mut positions = Vec::new();
            self.push_line_caret_positions(line, &mut positions);
            positions.into()
        })
    }

    fn derive_caret_positions(&self) -> Arc<[TextCaretPosition]> {
        let mut positions = Vec::new();
        for index in 0..self.lines.len() {
            self.push_line_caret_positions(index, &mut positions);
        }
        positions.into()
    }

    /// Two stops per cluster (logical start and end, placed by direction),
    /// plus the line's start and caret end when no cluster supplies them,
    /// sorted into screen order.
    fn push_line_caret_positions(&self, index: usize, positions: &mut Vec<TextCaretPosition>) {
        let line = &self.lines[index];
        let stop = |offset, affinity, x| TextCaretPosition {
            offset,
            affinity,
            x,
            line: index,
        };
        let first = positions.len();
        for cluster in line.clusters.iter() {
            let range = cluster.range();
            let (left, right) = if cluster.rtl {
                (range.end, range.start)
            } else {
                (range.start, range.end)
            };
            let (left_affinity, right_affinity) = if cluster.rtl {
                (TextAffinity::Upstream, TextAffinity::Downstream)
            } else {
                (TextAffinity::Downstream, TextAffinity::Upstream)
            };
            positions.push(stop(left, left_affinity, cluster.left));
            positions.push(stop(right, right_affinity, cluster.right));
        }
        let line_stops = &positions[first..];
        if !line_stops.iter().any(|stop| stop.offset == line.start) {
            positions.push(stop(line.start, TextAffinity::Downstream, line.offset));
        }
        if line.caret_end > line.start
            && !positions[first..]
                .iter()
                .any(|stop| stop.offset == line.caret_end)
        {
            positions.push(stop(
                line.caret_end,
                TextAffinity::Upstream,
                line.offset + line.width,
            ));
        }
        positions[first..].sort_by(|left, right| {
            left.x
                .total_cmp(&right.x)
                .then_with(|| left.offset.cmp(&right.offset))
        });
    }
}

/// Layout/cache counters. Legacy field names are retained for source
/// compatibility; Parley performs fallback and coverage resolution internally.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextDiagnostics {
    pub layouts_requested: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub font_resolution_requests: u64,
    pub font_resolution_cache_hits: u64,
    pub font_resolution_cache_misses: u64,
    pub coverage_queries: u64,
    pub fallback_runs: u64,
    pub fallback_fonts_used: u64,
    pub missing_clusters: u64,
    pub shaping_runs: u64,
    // Task 15 document-granularity instrumentation.
    /// Paragraphs examined by the document path.
    pub paragraphs_considered: u64,
    /// Paragraphs that required fresh shaping (cache misses at paragraph
    /// granularity).
    pub paragraphs_reshaped: u64,
    /// Parley layouts satisfied entirely from cache.
    pub parley_layouts_reused: u64,
    /// Multi-paragraph documents assembled from retained paragraphs.
    pub documents_composed: u64,
    /// Multi-paragraph documents returned whole from the cache.
    pub documents_reused: u64,
    /// Layouts served from a single-line layout shaped at another width.
    pub width_reuses: u64,
    /// Times a layout pool doubled because its working set did not fit.
    pub cache_growths: u64,
    /// Times a grown layout pool shrank back because its working set had
    /// become far smaller than its capacity.
    pub cache_shrinks: u64,
    /// Leading paragraphs carried unchanged from an earlier composition.
    pub paragraphs_carried: u64,
    /// Wrapping paragraphs re-broken at a new width without reshaping.
    pub shapes_reused: u64,
}

/// Identity of a retained layout. Lookups borrow the text, so a hit copies
/// nothing; only a pool insert takes an owned copy (`into_owned`).
#[derive(Hash, PartialEq, Eq)]
struct LayoutKey<'a> {
    text: Cow<'a, str>,
    family: FontFamily,
    fallbacks: Arc<[FontFamily]>,
    size: u32,
    weight: u16,
    style: FontStyle,
    line_height: Option<(u8, u32)>,
    letter_spacing: u32,
    font_variations: Option<Arc<str>>,
    font_features: Option<Arc<str>>,
    width: Option<u32>,
    align: TextAlign,
    text_direction: Option<TextDirection>,
    soft_wrap: bool,
    max_lines: Option<usize>,
    overflow: TextOverflow,
    generation: u64,
}

impl<'a> LayoutKey<'a> {
    fn new(text: &'a str, style: &TextStyle, options: TextLayoutOptions, generation: u64) -> Self {
        Self {
            text: Cow::Borrowed(text),
            family: style.family.clone(),
            fallbacks: style.fallback_families.clone(),
            size: style.size.to_bits(),
            weight: style.weight.0,
            style: style.style,
            line_height: style.line_height.map(|lh| match lh {
                crate::LineHeight::Normal => (0, 0),
                crate::LineHeight::Multiplier(m) => (1, m.to_bits()),
                crate::LineHeight::Absolute(a) => (2, a.to_bits()),
            }),
            letter_spacing: style.letter_spacing.to_bits(),
            font_variations: style.font_variations.clone(),
            font_features: style.font_features.clone(),
            width: options.max_width.map(f32::to_bits),
            align: options.align,
            text_direction: options.text_direction,
            soft_wrap: options.soft_wrap,
            max_lines: options.max_lines,
            overflow: options.overflow,
            generation,
        }
    }

    fn into_owned(self) -> LayoutKey<'static> {
        LayoutKey {
            text: Cow::Owned(self.text.into_owned()),
            family: self.family,
            fallbacks: self.fallbacks,
            size: self.size,
            weight: self.weight,
            style: self.style,
            line_height: self.line_height,
            letter_spacing: self.letter_spacing,
            font_variations: self.font_variations,
            font_features: self.font_features,
            width: self.width,
            align: self.align,
            text_direction: self.text_direction,
            soft_wrap: self.soft_wrap,
            max_lines: self.max_lines,
            overflow: self.overflow,
            generation: self.generation,
        }
    }
}

// Fonts every engine on this thread registers, in registration order.
thread_local! {
    static APPLICATION_FONTS: std::cell::RefCell<Vec<parley::fontique::Blob<u8>>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Registers a TrueType/OpenType font with every [`TextEngine`] on the
/// calling (UI) thread, including engines created earlier.
///
/// Families from these fonts also fill any generic family (`sans-serif`,
/// `serif`, `monospace`, ...) the platform leaves empty. Browser targets have
/// no system fonts, so a web application must register at least one font
/// before text can render; there every later font joins the generic families
/// too, as the fallback for characters the earlier ones lack. Native
/// platforms keep their system generics and fallback.
pub fn register_application_font(bytes: impl Into<Vec<u8>>) {
    let blob = parley::fontique::Blob::from(bytes.into());
    APPLICATION_FONTS.with(|fonts| fonts.borrow_mut().push(blob));
}

#[derive(Default)]
struct MissingCharacters {
    /// Every character reported so far; each is reported once.
    reported: HashSet<char>,
    pending: Vec<char>,
    listener: Option<std::rc::Rc<dyn Fn()>>,
}

thread_local! {
    static MISSING_CHARACTERS: std::cell::RefCell<MissingCharacters> =
        std::cell::RefCell::new(MissingCharacters::default());
}

/// Calls `listener` on the calling (UI) thread after a layout finds
/// characters no font could draw. The listener runs during layout: it should
/// only schedule work, then collect the characters with
/// [`take_missing_characters`]. Hosts without system fonts use this to load
/// a font that covers them and [`register_application_font`] it.
pub fn set_missing_characters_listener(listener: impl Fn() + 'static) {
    MISSING_CHARACTERS.with_borrow_mut(|missing| {
        missing.listener = Some(std::rc::Rc::new(listener));
    });
}

/// Returns the characters this thread's layouts drew with a missing glyph
/// since the last call. Each character is reported once per thread, even if
/// no font ever covers it.
#[must_use]
pub fn take_missing_characters() -> Vec<char> {
    MISSING_CHARACTERS.with_borrow_mut(|missing| std::mem::take(&mut missing.pending))
}

fn note_missing_characters(text: &str) {
    let listener = MISSING_CHARACTERS.with_borrow_mut(|missing| {
        let before = missing.pending.len();
        for character in text.chars() {
            // Layout controls and spaces have no glyph to load.
            if character.is_whitespace() || character.is_control() {
                continue;
            }
            if missing.reported.insert(character) {
                missing.pending.push(character);
            }
        }
        (missing.pending.len() > before)
            .then(|| missing.listener.clone())
            .flatten()
    });
    if let Some(listener) = listener {
        listener();
    }
}

/// The sole owner of font discovery and app-registered fonts. Parley owns the
/// actual font collection and resolves scripts, bidi direction and fallback.
pub struct TextEngine {
    /// One `FontHandle` per distinct font blob. Handles share the font bytes
    /// through a single `Arc`, so every cached text layout references the
    /// same allocation instead of holding a private copy of the whole font
    /// file (the Task 15 root cause of ~500 KB retained per shaped string
    /// and a full-file memcpy plus full-file hash per glyph run).
    font_handles: HashMap<(usize, usize, u32), FontHandle>,
    font_context: FontContext,
    layout_context: LayoutContext<()>,
    generation: u64,
    /// Count of thread application fonts already registered here.
    application_fonts: usize,
    cache: LayoutPool,
    /// Width-independent single-line layouts keyed without their width; see
    /// `shape_or_reuse`.
    unwrapped: LayoutPool,
    /// Shaped Parley layouts of paragraphs that wrapped, keyed without their
    /// width; see `shape_raw`.
    shaped: LayoutPool<Layout<()>>,
    /// Approximate bytes the three pools may retain together; see
    /// `set_cache_byte_budget`.
    cache_byte_budget: usize,
    /// Recent compositions, most recent first; see `carried_paragraphs`.
    documents: Vec<ComposedDocument>,
    /// Recent document layouts, most recent first; see `layout_paragraphs`.
    retained_documents: Vec<document::RetainedDocument>,
    diagnostics: TextDiagnostics,
}

/// Compositions retained for carrying paragraphs into the next edit: a few
/// fields or editor groups can be edited alternately.
const COMPOSED_DOCUMENTS: usize = 4;

/// The finished-layout pool's half of the engine's byte budget.
const fn cache_share(budget: usize) -> usize {
    budget / 2
}

/// The width-independent and shaped pools' quarter each of the budget.
const fn minor_share(budget: usize) -> usize {
    budget / 4
}

/// Composition state at the start of one paragraph.
#[derive(Clone, Copy)]
struct ParagraphStart {
    byte: usize,
    line: usize,
    font_run: usize,
    width: f32,
    height: f32,
    line_height: f32,
    overflowed: bool,
}

/// A composed document, kept so composing an edited version can carry over
/// the paragraphs before its first change instead of rebasing every line,
/// and so an unchanged document is returned whole.
struct ComposedDocument {
    /// Empty-text key carrying the style, paragraph options and generation.
    policy: LayoutKey<'static>,
    /// Requested overflow, which can clip `layout`.
    overflow: TextOverflow,
    text: String,
    /// Every composed line, before any clipping.
    lines: Arc<[TextLine]>,
    font_runs: Arc<[FontRunDebug]>,
    starts: Vec<ParagraphStart>,
    layout: Arc<TextLayout>,
}

impl Default for TextEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TextEngine {
    #[must_use]
    pub fn new() -> Self {
        Self {
            font_handles: HashMap::new(),
            font_context: FontContext::new(),
            layout_context: LayoutContext::new(),
            generation: 1,
            application_fonts: 0,
            cache: LayoutPool::new(cache_share(DEFAULT_CACHE_BYTE_BUDGET)),
            unwrapped: LayoutPool::new(minor_share(DEFAULT_CACHE_BYTE_BUDGET)),
            shaped: LayoutPool::new(minor_share(DEFAULT_CACHE_BYTE_BUDGET)),
            cache_byte_budget: DEFAULT_CACHE_BYTE_BUDGET,
            documents: Vec::new(),
            retained_documents: Vec::new(),
            diagnostics: TextDiagnostics::default(),
        }
    }

    /// Runs `f` with this thread's shared measuring engine.
    ///
    /// Application code that measures text while building or inside a
    /// `LayoutBuilder` should use this instead of `TextEngine::new()`: a new
    /// engine starts with an empty cache and reshapes every string on every
    /// rebuild, while the shared engine keeps layouts across frames. A nested
    /// call from inside `f` gets a temporary engine rather than panicking.
    pub fn with_shared<R>(f: impl FnOnce(&mut Self) -> R) -> R {
        thread_local! {
            static SHARED: std::cell::RefCell<TextEngine> =
                std::cell::RefCell::new(TextEngine::new());
        }
        SHARED.with(|shared| match shared.try_borrow_mut() {
            Ok(mut engine) => f(&mut engine),
            Err(_) => f(&mut Self::new()),
        })
    }

    #[must_use]
    pub const fn diagnostics(&self) -> TextDiagnostics {
        let mut diagnostics = self.diagnostics;
        diagnostics.cache_growths =
            self.cache.growths + self.unwrapped.growths + self.shaped.growths;
        diagnostics.cache_shrinks =
            self.cache.shrinks + self.unwrapped.shrinks + self.shaped.shrinks;
        diagnostics
    }

    /// Approximate bytes the layout pools may retain together. Defaults to
    /// 32 MiB.
    #[must_use]
    pub const fn cache_byte_budget(&self) -> usize {
        self.cache_byte_budget
    }

    /// Bounds the approximate bytes retained by the layout pools, evicting
    /// least recently used layouts beyond it. Half goes to finished layouts
    /// and a quarter each to width-independent lines and shaped wrapping
    /// paragraphs. The paragraphs of a document being composed are exempt,
    /// so a long file is never reshaped on each edit.
    pub fn set_cache_byte_budget(&mut self, bytes: usize) {
        self.cache_byte_budget = bytes;
        self.cache.set_byte_budget(cache_share(bytes));
        self.unwrapped.set_byte_budget(minor_share(bytes));
        self.shaped.set_byte_budget(minor_share(bytes));
    }

    /// Approximate bytes the layout pools retain now. Layouts shared between
    /// pools are counted by each, so this is an upper bound.
    #[must_use]
    pub const fn cache_bytes(&self) -> usize {
        self.cache.bytes() + self.unwrapped.bytes() + self.shaped.bytes()
    }

    /// Whether fonts were registered with [`register_application_font`]
    /// since this engine last laid out text. Retained layouts measured with
    /// the older fonts are stale.
    #[must_use]
    pub fn has_pending_application_fonts(&self) -> bool {
        APPLICATION_FONTS.with(|fonts| fonts.borrow().len() > self.application_fonts)
    }

    /// Registers thread application fonts added since this engine last laid
    /// out text, returning whether there were any.
    pub fn sync_application_fonts(&mut self) -> bool {
        let pending = APPLICATION_FONTS.with(|fonts| {
            fonts
                .borrow()
                .get(self.application_fonts..)
                .map(<[_]>::to_vec)
                .unwrap_or_default()
        });
        self.application_fonts += pending.len();
        let registered = !pending.is_empty();
        for blob in pending {
            self.register_font_blob(blob);
        }
        registered
    }

    /// Counts one layout request toward the pools' decay windows.
    fn tick_pools(&mut self) {
        self.cache.tick();
        self.unwrapped.tick();
        self.shaped.tick();
    }

    #[must_use]
    pub const fn font_database_generation(&self) -> u64 {
        self.generation
    }

    /// Registers all faces in a TrueType/OpenType byte buffer with Parley's
    /// Fontique collection. Invalid buffers are harmless: Fontique ignores
    /// them, while the generation still invalidates stale cached layouts.
    pub fn register_font_bytes(&mut self, bytes: impl Into<Vec<u8>>) {
        self.register_font_blob(parley::fontique::Blob::from(bytes.into()));
    }

    fn register_font_blob(&mut self, blob: parley::fontique::Blob<u8>) {
        use parley::fontique::GenericFamily;

        let collection = &mut self.font_context.collection;
        let families: Vec<_> = collection
            .register_fonts(blob, None)
            .into_iter()
            .map(|(family, _)| family)
            .collect();
        for generic in [
            GenericFamily::Serif,
            GenericFamily::SansSerif,
            GenericFamily::Monospace,
            GenericFamily::Cursive,
            GenericFamily::Fantasy,
            GenericFamily::SystemUi,
            GenericFamily::UiSerif,
            GenericFamily::UiSansSerif,
            GenericFamily::UiMonospace,
            GenericFamily::UiRounded,
            GenericFamily::Emoji,
            GenericFamily::Math,
            GenericFamily::FangSong,
        ] {
            if families.is_empty() {
                continue;
            }
            if collection.generic_families(generic).next().is_none() {
                collection.set_generic_families(generic, families.iter().copied());
            } else if cfg!(target_arch = "wasm32") {
                // A page has no system fallback: a later font, such as one
                // loaded for a script the first fonts lack, is only found
                // through the generic families.
                collection.append_generic_families(generic, families.iter().copied());
            }
        }
        self.generation = self
            .generation
            .checked_add(1)
            .expect("font database generation exhausted");
        self.cache.clear();
        self.unwrapped.clear();
        self.shaped.clear();
        self.documents.clear();
        self.retained_documents.clear();
    }

    pub fn layout(
        &mut self,
        text: &str,
        style: &TextStyle,
        max_width: Option<f32>,
        align: TextAlign,
    ) -> Arc<TextLayout> {
        self.layout_with_options(text, style, TextLayoutOptions::new(max_width, align))
    }

    /// Shapes a paragraph with retained wrapping and overflow policy.
    pub fn layout_with_options(
        &mut self,
        text: &str,
        style: &TextStyle,
        options: TextLayoutOptions,
    ) -> Arc<TextLayout> {
        self.diagnostics.layouts_requested += 1;
        self.sync_application_fonts();
        self.tick_pools();
        // Documents with hard line breaks compose from retained per-paragraph
        // layouts so editing one paragraph never reshapes the rest. Overflow
        // policies keep the exact monolithic semantics and stay on the
        // whole-string path.
        // With soft wrapping, `Clip` can only trigger on single-line
        // horizontal overflow, which a hard-break document never has; so this
        // route preserves monolithic semantics exactly. Without soft wrapping,
        // `Clip`/`Fade` against a fixed width only cut horizontal overflow to
        // the first line, which `layout_document` applies to the composition.
        let document = text.contains('\n')
            && options.max_lines.is_none()
            && (options.soft_wrap
                || options.overflow == TextOverflow::Visible
                || (options.max_width.is_some()
                    && matches!(options.overflow, TextOverflow::Clip | TextOverflow::Fade)));
        // Composed documents never enter the paragraph pool: each edited
        // version is a new key holding a full copy of the text and of every
        // line, so a pool sized for a long file's paragraphs retained
        // hundreds of stale versions. The few recent compositions kept for
        // prefix carry also return an unchanged document whole.
        if document {
            return self.layout_document(text, style, options);
        }
        let key = LayoutKey::new(text, style, options, self.generation);
        if let Some(layout) = self.cache.get(&key) {
            self.diagnostics.cache_hits += 1;
            return layout;
        }
        self.diagnostics.cache_misses += 1;
        let (layout, reused) = self.shape_or_reuse(text, style, options);
        // `unwrapped` already serves a reused layout at every width it fits;
        // retaining it under this width too copied each label's text on
        // every resize.
        if !reused {
            self.cache.put(key, layout.clone());
        }
        layout
    }

    /// Shapes `text`, or returns a layout shaped for the same text at another
    /// width when that layout is provably identical at this one.
    ///
    /// A single unclipped line whose alignment offset is zero is independent
    /// of the width it was shaped against, as long as it still fits: nothing
    /// wraps and start alignment in a left-to-right paragraph never moves it.
    /// Window resizes and flex measurement give labels new widths every
    /// frame; without this each such change reshaped every label.
    ///
    /// Also returns whether the layout was reused from another width.
    fn shape_or_reuse(
        &mut self,
        text: &str,
        style: &TextStyle,
        options: TextLayoutOptions,
    ) -> (Arc<TextLayout>, bool) {
        let key = LayoutKey::new(
            text,
            style,
            TextLayoutOptions {
                max_width: None,
                ..options
            },
            self.generation,
        );
        if let Some(layout) = self.unwrapped.get(&key)
            && options
                .max_width
                .is_none_or(|width| layout.metrics.size.width <= width)
        {
            self.diagnostics.width_reuses += 1;
            return (layout, true);
        }
        let layout = Arc::new(self.shape_and_wrap(text, style, options));
        if width_independent(&layout, options) {
            self.unwrapped.put(key, layout.clone());
        }
        (layout, false)
    }

    /// Grows the retained pool so `paragraphs` fit beside the base pool. A
    /// cyclic scan longer than an LRU's capacity evicts every entry before its
    /// reuse, which made each layout of a long file reshape all of it.
    fn reserve_document(&mut self, paragraphs: usize) {
        self.cache.pin(paragraphs);
        self.unwrapped.pin(paragraphs);
        let required = paragraphs.saturating_add(LAYOUT_CACHE_CAPACITY.get());
        if self.cache.capacity() < required {
            let capacity = required.next_power_of_two();
            self.cache.reserve(capacity);
            self.unwrapped.reserve(capacity);
        }
    }

    /// Composes a multi-paragraph document from per-paragraph retained
    /// layouts. Shaping reuse is exact (paragraph slice == cache key);
    /// assembly copies line data with vertical and byte-range rebasing.
    fn layout_document(
        &mut self,
        text: &str,
        style: &TextStyle,
        options: TextLayoutOptions,
    ) -> Arc<TextLayout> {
        let paragraph_options = TextLayoutOptions {
            max_lines: None,
            overflow: TextOverflow::Visible,
            ..options
        };
        let policy = LayoutKey::new("", style, paragraph_options, self.generation);
        // The router only sends `max_lines: None` here, so the policy and the
        // overflow determine the composition of a text.
        if let Some(index) = self.documents.iter().position(|document| {
            document.policy == policy
                && document.overflow == options.overflow
                && document.text == text
        }) {
            self.diagnostics.cache_hits += 1;
            self.diagnostics.documents_reused += 1;
            let document = self.documents.remove(index);
            let layout = document.layout.clone();
            self.documents.insert(0, document);
            return layout;
        }
        self.diagnostics.documents_composed += 1;
        self.reserve_document(text.bytes().filter(|byte| *byte == b'\n').count() + 1);
        let mut lines: Vec<TextLine> = Vec::new();
        let mut font_runs = Vec::new();
        let mut starts = Vec::new();
        let mut state = ParagraphStart {
            byte: 0,
            line: 0,
            font_run: 0,
            width: 0.,
            height: 0.,
            line_height: 0.,
            overflowed: false,
        };
        let mut document_carets = has_join_controls(text).then(Vec::new);
        // Lines before the first edited paragraph have the same byte offsets
        // and vertical position as last time, so they are carried as-is.
        if document_carets.is_none()
            && let Some((index, carried)) = self.carried_paragraphs(&policy, text)
        {
            let previous = &self.documents[index];
            state = previous.starts[carried];
            lines.extend_from_slice(&previous.lines[..state.line]);
            font_runs.extend_from_slice(&previous.font_runs[..state.font_run]);
            starts.extend_from_slice(&previous.starts[..carried]);
            self.diagnostics.paragraphs_carried += carried as u64;
        }
        let mut byte_start = state.byte;
        let mut width = state.width;
        let mut height = state.height;
        let mut line_height = state.line_height;
        let mut overflowed = state.overflowed;
        for paragraph in text[byte_start..].split('\n') {
            self.diagnostics.paragraphs_considered += 1;
            starts.push(ParagraphStart {
                byte: byte_start,
                line: lines.len(),
                font_run: font_runs.len(),
                width,
                height,
                line_height,
                overflowed,
            });
            let key = LayoutKey::new(paragraph, style, paragraph_options, self.generation);
            let paragraph_layout = if let Some(existing) = self.cache.get(&key) {
                self.diagnostics.parley_layouts_reused += 1;
                self.diagnostics.cache_hits += 1;
                existing
            } else {
                self.diagnostics.paragraphs_reshaped += 1;
                self.diagnostics.cache_misses += 1;
                let (built, _) = self.shape_or_reuse(paragraph, style, paragraph_options);
                self.cache.put(key, built.clone());
                built
            };
            if let Some(carets) = &mut document_carets {
                let line_start = lines.len();
                let paragraph_carets = paragraph_layout
                    .caret_positions
                    .get_or_init(|| paragraph_layout.derive_caret_positions());
                carets.extend(paragraph_carets.iter().map(|position| TextCaretPosition {
                    offset: position.offset + byte_start,
                    line: position.line + line_start,
                    ..*position
                }));
            }
            // Font-run metadata belongs to the paragraph, not each visual line.
            // Rebase it once even when soft wrapping creates multiple lines.
            for run in paragraph_layout.font_runs.iter() {
                let mut rebased = run.clone();
                rebased.range.start += byte_start;
                rebased.range.end += byte_start;
                font_runs.push(rebased);
            }
            for line in paragraph_layout.lines.iter() {
                // Exact-size iterators collect straight into `Arc<[T]>` with
                // one allocation; going through a `Vec` copied every line twice.
                let glyphs: Arc<[GlyphPosition]> = line
                    .glyphs
                    .iter()
                    .map(|glyph| {
                        let mut shifted = *glyph;
                        // Stored glyph y is negative parley space; moving the
                        // paragraph down by `height` subtracts from it.
                        shifted.offset.y -= height;
                        // Clusters index the full document string, so caret
                        // and selection mapping stay globally consistent.
                        shifted.cluster += byte_start as u32;
                        shifted
                    })
                    .collect();
                let shifted_runs: Arc<[Arc<GlyphRun>]> = line
                    .runs
                    .iter()
                    .map(|run| {
                        let mut shifted = (**run).clone();
                        shifted.origin.y += height;
                        Arc::new(shifted)
                    })
                    .collect();
                lines.push(TextLine {
                    runs: shifted_runs,
                    glyphs,
                    clusters: line
                        .clusters
                        .iter()
                        .map(|span| TextClusterSpan {
                            start: span.start + byte_start as u32,
                            end: span.end + byte_start as u32,
                            left: span.left,
                            right: span.right,
                            rtl: span.rtl,
                        })
                        .collect(),
                    width: line.width,
                    offset: line.offset,
                    baseline: line.baseline + height,
                    start: line.start + byte_start,
                    caret_end: line.caret_end + byte_start,
                    end: line.end + byte_start,
                });
            }
            width = width.max(paragraph_layout.metrics.size.width);
            height += paragraph_layout.metrics.size.height;
            line_height = paragraph_layout.metrics.line_height;
            overflowed |= paragraph_layout.overflowed;
            byte_start += paragraph.len() + 1; // include the hard break
        }
        if lines.is_empty() {
            lines.push(TextLine {
                runs: Arc::new([]),
                glyphs: Arc::new([]),
                clusters: Arc::new([]),
                width: 0.,
                offset: 0.,
                baseline: 0.,
                start: 0,
                caret_end: 0,
                end: 0,
            });
        }
        let baseline = lines[0].baseline;
        let lines: Arc<[TextLine]> = lines.into();
        let font_runs: Arc<[FontRunDebug]> = font_runs.into();
        let layout = TextLayout {
            lines: lines.clone(),
            metrics: TextMetrics {
                size: Size::new(width, height),
                baseline,
                line_height,
            },
            font_runs: font_runs.clone(),
            caret_positions: document_carets
                .map(|carets| OnceLock::from(Arc::from(carets)))
                .unwrap_or_default(),
            line_carets: OnceLock::new(),
            overflowed,
        };
        // Same horizontal-overflow policy as `shape_and_wrap` for unwrapped
        // text; the router only sends clipping overflows here.
        let horizontal_overflow = !options.soft_wrap
            && options.overflow != TextOverflow::Visible
            && options.max_width.is_some_and(|max| width > max);
        let layout = Arc::new(if horizontal_overflow {
            clipped_layout(layout, 1)
        } else {
            layout
        });
        self.documents.truncate(COMPOSED_DOCUMENTS - 1);
        self.documents.insert(
            0,
            ComposedDocument {
                policy,
                overflow: options.overflow,
                text: text.to_owned(),
                lines,
                font_runs,
                starts,
                layout: layout.clone(),
            },
        );
        layout
    }

    /// The retained composition sharing `policy` that carries the most
    /// leading paragraphs into `text`, and how many it carries. Paragraph
    /// `k` and everything before it are unchanged when the common byte
    /// prefix reaches the start of paragraph `k`.
    fn carried_paragraphs(&self, policy: &LayoutKey<'_>, text: &str) -> Option<(usize, usize)> {
        self.documents
            .iter()
            .enumerate()
            .filter(|(_, document)| document.policy == *policy)
            .map(|(index, document)| {
                let prefix = common_prefix_len(&document.text, text);
                let carried = document
                    .starts
                    .partition_point(|start| start.byte <= prefix)
                    .saturating_sub(1);
                (index, carried)
            })
            .filter(|(_, carried)| *carried > 0)
            .max_by_key(|(_, carried)| *carried)
    }

    fn font_stack(style: &TextStyle) -> parley::FontFamily<'static> {
        let mut names = vec![family_name(&style.family)];
        names.extend(style.fallback_families.iter().map(family_name));
        parley::FontFamily::Source(names.join(", ").into())
    }

    fn shape_and_wrap(
        &mut self,
        text: &str,
        style: &TextStyle,
        options: TextLayoutOptions,
    ) -> TextLayout {
        let wrap_width = options.soft_wrap.then_some(options.max_width).flatten();
        let layout = self.shape_raw(
            text,
            style,
            wrap_width,
            options.max_width,
            options.align,
            options.text_direction,
        );
        let horizontal_overflow = !options.soft_wrap
            && options
                .max_width
                .is_some_and(|width| layout.metrics.size.width > width);
        let line_limit = options
            .max_lines
            .or_else(|| horizontal_overflow.then_some(1));
        let line_overflow = line_limit.is_some_and(|limit| layout.lines.len() > limit);
        if (!horizontal_overflow && !line_overflow) || options.overflow == TextOverflow::Visible {
            return layout;
        }

        let limit = line_limit.unwrap_or(layout.lines.len());
        match options.overflow {
            TextOverflow::Visible => layout,
            TextOverflow::Clip | TextOverflow::Fade => clipped_layout(layout, limit),
            TextOverflow::Ellipsis => self.ellipsized_layout(text, style, options, limit),
        }
    }

    fn ellipsized_layout(
        &mut self,
        text: &str,
        style: &TextStyle,
        options: TextLayoutOptions,
        limit: usize,
    ) -> TextLayout {
        if limit == 0 {
            return clipped_layout(
                self.shape_raw(
                    "",
                    style,
                    None,
                    options.max_width,
                    options.align,
                    options.text_direction,
                ),
                0,
            );
        }
        let wrap_width = options.soft_wrap.then_some(options.max_width).flatten();
        let initial = self.shape_raw(
            text,
            style,
            wrap_width,
            options.max_width,
            options.align,
            options.text_direction,
        );
        let end = initial
            .lines
            .get(limit.saturating_sub(1))
            .map_or(0, |line| line.caret_end.min(text.len()));
        let mut end = grapheme_boundary_at_or_before(text, end);

        loop {
            let mut candidate = text[..end].to_owned();
            candidate.push('\u{2026}');
            // The ellipsis always enters Parley as source text and is shaped
            // with the selected/fallback font stack, never appended as a raw
            // renderer glyph or a byte-truncated source suffix.
            let shaped = self.shape_raw(
                &candidate,
                style,
                wrap_width,
                options.max_width,
                options.align,
                options.text_direction,
            );
            let fits_width = options.max_width.is_none_or(|width| {
                shaped
                    .lines
                    .iter()
                    .all(|line| line.width <= width + f32::EPSILON)
            });
            if shaped.lines.len() <= limit && fits_width {
                return TextLayout {
                    overflowed: true,
                    ..shaped
                };
            }
            let previous = previous_grapheme_boundary(text, end);
            if previous == end {
                return TextLayout {
                    overflowed: true,
                    ..shaped
                };
            }
            end = previous;
        }
    }

    /// Shapes `text` into an unbroken Parley layout.
    fn shape_paragraph(
        &mut self,
        text: &str,
        style: &TextStyle,
        max_width: Option<f32>,
        text_direction: Option<TextDirection>,
    ) -> Layout<()> {
        let mut builder =
            self.layout_context
                .ranged_builder(&mut self.font_context, text, 1.0, true);
        builder.set_base_direction(match text_direction {
            Some(TextDirection::Ltr) => BaseDirection::Ltr,
            Some(TextDirection::Rtl) => BaseDirection::Rtl,
            None => BaseDirection::Auto,
        });
        // Ranged builders retain the source text; this restores the previous
        // hanging-space policy without collapsing any source characters.
        builder.push_default(StyleProperty::WhiteSpaceCollapse(
            parley::WhiteSpaceCollapse::Collapse,
        ));
        if max_width.is_none() {
            builder.push_default(StyleProperty::TextWrapMode(parley::TextWrapMode::NoWrap));
        }
        builder.push_default(StyleProperty::FontFamily(Self::font_stack(style)));
        builder.push_default(StyleProperty::FontSize(style.size.max(0.0)));
        builder.push_default(StyleProperty::FontWeight(ParleyFontWeight::new(
            style.weight.0 as f32,
        )));
        builder.push_default(StyleProperty::FontStyle(match style.style {
            FontStyle::Normal => parley::FontStyle::Normal,
            FontStyle::Italic => parley::FontStyle::Italic,
        }));
        builder.push_default(StyleProperty::LetterSpacing(style.letter_spacing));
        if let Some(settings) = &style.font_variations {
            builder.push_default(StyleProperty::FontVariations(
                parley::FontVariations::Source(Cow::Owned(settings.to_string())),
            ));
        }
        if let Some(settings) = &style.font_features {
            builder.push_default(StyleProperty::FontFeatures(parley::FontFeatures::Source(
                Cow::Owned(settings.to_string()),
            )));
        }
        if let Some(line_height) = style.line_height
            && let Some(parley_lh) = line_height.to_parley(style.size)
        {
            builder.push_default(StyleProperty::LineHeight(parley_lh));
        }

        builder.build(text)
    }

    fn shape_raw(
        &mut self,
        text: &str,
        style: &TextStyle,
        max_width: Option<f32>,
        align_width: Option<f32>,
        align: TextAlign,
        text_direction: Option<TextDirection>,
    ) -> TextLayout {
        // Shaping does not depend on the wrap width; only line breaking and
        // alignment do. A wrapping paragraph keeps its shaped layout so a new
        // width re-breaks it instead of shaping it again.
        let shaping_key = LayoutKey::new(
            text,
            style,
            TextLayoutOptions {
                max_width: max_width.map(|_| 1.),
                align: TextAlign::Start,
                text_direction,
                soft_wrap: true,
                max_lines: None,
                overflow: TextOverflow::Visible,
            },
            self.generation,
        );
        let mut layout = match self.shaped.take(&shaping_key) {
            Some(layout) => {
                self.diagnostics.shapes_reused += 1;
                layout
            }
            None => self.shape_paragraph(text, style, max_width, text_direction),
        };
        layout.break_all_lines(align_width.or(max_width));
        // Alignment resolves against the measured width even when the text
        // does not wrap: single-line content otherwise ignores alignment
        // because the wrap width is empty.
        layout.align(
            match align {
                TextAlign::Start => Alignment::Start,
                TextAlign::Center => Alignment::Center,
                TextAlign::End => Alignment::End,
                TextAlign::Justify => Alignment::Justify,
                TextAlign::Left => Alignment::Left,
                TextAlign::Right => Alignment::Right,
            },
            AlignmentOptions::default(),
        );

        let mut translated = self.translate_layout(&layout, text);
        if text.is_empty() {
            // Parley uses an internal space to obtain empty-paragraph font
            // metrics. That space must not become a visible advance or move
            // the editor's caret away from the empty aligned paragraph edge.
            let width = align_width.unwrap_or(0.0).max(0.0);
            let offset = match (align, layout.is_rtl()) {
                (TextAlign::Right, _)
                | (TextAlign::Start | TextAlign::Justify, true)
                | (TextAlign::End, false) => width,
                (TextAlign::Center, _) => width * 0.5,
                _ => 0.0,
            };
            for line in Arc::make_mut(&mut translated.lines) {
                line.runs = Arc::new([]);
                line.glyphs = Arc::new([]);
                line.clusters = Arc::new([]);
                line.width = 0.0;
                line.offset = offset;
            }
            translated.metrics.size.width = 0.0;
        }
        if layout.len() > 1 {
            self.shaped.put(shaping_key, layout);
        }
        translated
    }

    fn font_handle_for(&mut self, font: &parley::FontData) -> FontHandle {
        font_handle(self, font)
    }

    fn translate_layout(&mut self, layout: &Layout<()>, text: &str) -> TextLayout {
        let mut lines = Vec::new();
        let mut font_runs = Vec::new();
        let mut used_fonts = HashSet::new();
        // First glyph run decides the primary font; resolve its handle before
        // the mutable translation loop so borrowck stays trivial.
        let primary_handle = layout
            .lines()
            .flat_map(|line| line.runs())
            .next()
            .map(|run| self.font_handle_for(run.font()));
        let primary = primary_handle.as_ref().map(FontHandle::id);

        for line in layout.lines() {
            let metrics = line.metrics();
            let range = line.text_range();
            let mut runs: Vec<Arc<GlyphRun>> = Vec::new();
            let mut glyphs = Vec::new();
            let mut caret_end = range.start;
            // Walk Parley's visual cluster advances once for the whole line.
            // This is the same cell geometry Parley uses for hit testing and
            // accounts for per-cluster spacing and justified advances. A
            // source cluster can be split across font/style glyph runs, so
            // merge duplicate byte ranges into one logical span and caret
            // pair here rather than deriving cells separately per glyph run.
            let mut cluster_positions: Vec<ClusterPosition> = Vec::new();
            let mut cluster_position_index: HashMap<(usize, usize), usize> = HashMap::new();
            let mut cluster_x = metrics.offset;
            for run in line.runs() {
                let rtl = run.is_rtl();
                for cluster in run.visual_clusters() {
                    let cluster_range = cluster.text_range();
                    // Glyph 0 is the font's "missing glyph" box.
                    if cluster.glyphs().any(|glyph| glyph.id == 0)
                        && let Some(source) = text.get(cluster_range.clone())
                    {
                        note_missing_characters(source);
                    }
                    // Invisible continuation clusters still own source bytes.
                    // Keep the logical line endpoint through their ranges so
                    // a final ligature component does not lose its caret stop.
                    caret_end = caret_end.max(cluster_range.end);
                    let edge = cluster_x + cluster.advance();
                    let ligature_start = cluster.is_ligature_start();
                    let ligature_continuation = cluster.is_ligature_continuation();
                    let mut cluster_glyphs = cluster.glyphs();
                    let Some(first_glyph) = cluster_glyphs.next() else {
                        // A shaping cluster can carry advance for an
                        // invisible continuation. Ligature components remain
                        // part of the start cluster's selectable range, while
                        // standalone join controls have no visible selection
                        // geometry of their own.
                        let is_join_control =
                            text.get(cluster_range.clone()).is_some_and(|component| {
                                !component.is_empty()
                                    && component.chars().all(|character| {
                                        matches!(character, '\u{200C}' | '\u{200D}')
                                    })
                            });
                        if ligature_continuation && !is_join_control {
                            let position = ClusterPosition {
                                range: cluster_range.clone(),
                                left: cluster_x.min(edge),
                                right: cluster_x.max(edge),
                                rtl,
                                ligature_start,
                                ligature_continuation,
                            };
                            let key = (cluster_range.start, cluster_range.end);
                            if let Some(index) = cluster_position_index.get(&key).copied() {
                                cluster_positions[index].include(&position);
                            } else {
                                cluster_position_index.insert(key, cluster_positions.len());
                                cluster_positions.push(position);
                            }
                        }
                        cluster_x = edge;
                        continue;
                    };
                    let glyph_advance = std::iter::once(first_glyph)
                        .chain(cluster_glyphs)
                        .map(|glyph| glyph.advance)
                        .sum::<f32>();
                    // Ligature components can share a larger glyph advance
                    // than their divided cluster advances; retain the glyph's
                    // full cell so no caret or selection edge lands inside
                    // that ligature. Justification and letter spacing can
                    // also grow the cluster cell beyond its raw glyphs.
                    let visible_edge = cluster_x + cluster.advance().max(glyph_advance);
                    let left = cluster_x.min(visible_edge);
                    let right = cluster_x.max(visible_edge);
                    let position = ClusterPosition {
                        range: cluster_range.clone(),
                        left,
                        right,
                        rtl,
                        ligature_start,
                        ligature_continuation,
                    };
                    let key = (cluster_range.start, cluster_range.end);
                    if let Some(index) = cluster_position_index.get(&key).copied() {
                        cluster_positions[index].include(&position);
                    } else {
                        cluster_position_index.insert(key, cluster_positions.len());
                        cluster_positions.push(position);
                    }
                    cluster_x = edge;
                }
            }
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(parley_run) = item else {
                    continue;
                };
                let run = parley_run.run();
                let font = self.font_handle_for(run.font());
                let font_id = font.id();
                if Some(font_id) != primary {
                    self.diagnostics.fallback_runs += 1;
                }
                used_fonts.insert(font_id);
                self.diagnostics.shaping_runs += 1;

                // `PositionedLayoutItem::GlyphRun` is a line-local slice of
                // the underlying shaping run. Parley's renderer-facing Glyph
                // intentionally has no byte index, so recover each glyph's
                // cluster from the authoritative visual-cluster sequence
                // before translating it. This keeps caret movement and
                // selection on shaped cluster boundaries instead of assigning
                // every glyph to the run's first byte.
                //
                // Order and cardinality correspond structurally, not by
                // length coincidence: `glyphs()` and `positioned_glyphs()`
                // both iterate `Run::visual_clusters().flat_map(glyphs)`,
                // the latter only adding running x (which already carries
                // the line offset). The window below re-locates this
                // style-uniform slice within that same sequence, so the
                // zip pairs each positioned glyph with its own cluster
                // for RTL runs, ligatures, and multi-glyph clusters alike.
                let visible_glyphs = parley_run.glyphs().collect::<Vec<_>>();
                let all_clustered_glyphs = run
                    .visual_clusters()
                    .flat_map(|cluster| {
                        let cluster_range = cluster.text_range();
                        cluster
                            .glyphs()
                            .map(move |glyph| (glyph, cluster_range.clone()))
                    })
                    .collect::<Vec<_>>();
                let first_glyph = all_clustered_glyphs
                    .windows(visible_glyphs.len())
                    .position(|candidate| {
                        candidate
                            .iter()
                            .zip(&visible_glyphs)
                            .all(|((glyph, _), visible)| glyph == visible)
                    })
                    .unwrap_or(0);
                let clusters = all_clustered_glyphs
                    .iter()
                    .skip(first_glyph)
                    .map(|(_, range)| range.clone())
                    .take(visible_glyphs.len())
                    .collect::<Vec<_>>();
                let positions: Vec<_> = parley_run
                    .positioned_glyphs()
                    .zip(
                        clusters
                            .iter()
                            .map(|range| range.start as u32)
                            .chain(std::iter::repeat(run.text_range().start as u32)),
                    )
                    .map(|(glyph, cluster)| GlyphPosition {
                        id: glyph.id as u16,
                        // Incular's renderer uses `origin.y - offset.y`; Parley
                        // exposes glyph y from its top-left layout coordinate.
                        // Parley seeds positioned glyph x with the line
                        // alignment offset, so adding it again would translate
                        // aligned text twice.
                        offset: Offset::new(glyph.x, -glyph.y),
                        advance: glyph.advance,
                        cluster,
                    })
                    .collect();
                if runs.len() == 1 {
                    glyphs.extend(runs[0].glyphs.iter().copied());
                }
                if !runs.is_empty() {
                    glyphs.extend(positions.iter().copied());
                }
                runs.push(Arc::new(GlyphRun {
                    font,
                    font_size: run.font_size(),
                    origin: Offset::ZERO,
                    glyphs: positions.into(),
                }));
                font_runs.push(FontRunDebug {
                    range: run.text_range(),
                    font: font_id,
                    direction: if run.is_rtl() { "rtl" } else { "ltr" },
                });
            }
            // Every merged cluster, including zero-advance ones that only
            // carry caret stops; the layout derives its stops from these.
            let cluster_spans: Vec<_> = merge_ligature_components(cluster_positions)
                .iter()
                .map(|position| TextClusterSpan {
                    start: position.range.start as u32,
                    end: position.range.end as u32,
                    left: position.left,
                    right: position.right,
                    rtl: position.rtl,
                })
                .collect();
            // The common one-run line has exactly the same positioned glyphs
            // for caret handling and painting. Share the immutable allocation
            // instead of retaining a second copy in TextLine.
            let glyphs = if runs.len() == 1 {
                runs[0].glyphs.clone()
            } else {
                glyphs.into()
            };
            lines.push(TextLine {
                runs: runs.into(),
                glyphs,
                clusters: cluster_spans.into(),
                width: metrics.advance,
                offset: metrics.offset,
                baseline: metrics.baseline,
                start: range.start,
                caret_end,
                end: range.end,
            });
        }

        if lines.is_empty() {
            lines.push(TextLine {
                runs: Arc::new([]),
                glyphs: Arc::new([]),
                clusters: Arc::new([]),
                width: 0.0,
                offset: 0.0,
                baseline: 0.0,
                start: 0,
                caret_end: 0,
                end: 0,
            });
        }
        self.diagnostics.fallback_fonts_used += used_fonts.len().saturating_sub(1) as u64;
        let baseline = lines[0].baseline;
        let line_height = layout
            .lines()
            .next()
            .map_or(0.0, |line| line.metrics().line_height);
        let mut translated = TextLayout {
            lines: lines.into(),
            metrics: TextMetrics {
                size: Size::new(layout.width(), layout.height()),
                baseline,
                line_height,
            },
            font_runs: font_runs.into(),
            caret_positions: OnceLock::new(),
            line_carets: OnceLock::new(),
            overflowed: false,
        };
        if has_join_controls(text) {
            // Carets keep Parley's complete grapheme boundaries. Selection
            // coverage separately excludes invisible format controls, even
            // when Parley attaches them to a visible grapheme.
            translated
                .caret_positions
                .get_or_init(|| translated.derive_caret_positions());
            for line in Arc::make_mut(&mut translated.lines) {
                let mut visible = Vec::with_capacity(line.clusters.len());
                for span in line.clusters.iter() {
                    let Some(component) = text.get(span.range()) else {
                        visible.push(*span);
                        continue;
                    };
                    let mut start = span.start;
                    for (offset, character) in component.char_indices() {
                        if matches!(character, '\u{200C}' | '\u{200D}') {
                            let end = span.start + offset as u32;
                            if start < end {
                                visible.push(TextClusterSpan {
                                    start,
                                    end,
                                    ..*span
                                });
                            }
                            start = end + character.len_utf8() as u32;
                        }
                    }
                    if start < span.end {
                        visible.push(TextClusterSpan { start, ..*span });
                    }
                }
                line.clusters = visible.into();
            }
        }
        translated
    }
}

fn has_join_controls(text: &str) -> bool {
    text.contains('\u{200C}') || text.contains('\u{200D}')
}

/// Coalesces Parley's glyphless ligature components with their rendered
/// cluster. Visual cluster iteration is forward for LTR runs and reversed for
/// RTL runs, so the component sequence starts at opposite ends of a ligature
/// in each direction.
fn merge_ligature_components(positions: Vec<ClusterPosition>) -> Vec<ClusterPosition> {
    let mut merged = Vec::with_capacity(positions.len());
    let mut pending: Option<ClusterPosition> = None;
    let mut pending_has_start = false;

    for position in positions {
        let is_start = position.ligature_start;
        let is_continuation = position.ligature_continuation;
        if !is_start && !is_continuation {
            if let Some(pending) = pending.take() {
                merged.push(pending);
            }
            pending_has_start = false;
            merged.push(position);
            continue;
        }
        // A single source cluster can already cover the whole ligature.
        if is_start && is_continuation {
            if let Some(pending) = pending.take() {
                merged.push(pending);
            }
            pending_has_start = false;
            merged.push(position);
            continue;
        }

        if position.rtl {
            if is_continuation {
                // In visual RTL order, continuations precede their start. A
                // continuation after a completed group begins the next one.
                if pending_has_start {
                    if let Some(pending) = pending.take() {
                        merged.push(pending);
                    }
                    pending_has_start = false;
                }
                let can_extend = pending
                    .as_ref()
                    .is_some_and(|pending| pending.can_join(&position));
                if can_extend {
                    pending
                        .as_mut()
                        .expect("checked pending ligature group")
                        .include(&position);
                } else {
                    if let Some(pending) = pending.take() {
                        merged.push(pending);
                    }
                    pending_has_start = false;
                    pending = Some(position);
                }
            } else {
                // The start completes the pending continuation group.
                match pending.take() {
                    Some(mut pending_group)
                        if !pending_has_start && pending_group.can_join(&position) =>
                    {
                        pending_group.include(&position);
                        merged.push(pending_group);
                    }
                    Some(pending_group) => {
                        merged.push(pending_group);
                        merged.push(position);
                    }
                    None => merged.push(position),
                }
                pending_has_start = false;
            }
        } else if is_start {
            if let Some(pending) = pending.take() {
                merged.push(pending);
            }
            pending = Some(position);
            pending_has_start = true;
        } else if pending.is_some() {
            // LTR starts precede their continuations in visual order. The
            // byte check prevents an excluded join control from being
            // bridged by a later glyphless continuation.
            if pending
                .as_ref()
                .is_some_and(|pending_group| pending_group.can_join(&position))
            {
                pending
                    .as_mut()
                    .expect("checked pending ligature group")
                    .include(&position);
            } else {
                let pending_group = pending.take().expect("pending ligature group");
                merged.push(pending_group);
                pending = Some(position);
                pending_has_start = false;
            }
        } else {
            // A run can begin in the middle of a shaped ligature if a prior
            // fallback/style run owns its start.
            pending = Some(position);
            pending_has_start = false;
        }
    }

    if let Some(pending) = pending {
        merged.push(pending);
    }
    merged
}

/// Length in bytes of the longest common prefix of `left` and `right`.
fn common_prefix_len(left: &str, right: &str) -> usize {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    let mut prefix = 0;
    for (a, b) in left.chunks(64).zip(right.chunks(64)) {
        if a == b {
            prefix += a.len();
        } else {
            return prefix + a.iter().zip(b).take_while(|(x, y)| x == y).count();
        }
    }
    prefix
}

/// Whether `layout` lays out identically at every width it fits: one
/// unclipped line, start-aligned in a left-to-right paragraph, unshifted.
fn width_independent(layout: &TextLayout, options: TextLayoutOptions) -> bool {
    matches!(
        options.align,
        TextAlign::Start | TextAlign::Left | TextAlign::Justify
    ) && options.text_direction != Some(TextDirection::Rtl)
        && !layout.overflowed
        && layout.lines.len() == 1
        && layout.lines[0].offset == 0.0
        && layout.font_runs.iter().all(|run| run.direction != "rtl")
}

fn clipped_layout(mut layout: TextLayout, limit: usize) -> TextLayout {
    let lines = layout.lines.iter().take(limit).cloned().collect::<Vec<_>>();
    if let Some(carets) = layout.caret_positions.get_mut() {
        let retained = carets.partition_point(|position| position.line < lines.len());
        if retained < carets.len() {
            *carets = Arc::from(&carets[..retained]);
        }
    }
    let width = lines
        .iter()
        .fold(0.0_f32, |width, line| width.max(line.width));
    layout.metrics.size = Size::new(width, layout.metrics.line_height * lines.len() as f32);
    layout.metrics.baseline = lines.first().map_or(0.0, |line| line.baseline);
    layout.lines = lines.into();
    layout.line_carets = OnceLock::new();
    layout.overflowed = true;
    layout
}

fn grapheme_boundary_at_or_before(text: &str, offset: usize) -> usize {
    GraphemeClusterSegmenter::new()
        .segment_str(text)
        .take_while(|boundary| *boundary <= offset)
        .last()
        .unwrap_or(0)
}

fn previous_grapheme_boundary(text: &str, offset: usize) -> usize {
    GraphemeClusterSegmenter::new()
        .segment_str(text)
        .take_while(|boundary| *boundary < offset)
        .last()
        .unwrap_or(0)
}

fn family_name(family: &FontFamily) -> String {
    match family {
        FontFamily::SansSerif => "sans-serif".into(),
        FontFamily::Serif => "serif".into(),
        FontFamily::Monospace => "monospace".into(),
        FontFamily::Cursive => "cursive".into(),
        FontFamily::SystemUi => "system-ui".into(),
        FontFamily::Named(name) => format!("'{name}'"),
    }
}

/// Identity for one font blob sighting. Pointer identity is stable because
/// `Blob` clones share the same backing allocation, so repeated sightings of
/// the same face never re-copy or re-hash the bytes.
type FontBlobKey = (usize, usize, u32);

fn font_blob_key(font: &parley::FontData) -> FontBlobKey {
    (
        font.data.as_ref().as_ptr() as usize,
        font.data.as_ref().len(),
        font.index,
    )
}

/// Returns the shared handle for this font blob, building it (one full-file
/// hash, no copy) on first sight only.
fn font_handle(engine: &mut TextEngine, font: &parley::FontData) -> FontHandle {
    let key = font_blob_key(font);
    if let Some(handle) = engine.font_handles.get(&key) {
        return handle.clone();
    }
    let mut hasher = DefaultHasher::new();
    font.data.as_ref().hash(&mut hasher);
    hasher.write_u64(u64::from(font.index));
    let id = FontId(hasher.finish());
    // Share discovery's (typically memory-mapped) source instead of copying
    // the whole font file into a private heap allocation.
    let handle = FontHandle::with_face_index(
        id,
        FontBytes::from_shared(font.data.clone().into_raw_parts().0),
        font.index,
    );
    engine.font_handles.insert(key, handle.clone());
    handle
}
