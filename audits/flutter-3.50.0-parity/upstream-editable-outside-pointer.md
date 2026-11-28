# EditableText outside pointer callbacks

Pinned Flutter: `3.50.0` (`ab2912035e160d9cad6306df307576da6128ebdb`).

The four originals in `widgets/editable_text_test.dart:18562,18599,18635,18672`
ran in the reference. Their native counterparts mount the editable control
and dispatch real down/up samples through the input router. Three outside
taps deliver exactly three notifications for each focused phase under
Android touch policy, but never notify an unfocused field. One inside tap
does not notify. The tests check live focus, global position, device kind,
and the phase on which each callback fires.

A supplied outside-down callback replaces desktop's default blur. An
outside-up callback is independent and only fires if focus still exists.
The authored `editable_outside_pointer_group_test.dart` and native Linux
mouse control verify that the shared text field group is treated as inside
and a custom callback keeps the editor focused on an actual outside press.
The behavior belongs to EditableText, not to Material or Cupertino.

When a pointer crosses the boundary before release, Flutter classifies the
release using the press route but delivers the release's actual position.
The same authored probe and the native crossing test verify both directions.
A touch release intentionally avoids a fresh hit test in Incular; using an
empty fresh result here had falsely labeled even a direct inside tap as
outside. The retained route now carries the editable-group membership.
Six focused native editable tests and eight adjacent tap-region tests passed.

## EditableText group identity

Pinned group ID originals `editable_text_test.dart:378,412` passed.
Three mounted native editors retain their own named group overrides,
while three unconfigured editors all use the common EditableText
group. The descriptor now exposes the typed group and the retained
editor uses it during mouse dispatch: only a matching companion in
the same TapRegionSurface is inside. Unrelated text-field companions
no longer prevent default desktop blur.

The authored Flutter probe and native test cross the boundary in
both directions. Starting in the matching group produces no outside
callbacks even when released elsewhere. Starting in a foreign group
produces both outside notifications even when released over a
matching companion. Native contacts capture group identity at press
time, with no extra allocation for the common default-only group.
All 50 focused editor, tap-region, menu and handle tests passed.
