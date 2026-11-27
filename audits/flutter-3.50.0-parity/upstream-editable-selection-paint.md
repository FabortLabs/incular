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
