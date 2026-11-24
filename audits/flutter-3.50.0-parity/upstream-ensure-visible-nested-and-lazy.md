# Nested and lazy-list ensureVisible coordinates

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

Three selected originals and their three Rust counterparts pass.
The 7×7 grid inside horizontal and vertical single-child
viewports (:318) matches all seven global coordinate pairs,
including centered alignment across both axes. The two
ListView cases (:394, :430) use each original pre-reveal
offset, verify the original targets are mounted without widening
the cache, and compare the five sampled edge coordinates per
axis including the one-second animation. The eight Rust tests
in `flutter350_ensure_visible_upstream_parity` now all pass.
Other lazy/small/reversed list variants remain separate.
