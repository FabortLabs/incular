//! Documents laid out paragraph by paragraph, for editors of long text.

use super::{LayoutKey, LineCaretStops, TextEngine, TextLayoutOptions, common_prefix_len};
use crate::geometry::{ParagraphSlot, TextLineRef, TextLines};
use crate::{TextCaretPosition, TextGeometry, TextMetrics, TextOverflow, TextStyle};
use incular_core::Size;
use std::sync::{Arc, OnceLock};

/// Text laid out one hard-broken paragraph at a time.
///
/// Each paragraph keeps its own [`TextLayout`](crate::TextLayout), and the document records
/// where each one starts. Laying out an edited version of a document reuses
/// every paragraph before and after the edit: those after it only move, so
/// an edit near the top of a long document costs about as much as one at
/// its end. A [`TextLayout`](crate::TextLayout) of the same text instead stores every line in
/// document coordinates, like Flutter's paragraph, and so rewrites every
/// line after an edit.
///
/// Geometry is read through [`TextGeometry`]; it matches a [`TextLayout`](crate::TextLayout) of
/// the same text laid out with soft wrapping and visible overflow.
#[derive(Debug)]
pub struct TextDocumentLayout {
    paragraphs: Arc<[ParagraphSlot]>,
    line_count: usize,
    pub metrics: TextMetrics,
    /// Caret stops in document coordinates, derived per line on first use.
    line_carets: OnceLock<LineCaretStops>,
}

impl TextDocumentLayout {
    fn new(paragraphs: Arc<[ParagraphSlot]>) -> Self {
        let first = &paragraphs[0];
        let last = &paragraphs[paragraphs.len() - 1];
        let metrics = TextMetrics {
            size: Size::new(
                paragraphs.iter().fold(0f32, |width, slot| {
                    width.max(slot.layout.metrics.size.width)
                }),
                last.y + last.layout.metrics.size.height,
            ),
            baseline: first.y + first.layout.metrics.baseline,
            line_height: last.layout.metrics.line_height,
        };
        let line_count = paragraphs.iter().map(|slot| slot.layout.lines.len()).sum();
        Self {
            paragraphs,
            line_count,
            metrics,
            line_carets: OnceLock::new(),
        }
    }

    /// Number of hard-broken paragraphs; text without a line break has one.
    #[must_use]
    pub fn paragraph_count(&self) -> usize {
        self.paragraphs.len()
    }

    #[must_use]
    pub fn glyph_count(&self) -> usize {
        self.paragraphs
            .iter()
            .map(|slot| slot.layout.glyph_count())
            .sum()
    }

    fn slot_for_line(&self, line: usize) -> Option<&ParagraphSlot> {
        let slot = self
            .paragraphs
            .partition_point(|slot| slot.first_line <= line)
            .checked_sub(1)?;
        Some(&self.paragraphs[slot])
    }
}

impl TextGeometry for TextDocumentLayout {
    fn metrics(&self) -> TextMetrics {
        self.metrics
    }

    fn line_count(&self) -> usize {
        self.line_count
    }

    fn get_line(&self, index: usize) -> Option<TextLineRef<'_>> {
        if index >= self.line_count {
            return None;
        }
        let slot = self.slot_for_line(index)?;
        let line = slot.layout.lines.get(index - slot.first_line)?;
        Some(TextLineRef::new(line, index, slot.byte, slot.y))
    }

    fn lines(&self) -> TextLines<'_> {
        TextLines::paragraphs(&self.paragraphs)
    }

    fn lines_between(&self, top: f32, bottom: f32) -> TextLines<'_> {
        let first = self
            .paragraphs
            .partition_point(|slot| slot.y <= top)
            .saturating_sub(1);
        let end = self
            .paragraphs
            .partition_point(|slot| slot.y < bottom)
            .max(first);
        TextLines::paragraphs(&self.paragraphs[first..end])
    }

    fn line_for_offset(&self, byte: usize) -> usize {
        let last = self.line_count.saturating_sub(1);
        let Some(slot) = self
            .paragraphs
            .partition_point(|slot| slot.byte <= byte)
            .checked_sub(1)
        else {
            return last;
        };
        let slot = &self.paragraphs[slot];
        let local = byte - slot.byte;
        let lines = &slot.layout.lines;
        match lines
            .partition_point(|line| line.start <= local)
            .checked_sub(1)
        {
            Some(line) if local < lines[line].end => slot.first_line + line,
            _ => last,
        }
    }

    fn line_caret_positions(&self, line: usize) -> &[TextCaretPosition] {
        let Some(slot) = self.slot_for_line(line).filter(|_| line < self.line_count) else {
            return &[];
        };
        let lines = self.line_carets.get_or_init(|| {
            (0..self.line_count)
                .map(|_| OnceLock::new())
                .collect::<Vec<_>>()
                .into()
        });
        lines[line].get_or_init(|| {
            slot.layout
                .line_caret_positions(line - slot.first_line)
                .iter()
                .map(|position| TextCaretPosition {
                    offset: position.offset + slot.byte,
                    line,
                    ..*position
                })
                .collect()
        })
    }
}

/// A document layout kept so laying out an edited version can carry over the
/// paragraphs around its change.
pub(super) struct RetainedDocument {
    /// Empty-text key carrying the style, paragraph options and generation.
    policy: LayoutKey<'static>,
    text: String,
    layout: Arc<TextDocumentLayout>,
}

/// Document layouts retained for carrying paragraphs into the next edit: a
/// few editors can be edited alternately.
const RETAINED_DOCUMENTS: usize = 4;

impl TextEngine {
    /// Lays out `text` paragraph by paragraph; see [`TextDocumentLayout`].
    ///
    /// Paragraphs are laid out with `options` and visible overflow:
    /// `max_lines` and `overflow` apply to whole-text layouts only.
    pub fn layout_paragraphs(
        &mut self,
        text: &str,
        style: &TextStyle,
        options: TextLayoutOptions,
    ) -> Arc<TextDocumentLayout> {
        self.diagnostics.layouts_requested += 1;
        self.tick_pools();
        let paragraph_options = TextLayoutOptions {
            max_lines: None,
            overflow: TextOverflow::Visible,
            ..options
        };
        let policy = LayoutKey::new("", style, paragraph_options, self.generation);
        let carried = self
            .retained_documents
            .iter()
            .enumerate()
            .filter(|(_, document)| document.policy == policy)
            .map(|(index, document)| {
                let (prefix, suffix) =
                    carried_paragraphs(&document.layout.paragraphs, &document.text, text);
                let count = document.layout.paragraphs.len();
                (index, prefix, suffix, prefix + count - suffix)
            })
            .filter(|(.., carried)| *carried > 0)
            .max_by_key(|(.., carried)| *carried);
        if let Some((index, ..)) = carried
            && self.retained_documents[index].text == text
        {
            self.diagnostics.cache_hits += 1;
            self.diagnostics.documents_reused += 1;
            let document = self.retained_documents.remove(index);
            let layout = document.layout.clone();
            self.retained_documents.insert(0, document);
            return layout;
        }
        self.diagnostics.documents_composed += 1;
        let paragraph_count = text.bytes().filter(|byte| *byte == b'\n').count() + 1;
        self.reserve_document(paragraph_count);
        let mut slots: Vec<ParagraphSlot> = Vec::with_capacity(paragraph_count);
        let (mut byte_start, mut height, mut line_count) = (0usize, 0f32, 0usize);
        let mut suffix = None;
        if let Some((index, prefix, first_suffix, _)) = carried {
            let document = &self.retained_documents[index];
            let previous = document.layout.paragraphs.clone();
            slots.extend_from_slice(&previous[..prefix]);
            let next = &previous[prefix];
            (byte_start, height, line_count) = (next.byte, next.y, next.first_line);
            self.diagnostics.paragraphs_carried += prefix as u64;
            if first_suffix < previous.len() {
                let start = previous[first_suffix].byte + text.len() - document.text.len();
                suffix = Some((previous, first_suffix, start));
            }
        }
        loop {
            if let Some((previous, first, start)) = &suffix
                && *start == byte_start
            {
                let anchor = &previous[*first];
                for slot in &previous[*first..] {
                    slots.push(ParagraphSlot {
                        byte: slot.byte - anchor.byte + byte_start,
                        y: slot.y - anchor.y + height,
                        first_line: slot.first_line - anchor.first_line + line_count,
                        layout: slot.layout.clone(),
                    });
                }
                self.diagnostics.paragraphs_carried += (previous.len() - *first) as u64;
                break;
            }
            let paragraph_end = text[byte_start..]
                .find('\n')
                .map_or(text.len(), |end| byte_start + end);
            let paragraph = &text[byte_start..paragraph_end];
            self.diagnostics.paragraphs_considered += 1;
            let key = LayoutKey::new(paragraph, style, paragraph_options, self.generation);
            let layout = if let Some(existing) = self.cache.get(&key) {
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
            let (y, first_line) = (height, line_count);
            height += layout.metrics.size.height;
            line_count += layout.lines.len();
            slots.push(ParagraphSlot {
                byte: byte_start,
                y,
                first_line,
                layout,
            });
            if paragraph_end == text.len() {
                break;
            }
            byte_start = paragraph_end + 1; // include the hard break
        }
        let layout = Arc::new(TextDocumentLayout::new(slots.into()));
        self.retained_documents.truncate(RETAINED_DOCUMENTS - 1);
        self.retained_documents.insert(
            0,
            RetainedDocument {
                policy,
                text: text.to_owned(),
                layout: layout.clone(),
            },
        );
        layout
    }
}

/// Paragraphs of `previous` that `text` shares, as the number carried
/// unchanged from the start and the index of the first one carried from the
/// end (the paragraph count when none is).
///
/// Paragraph `k` and everything before it are unchanged when the common
/// byte prefix reaches the start of paragraph `k`, so paragraphs before the
/// one holding the first change are carried. Paragraph `j` and everything
/// after it are unchanged when the common byte suffix covers the hard break
/// before it, or when the whole previous text follows a new hard break. The
/// suffix starts at or after the first recomposed paragraph in both texts,
/// so no paragraph is carried twice.
fn carried_paragraphs(paragraphs: &[ParagraphSlot], previous: &str, text: &str) -> (usize, usize) {
    let prefix = common_prefix_len(previous, text);
    let carried = paragraphs
        .partition_point(|slot| slot.byte <= prefix)
        .saturating_sub(1);
    let floor = paragraphs[carried].byte;
    let shared = common_suffix_len(previous, text);
    if carried == 0
        && shared == previous.len()
        && text.len() > previous.len()
        && text.as_bytes()[text.len() - previous.len() - 1] == b'\n'
    {
        return (0, 0);
    }
    // Lowest start whose preceding hard break is shared and which lands at
    // or after `floor` in the new text.
    let threshold = floor
        .max((floor + previous.len()).saturating_sub(text.len()))
        .max(previous.len() - shared + 1);
    let suffix = paragraphs
        .partition_point(|slot| slot.byte < threshold)
        .max(carried);
    (carried, suffix)
}

/// Length in bytes of the longest common suffix of `left` and `right`.
fn common_suffix_len(left: &str, right: &str) -> usize {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    let mut suffix = 0;
    for (a, b) in left.rchunks(64).zip(right.rchunks(64)) {
        if a == b {
            suffix += a.len();
        } else {
            return suffix
                + a.iter()
                    .rev()
                    .zip(b.iter().rev())
                    .take_while(|(x, y)| x == y)
                    .count();
        }
    }
    suffix
}
