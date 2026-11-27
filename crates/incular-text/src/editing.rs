//! Renderer-independent editing and selection state.

use crate::{TextGeometry, TextStyle};
use icu_segmenter::GraphemeClusterSegmenter;
use incular_core::time::Instant;
use incular_core::{RestorationKey, RestorationScope};
use std::{
    cell::RefCell,
    fmt,
    hash::{Hash, Hasher},
    ops::Range,
    rc::{Rc, Weak},
};

/// Whether a caret is associated with the leading or trailing edge of a
/// bidirectional run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextAffinity {
    #[default]
    Downstream,
    Upstream,
}

/// Flutter's signed UTF-16 selection coordinates. The retained editor
/// uses unsigned UTF-8 positions; this type preserves the invalid
/// sentinel and equality at the Flutter boundary.
#[derive(Clone, Copy, Debug)]
pub struct FlutterTextSelection {
    pub base_offset: i64,
    pub extent_offset: i64,
    pub affinity: TextAffinity,
    pub is_directional: bool,
}

impl FlutterTextSelection {
    #[must_use]
    pub const fn new(base_offset: i64, extent_offset: i64) -> Self {
        Self {
            base_offset,
            extent_offset,
            affinity: TextAffinity::Downstream,
            is_directional: false,
        }
    }

    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.base_offset >= 0 && self.extent_offset >= 0
    }

    fn identity(self) -> Option<(i64, i64, TextAffinity, bool)> {
        self.is_valid().then_some((
            self.base_offset,
            self.extent_offset,
            if self.base_offset == self.extent_offset {
                self.affinity
            } else {
                TextAffinity::Downstream
            },
            self.is_directional,
        ))
    }

    pub(crate) fn utf16_offsets(self) -> Option<TextSelection> {
        Some(TextSelection::new(
            self.base_offset.try_into().ok()?,
            self.extent_offset.try_into().ok()?,
        ))
    }
}

impl PartialEq for FlutterTextSelection {
    fn eq(&self, other: &Self) -> bool {
        self.identity() == other.identity()
    }
}

impl Eq for FlutterTextSelection {}

impl Hash for FlutterTextSelection {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.identity().hash(state);
    }
}

/// A half-open UTF-8 byte range. Text engines use byte offsets so ranges can
/// be passed directly to shaping and grapheme segmentation without lossy
/// conversions. Call [`Self::clamp_to`] before slicing arbitrary user input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    pub const EMPTY: Self = Self { start: 0, end: 0 };

    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub const fn collapsed(offset: usize) -> Self {
        Self::new(offset, offset)
    }

    #[must_use]
    pub const fn is_collapsed(self) -> bool {
        self.start == self.end
    }

    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.start <= self.end
    }

    /// Whether this range can describe a slice of the supplied UTF-8 text.
    #[must_use]
    pub fn is_valid_for(self, text: &str) -> bool {
        self.is_valid() && text.is_char_boundary(self.start) && text.is_char_boundary(self.end)
    }

    #[must_use]
    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    #[must_use]
    pub const fn as_range(self) -> Range<usize> {
        self.start..self.end
    }

    /// Clamps the range to a string and moves each endpoint to a valid UTF-8
    /// boundary. This keeps editing operations panic-free for IME byte offsets.
    #[must_use]
    pub fn clamp_to(self, text: &str) -> Self {
        let start = nearest_char_boundary(text, self.start.min(text.len()));
        let end = nearest_char_boundary(text, self.end.min(text.len())).max(start);
        Self::new(start, end)
    }

    #[must_use]
    pub fn normalized(self) -> Self {
        if self.start <= self.end {
            self
        } else {
            Self::new(self.end, self.start)
        }
    }
}

/// A selection preserves directional base/extent while exposing a normalized
/// range for text replacement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextSelection {
    pub base: usize,
    pub extent: usize,
}

impl TextSelection {
    #[must_use]
    pub const fn new(base: usize, extent: usize) -> Self {
        Self { base, extent }
    }

    #[must_use]
    pub const fn collapsed(offset: usize) -> Self {
        Self::new(offset, offset)
    }

    #[must_use]
    pub const fn start(self) -> usize {
        if self.base < self.extent {
            self.base
        } else {
            self.extent
        }
    }

    #[must_use]
    pub const fn end(self) -> usize {
        if self.base > self.extent {
            self.base
        } else {
            self.extent
        }
    }

    #[must_use]
    pub const fn is_collapsed(self) -> bool {
        self.base == self.extent
    }

    #[must_use]
    pub const fn is_forward(self) -> bool {
        self.base <= self.extent
    }

    #[must_use]
    pub const fn range(self) -> TextRange {
        TextRange::new(self.start(), self.end())
    }

    /// Clamps both edges into the text at UTF-8 char boundaries,
    /// snapping down. Char boundaries are deliberately not grapheme
    /// boundaries: a mid-grapheme edge survives clamping and the caret
    /// mapping below resolves it to the shaping-cluster edge instead.
    #[must_use]
    pub fn clamp_to(self, text: &str) -> Self {
        let base = nearest_char_boundary(text, self.base.min(text.len()));
        let extent = nearest_char_boundary(text, self.extent.min(text.len()));
        Self::new(base, extent)
    }
}

/// A composing/marked range supplied by an IME.
pub type ComposingRange = TextRange;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEditingValue {
    pub text: String,
    pub selection: TextSelection,
    pub composing: Option<ComposingRange>,
}

impl TextEditingValue {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let caret = text.len();
        Self {
            text,
            selection: TextSelection::collapsed(caret),
            composing: None,
        }
    }

    #[must_use]
    pub const fn empty() -> Self {
        Self {
            text: String::new(),
            selection: TextSelection::collapsed(0),
            composing: None,
        }
    }

    /// Flutter's `isComposingRangeValid`, using Incular's byte offsets.
    /// An absent range is not valid; a collapsed range at a character
    /// boundary is valid, including at the end of the string.
    #[must_use]
    pub fn is_composing_range_valid(&self) -> bool {
        self.composing
            .is_some_and(|range| range.is_valid_for(&self.text))
    }

    #[must_use]
    pub fn with_selection(mut self, selection: TextSelection) -> Self {
        self.selection = selection.clamp_to(&self.text);
        self
    }

    #[must_use]
    pub fn with_composing(mut self, composing: Option<ComposingRange>) -> Self {
        self.composing = composing.map(|range| range.clamp_to(&self.text));
        self
    }

    /// Replaces a byte range, shifting selection and composing endpoints
    /// without changing their direction. Use the controller's editing
    /// operations when the intention is to collapse the active selection.
    #[must_use]
    pub fn replace(mut self, range: TextRange, replacement: &str) -> Self {
        let range = range.normalized().clamp_to(&self.text);
        let removed = &self.text[range.as_range()];
        let end = range.start + replacement.len();
        let mut same_utf16_length = None;
        let mut adjust = |offset: usize| {
            let offset = nearest_char_boundary(&self.text, offset.min(self.text.len()));
            if offset < range.start || (offset == range.start && !range.is_collapsed()) {
                return offset;
            }
            if offset >= range.end {
                return end + (offset - range.end);
            }
            // Flutter preserves interior UTF-16 offsets for an equal-length
            // replacement. Keep the corresponding Rust byte offset when it
            // is representable; snap down if a new scalar straddles it.
            if *same_utf16_length.get_or_insert_with(|| {
                removed.encode_utf16().count() == replacement.encode_utf16().count()
            }) {
                let units = removed[..offset - range.start].encode_utf16().count();
                range.start + byte_offset_at_utf16_boundary(replacement, units)
            } else {
                end
            }
        };
        self.selection =
            TextSelection::new(adjust(self.selection.base), adjust(self.selection.extent));
        self.composing = self
            .composing
            .map(|composing| TextRange::new(adjust(composing.start), adjust(composing.end)));
        self.text.replace_range(range.as_range(), replacement);
        self
    }
}

/// Converts a UTF-16 offset to bytes, snapping an interior surrogate down.
#[doc(hidden)]
#[must_use]
pub fn byte_offset_at_utf16_boundary(text: &str, units: usize) -> usize {
    let mut consumed = 0;
    for (byte, scalar) in text.char_indices() {
        consumed += scalar.len_utf16();
        if consumed > units {
            return byte;
        }
    }
    text.len()
}

impl Default for TextEditingValue {
    fn default() -> Self {
        Self::empty()
    }
}

/// An atomic editing operation, useful for integrating native IMEs and undo
/// stacks without coupling those systems to a widget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEditingDelta {
    pub range: TextRange,
    pub replacement: String,
    pub selection: Option<TextSelection>,
    pub composing: Option<ComposingRange>,
}

/// A delta addressed a reversed, out-of-bounds, or non-UTF-8-boundary
/// replacement range in the previous text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidTextDeltaRange {
    pub range: TextRange,
    pub text_byte_len: usize,
}

impl fmt::Display for InvalidTextDeltaRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "delta range {}..{} is not a valid UTF-8 range in {} bytes",
            self.range.start, self.range.end, self.text_byte_len,
        )
    }
}

impl std::error::Error for InvalidTextDeltaRange {}

impl TextEditingDelta {
    #[must_use]
    pub fn replace(range: TextRange, replacement: impl Into<String>) -> Self {
        Self {
            range,
            replacement: replacement.into(),
            selection: None,
            composing: None,
        }
    }

    /// The smallest delta that turns text `old` into `new`: the changed
    /// middle of the text, plus `new`'s selection and composing range.
    /// Platforms that report their whole editing state (Android's
    /// `InputConnection`, Flutter's `updateEditingState`) apply it through
    /// [`TextEditingController::apply_delta`], so input formatters see
    /// only what changed.
    #[must_use]
    pub fn between(old: &str, new: &TextEditingValue) -> Self {
        let (old_bytes, new_bytes) = (old.as_bytes(), new.text.as_bytes());
        let mut prefix = old_bytes
            .iter()
            .zip(new_bytes)
            .take_while(|(a, b)| a == b)
            .count();
        while !old.is_char_boundary(prefix) || !new.text.is_char_boundary(prefix) {
            prefix -= 1;
        }
        let limit = old.len().min(new.text.len()) - prefix;
        let mut suffix = old_bytes
            .iter()
            .rev()
            .zip(new_bytes.iter().rev())
            .take(limit)
            .take_while(|(a, b)| a == b)
            .count();
        while !old.is_char_boundary(old.len() - suffix)
            || !new.text.is_char_boundary(new.text.len() - suffix)
        {
            suffix -= 1;
        }
        Self {
            range: TextRange::new(prefix, old.len() - suffix),
            replacement: new.text[prefix..new.text.len() - suffix].to_owned(),
            selection: Some(new.selection),
            composing: new.composing,
        }
    }

    /// Applies the edit, leaving the value untouched if the replacement
    /// range is malformed. Use [`Self::try_apply`] for the original error.
    #[must_use]
    pub fn apply(&self, value: &TextEditingValue) -> TextEditingValue {
        self.try_apply(value).unwrap_or_else(|_| value.clone())
    }

    /// Applies a checked edit without silently clamping a malformed
    /// replacement range into a different edit.
    ///
    /// # Errors
    /// Returns [`InvalidTextDeltaRange`] unless both endpoints describe an
    /// ordered UTF-8 byte range in the previous text.
    pub fn try_apply(
        &self,
        value: &TextEditingValue,
    ) -> Result<TextEditingValue, InvalidTextDeltaRange> {
        if !self.range.is_valid_for(&value.text) {
            return Err(InvalidTextDeltaRange {
                range: self.range,
                text_byte_len: value.text.len(),
            });
        }
        let mut next = value.clone().replace(self.range, &self.replacement);
        if let Some(selection) = self.selection {
            next.selection = selection.clamp_to(&next.text);
        }
        next.composing = self.composing.map(|range| range.clamp_to(&next.text));
        Ok(next)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SelectionChangedCause {
    #[default]
    Unknown,
    Keyboard,
    Tap,
    DoubleTap,
    LongPress,
    Drag,
    /// Selection made by an IME's floating-cursor gesture.
    ForcePress,
    /// A command invoked through the editor selection toolbar.
    Toolbar,
    Accessibility,
    Programmatic,
}

type Listener = Rc<dyn Fn(&TextEditingValue)>;
type EditTransform = Rc<dyn Fn(&TextEditingValue, &TextEditingValue) -> TextEditingValue>;

#[derive(Default)]
struct ControllerState {
    value: TextEditingValue,
    listeners: Vec<(usize, Listener)>,
    next_listener: usize,
    content_revision: u64,
    visual_revision: u64,
    caret_reset: Option<Instant>,
    preferred_caret_x: Option<f32>,
    caret_affinity: TextAffinity,
    selection_is_directional: bool,
    preedit: Option<String>,
    preedit_selection: Option<TextRange>,
    restoration: Option<TextRestoration>,
    restoration_generation: u64,
    /// Flutter's invalid selection (`TextSelection.collapsed(offset: -1)`): no
    /// selection was set since the text was. `value.selection` then holds
    /// the collapsed end Incular has always reported.
    selection_unset: bool,
    user_edit_notification_depth: usize,
    original_user_edit_text: Option<String>,
    focused_selection_policies: Vec<(u64, bool)>,
    next_focus_policy: u64,
}

#[derive(Clone)]
struct TextRestoration {
    scope: RestorationScope,
    key: RestorationKey,
}

/// Cloneable editing controller. It owns no platform resources; platform and
/// widget layers can subscribe to value changes and decide when to repaint.
#[derive(Clone, Default)]
pub struct TextEditingController {
    changes: incular_core::reactivity::DependencySource,
    /// Notified only for user edits that changed the text (see
    /// [`TextEditingController::observe_user_edits`]).
    user_edits: incular_core::reactivity::DependencySource,
    transforms: incular_core::reactivity::DependencySource,
    inner: Rc<RefCell<ControllerState>>,
}

/// Delays user text-change observers until the enclosing editor action has
/// notified its selection callback.
#[doc(hidden)]
#[must_use = "keep the batch alive until the editor callbacks have run"]
pub struct UserEditNotificationBatch {
    controller: TextEditingController,
}

impl Drop for UserEditNotificationBatch {
    fn drop(&mut self) {
        let changed = {
            let mut state = self.controller.inner.borrow_mut();
            state.user_edit_notification_depth -= 1;
            if state.user_edit_notification_depth == 0 {
                state.original_user_edit_text.take().is_some_and(|text| text != state.value.text)
            } else {
                false
            }
        };
        if changed && !std::thread::panicking() {
            self.controller.user_edits.notify();
        }
    }
}

/// Owns a text-controller listener until dropped.
#[must_use = "keep the subscription alive to observe changes"]
pub struct TextEditingSubscription {
    _subscription: incular_core::reactivity::Subscription,
}

/// RAII ownership of a focused editor's invalid-selection repair policy.
#[doc(hidden)]
pub struct FocusedSelectionLease {
    owner: Weak<RefCell<ControllerState>>,
    id: u64,
    select_all: bool,
}

impl FocusedSelectionLease {
    #[must_use]
    pub fn matches(&self, controller: &TextEditingController, select_all: bool) -> bool {
        self.select_all == select_all && self.owner.as_ptr() == Rc::as_ptr(&controller.inner)
    }
}

impl Drop for FocusedSelectionLease {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.upgrade() {
            owner
                .borrow_mut()
                .focused_selection_policies
                .retain(|(id, _)| *id != self.id);
        }
    }
}

impl PartialEq for TextEditingController {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl fmt::Debug for TextEditingController {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TextEditingController")
            .field("value", &self.value())
            .finish()
    }
}

#[derive(Clone, Copy)]
enum RangeSelectionChange {
    Preserve,
    Set(TextAffinity),
    Clear,
}

enum CommitSelection {
    Set,
    Reset,
    Explicit {
        is_set: bool,
        affinity: TextAffinity,
        is_directional: bool,
    },
}

impl TextEditingController {
    #[must_use]
    pub fn new() -> Self {
        Self::unset(TextEditingValue::default())
    }

    /// Creates a controller initialized with committed text.
    ///
    /// Like Flutter's `TextEditingController(text:)`, the selection starts
    /// unset ([`Self::selection_if_set`] is `None`) until a selection or an
    /// edit is applied or an `EditableText` takes focus.
    #[must_use]
    pub fn with_text(text: impl Into<String>) -> Self {
        Self::unset(TextEditingValue::new(text))
    }

    fn unset(value: TextEditingValue) -> Self {
        let controller = Self::from_value(value);
        controller.inner.borrow_mut().selection_unset = true;
        controller
    }

    /// Creates an empty editor and restores its committed state, if present.
    #[must_use]
    pub fn restored(scope: RestorationScope, key: RestorationKey) -> Self {
        let controller = Self::new();
        controller.bind_restoration(scope, key);
        controller
    }

    /// Binds committed text and selection to a stable restoration value.
    pub fn bind_restoration(&self, scope: RestorationScope, key: RestorationKey) {
        let restored = scope.get_json(&key).and_then(value_from_json);
        let mut state = self.inner.borrow_mut();
        let changed = restored.as_ref().is_some_and(|restored| {
            state.value != restored.value
                || state.selection_unset != restored.selection_unset
                || state.caret_affinity != restored.affinity
                || state.selection_is_directional != restored.is_directional
                || state.preedit.is_some()
        });
        state.restoration = Some(TextRestoration { scope, key });
        // Restoration is a baseline, not an edit: bump the generation so
        // history observers rebaseline instead of recording an undo step.
        state.restoration_generation = state
            .restoration_generation
            .checked_add(1)
            .expect("text restoration generation exhausted");
        if let Some(restored) = restored {
            state.value = restored.value;
            state.selection_unset = restored.selection_unset;
            state.content_revision = state
                .content_revision
                .checked_add(1)
                .expect("content revision exhausted");
            state.visual_revision = state
                .visual_revision
                .checked_add(1)
                .expect("visual revision exhausted");
            state.preedit = None;
            state.preedit_selection = None;
            state.caret_reset = None;
            state.preferred_caret_x = None;
            state.caret_affinity = restored.affinity;
            state.selection_is_directional = restored.is_directional;
        }
        drop(state);
        if changed {
            self.notify_listeners();
        }
    }

    /// Stops persisting subsequent editor mutations without deleting the saved value.
    pub fn unbind_restoration(&self) {
        self.inner.borrow_mut().restoration = None;
    }

    #[must_use]
    pub fn from_value(value: TextEditingValue) -> Self {
        Self {
            changes: incular_core::reactivity::DependencySource::default(),
            user_edits: incular_core::reactivity::DependencySource::default(),
            transforms: incular_core::reactivity::DependencySource::default(),
            inner: Rc::new(RefCell::new(ControllerState {
                value,
                ..ControllerState::default()
            })),
        }
    }

    #[must_use]
    pub fn value(&self) -> TextEditingValue {
        self.changes.track();
        self.inner.borrow().value.clone()
    }

    /// Runs `f` against the current value without cloning it. Dependency
    /// tracking matches [`Self::value`]. The controller must not be mutated
    /// from inside `f`.
    pub fn with_value<R>(&self, f: impl FnOnce(&TextEditingValue) -> R) -> R {
        self.changes.track();
        f(&self.inner.borrow().value)
    }

    #[must_use]
    pub fn text(&self) -> String {
        self.changes.track();
        self.inner.borrow().value.text.clone()
    }

    #[must_use]
    pub fn selection(&self) -> TextSelection {
        self.changes.track();
        self.inner.borrow().value.selection
    }

    /// The selection, or `None` while it is unset: Flutter's invalid
    /// selection (`selection.isValid == false`, offsets -1) after
    /// [`Self::with_text`], [`Self::new`] or [`Self::set_text`], before any
    /// selection or edit. [`Self::selection`] reports the collapsed end
    /// instead.
    #[must_use]
    pub fn selection_if_set(&self) -> Option<TextSelection> {
        self.changes.track();
        let state = self.inner.borrow();
        (!state.selection_unset).then_some(state.value.selection)
    }

    /// Whether the last platform selection explicitly distinguished
    /// anchor and extent instead of allowing platform direction inference.
    #[must_use]
    pub fn selection_is_directional(&self) -> bool {
        self.inner.borrow().selection_is_directional
    }

    /// Collapses an unset selection at the end of the text, as Flutter's
    /// `EditableText` does when it gains focus with an invalid selection
    /// (`_handleFocusChanged`). A set selection is left as it is.
    pub fn collapse_unset_selection(&self) {
        if self.inner.borrow().selection_unset {
            let end = self.inner.borrow().value.text.len();
            self.set_selection(TextSelection::collapsed(end));
        }
    }

    #[must_use]
    pub fn composing(&self) -> Option<ComposingRange> {
        self.changes.track();
        self.inner.borrow().value.composing
    }

    /// Replaces the whole value programmatically (Flutter's `controller.value`).
    /// Edit transforms and user-edit observers do not run; only [`Self::observe`]
    /// listeners are notified.
    pub fn set_value(&self, value: TextEditingValue) {
        self.commit_with_selection(
            value,
            true,
            false,
            false,
            CommitSelection::Explicit {
                is_set: true,
                affinity: TextAffinity::Downstream,
                is_directional: false,
            },
        );
    }

    /// Replaces the text programmatically (Flutter's `controller.text`). Like
    /// [`Self::set_value`], the text is stored as given: no input formatters run.
    /// The selection becomes unset unless a focused editor repairs it.
    pub fn set_text(&self, text: impl Into<String>) {
        self.update_committed(TextEditingValue::new(text), true, false, false, true);
    }

    /// Marks the controller as the focused editor until the lease is dropped.
    #[doc(hidden)]
    #[must_use]
    pub fn bind_focused_selection(&self, select_all: bool) -> FocusedSelectionLease {
        let mut state = self.inner.borrow_mut();
        let id = state.next_focus_policy;
        state.next_focus_policy = id.checked_add(1).expect("focus policy identity exhausted");
        state.focused_selection_policies.push((id, select_all));
        FocusedSelectionLease {
            owner: Rc::downgrade(&self.inner),
            id,
            select_all,
        }
    }

    /// Clears text and composition and sets a valid collapsed caret at zero,
    /// even without a focused field.
    pub fn clear(&self) {
        self.set_value(TextEditingValue::new(""));
    }

    pub fn set_selection(&self, selection: TextSelection) {
        self.set_selection_with_affinity(selection, TextAffinity::Downstream);
    }

    /// Sets a selection while preserving which visual edge of a bidi run owns
    /// a collapsed caret. Native hit testing should use this overload.
    pub fn set_selection_with_affinity(&self, selection: TextSelection, affinity: TextAffinity) {
        self.update_ranges(RangeSelectionChange::Set(affinity), |value| value.selection = selection);
    }

    /// Makes the selection unavailable until the next selection or edit.
    /// The committed text is untouched and no caret is painted meanwhile.
    pub fn clear_selection(&self) {
        self.update_ranges(RangeSelectionChange::Clear, |value| {
            value.selection = TextSelection::collapsed(value.text.len());
        });
    }

    pub fn set_composing(&self, composing: Option<ComposingRange>) {
        self.update_ranges(RangeSelectionChange::Preserve, |value| value.composing = composing);
    }

    /// Clears the input method's composing range without moving the caret
    /// or changing whether the selection is set.
    pub fn clear_composing(&self) {
        self.set_composing(None);
    }

    /// Observes committed editing changes with automatic cleanup on drop.
    pub fn observe(
        &self,
        listener: impl Fn(&TextEditingValue) + 'static,
    ) -> TextEditingSubscription {
        self.observe_source(&self.changes, listener)
    }

    /// Observes user edits that changed the text, with automatic cleanup on
    /// drop. User edits are the editing operations (typing, deletion, IME
    /// commit, [`Self::apply_delta`]); programmatic updates, restoration,
    /// and selection or composing changes do not notify. This is Flutter's
    /// `onChanged` contract.
    pub fn observe_user_edits(
        &self,
        listener: impl Fn(&TextEditingValue) + 'static,
    ) -> TextEditingSubscription {
        self.observe_source(&self.user_edits, listener)
    }

    fn observe_source(
        &self,
        source: &incular_core::reactivity::DependencySource,
        listener: impl Fn(&TextEditingValue) + 'static,
    ) -> TextEditingSubscription {
        let state = Rc::downgrade(&self.inner);
        TextEditingSubscription {
            _subscription: source.subscribe((), move || {
                if let Some(state) = state.upgrade() {
                    let value = state.borrow().value.clone();
                    listener(&value);
                }
            }),
        }
    }

    #[doc(hidden)]
    pub fn register_edit_transform(
        &self,
        transform: impl Fn(&TextEditingValue, &TextEditingValue) -> TextEditingValue + 'static,
    ) -> incular_core::reactivity::Subscription {
        let transform: EditTransform = Rc::new(transform);
        self.transforms.subscribe(transform, || {})
    }

    /// Low-level manual registration for backend integrations. Prefer `observe`.
    /// Returns an opaque token accepted by
    /// [`Self::remove_listener`].
    pub fn add_listener(&self, listener: impl Fn(&TextEditingValue) + 'static) -> usize {
        let mut state = self.inner.borrow_mut();
        let token = state.next_listener;
        state.next_listener = state
            .next_listener
            .checked_add(1)
            .expect("text editing listener identity space exhausted");
        state.listeners.push((token, Rc::new(listener)));
        token
    }

    pub fn remove_listener(&self, token: usize) -> bool {
        let mut state = self.inner.borrow_mut();
        let before = state.listeners.len();
        state.listeners.retain(|(id, _)| *id != token);
        before != state.listeners.len()
    }

    /// Batches the user text-change notification, without deferring
    /// committed editing state or ordinary controller listeners.
    #[doc(hidden)]
    pub fn batch_user_edit_notifications(&self) -> UserEditNotificationBatch {
        {
            let mut state = self.inner.borrow_mut();
            state.user_edit_notification_depth = state
                .user_edit_notification_depth
                .checked_add(1)
                .expect("user edit notification nesting exhausted");
        }
        UserEditNotificationBatch { controller: self.clone() }
    }

    /// Applies a user (IME or platform) delta. Input formatters run only
    /// on valid edits. Use [`Self::try_apply_delta`] to inspect a rejection.
    pub fn apply_delta(&self, delta: &TextEditingDelta) {
        let _ = self.try_apply_delta(delta);
    }

    /// Applies a user delta and returns an error for an invalid range,
    /// without invoking formatters or notifying edit listeners.
    pub fn try_apply_delta(&self, delta: &TextEditingDelta) -> Result<(), InvalidTextDeltaRange> {
        let next = delta.try_apply(&self.inner.borrow().value)?;
        self.update_user(next);
        Ok(())
    }

    /// Applies one tagged autofill value as a platform edit. A missing
    /// selection stays unset rather than looking like a tap at offset zero.
    pub fn apply_autofill_value(
        &self,
        value: TextEditingValue,
        selection_is_set: bool,
        affinity: TextAffinity,
        selection_is_directional: bool,
    ) {
        self.apply_platform_editing_value(
            value,
            selection_is_set,
            affinity,
            selection_is_directional,
        );
    }

    /// Applies an already validated platform value in one transaction,
    /// including its selection metadata. Selection-only changes do not run
    /// text formatters and do not invalidate shaped content.
    #[doc(hidden)]
    pub fn apply_platform_editing_value(
        &self,
        value: TextEditingValue,
        selection_is_set: bool,
        affinity: TextAffinity,
        selection_is_directional: bool,
    ) {
        let text_or_composing_changed = self.with_value(|old| {
            old.text != value.text || old.composing != value.composing
        });
        self.commit_with_selection(
            value,
            text_or_composing_changed,
            true,
            text_or_composing_changed,
            CommitSelection::Explicit {
                is_set: selection_is_set,
                affinity,
                is_directional: selection_is_directional,
            },
        );
    }

    /// Replaces a byte range through the user-edit path, adjusting the
    /// existing selection and composing range rather than collapsing them.
    ///
    /// Equal UTF-16 lengths preserve the selection's affinity and directional
    /// bit, as Flutter's `TextEditingValue.replaced` does. A resize resets
    /// that metadata but keeps the order of the two selection endpoints.
    ///
    /// # Errors
    /// A malformed byte range is rejected before any formatter or listener runs.
    pub fn replace_range_as_user_edit(
        &self,
        range: TextRange,
        replacement: &str,
    ) -> Result<bool, InvalidTextDeltaRange> {
        let (next, selection) = {
            let state = self.inner.borrow();
            let value = &state.value;
            if !range.is_valid_for(&value.text) {
                return Err(InvalidTextDeltaRange {
                    range,
                    text_byte_len: value.text.len(),
                });
            }
            let removed = &value.text[range.as_range()];
            if removed == replacement {
                return Ok(false);
            }
            let same_length = removed.encode_utf16().count() == replacement.encode_utf16().count();
            (
                value.clone().replace(range, replacement),
                CommitSelection::Explicit {
                    is_set: !state.selection_unset,
                    affinity: if same_length {
                        state.caret_affinity
                    } else {
                        TextAffinity::Downstream
                    },
                    is_directional: same_length && state.selection_is_directional,
                },
            )
        };
        self.commit_with_selection(next, true, false, true, selection);
        Ok(true)
    }

    /// Replaces the selection as a user edit: input formatters run on the
    /// result, and [`Self::observe_user_edits`] observers see text changes.
    pub fn replace_selection(&self, replacement: &str) {
        let next = self.replacing(|value| value.selection.range(), replacement);
        self.update_user(next);
    }

    pub fn insert_text(&self, text: &str) {
        self.replace_selection(text);
    }

    pub fn delete_backward(&self) {
        self.backspace();
    }

    pub fn delete_forward(&self) {
        let next = self.replacing(
            |value| {
                if value.selection.is_collapsed() {
                    next_grapheme_boundary(&value.text, value.selection.extent)
                        .map_or(TextRange::collapsed(value.selection.extent), |end| {
                            TextRange::new(value.selection.extent, end)
                        })
                } else {
                    value.selection.range()
                }
            },
            "",
        );
        self.update_user(next);
    }

    /// Alias used by the Widgets editor API.
    pub fn insert(&self, text: &str) {
        self.insert_text(text);
    }

    /// Deletes one grapheme cluster behind the caret, or the active selection.
    pub fn backspace(&self) {
        let next = self.replacing(
            |value| {
                if value.selection.is_collapsed() {
                    previous_grapheme_boundary(&value.text, value.selection.extent)
                        .map_or(TextRange::collapsed(value.selection.extent), |start| {
                            TextRange::new(start, value.selection.extent)
                        })
                } else {
                    value.selection.range()
                }
            },
            "",
        );
        self.update_user(next);
    }

    /// Deletes one grapheme cluster ahead of the caret, or the active selection.
    pub fn delete(&self) {
        self.delete_forward();
    }

    pub fn move_left(&self, extend: bool) {
        let target = self.with_value(|value| {
            previous_grapheme_boundary(&value.text, value.selection.extent).unwrap_or(0)
        });
        self.move_cursor(target, extend);
    }

    pub fn move_right(&self, extend: bool) {
        let target = self.with_value(|value| {
            next_grapheme_boundary(&value.text, value.selection.extent).unwrap_or(value.text.len())
        });
        self.move_cursor(target, extend);
    }

    /// Moves one grapheme in screen/visual order using the bidi caret stops
    /// of a [`TextGeometry`]. The logical byte offset may move backwards
    /// while the caret moves right, which is required for right-to-left text.
    pub fn move_left_visual(&self, layout: &(impl TextGeometry + ?Sized), extend: bool) {
        self.move_visual(layout, false, extend);
    }

    /// Moves one grapheme in screen/visual order using the bidi caret stops
    /// of a [`TextGeometry`].
    pub fn move_right_visual(&self, layout: &(impl TextGeometry + ?Sized), extend: bool) {
        self.move_visual(layout, true, extend);
    }

    pub fn move_home(&self, extend: bool) {
        self.move_cursor(0, extend);
    }

    pub fn move_end(&self, extend: bool) {
        self.move_cursor(self.with_value(|value| value.text.len()), extend);
    }

    pub fn select_all(&self) {
        let end = self.with_value(|value| value.text.len());
        self.set_selection(TextSelection::new(0, end));
    }

    #[must_use]
    pub fn selected_text(&self) -> String {
        self.with_value(|value| value.text[value.selection.range().as_range()].to_owned())
    }

    /// Sets transient IME preedit text without changing committed content.
    pub fn set_preedit(&self, text: impl Into<String>, selection: Option<TextRange>) {
        incular_core::reactivity::assert_mutation_allowed();
        let text = text.into();
        let mut state = self.inner.borrow_mut();
        state.preedit_selection = selection.map(|range| range.clamp_to(&text));
        state.preedit = (!text.is_empty()).then_some(text);
        state.visual_revision = state
            .visual_revision
            .checked_add(1)
            .expect("visual revision exhausted");
        drop(state);
        self.notify_listeners();
    }

    pub fn commit_preedit(&self, text: &str) {
        let next = self.replacing(|value| value.selection.range(), text);
        self.update_committed(next, true, true, true, false);
    }

    pub fn clear_preedit(&self) {
        incular_core::reactivity::assert_mutation_allowed();
        let mut state = self.inner.borrow_mut();
        if state.preedit.take().is_some() {
            state.preedit_selection = None;
            state.visual_revision = state
                .visual_revision
                .checked_add(1)
                .expect("visual revision exhausted");
            drop(state);
            self.notify_listeners();
        }
    }

    #[must_use]
    pub fn preedit(&self) -> Option<String> {
        self.inner.borrow().preedit.clone()
    }

    #[must_use]
    pub fn preedit_selection(&self) -> Option<TextRange> {
        self.inner.borrow().preedit_selection
    }

    /// Returns committed and visual revisions used by retained repaint scheduling.
    #[doc(hidden)]
    #[must_use]
    pub fn revisions(&self) -> (u64, u64) {
        let state = self.inner.borrow();
        (state.content_revision, state.visual_revision)
    }

    /// Restoration baseline generation; history rebaselines when observed.
    #[doc(hidden)]
    #[must_use]
    pub fn restoration_generation(&self) -> u64 {
        self.inner.borrow().restoration_generation
    }

    /// Restarts the caret blink after an editing interaction.
    #[doc(hidden)]
    pub fn reset_caret(&self, now: Instant) {
        let mut state = self.inner.borrow_mut();
        state.caret_reset = Some(now);
        state.visual_revision = state
            .visual_revision
            .checked_add(1)
            .expect("visual revision exhausted");
    }

    /// Restarts only the presentation clock; the caller owns paint invalidation.
    #[doc(hidden)]
    pub fn restart_caret_clock(&self, now: Instant) {
        self.inner.borrow_mut().caret_reset = Some(now);
    }

    /// Returns whether the caret should be painted at `now`.
    #[doc(hidden)]
    #[must_use]
    pub fn caret_visible(&self, now: Instant) -> bool {
        self.caret_opacity(now, false) > 0.0
    }

    /// The caret's opacity at `now` since the blink last restarted. Without
    /// `animates` the caret is on, then off, for `_kCursorBlinkHalfPeriod`
    /// (500 ms) each; with it, it follows Flutter's
    /// `_KeyFrame.iOSBlinkingCaretKeyFrames` (iOS 15.4 UIKit) once a second
    /// (`EditableText.cursorOpacityAnimates`).
    #[must_use]
    pub fn caret_opacity(&self, now: Instant, animates: bool) -> f32 {
        self.caret_blink_sample(now, animates).0
    }

    /// Samples the current phase and the next opacity change on one clock.
    #[doc(hidden)]
    #[must_use]
    pub fn caret_blink_sample(&self, now: Instant, animates: bool) -> (f32, Option<Instant>) {
        use std::time::Duration;

        // Times are integral microseconds; omit keyframes that hold opacity.
        const IOS_PHASES: [(u128, f32); 8] = [
            (0, 1.0),
            (537_500, 0.75),
            (575_000, 0.5),
            (612_500, 0.25),
            (650_000, 0.0),
            (887_500, 0.25),
            (925_000, 0.5),
            (962_500, 0.75),
        ];
        const PERIOD: u128 = 1_000_000;
        const HALF_PERIOD: u128 = PERIOD / 2;
        let Some(start) = self.inner.borrow().caret_reset else {
            return (0.0, None);
        };
        let elapsed = now.checked_duration_since(start).unwrap_or_default().as_micros();
        let (opacity, next) = if animates {
            let phase = elapsed % PERIOD;
            let next = IOS_PHASES.partition_point(|(at, _)| *at <= phase);
            let opacity = IOS_PHASES[next - 1].1;
            let boundary = IOS_PHASES.get(next).map_or(PERIOD, |(time, _)| *time);
            (opacity, elapsed - phase + boundary)
        } else {
            let half_cycle = elapsed / HALF_PERIOD;
            let opacity = if half_cycle.is_multiple_of(2) { 1.0 } else { 0.0 };
            (opacity, (half_cycle + 1) * HALF_PERIOD)
        };
        let deadline = u64::try_from(next)
            .ok()
            .and_then(|micros| start.checked_add(Duration::from_micros(micros)));
        (opacity, deadline)
    }

    #[doc(hidden)]
    #[must_use]
    pub fn preferred_caret_x(&self) -> Option<f32> {
        self.inner.borrow().preferred_caret_x
    }

    /// Retains the horizontal column used by vertical movement without
    /// changing the editing value.
    #[doc(hidden)]
    pub fn set_preferred_caret_x(&self, x: f32) {
        self.inner.borrow_mut().preferred_caret_x = x.is_finite().then_some(x);
    }

    /// Returns the visual affinity of the collapsed caret.
    #[must_use]
    pub fn caret_affinity(&self) -> TextAffinity {
        self.inner.borrow().caret_affinity
    }

    #[doc(hidden)]
    pub fn move_cursor(&self, target: usize, extend: bool) {
        self.move_cursor_impl(target, extend, None, TextAffinity::Downstream);
    }

    #[doc(hidden)]
    pub fn move_cursor_with_affinity(&self, target: usize, affinity: TextAffinity, extend: bool) {
        self.move_cursor_impl(target, extend, None, affinity);
    }

    fn move_cursor_impl(
        &self,
        target: usize,
        extend: bool,
        preferred_x: Option<f32>,
        affinity: TextAffinity,
    ) {
        let base = self.with_value(|value| value.selection.base);
        let target = self
            .with_value(|value| nearest_char_boundary(&value.text, target.min(value.text.len())));
        let selection = if extend {
            TextSelection::new(base, target)
        } else {
            TextSelection::collapsed(target)
        };
        self.update_ranges(RangeSelectionChange::Set(affinity), |value| value.selection = selection);
        self.inner.borrow_mut().preferred_caret_x = preferred_x;
    }

    fn move_visual(&self, layout: &(impl TextGeometry + ?Sized), right: bool, extend: bool) {
        let selection = self.with_value(|value| value.selection);
        let extent = selection.extent;
        // An unextended arrow first collapses an active selection, matching
        // native editors and avoiding a surprising extra visual step.
        if !extend && !selection.is_collapsed() {
            let target = if right {
                selection.end()
            } else {
                selection.start()
            };
            self.move_cursor_with_affinity(target, TextAffinity::Downstream, false);
            return;
        }
        let line = layout
            .lines()
            .position(|line| extent >= line.start() && extent <= line.end())
            .unwrap_or_else(|| layout.line_count().saturating_sub(1));
        let stops = layout.line_caret_positions(line);
        if stops.is_empty() {
            return;
        }
        let affinity = self.caret_affinity();
        let current = stops
            .iter()
            .position(|stop| stop.offset == extent && stop.affinity == affinity)
            .or_else(|| stops.iter().position(|stop| stop.offset == extent))
            .unwrap_or_else(|| {
                if right {
                    0
                } else {
                    stops.len().saturating_sub(1)
                }
            });
        let target = if right {
            stops.get(current + 1).copied().or_else(|| {
                (line + 1..layout.line_count())
                    .find_map(|next_line| layout.line_caret_positions(next_line).first().copied())
            })
        } else {
            current
                .checked_sub(1)
                .and_then(|index| stops.get(index).copied())
                .or_else(|| {
                    (0..line).rev().find_map(|previous_line| {
                        layout.line_caret_positions(previous_line).last().copied()
                    })
                })
        };
        let Some(target) = target else {
            return;
        };
        self.move_cursor_with_affinity(target.offset, target.affinity, extend);
    }

    /// A programmatic update: stored as given, with no edit transforms.
    /// A user edit: edit transforms (input formatters) run first, and
    /// user-edit observers hear about a resulting text change.
    fn update_user(&self, value: TextEditingValue) {
        self.update_committed(value, true, false, true, false);
    }

    /// The current value with the range `range` picks replaced by
    /// `replacement`, as [`TextEditingValue::replace`] gives. The new text
    /// is assembled in one exactly sized allocation: copying the old value
    /// and editing the copy moved a long document twice per keystroke and
    /// left it holding twice its length.
    fn replacing(
        &self,
        range: impl FnOnce(&TextEditingValue) -> TextRange,
        replacement: &str,
    ) -> TextEditingValue {
        self.with_value(|value| {
            let range = range(value).normalized().clamp_to(&value.text);
            let range = range.as_range();
            let mut text = String::with_capacity(
                value.text.len() - (range.end - range.start) + replacement.len(),
            );
            text.push_str(&value.text[..range.start]);
            text.push_str(replacement);
            text.push_str(&value.text[range.end..]);
            TextEditingValue {
                text,
                selection: TextSelection::collapsed(range.start + replacement.len()),
                composing: None,
            }
        })
    }

    /// Changes the selection or the composing range, which never needs a
    /// copy of the text. Input formatters do not run here: they format
    /// text edits, and a range change leaves the text as it is.
    fn update_ranges(
        &self,
        selection_change: RangeSelectionChange,
        apply: impl Fn(&mut TextEditingValue),
    ) {
        incular_core::reactivity::assert_mutation_allowed();
        self.changes.track();
        let listeners = {
            let mut state = self.inner.borrow_mut();
            let was_unset = state.selection_unset;
            let selection_affinity = match selection_change {
                RangeSelectionChange::Preserve => None,
                RangeSelectionChange::Set(affinity) => {
                    state.selection_unset = false;
                    Some(affinity)
                }
                RangeSelectionChange::Clear => {
                    state.selection_unset = true;
                    state.selection_is_directional = false;
                    None
                }
            };
            let sets_selection = selection_affinity.is_some();
            let old_affinity = state.caret_affinity;
            let validity_changed = was_unset != state.selection_unset;
            let value = &mut state.value;
            let before = (value.selection, value.composing);
            apply(value);
            value.selection = value.selection.clamp_to(&value.text);
            value.composing = value.composing.map(|range| range.clamp_to(&value.text));
            if sets_selection && value.composing.is_some_and(|range| {
                value.selection.start() < range.start || value.selection.end() > range.end
            }) {
                value.composing = None;
            }
            let moved_caret_edge = value.selection.is_collapsed()
                && selection_affinity.is_some_and(|affinity| affinity != old_affinity);
            if (value.selection, value.composing) == before && !validity_changed && !moved_caret_edge {
                return;
            }
            if let Some(affinity) = selection_affinity {
                state.caret_affinity = affinity;
            }
            state.visual_revision = state
                .visual_revision
                .checked_add(1)
                .expect("visual revision exhausted");
            state.preedit = None;
            state.preedit_selection = None;
            state.preferred_caret_x = None;
            state
                .listeners
                .iter()
                .map(|(_, listener)| listener.clone())
                .collect::<Vec<_>>()
        };
        self.notify_value(listeners);
        self.persist_restoration();
        self.changes.notify();
    }

    /// Hands the current value to `listeners`. Listeners may edit the
    /// controller, so they get a copy; without listeners none is made.
    fn notify_value(&self, listeners: Vec<Listener>) {
        if listeners.is_empty() {
            return;
        }
        let value = self.inner.borrow().value.clone();
        for listener in listeners {
            listener(&value);
        }
    }

    /// Commits `value`. Only a user edit (`user`) runs the edit transforms;
    /// a user edit that changes the text also notifies user-edit observers.
    fn update_committed(
        &self,
        value: TextEditingValue,
        content_changed: bool,
        end_preedit: bool,
        user: bool,
        reset_selection: bool,
    ) {
        self.commit_with_selection(
            value, content_changed, end_preedit, user,
            if reset_selection { CommitSelection::Reset } else { CommitSelection::Set },
        );
    }

    fn commit_with_selection(
        &self,
        value: TextEditingValue,
        content_changed: bool,
        end_preedit: bool,
        user: bool,
        selection: CommitSelection,
    ) {
        incular_core::reactivity::assert_mutation_allowed();
        let mut value = {
            let transforms = if user {
                self.transforms.subscribers::<EditTransform>()
            } else {
                Vec::new()
            };
            let mut next = value;
            if !transforms.is_empty() {
                // Only a transform needs the value being replaced.
                let (old, revision) = {
                    let state = self.inner.borrow();
                    (state.value.clone(), state.visual_revision)
                };
                for transform in transforms {
                    let _guard = incular_core::reactivity::ReadOnlyGuard::enter();
                    next = transform(&old, &next);
                    if self.inner.borrow().visual_revision != revision {
                        return;
                    }
                }
            }
            next.selection = next.selection.clamp_to(&next.text);
            next.composing = next.composing.map(|range| range.clamp_to(&next.text));
            next
        };
        let (listeners, text_changed) = {
            let mut state = self.inner.borrow_mut();
            let (selection_unset, affinity, is_directional) = match selection {
                CommitSelection::Reset => {
                    if state.focused_selection_policies.last().is_some_and(|(_, all)| *all) {
                        value.selection = TextSelection::new(0, value.text.len());
                    }
                    (state.focused_selection_policies.is_empty(), TextAffinity::Downstream, false)
                }
                CommitSelection::Set => (false, state.caret_affinity, false),
                CommitSelection::Explicit {
                    is_set,
                    affinity,
                    is_directional,
                } => (!is_set, affinity, is_directional),
            };
            let selection_equal = if state.selection_unset || selection_unset {
                state.selection_unset && selection_unset
            } else {
                state.value.selection == value.selection
                    && state.selection_is_directional == is_directional
                    && (!value.selection.is_collapsed() || state.caret_affinity == affinity)
            };
            if state.value.text == value.text
                && state.value.composing == value.composing
                && !(end_preedit && state.preedit.is_some())
                && selection_equal
            {
                return;
            }
            state.selection_unset = selection_unset;
            state.caret_affinity = affinity;
            state.selection_is_directional = is_directional;
            let text_changed = user && state.value.text != value.text;
            let defer_notification = text_changed && state.user_edit_notification_depth != 0;
            if defer_notification && state.original_user_edit_text.is_none() {
                // Move the old string out, rather than copying a potentially
                // large document just to compare after selection callbacks.
                let original = std::mem::replace(&mut state.value, value);
                state.original_user_edit_text = Some(original.text);
            } else {
                state.value = value;
            }
            if content_changed {
                state.content_revision = state
                    .content_revision
                    .checked_add(1)
                    .expect("content revision exhausted");
            }
            state.visual_revision = state
                .visual_revision
                .checked_add(1)
                .expect("visual revision exhausted");
            state.preedit = None;
            state.preedit_selection = None;
            state.preferred_caret_x = None;
            let listeners = state
                .listeners
                .iter()
                .map(|(_, listener)| listener.clone())
                .collect::<Vec<_>>();
            (listeners, text_changed && !defer_notification)
        };
        self.notify_value(listeners);
        self.persist_restoration();
        self.changes.notify();
        if text_changed {
            self.user_edits.notify();
        }
    }

    fn notify_listeners(&self) {
        let listeners = self
            .inner
            .borrow()
            .listeners
            .iter()
            .map(|(_, listener)| listener.clone())
            .collect::<Vec<_>>();
        self.notify_value(listeners);
        self.changes.notify();
    }

    fn persist_restoration(&self) {
        let (restoration, value) = {
            let state = self.inner.borrow();
            let Some(restoration) = state.restoration.clone() else {
                return;
            };
            (restoration, value_to_json(&state))
        };
        restoration.scope.set_json(&restoration.key, value);
    }
}

fn value_to_json(state: &ControllerState) -> serde_json::Value {
    let value = &state.value;
    serde_json::json!({
        "text": value.text,
        "selection": {
            "base": value.selection.base,
            "extent": value.selection.extent,
            "isSet": !state.selection_unset,
            "affinity": match state.caret_affinity {
                TextAffinity::Downstream => "downstream",
                TextAffinity::Upstream => "upstream",
            },
            "isDirectional": state.selection_is_directional,
        },
    })
}

struct RestoredTextValue {
    value: TextEditingValue,
    selection_unset: bool,
    affinity: TextAffinity,
    is_directional: bool,
}

fn value_from_json(value: serde_json::Value) -> Option<RestoredTextValue> {
    let object = value.as_object()?;
    let text = object.get("text")?.as_str()?.to_owned();
    let saved_selection = object
        .get("selection")
        .and_then(serde_json::Value::as_object);
    let selection = saved_selection
        .and_then(|selection| {
            let base = usize::try_from(selection.get("base")?.as_u64()?).ok()?;
            let extent = usize::try_from(selection.get("extent")?.as_u64()?).ok()?;
            Some(TextSelection::new(base, extent))
        })
        .unwrap_or_else(|| TextSelection::collapsed(text.len()));
    let selection = selection.clamp_to(&text);
    let flag = |key| {
        saved_selection
            .and_then(|selection| selection.get(key))
            .and_then(serde_json::Value::as_bool)
    };
    Some(RestoredTextValue {
        value: TextEditingValue::new(text).with_selection(selection),
        // Legacy snapshots contained only offsets and loaded as valid.
        selection_unset: flag("isSet") == Some(false),
        affinity: if saved_selection
            .and_then(|selection| selection.get("affinity"))
            .and_then(serde_json::Value::as_str)
            == Some("upstream")
        {
            TextAffinity::Upstream
        } else {
            TextAffinity::Downstream
        },
        is_directional: flag("isDirectional") == Some(true),
    })
}

/// A renderer-neutral editable text description.
#[derive(Clone, Debug)]
pub struct EditableText {
    pub controller: TextEditingController,
    pub style: TextStyle,
    pub read_only: bool,
    pub obscure_text: bool,
    pub max_lines: Option<usize>,
}

impl EditableText {
    #[must_use]
    pub fn new(controller: TextEditingController) -> Self {
        Self {
            controller,
            style: TextStyle::default(),
            read_only: false,
            obscure_text: false,
            max_lines: Some(1),
        }
    }

    #[must_use]
    pub fn style(mut self, style: TextStyle) -> Self {
        self.style = style;
        self
    }

    #[must_use]
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    #[must_use]
    pub fn obscure_text(mut self, obscure_text: bool) -> Self {
        self.obscure_text = obscure_text;
        self
    }

    #[must_use]
    pub fn max_lines(mut self, max_lines: Option<usize>) -> Self {
        self.max_lines = max_lines;
        self
    }
}

/// Convenience text-field configuration built on [`EditableText`].
#[derive(Clone, Debug)]
pub struct TextField {
    pub editable: EditableText,
    pub placeholder: String,
}

impl TextField {
    #[must_use]
    pub fn new(controller: TextEditingController) -> Self {
        Self {
            editable: EditableText::new(controller),
            placeholder: String::new(),
        }
    }

    #[must_use]
    pub fn style(mut self, style: TextStyle) -> Self {
        self.editable = self.editable.style(style);
        self
    }

    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    #[must_use]
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.editable = self.editable.read_only(read_only);
        self
    }
}

fn nearest_char_boundary(text: &str, offset: usize) -> usize {
    if text.is_char_boundary(offset) {
        return offset;
    }
    let mut boundary = offset;
    while boundary > 0 && !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    boundary
}

fn previous_grapheme_boundary(text: &str, offset: usize) -> Option<usize> {
    let offset = nearest_char_boundary(text, offset.min(text.len()));
    (offset > 0).then(|| {
        GraphemeClusterSegmenter::new()
            .segment_str(text)
            .take_while(|index| *index < offset)
            .last()
            .unwrap_or(0)
    })
}

fn next_grapheme_boundary(text: &str, offset: usize) -> Option<usize> {
    let offset = nearest_char_boundary(text, offset.min(text.len()));
    (offset < text.len()).then(|| {
        GraphemeClusterSegmenter::new()
            .segment_str(text)
            .find(|index| *index > offset)
            .unwrap_or(text.len())
    })
}
