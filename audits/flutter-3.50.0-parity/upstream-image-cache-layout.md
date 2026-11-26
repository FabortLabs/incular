# Image cache dimensions versus widget layout

The three remaining pinned `widgets/image_resolution_test.dart`
registrations request physical cache sizes of 20 or 1 pixels and
independently display the tinted 1x1 source in explicit 5 or 50
logical-pixel boxes. The mounted Incular tests inspect both the
painted resource dimensions and the actual layout bounds. A cache
request does not enable upscaling and never replaces the requested
widget width or height.

The additional mounted control updates the cache bounds and inherited
DPR while keeping the layout dimensions unchanged. It also switches
between physical and logical cache dimensions, checking rounding at
DPR 1.25 and reevaluation when only the cache request changes.
A no-dimension resize request leaves its source provider unwrapped
(pinned painting resize registration 589).

```sh
cargo test --locked -j 1 -p incular-widgets --test flutter350_image_resize_layout --test provider_image --test flutter350_bundled_asset_image -- --test-threads=1
```
