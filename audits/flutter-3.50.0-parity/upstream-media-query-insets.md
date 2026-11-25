# Flutter 3.50 MediaQuery inset removal

The six pinned `widgets/media_query_test.dart` registrations at
:776, :852, :927, :1005, :1080, and :1158 passed. Rust reproduces each
all-sides and single-side case with the original asymmetric safe
padding, persistent view padding, and transient insets. The test
observes the retained descendant scope, compares the full snapshot
to ensure unrelated data is unchanged, and verifies the per-aspect
accessors agree with the full snapshot.

Removing padding subtracts exactly the selected safe padding from
view padding; removing view insets instead subtracts the selected
transient inset. Both floor the result at zero. Removing view padding
clears both padding fields but leaves transient insets alone. A zero
edge mask leaves the original data unchanged.
