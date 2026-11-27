# Native system font notifications

Pinned Flutter `painting/system_fonts_test.dart:40,66,215` passes.
Incular now accepts the same `fontsChange` system message. Its mounted
paragraphs and raw editable text remain clean when the message arrives,
then are marked dirty at the next frame's transient boundary. They
relayout and repaint without rebuilding their widgets. A notice that
arrives during transient callbacks leaves the current text layout valid
and is applied on the following frame, not during the current layout.

Successful local and application font registrations are restored in
their original order when the platform collection is rebuilt. Existing
layout handles remain readable; two unprocessed notices refresh the
collection once. An engine that opted out of system faces is unaffected,
as are non-text render objects. Sixteen focused checks passed, including
five existing platform memory-pressure tests and four package-font
checks. These tests exercise explicit native notification delivery; they
do not certify automatic detection by each operating-system adapter.

Pinned :46 and :73 also passed, but these are not counted. The native
suite checks that previous text measures stay usable before refresh, not
that a custom render mixin exposes the same interface as :46. Incular's
Banner is repainted when applying the queued generation, rather than
proving Flutter's immediate dirty flag at notice receipt as :73 asks.
Design-system controls stay out of scope.
