# ResizeImage target geometry and delayed frames

The pinned painting resize controls compare exact upscaling, downscaling,
default no-upscale behavior, physical versus logical dimensions, and
FIT geometry. The matrix separately checks all sixteen no-upscale and
sixteen upscale FIT cases against actual decoded PNG dimensions. Logical
cache dimensions multiply by the configured DPR and round up on each
axis. FIT floors its derived dimensions; the native exact one-width
path floors height, while a one-height request rounds width to nearest.

The Rust resize does not change the input handle. When the effective
size already matches, it returns the original resource unchanged.
Otherwise it samples into bounded output storage. The premultiplied
filter prevents RGB in fully transparent neighbors from coloring an
edge. A delayed source's frame and progress remain reactive; resize is
applied once to the delivered frame, not on each widget build.
Decode and resize enforce the same output byte and dimension ceilings.

The source decoder still allocates its native-size input first. This
comparison covers size and edge behavior, not allocation or
platform-specific decoder sampling equivalence. Cross-widget resized
cache identity and cache-layout integration are separate controls.

```sh
cargo test --locked -j 1 -p incular-image --test flutter350_resize_image --test flutter350_bundle_image_cache -- --test-threads=1
```
