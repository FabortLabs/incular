# PageView storage, semantics and numeric boundaries

All six associated pinned Flutter originals passed. The focused Rust target passes four checks and intentionally fails one.

- :413 is reference-only. The Rust next_page / previous_page surface returns a bool, not a future; use the separate controller control to verify actual transition results, not completion handles.
- :953 partially mismatches. The view starts on 0, reaches 2, restores 2 and initializes an independent keep-offset-disabled controller to 0. But after removing the view, the previously attached controller still reports Some(2); Flutter refuses to expose a page until it is reattached. A separate Rust control proves actual bucket reuse by attaching a fresh controller with the same stored key and getting page 2.
- :1030 fully matches. At selected pages 0, 1, 2 the genuine framework semantics tree exposes only the corresponding Page # label.
- :1069 partially matches. The arithmetic helper produces [6,4] for requested pixels [150,50] clamped to [100,200], with viewport extent 25. Rust has no matching PageMetrics.copyWith surface in this check.
- :1084 partially matches; converting the exact Dart 799.99999999999 input to Rust's f32 coalesces to 800, so the retained controller reports 1. This does not independently verify Flutter's f64 near-boundary tolerance.
- :1104 is reference-only. Dart enables allowImplicitScrolling, checks hidden adjacent labels and exercises accessible page scrolling. Rust lacks that option. The default false behavior above is not a substitute.

Run cargo test --locked -p incular-runtime --test flutter350_upstream_page_storage_semantics -- --nocapture.
