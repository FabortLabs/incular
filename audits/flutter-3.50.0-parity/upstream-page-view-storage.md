# PageView storage, semantics and numeric boundaries

All six associated pinned Flutter originals passed. The focused Rust target now passes eight checks.

- :413 now pairs via the awaitable variants described below.
- :953 partially matches. Page 2 is restored, an independent keep-offset-disabled controller starts on 0, and a detached controller exposes no page. Flutter asserts where Incular returns None. A fresh controller reads page 2 from the same bucket.
- :1030 fully matches. At selected pages 0, 1, 2 the genuine framework semantics tree exposes only the corresponding Page # label.
- :1069 matches. A PageMetrics snapshot yields page 6; copying it with pixels reduced from 150 to 50 yields page 4 after clamping to [100,200]. The original stays unchanged.
- :1084 partially matches; converting the exact Dart 799.99999999999 input to Rust's f32 coalesces to 800, so the retained controller reports 1. This does not independently verify Flutter's f64 near-boundary tolerance.
- :1104 matches. The scroll semantics node advertises implicit scrolling. The visible page, hidden adjacent labels, and absence of uncached labels agree at pages 0, 1, and 2 after two 150ms Ease animations. This is a framework semantics check, not a native screen reader run.

Run cargo test --locked -p incular-runtime --test flutter350_upstream_page_storage_semantics -- --nocapture.

## Restore pages rather than pixels

The repaired Rust `:953` behavior no longer reports obsolete page
metrics after detachment. Flutter asserts when detached; Incular uses
`None`. A remount reads the logical page from PageStorage, and changing
the typed PageStorage key distinguishes its mounted widget identity.

Both frameworks pass an authored width/fraction comparison: page 2
saved at 600px under a 400px, 0.75-fraction viewport is restored at
500px under a 500px, 0.5-fraction viewport. Disabling keep-page starts
at configured page 1 and does not overwrite the saved page, even
after scrolling to page 3. A Rust-only control verifies an explicit
pre-layout request takes precedence over the saved page; Flutter
rejects that request timing instead.

## Page metrics carried by notifications

An authored counterpart checks a 200-pixel viewport at fraction 1.25.
At offset 425, the live position reports centered page 1.6 and its
update notification reports snapshot page 1.7. Copying the notification
snapshot to offset 500 yields page 2 without mutating the original
or the live position. These are distinct calculations in Flutter.
The snapshot check does not verify native device pixel ratio propagation.

## Awaiting a driven page animation

The pinned :413 now pairs through Rust's existing
`PageController` plus the additive `next_page_async`,
`previous_page_async`, and `animate_to_page_async` methods. An
attached controller returns an awaitable completion while the
boolean methods remain available. Both forward and backward 150 ms
motions remain unfinished after their first 200 ms frame and
complete after their second. The native comparison polls real Rust
futures and checks exactly one wake and the resulting page each time.

An interrupted motion resolves its own completion without reporting
that it reached the requested page. A zero-width viewport returns an
already-completed future for a target that will be applied on resize.
Both behaviors passed authored Flutter and native checks. Native
additional controls cover a queued request replaced before its
first frame, an active request replaced by another one, cancellation
of queued motion by a same-position jump, and viewport teardown.
Detached Rust controllers return `None`, rather than asserting.

Forty-five focused native animation, controller, ownership, and attachment tests passed.
