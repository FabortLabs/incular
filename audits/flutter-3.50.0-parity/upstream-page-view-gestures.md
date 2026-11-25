# PageView tap, drag, fling, and resize behavior

All four corresponding pinned Flutter originals passed, including both expanded iOS/macOS variants for overscroll. The focused Rust runtime binary passes three tests and intentionally fails one.

| Registration | Result |
| --- | --- |
| :202 | Actual touch taps reach page 0 then page 1. A -20px swipe briefly displays both pages then settles back to page 0; -401px selects page 1; directional flings select page 2 then page 1. Taps, signed geometry and page selections match. |
| :267 | Under explicit bouncing physics, page 0 stays 800×600 during the first +100px drag and both -200px/-500px reverse sweeps, shifting to positive x then negative x. Flutter provides those sweeps as two framework platform variants; this is not a native OS claim. |
| :361 | A controllerless page view reaches page 2 from a -1250px swipe at 600×400 in both frameworks. Flutter keeps page 2 at 250×100 and then 450×400, while Incular resets to page 0 at the first resize and stays there. This differs from the passing explicit-controller case in the resize report. |
| :485 | During a live drag, crossing the 400px midpoint dispatches page 1 immediately, without waiting for a frame. Later moves and settling do not emit it redundantly; page 1 stays visible. This matches Flutter. |

The tests use synthetic normalized touch samples through the real Runtime dispatcher and monotonic frames. Run cargo test --locked -p incular-runtime --test flutter350_upstream_page_gestures -- --nocapture.

## Snapping switches and outward flings

Two more pinned originals passed. The expanded Rust target has four matches and two expected failures overall.

- :536: a rightward 100px/800px-per-second fling from the first 0.9-fraction page settles back to page 0, emits no callback, and keeps pages 0 and 1 visible at both 800×600 and a 414×736 OverflowBox viewport. This matches.
- :634: the first -420px swipe selects page 1 correctly. Turning snapping off then swiping reports page 1 again and exposes pages [0,1], instead of reporting 2 and exposing [1,2]. Reenabling snapping settles on 0 instead of 2, without an extra notification. The controllerless rebuild lost the active position.

## Retain the controllerless position across rebuilds

After the repair, the pinned `:361` geometry sequence passes on both
sides: page 2 remains selected at 600×400, 250×100, and 450×400.
The implicit controller belongs to the mounted view rather than a
transient descriptor. Last-reported page is scoped to that same view,
including if the caller replaces the callback. An authored comparison
also verifies that reusing one immutable PageView description in two
mounts leaves their positions independent and that a new mount starts
on page 0. This authored comparison does not add a ledger registration.

The first two stages of `:634` now agree: disabling snapping neither
jumps to the beginning nor repeats the last reported page, and the
second swipe selects page 2. Reenabling snapping still leaves pages 1
and 2 partly visible instead of aligning page 2. That assertion remains
red until the position starts the correct settling motion.
