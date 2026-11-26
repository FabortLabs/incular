# Mounted bundled image resolution and delayed delivery

Nine pinned `widgets/image_resolution_test.dart` scenarios paired.
Seven individual window DPRs (0.5, 1.0, 1.5, 1.75, 2.3, 3.7, 5.1)
use independently checked physical sizes, chosen scale, and both the
200x200 explicit box and the 48x48 intrinsic box. The explicit 1.0x
variant test decodes a 480x480 file marked as 1.0x instead of using
the physical base file. The low-resolution control verifies all 11
reference ratios using a bias toward available higher density below
2.0 and a clamp when nothing denser exists. The three cache-resize
tests in the same source file are not part of this comparison.

A callback-only network-bundle double exercises a mounted Incular
image: first the manifest request is pending, then only the selected
physical file is requested. A worker completes the selected PNG,
wakes the existing UI task scheduler, and cannot mutate retained
widget state until the UI thread delivers the result. The frame
builder sees a delayed rather than synchronous first frame.
An absent exact asset fails without a manifest request or trying
another variant. These transport checks use no external network or
live Flutter browser and are not additional upstream case credit.

```sh
cargo test --locked -j 1 -p incular-widgets --test flutter350_bundled_asset_image --test flutter350_async_bundled_image --test provider_image -p incular-image --test flutter350_asset_image_keys -- --test-threads=1
```
