# Flutter 3.50 RelativeRect geometry

All ten pinned `rendering/relative_rect_test.dart` registrations passed.
The focused Incular target `flutter350_relative_rect` pairs nine of them:
equality/from-size, directional construction, shift, inflate, deflate,
intersection, local rect conversion, signed size conversion, and interpolation.
The tenth (:73) pairs the absent result and equal values only; Rust's copyable
value cannot assert Dart object identity. It remains partial in the ledger.

Incular had added the container origin in `to_rect`. The pinned offset test
reproduced the extra (10, 20) before the fix. `to_rect` is now local and
`to_global_rect` explicitly applies the origin. The authored
`relative_rect_boundaries_test.dart` also verifies that disjoint
intersections preserve negative extents and that the signed local/global
round-trip works with an asymmetric container.

Flutter's documented nullable interpolation uses the fill rectangle for a
missing endpoint. At this revision, the non-null-start/null-end branch
dereferences the missing end and throws. The authored probe confirms it.
Incular intentionally follows the documented symmetric behavior rather
than introducing that crash. This authored divergence and the authored
geometry controls do not increment the upstream registration count.
