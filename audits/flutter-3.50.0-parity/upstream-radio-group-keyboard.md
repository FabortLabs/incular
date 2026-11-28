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
