# text: Text and editing

## Summary

The audit confirms rendering and layout gaps in rich spans, WidgetSpan placeholders, and inherited `DefaultTextStyle.maxLines`. Editing differs in offset units and initial selection state; Incular also applies user-edit transforms and callbacks to programmatic controller changes, and its EditableText path does not select a word on a long press. Source inspection also shows that Incular paints a caret for non-collapsed selections while Flutter gates the blinking caret on a collapsed selection. The default blink half-period matches at 500 ms in both implementations, but the harness cannot compare cursor paint timing. The frame recorder cannot compare glyph paint, so no pixel runs were made. The rest of the text and editing surface remains only partially audited.

## Coverage

| Flutter item | Incular counterpart | Method | Scenarios | Result |
| --- | --- | --- | --- | --- |
| RichText and nested TextSpan styles | RichText conversion and text layout | source + frame twin | `text__rich_span_style_metrics` | divergent: 220×36 Flutter, 220×17 Incular |
| WidgetSpan placeholder layout and paint | WidgetSpan flattening and RichText conversion | source + frame twin | `text__widget_span_layout` | divergent: 220×40 Flutter, 220×19 Incular; inline child fill is absent |
| DefaultTextStyle maxLines inheritance | DefaultTextStyle and Text descriptor | source + missing-API diagnostic twin | `text__default_text_style_max_lines` | missing API: Flutter sets inherited maxLines=1; Incular cannot express it |
| TextEditingValue selection offsets | TextSelection and TextRange | source + model-value frame twin | `text__editing_selection_offsets` | API mismatch: Flutter length/extent 3, Incular 5 for `😀x`; host round-trip unchecked |
| EditableText initial selection and first focus | TextEditingController and EditableText focus handling | source + frame twin | `text__editable_initial_selection_focus` | divergent before focus: Flutter invalid at -1, Incular valid at 3; tap moves both to a valid end caret |
| Programmatic controller update with input formatter and onChanged | EditableText controller and formatter path | source + frame twin | `text__editable_programmatic_controller_update` | divergent: Flutter stores `a1b2` and calls onChanged 0 times; Incular stores `12` and calls on_changed once |
| Long-press word selection | EditableText selection gesture path | source + frame twin | `text__editable_long_press_word_selection` | divergent: Flutter selects `[0,5]`; Incular leaves a collapsed caret at 1 |
| Caret visibility during a range selection | EditableText cursor and retained text-field painter | source reading | none | divergent: Flutter gates blinking on collapsed selection; Incular's painter does not |
| Default caret blink interval | EditableText cursor timer and Incular caret opacity | source reading | none | both use 500 ms per on/off half-cycle by default; visual timing not frame-verified |
| Other plain Text options (alignment, wrapping, explicit max lines, overflow, scaling, height behavior) | Text descriptor and TextEngine | source skim only | none | not checked against Flutter |
| Remaining editing and selection lifecycle (IME composition, affinity, double-tap/drag selection, cursor geometry, clipboard, semantics) | EditableText and text controller | not checked | none | not checked |

## Findings

### text-001 — RichText drops styles after the first flattened run

- Severity: P1
- Category: paint
- Flutter: `painting/text_span.dart:286-320` pushes each span's style, adds its text, then recursively builds each child into the paragraph.
- Incular: `crates/incular-text/src/spans.rs:322-334` lays out all plain text with the first flattened run's style; `crates/incular-widgets/src/tree/descriptors.rs:656-668` does the same when converting RichText into a retained widget.
- Evidence: source reading and the `text__rich_span_style_metrics` frame twin. A 14 px first run followed by a 30 px run measures 220×36 in Flutter and 220×17 in Incular.
- Impact: child colors, weights, decorations, and font sizes are lost. Larger or smaller child sizes also change line metrics and wrapping.
- Likely cause: flattening correctly retains styled runs, but both layout paths collapse the run list to its first style.
- Fix direction: shape and paint styled runs together in one paragraph, including their run ranges and paragraph-wide layout.
- Confidence: high

### text-002 — Text editing offsets are API-incompatible with Flutter's UTF-16 units

- Severity: P2
- Category: input
- Flutter: `services/text_boundary.dart:16-24` defines text boundary positions as offsets into UTF-16 strings; `services/text_input.dart:954-965` carries selection and composing ranges in `TextEditingValue`.
- Incular: `crates/incular-text/src/editing.rs:18-20,65-71,160-165` documents and computes byte offsets with `String::len`; `crates/incular-core/src/input.rs:268-275` also defines IME preedit selection in UTF-8 byte offsets.
- Evidence: source reading and the `text__editing_selection_offsets` model-value frame twin. For `😀x`, Flutter's controller reports text length and selection extent 3; Incular's controller reports 5.
- Impact: callers porting Flutter selection or composing offsets cannot pass those values directly to Incular's editing API when text contains non-ASCII characters. This audit did not verify an error in native cursor or input behavior.
- Fix direction: if Flutter-compatible offsets are intended at the public editing boundary, convert UTF-16 indices to byte offsets internally and convert back when reporting values. Otherwise document the unit prominently at every API and adapter boundary.
- Confidence: high

### text-003 — WidgetSpan children are replaced by plain text during layout

- Severity: P2
- Category: layout
- Flutter: `widgets/widget_span.dart:157-176` submits a measured placeholder size and alignment to the paragraph builder; the WidgetSpan child is laid out as inline content.
- Incular: `crates/incular-text/src/spans.rs:13-17,163-169,192-204` stores only a widget identity and size, then emits U+FFFC as a text run; `visitor_placeholder` at `spans.rs:241-249` is a no-op. `crates/incular-widgets/src/tree/descriptors.rs:656-668` converts only `plain_text()` and one style into a Text widget.
- Evidence: source reading and the `text__widget_span_layout` frame twin. A 40×40 inline child yields 220×40 in Flutter and 220×19 in Incular; Flutter's blue child fill is also absent on the Incular side.
- Impact: inline icons and other widgets disappear, and the line no longer reserves their width or height.
- Fix direction: pass inline child identity and dimensions through paragraph layout, then compose and paint the child at its placeholder position.
- Confidence: high

### text-004 — DefaultTextStyle has no inherited maxLines policy

- Severity: P2
- Category: missing-api
- Flutter: `widgets/text.dart:63-72` exposes paragraph policy on DefaultTextStyle, and `text.dart:717-790` resolves Text's `maxLines` from that inherited value.
- Incular: `crates/incular-widgets/src/environment.rs:228-254` stores only the inherited TextStyle. `crates/incular-widgets/src/tree/descriptors.rs:563-584,638-649` initializes Text with `max_lines: None` and lowers that value directly; `crates/incular-widgets/src/tree/rendering/lowering/visual.rs:69-95` resolves no inherited max-lines policy.
- Evidence: source inspection is primary. The `text__default_text_style_max_lines` frame diagnostic uses Flutter `DefaultTextStyle(maxLines: 1)` and Incular's closest available `DefaultTextStyle`, which carries only style. This is a non-equivalent configuration because Incular cannot express inherited `maxLines`; it measures 220×17 in Flutter and 220×34 in Incular.
- Impact: applications cannot set one inherited line cap for an Incular text subtree; they must configure each Text widget that needs a cap.
- Fix direction: add paragraph policy fields to the ambient default text configuration and resolve them when a Text widget has no explicit value.
- Confidence: high

### text-005 — Text controllers with initial text begin in different selection states

- Severity: P2
- Category: input
- Flutter: `widgets/editable_text.dart:221-245` constructs `TextEditingController(text: ...)` with the default invalid selection; `services/text_input.dart:960-965` defines that default as offset -1. `editable_text.dart:4752-4791` places the caret at the end when an invalid selection is focused.
- Incular: `crates/incular-text/src/editing.rs:158-167,314-318` creates `TextEditingValue::new` with a collapsed caret at `text.len()` and uses it for `TextEditingController::with_text`.
- Evidence: source reading and the `text__editable_initial_selection_focus` frame twin. Before focus, Flutter reports invalid selection at -1 while Incular reports a valid selection at 3 for `abc`. After tapping the field, both report a valid collapsed end caret.
- Impact: code that reads a controller before its field receives focus observes different selection state. The unfocused controller mismatch does not itself establish a painted-caret difference.
- Fix direction: represent an unset selection separately from a collapsed caret, then normalize it to the text end when the field gains focus; otherwise document this constructor difference.
- Confidence: high

### text-006 — Edit transforms and on_changed run for programmatic controller updates

- Severity: P0
- Category: behavior
- Flutter: `widgets/editable_text.dart:266-282` updates `TextEditingController.text` directly. Input formatters and `onChanged` are invoked in `_formatAndSetValue` at `editable_text.dart:4516-4591`, the user-edit path.
- Incular: `crates/incular-widgets/src/tree/reconciliation.rs:31-60` registers `edit_transform` and `on_changed` as controller-wide transform and listener subscriptions. `crates/incular-text/src/editing.rs:418-420,822-876` runs transforms and notifies listeners on `set_text` too.
- Evidence: source reading and the `text__editable_programmatic_controller_update` frame twin. Setting `controller.text = 'a1b2'` leaves that text intact and does not call Flutter `onChanged`; setting Incular's controller to the same text applies the digits transform, producing `12` and one callback.
- Impact: application-driven controller updates can be filtered or reported as user edits in Incular.
- Fix direction: keep formatter and on_changed callbacks on user edit paths, separate from direct controller value updates.
- Confidence: high

### text-007 — EditableText long press does not select a word

- Severity: P1
- Category: input
- Flutter: `widgets/text_selection.dart:2720-2753` selects a word or position on single long tap by platform; `text_selection.dart:3385-3390` installs those long-press handlers on the text selection detector.
- Incular: the text-field pointer path in `crates/incular-runtime/src/frame.rs:1443-1515` handles pointer down as a tap; `crates/incular-runtime/src/frame/text_gestures.rs:531-577` handles single, double, and triple tap counts but has no text-field long-press selection path.
- Evidence: source reading and the `text__editable_long_press_word_selection` frame twin. Holding touch at the same position for 600 ms selects Flutter range `[0,5]`; Incular leaves the collapsed caret at offset 1.
- Impact: users cannot select a word or reach the selection toolbar through touch-and-hold in Incular text fields.
- Fix direction: connect the EditableText pointer path to the long-press recognizer and select a word or position according to platform behavior.
- Confidence: high

### text-008 — Incular paints a caret during non-collapsed selection

- Severity: P2
- Category: paint
- Flutter: `widgets/editable_text.dart:4630-4635` includes collapsed selection in `_showBlinkingCursor`; `editable_text.dart:4723-4728` stops cursor blinking otherwise.
- Incular: `crates/incular-widgets/src/tree/painting/behavior/content.rs:198-217` paints selection rectangles, then paints the caret whenever the field is focused, `show_cursor` is true and the caret opacity is nonzero. Selection updates reset that opacity at `crates/incular-widgets/src/tree/text.rs:587-605`.
- Evidence: source reading. The frame recorder cannot capture the caret glyph, so no frame or pixel result is claimed.
- Impact: a highlighted text range can display a blinking insertion caret at its extent in Incular while Flutter hides the caret.
- Fix direction: gate caret paint on a collapsed selection and stop the blink while a range is selected.
- Confidence: medium

## Pixel scene proposals

The frame twins cover line metrics, placeholder allocation, and selection values, but the frame recorder cannot establish whether individual styled glyphs or the caret paint correctly. A future pixel scene can compare a two-color/two-size span and a WidgetSpan child against the same loaded Inter font. The caret comparison remains an open harness limitation: it needs the same editable selection range on both sides, a focused field, and deterministic cursor visibility. Flutter exposes `EditableText.debugDeterministicCursor`, but it still needs focus and does not override the collapsed-selection check (`widgets/editable_text.dart:1750-1754,4630-4635`). Incular's painter reads `Instant::now()` and needs a focused text field with a nonzero caret reset (`crates/incular-widgets/src/tree/painting/behavior/content.rs:198-217`; `crates/incular-text/src/editing.rs:659-707`). The current pixel runners mount static scenes and render them without driving focus or selection (`harness/flutter/lib/main.dart:244-261`; `harness/incular/tests/pixels.rs:607-645,699-706`; the Rust window-event handler is empty at `pixels.rs:705-706`). Because neither side has a matched deterministic setup in this harness, no executable caret snippets or pixel captures were added.

## Harness requests

The seven deterministic frame scenarios are in `harness/flutter/test/frames/text_test.dart` and `harness/incular/tests/frames/text.rs`. They reuse the current recorder and isolate layout, editing-value, controller-update, and long-press differences. No recorder or pixel harness changes were needed.

## Open questions

The source and behavior of plain Text alignment/wrapping/explicit max-lines/overflow/scaling, text style property merging, font fallback, line-height behavior, grapheme boundaries, bidi caret affinity, keyboard selection movement, double-tap/drag selection, IME composition/formatting, obscuring, input actions, and text semantics still need independent review. Default blink interval was compared in source, but cursor timing was not run because the painter reads wall-clock time and the frame recorder cannot observe glyph opacity. A matched caret pixel test remains open until the pixel harness can set focus and selection and give Incular deterministic caret time/visibility. The frame twins do not verify whether host text-input paths translate selection and composing offsets. The findings above do not establish parity for these unchecked areas.
