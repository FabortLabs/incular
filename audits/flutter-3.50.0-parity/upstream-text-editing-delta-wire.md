# Services: Flutter text-editing delta envelopes

Pinned Flutter 3.50.0 `ab2912035e160d9cad6306df307576da6128ebdb`.
`services/text_editing_delta.dart` decodes engine replacements, infers
insert/delete/replace/non-text, and applies them to the engine's
`oldText` snapshot rather than rebasing onto possibly stale local text.
Earlier Incular checks derived similar byte deltas after receiving
the whole replacement value; they did not exercise this wire behavior.

The new `incular::text::FlutterTextEditingDelta` keeps the public wire
offsets in UTF-16, preserves selection direction and affinity metadata,
and validates both old and resulting ranges before accepting the payload.
Application converts to the existing UTF-8 editing value. An unset
selection is retained on the envelope and materializes as Incular's end
caret on a standalone editing value. Split surrogate pairs cannot be
represented in Rust strings, so such edits return a typed error. The
native `TextEditingDelta` and ordinary platform IME paths are unchanged.

Twelve exact reference registrations passed: :13, :40, :68, :110,
:138, :166, :206, :235, :264, :293, :336, :361. The paired native test
checks classification, old/removed text, offsets, resulting selection
and composition, and rejects the four invalid applications. Flutter
uses debug assertions there; Incular returns a typed range error and
does not modify state. Seven new Rust tests and twelve adjacent
Rust text-state/delta tests pass. Two neutral Flutter Unicode probes
also pass; they confirm UTF-16 coordinates, direction/affinity, and the
last-write-wins snapshot.

The four original `debugFillProperties` tests passed only in Flutter.
A Rust Debug derive is not a Flutter diagnostic-property builder. This
slice does not claim opt-in `flutter/textinput` channel registration or
native adapter delivery to a `DeltaTextInputClient`.
