# Flutter 3.50 MediaQuery scaling and scope

Pinned `widgets/media_query_test.dart:1295,1320,1700` passed. The
Rust environment target checks fallback scale 1 and light brightness,
nested scale 4 and dark brightness, and retained dependency counters:
a size, scale, or DPR subscriber rebuilds only when that specific
aspect changes; a whole-query subscriber sees all changes.

Previously, the aspect lookup reported the nested text scale correctly,
but paragraph layout ignored it and read the native runtime value.
The retained text pipeline now takes the nearest MediaQuery scale,
then falls back to the window scale without a scope. An explicit
scaler is still authoritative. Both the authored pinned
`scoped_text_scale_reference_test.dart` and the Rust environment target
check measured text width/height at factors 1 and 2, zero width at
factor 0, and unchanged explicit text size without remounting. Flutter's
nonlinear system scaler and the full aspect matrix are not covered here.
