# SliverFillViewport sizing and accessibility controls (R03)

All three named originals in widgets/sliver_fill_viewport_test.dart passed against pinned Flutter 3.50.0. The focused Rust runtime target passes all three checks.

- :12 partial: at offsets 0, 700, 500, and 0 the intersecting 800×600 pages are [0], [1,2], [0,1], [0]. Rust drives equivalent settled offsets programmatically and checks real mounted child bounds. Flutter's drag trajectory, rendered debug tree string, and repaint metrics were not mapped.
- :174 exact: one half-fraction page has 600px reported paint extent with end padding and 300px without it. The Rust test queries SliverGeometry from the actual retained sliver's standard viewport protocol.
- :212 originally partial. The disabled and re-enabled branches are now paired below without removing cached pages.

Run cargo test --locked -p incular-runtime --test flutter350_sliver_fill_viewport_parity -- --nocapture.

## Excluding cached pages when implicit scrolling is disabled

The original :212 now pairs. A cached second page remains mounted but
does not contribute a semantic node while the option is false. Switching
back to the default includes both labels without replacing the page;
switching it off again excludes only the cached page. PageView forwards
its option to the sliver as well as to viewport scrolling.

The two authored Flutter controls and native edge test advance through
0, 599, 600, 601, and 600 logical pixels in a 600-pixel viewport. At
599 both pages overlap; at 600 only the second page is visible; at 601
the third page also overlaps by one pixel. This matches in the forward
vertical and reversed horizontal directions. The native test checks
the keyed cached pages keep their mounted identities across the
changes. Seven focused native widget/runtime tests passed, including
an independent runtime check that the hidden cached page stays mounted.
The test also checks the exact existing :174 extent and its
300-pixel page constraints with and without padding.

There were four failures in the neighboring `scroll_pages_reorder`
target, reproduced without any changes to the three scrolling sources
at baseline `3b235f1` and with the same failure names. They do not
block the focused SliverFillViewport checks. They are tracked separately
rather than counted as new paired cases.
