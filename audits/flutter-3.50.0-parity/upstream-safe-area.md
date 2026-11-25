# Flutter 3.50 SafeArea

Pinned `widgets/safe_area_test.dart:11,22,37,48,111,144,175,210,245`
passed. Rust repeats exact 800×600 child corner coordinates for
disabled edges, minimums, nested areas and initial/changed padding.
It also verifies zero-area layout and the unchanged child rectangle
with full or partial simulated keyboard occlusion, with or without a
fixed-size intermediate widget.

Maintained bottom padding is the persistent view padding, not the
maximum of transient safe padding and persistent padding. The authored
`safe_area_maintained_reference_test.dart` demonstrates a 35px safe
bottom with 20px persistent yields exactly 20px maintained padding.
It also verifies a nested maintained area sees only remaining view
padding: a visible 20px safe margin is consumed once, while a keyboard
that has already consumed the safe margin leaves both maintained
areas able to use the persistent value. Rust now uses the same scoped
inset mechanism for ordinary and maintained SafeArea, and continues to
forward the keyboard inset to descendant queries without using it to
calculate padding. This removed the old all-render invalidation scan.

The Flutter diagnostics/FlagProperty registration is separate.
