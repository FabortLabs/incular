# Scribble client registration and queries

Three pinned services originals pair. The local registry tracks the
host beginning and ending an indirect handwriting session. A focus
request reaches only its registered identifier, with the original
coordinates. A query returns only overlapping mounted targets, in
registration order, with the exact identifier and rectangle fields
in the JSON success envelope.

Zero/nonfinite rectangles cannot become handwriting targets.
Replacing an identifier retains its position; unregistering it
prevents later focus messages. Four native registry tests and
twelve neighboring wire-error tests passed. This verifies the
optional host registry, not an iPad device interaction or automatic
registration by Incular's iOS adapter.
