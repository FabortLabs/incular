# PageView storage, semantics and numeric boundaries

All six associated pinned Flutter originals passed. The focused Rust target now passes seven checks.

- :413 is reference-only. The Rust next_page / previous_page surface returns a bool, not a future; use the separate controller control to verify actual transition results, not completion handles.
- :953 partially matches. Page 2 is restored, an independent keep-offset-disabled controller starts on 0, and a detached controller exposes no page. Flutter asserts where Incular returns None. A fresh controller reads page 2 from the same bucket.
- :1030 fully matches. At selected pages 0, 1, 2 the genuine framework semantics tree exposes only the corresponding Page # label.
- :1069 partially matches. The arithmetic helper produces [6,4] for requested pixels [150,50] clamped to [100,200], with viewport extent 25. Rust has no matching PageMetrics.copyWith surface in this check.
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
