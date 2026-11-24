# SliverFillViewport sizing and accessibility controls (R03)

All three named originals in widgets/sliver_fill_viewport_test.dart passed against pinned Flutter 3.50.0. The focused Rust runtime target passes all three checks.

- :12 partial: at offsets 0, 700, 500, and 0 the intersecting 800×600 pages are [0], [1,2], [0,1], [0]. Rust drives equivalent settled offsets programmatically and checks real mounted child bounds. Flutter's drag trajectory, rendered debug tree string, and repaint metrics were not mapped.
- :174 exact: one half-fraction page has 600px reported paint extent with end padding and 300px without it. The Rust test queries SliverGeometry from the actual retained sliver's standard viewport protocol.
- :212 partial: the default exposes both first and cached second page labels as in Flutter. Rust does not expose allowImplicitScrolling(false), so the original's exclusion/repump branch is unpaired. A zero cache extent would check a different contract.

Run cargo test --locked -p incular-runtime --test flutter350_sliver_fill_viewport_parity -- --nocapture.
