# Backdrop filter widgets

All three pinned Flutter originals in `widgets/backdrop_filter_test.dart`
passed. Incular's `backdrop_group` tests check that the two retained
stages receive the same explicit key and both clear it on rebuild, the
nearest inherited group gives both stages its key, and a childless filter
at zero viewport size lays out to zero without creating a backdrop pass.

An additional native test checks that a configured bounded blur reaches
the flattened scene with local bounds after mount and resize, keeps its
selected tile mode and two sigmas, and stops filtering when disabled.
The layout origin stays local because the retained layer already carries
the placement transform. The 15 adjacent shader/backdrop tests and the
retained-layer reuse test also passed. These are scene and lifecycle
contracts, not screenshot golden comparisons.
