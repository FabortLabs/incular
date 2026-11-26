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

## Real keyboard scroll actions

The four pinned registrations in `widgets/scrollable_helpers_test.dart`
(:233, :289, :348, :419) passed with both Flutter key transit variants.
The Rust comparison sends modifier down, arrow down/up, and modifier
up to the Runtime, then settles the scroll animation. It checks the
same initial offset and complete box rectangles, horizontal 50px line
moves in both directions, and vertical reversed PageUp clamped to the
400px end and PageDown back to the beginning. Its focus target is in
the same viewport; the reversed controls use the first autofocus
wrapper rather than sharing a single FocusNode among all twenty rows.
The portable Runtime receives normalized key events and does not try
to reproduce Flutter's two deprecated transport APIs.

Incular initially applied the global text direction again to an
already-resolved physical axis. A local LTR view in an RTL environment,
a local RTL view in an RTL environment, and an explicit reversal could
all stay at zero when Flutter moved by 50. Keyboard actions now use the
mounted viewport's physical axis. The original Linux key paths and
additional mismatched ambient/global controls pass.

## Keyboard physics and a centered viewport

The selected originals :101, :166 and :475 passed both pinned Flutter
key transit variants. The same line/page shortcut sequence leaves a
never-scrollable viewport at offset zero and places its first 50px row
unchanged. The normal viewport moves by a 50px line, returns to zero,
clamps a 480px page request to its 400px scroll range, and returns
again. The authored `scroll_keyboard_physics_reference_test.dart` also
mounts actual WidgetsApp default shortcuts (not just Directionality)
and confirms a direct controller jump to 80px remains allowed under
never-scrollable physics.

A keyboard action previously bypassed the mounted scroll physics even
when pointer input was disabled. The keyboard action now checks the
same user-input eligibility encoded by its resolved physics before
animating. The focused viewport still claims the shortcut. Across a
centered viewport, the configured center item's signed offset matches
through 0 → 400 → −100 after ten downward and ten upward line actions;
its retained rectangles match at each checkpoint.

The direct intent original :546 passed in Flutter but remains open:
there is no equivalent first-class public ScrollIntent/ScrollAction
invocation in Incular. The keyboard fallback and controller animation
are not substitutes for that API.

The wider Runtime binary showed 200 passing and 15 failing checks
both immediately before and after the keyboard direction change,
with identical failing test names. The existing failures in editing,
focus invalidation, fractional placement and routing do not change
the targeted keyboard results.

Thirty-two targeted Rust tests passed across the keyboard, focus
routing, traversal and scroll-physics binaries.

## Native keyboard modifiers on Apple and Linux

The authored `scroll_keyboard_platform_reference_test.dart` runs
the native default shortcuts with the pinned platform override for
iOS, macOS, and Linux. An arrow with the other platform's modifier
does nothing; Command+arrow on Apple or Control+arrow on Linux moves
a 50px line. Unmodified page keys still clamp to 400px and return to
zero. The original Rust iOS comparison moved 50px for Control; the
Runtime now chooses the native shortcut table's modifier. All three
Flutter variants and 22 targeted Rust tests passed. This doesn't
count as an additional original registration or verify web defaults.

## ListView, GridView and PageView horizontal axes

The authored `box_scroll_direction_reference_test.dart` passes for all
three high-level views with explicit reverse on and off. It keeps the
same first child while switching LTR → RTL → LTR, checking its full
paint rectangle and a pointer hit at offsets zero and 30. Reverse
placement mirrors a 250px list/grid item to x=50 and a 240px
unpadded fractional PageView page to x=60. Before the fix, all three
stayed at x=0 under ambient RTL without explicit reverse.

All three now use the common resolved horizontal axis without
recreating their built-in sliver delegate. Raw viewports retain an
explicit axis and the custom-sliver constructor path still receives
physical reverse. Five focused widget binaries passed (67 tests),
as did the nine PageView cache controls. The separate known PageView
eviction failure after a partial jump remains open; see
[the prior retained-lifecycle recheck](frame-retained-text-implicit-rechecks.md).

## Programmatic scroll intents

The pinned original :546 now pairs. Invoking two native `ScrollIntent`
page actions from a mounted list child moves the viewport from 0 to 480
to 960 logical pixels, with the exact intersecting page combinations.
No keyboard event is synthesized. The default keyboard path also goes
through the same typed action dispatcher. A nearer authored handler
shadows the built-in action rather than scrolling as well.

The primary-controller fallback from a sibling and the physical
direction/cross-axis behavior on a reversed viewport passed on both
pinned Flutter and native. Four native tests and all three pinned
comparisons passed. Custom increment calculators remain separate.
