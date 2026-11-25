# RenderBaseline signed placement and real metrics

Both selected pinned registrations in `rendering/baseline_test.dart`
passed. The retained equivalent of `:13` keeps a 100×100 child
and updates the requested baseline through [0,25,90,100,110].
For each state it checks the parent's exact height and the child's
horizontal and vertical offsets. The child offsets are
[-100,-75,-10,0,10] vertically; before the fix Incular clamped
the first three to zero and grew the wrapper to 100px.

The bottom edge of a baselineless child positions it but is not
an actual baseline to pass to an ancestor. An authored pinned
probe accepts a negative target for a real text baseline, then
retains the wrapper while replacing the text with a plain 100px
box. The plain box moves to -105px, the wrapper height becomes
zero, and no real baseline remains. The Rust fixture checks the
same retained geometry, plus the directly reported actual
alphabetic metric. `:56` remains partial: this checks clearing
the alphabetic metric, not Flutter's distinct ideographic metric
or the direct dry-baseline interface.

The existing LayoutBuilder boundary cases now use a real text
baseline instead of relying on a synthetic baseline from a
baselineless wrapper. They still verify direct, generated, and
cleared placement, including the pinned 160px generated case.
The nested Flex and IndexedStack baseline regressions also pass.

The neighboring loose OverflowBox default case is a separate
existing failure: Incular shrink-wraps instead of filling.
