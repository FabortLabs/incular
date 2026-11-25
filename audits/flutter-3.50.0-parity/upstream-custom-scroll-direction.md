# Physical axis of a horizontal CustomScrollView

The pinned, authored `custom_scroll_direction_reference_test.dart` runs
a 300×100 viewport with two 250px slivers. The comparisons keep the
same child and controller while switching LTR → RTL → LTR. With no
explicit reverse, the first box's left edge moves 0 → 50 → 0. With
reverse, that sequence becomes 50 → 0 → 50. The paired tests also
check real child hits near both edges and the physical movement after
a positive 30px scroll offset.

With 20px start and 5px end padding, the high-level view's first box
starts at 20 → 30 → 20 (normal) or 45 → 5 → 45 (reversed). A raw
Viewport keeps its requested physical axis: only the padded edge
moves, to 20 → 5 → 20 or 45 → 30 → 45 respectively. Before the repair
Incular resolved the edges but left the high-level RTL scroll axis
pointing right, placing the padded child at x=5 instead of x=30.

The viewport now resolves its physical direction before layout.
The alternate sequence is built only if RTL needs it. Custom sliver
constructors receive the same direction used by the built-in layout.
Only the horizontal high-level CustomScrollView selects that sequence;
explicit axes and vertical main axes are not mirrored by text direction.

Both authored Flutter probes and 71 tests in six focused Rust
binaries passed. Authored probes do not add original registrations to
the audit's denominator or its behavior-compared count.
