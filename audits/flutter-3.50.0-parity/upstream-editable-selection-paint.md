# Retained EditableText selection paint

## Explicit selection is independent of focus

The pinned neutral probe `editable_selection_paint_policy_test.dart` paints
a valid controller selection in the raw EditableText before gaining focus,
while focused, and after losing focus. A collapsed selection paints no
highlight. No design-system widget or handle decoration is involved.

The old Incular renderer gated a configured selection color on focus.
With the FlutterTest font and the exact original :336 source range
`abc\nabcdefghij`, selecting the first line plus the newline produced no
highlight before the fix. It now paints only the 30×10 rectangle over
the first line without requesting focus. Caret, prompt, and interactive
handles keep their existing separate focus rules. All 32 focused native
editable and paragraph tests passed. The authored focus probe adds no
original registration and the explicit selection style is tracked
separately.

## Height and width styles at the actual highlight

The exact :336 case pairs with an explicitly Tight retained editor:
selecting the first hard break in `abc\nabcdefghij` paints (0, 0)–(30, 10)
without extending into the longer next line. The neutral reference also
switches the same editor to Max and paints the separate extension
(30, 0)–(100, 10), without changing its size or focus. Incular forwards
both styles to its real paint commands, without invalidating layout.

The native default uses middle line spacing, while WebAssembly uses
Max, matching the Flutter widget's platform default; width defaults
to Tight everywhere. The neutral probe and native test also compare
the three default and Max-height boxes across hard breaks. Both
whole-document and segmented long-editor layout produce the same
coordinates; segmentation does not restart the first/last-line rules.
Three authored probes, 34 focused widget tests and 55 painter tests
passed; the existing separate Unicode separator case remains ignored.
The themed selection goldens at :297 and :9728 remain open.

## Select All and private editor scroll

Pinned :13726 passes on both iOS and macOS. The native counterpart uses
the same 64 lines, two-line field and initial 0→1 range. Before the fix,
toolbar Select All moved the private iOS editor viewport from zero
to its maximum (1184 px in the native fixture). Suppression needed
both automatic paths: the enclosing-viewport reveal and the text
painter's private scroll. The fixed editor selects all without moving
after the command or on the next frame. A later collapsed caret moves
into view normally.

The non-Apple native controls still reveal the end on toolbar Select All,
while an explicit keyboard cause preserves the original offset. The
pinned Android companion also passed; its other clipboard observations
are tracked separately. All 20 focused native editor, clipboard and
viewport tests passed.
