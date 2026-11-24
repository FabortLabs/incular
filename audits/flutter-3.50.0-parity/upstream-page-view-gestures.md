# PageView tap, drag, fling, and resize behavior

All four corresponding pinned Flutter originals passed, including both expanded iOS/macOS variants for overscroll. The focused Rust runtime binary passes three tests and intentionally fails one.

| Registration | Result |
| --- | --- |
| :202 | Actual touch taps reach page 0 then page 1. A -20px swipe briefly displays both pages then settles back to page 0; -401px selects page 1; directional flings select page 2 then page 1. Taps, signed geometry and page selections match. |
| :267 | Under explicit bouncing physics, page 0 stays 800×600 during the first +100px drag and both -200px/-500px reverse sweeps, shifting to positive x then negative x. Flutter provides those sweeps as two framework platform variants; this is not a native OS claim. |
| :361 | A controllerless page view reaches page 2 from a -1250px swipe at 600×400 in both frameworks. Flutter keeps page 2 at 250×100 and then 450×400, while Incular resets to page 0 at the first resize and stays there. This differs from the passing explicit-controller case in the resize report. |
| :485 | During a live drag, crossing the 400px midpoint dispatches page 1 immediately, without waiting for a frame. Later moves and settling do not emit it redundantly; page 1 stays visible. This matches Flutter. |

The tests use synthetic normalized touch samples through the real Runtime dispatcher and monotonic frames. Run cargo test --locked -p incular-runtime --test flutter350_upstream_page_gestures -- --nocapture.
