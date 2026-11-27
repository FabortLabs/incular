# Editable cursor visibility and focus

Five pinned originals in `widgets/editable_text_cursor_test.dart`
passed. The renderer-neutral runtime checks:

- The configured 10×10 cursor with radius 2 reaches the retained rounded
  paint command.
- An explicitly hidden cursor emits no caret paint even while its editor
  has focus, including the three 200 ms checkpoints.
- Blur stops cursor paint and animation demand; requesting focus again
  restores the cursor on the same retained editor.
- Toggling visibility off, on, and off changes the caret without replacing
  the render ID or stealing keyboard focus.
- An initially unset selection in an autofocused scope collapses at the
  end of `hello world`; the focused editor paints the first caret.

All 10 focused runtime caret and focus tests passed. The rounded paint
assertion is not a rendered-pixel golden. Exact repeated blink phase
timing is a separate audit slice.

## Caret blink cadence and timestamped keyboard input

Three exact originals ran, including both desktop key-transit
expansions. The runtime now samples the focused cursor on the frame
clock and schedules the next alpha change on its existing deadline
channel. Previously a wall-clock sample occurred only during an
unrelated repaint; the retained cursor could freeze while idle.

The native tests inspect the actual paint stream before each of
Flutter's discrete opacity keyframes, including the return to full
opacity after 1 s. Non-animated opacity toggles every 500 ms, but
40 successive checkpoints request no continuous animation frames and
perform no new layout. Timestamped left/right presses on the macOS
shortcut path keep the cursor on for another 500 ms; it switches off
on the final edge. Native input normalizes the upstream transport
variants to one key event. An authored case verifies hiding or blur
disarms the timer and showing again restarts the visible phase without
losing focus. All 15 focused runtime and text tests passed. This
validates runtime deadlines and paint commands, not a compositor
screenshot or a separate mobile/web host wake test.

## Deterministic cursor mode

The exact pinned debug-deterministic original passed even with opacity
animation enabled. The native fixture keeps the focused multiline
cursor fully painted through and well beyond an ordinary fade cycle,
without active animation or a pending cursor deadline. Disabling the
flag starts a fresh normal blink without changing focus. Incular
scopes the debug setting to one runtime so tests and unrelated windows
can coexist; Flutter uses a process-wide flag. All 10 focused native
cursor tests passed.

## Vertical placement of explicit caret heights

Both non-Apple originals passed across Flutter's Android, Fuchsia,
Linux, and Windows variants. The native renderer-neutral fixture
uses a 16 px line. A 4×12 caret paints from y=2; a 4×24 caret paints
from y=−4. Previously the tall caret started at y=0 because its
vertical offset was clamped, misaligning it with the text. The same
calculation is used by the IME caret rectangle. All 19 focused
native tests passed (including the Ahem-backed IME geometry tests).
Apple's taller caret convention and native GPU pixels are not
part of this result.

## Local caret geometry and trailing whitespace

Two exact pinned originals passed. Deleting `12` to the empty string
keeps the caret's top and height and shifts its left position inward
with center alignment. The native query reports the actual two-pixel
painted caret, not the existing one-pixel global selection marker.
The native fixture verifies both the local paint and the marker's
projection through an enclosing 8 px translation.

A centered multiline editor with 1,000 trailing spaces now keeps its
13×17 cursor at x=786 on the first line of an 800 px box, with one
pixel left after the caret. Previously the native shaper's trailing
advances put the caret paint at x≈4,828. Painting and the local query
now apply the same wrapped horizontal bound. The single-line path
still allows horizontal scrolling. All 30 focused native cases
passed, including the Ahem-backed IME geometry beforehand and after
the retained cursor change.
