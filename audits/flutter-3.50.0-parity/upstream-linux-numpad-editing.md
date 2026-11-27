# Linux NumLock and text editing

The pinned locked and unlocked Linux keypad registrations both passed.
They use an action spy outside the editable target and assert intent
types and flags. The native comparison checks actual input and
caret/selection results, not Flutter's non-editor action-spy path, so
both remain partial.

An additional paired Flutter probe toggles NumLock with physical key
events. With NumLock on, keypad 6 without Shift leaves the caret at
3, while Shift extends to 4. After toggling off, keypad 6 moves the
caret to 4 without Shift; Ctrl+keypad moves or deletes a full word.
Incular previously moved the caret on the first unshifted key despite
its hardware lock state being on. Both the editable and read-only
selectable paths now use the observed lock state for shortcut
resolution. The original physical key event is unchanged, so callbacks
that bind the original flags still work.

The 19 native text-shortcut checks and seven focused lock/shortcut
regressions passed.
