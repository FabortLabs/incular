# Asset image keys and pixel density

All 14 pinned `painting/image_resolution_test.dart` registrations paired
(three slices of at most five names plus the final single case). The
six default-scale registrations distinguish literal filename/path shapes
from declared DPR, including the duplicate source registration. The two
high-resolution registrations cover a variant-only row with absent and
present DPR, and an empty row. Six additional controls exercise both
exact matches, the midpoint at 2.0, two values above it, and the upper
clamp. The Rust tests assert both key and scale.

For real bundled assets, the selected key is the manifest's physical key,
not an inferred filesystem path. An explicit bundle wins over the
configuration's bundle and the root. Package keys are prefixed with
`packages/<name>/`. The picker visits the list once without a temporary
sorted map; equal DPR entries prefer the last manifest declaration.
`ExactAssetImage` bypasses the manifest. The unbundled native path
adapter remains, including a deterministic 1x decision when the base
file and its 1x variant are both present.

An authored Flutter VM probe and mounted Incular widget test decode
different-colored 1x and 2x PNGs with content-hashed physical names and
no image at the logical name. The 3x2 red 1x and 6x4 blue 2x assets
both paint at 3x2 logical pixels. The mounted test changes the inherited
DPR in both directions, uses the nearest bundle, and asserts the paint
source size, destination size, density and decoded pixels. No GPU
sampling parity is inferred from CPU decode and paint commands.

The synchronous provider path covers immediate bundles. Async key lookup
also works through the bundled transport; driving a loading widget with
callback-only image bytes and Flutter's full decoded image cache remain
separate work.

```sh
cargo test --locked -j 1 -p incular-image --test flutter350_asset_image_keys --test asset_variants --test defaults -- --test-threads=1
cargo test --locked -j 1 -p incular-widgets --test flutter350_bundled_asset_image --test provider_image --test default_asset_bundle -- --test-threads=1
bash audits/flutter-3.50.0-parity/run-reference-linux.sh probe asset_image_hashed_pixels_test.dart
```
