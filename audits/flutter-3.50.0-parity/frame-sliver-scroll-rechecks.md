# Further sliver and scroll frame comparisons

Pinned Flutter 3.50.0 `ab2912035e`, Incular production `c647cb85`.
Scratch batches: `20261124-161738-215` and `20261124-161928-324`.

| Scenario | Outcome | Frames |
| --- | --- | ---: |
| `slivers__pinned_header_and_main_axis_group__scroll` | match | 6 |
| `slivers__cross_axis_group__lanes` | match | 3 |
| `slivers__list_builder__default_cache_window` | match | 4 |
| `slivers__floating_header__default_snap_after_wheel_scroll` | match | 35 |
| `scroll__controller__animate_activity_notifications` | match | 10 |
| `scroll__drag__notification_order` | mismatch | 63 |
| `scroll__fling__notification_order` | mismatch | 96 |
| `scroll__draggable_sheet__drag_extent` | match | 47 |

The drag and fling fixtures matched offsets and other sampled notification
types. Both miss UserScroll, already reproduced by the permanent
`flutter_user_scroll_notification_parity` target. First differences:
drag frame 10 at 160 ms, fling frame 3 at 48 ms, expected 1 vs Incular 0.
These add different gestures to the same confirmed notification gap.

Two additional raw runs await fixture review, not included above:
KeepAlive differs on retained offscreen geometry (mounted on both);
PageView differs only on old page probe presence from 704 ms.
Neither is counted as a valid pair until ownership and paint contracts
are made explicit.

For the earlier extreme `scroll__controller__jump_clamp` mismatch,
the reported Flutter 97856.2010603 narrows **exactly** to
97856.203125 as f32, the observed Incular value. Preserve the strict
scalar result, but do not treat it as a separate observed scroll motion
bug or assert a target finer than its public numeric representation.

## Accepted user-drag direction changes

The two native touch fixtures originally emitted Start followed immediately
by Overscroll or Update. They now publish the direction event before that
offset sample, once per direction, and keep it through the ballistic
continuation. A terminal End is followed by a zero-delta idle event when
the drag had an active direction. An older activity does not send idle
into a replacement bracket, and a reentrant takeover in a direction
listener prevents the old drag from writing.

Two pinned authored controls pass: steady leading-edge clamping followed
by a reversal, and a bouncing fling that retains direction until it
settles. Both pre-existing native touch regressions pass, as do the nine
SliverFillRemaining drag/semantics checks and focused ownership/coordination
suites. The strict existing drag and fling frame registrations are being
rechecked separately; this targeted repair does not itself change them.

## Strict drag and fling recheck

Fresh pinned Flutter and Incular runs agree for all 63 drag and 96
fling frames. The offsets and cumulative Start, Update, Overscroll,
End, and UserScroll registers now match. Both existing frame
registrations move to match. The Linux runner uses a scratch package
and executes only the selected existing scenarios.

## Pointer-wheel notification order

A native wheel signal is not an accepted touch drag. Pinned Flutter
and both focused Runtime probes now clamp the wheel even under
BouncingScrollPhysics. An outward wheel shift at a pinned extent does
not publish a bracket. A moving signal publishes its direction with
the old pixels, then Start, Update, End and the idle direction with
the committed pixels. A partial clamp consumes 60px of a 90px
return without publishing a touch Overscroll event. The independent
nested remainder policy is unchanged.

A newer activity opened during the direction callback owns its own
Start/End. The superseded wheel sample does not overwrite its offset
or terminate the replacement. The touch direction tests and eleven
neighboring pointer-input tests also pass.
