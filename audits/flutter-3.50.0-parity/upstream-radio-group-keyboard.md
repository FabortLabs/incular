# RawRadio keyboard traversal and group boundaries

The Flutter 3.50.0 originals in `widgets/radio_group_test.dart` at
128, 182, and 235 were replayed in a mounted Incular runtime with
real focus nodes and controlled values. Arrow keys follow the focus
reading order, wrap in both directions, and skip disabled clients.
Tab enters on the first eligible radio when unset and exits on the
very next Tab. After selecting a different radio, Shift+Tab from
the trailing editable field returns to that selected radio rather
than the initial tab stop.

Radio registrations live with their mounted controls and use the
real transformed focus bounds. Sequential traversal and raw arrow
dispatch share the typed group without forcing non-radio controls
into that group's selected state. Existing tap, Space toggle and
focus-retention tests still pass.

All five selected Flutter originals ran. Three are paired by this
slice; the non-radio interception and zero-area originals need
their own matching native controls.

## Focused shortcuts and registration lifetime

The remaining seven pinned registrations are paired. Keys bound by
the parent are suppressed while a correctly typed radio has focus,
then pass to the parent unchanged while the actual editable field
inside the group has focus. The standalone parent callbacks observed
all five expected keys, and the focused radio took over again on
return. Logical key identity takes priority; unidentified native
keys can still use the physical code.

Mounted control fixtures reproduce checked, enabled and mutually
exclusive states after each pointer tap, group separation from
surrounding checkboxes, duplicate-value diagnosis only after that
value is selected, and reducing the child count without reviving
the removed entry during arrow wrap. Two independent RawRadio
registries at zero area also lay out without error. The duplicate
condition uses a typed retained semantics report rather than a Dart
exception. All ten originals in `radio_group_test.dart` are now
paired, and nine focused native runtime tests passed.
