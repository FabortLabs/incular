# Pixel comparison after rendering changes

Revision: `a425aa2e`. Flutter 3.41.7 Windows release captures and Incular WGPU captures were compared for 16 existing scenes at device pixel ratios 1, 1.25, 1.5 and 2 (64 pairs). The Flutter process wrote all 64 captures but `flutter run` did not return after the app exited; it was interrupted and the documented `--incular-only` mode completed using those captures. The pixel test exited successfully. Generated images and diffs are in `harness/out/pixels/` and are not versioned.

The `image_default_fit` scene was subsequently added to both harnesses and the expanded 17-scene set was rerun successfully (68 pairs), with the same Flutter-runner workaround. Its white 40×20 fixture is painted into a 100×100 box with fit omitted on both sides. At 1x, 1.25x, 1.5x and 2x respectively, 16.41%, 16.25%, 16.41% and 16.41% of pixels differ by more than 16 channel levels; the maximum difference is 255 at every scale.

After adding `decoration_image_default_fit` with the same fixture and pre-caching Flutter's image provider, the 18-scene set (72 pairs) was compared successfully. The decoration scene differs by more than 16 channel levels in 35.94% of pixels at each scale, with maximum difference 255. Flutter defaults to scale-down and Incular to cover; the source comparison is documented as paint-003.

The subsequent 19-scene run (76 pairs) includes `physical_model_circle`. This is intentionally a missing-API diagnostic: Flutter is configured with a circle, but Incular exposes no circle-shape option, so its default rectangle is used. At 1x, 1.25x, 1.5x and 2x respectively, 12.66%, 12.59%, 12.51% and 12.39% of pixels differ by more than 16 channel levels; the maximum difference is 243. It illustrates the missing shape option, not a same-configuration rendering comparison.

A 20-scene run (80 pairs) added `overlay_portal_ancestor_clip`: at 1x Flutter paints an orange 40×40 popup outside its 80×80 clipped anchor, while the corresponding Incular region is black. At 1x, 1.25x, 1.5x and 2x respectively, 6.25%, 6.50%, 6.25% and 6.25% of pixels differ by more than 16 channel levels. The precise cause and equivalent placement are under review; this measurement alone does not establish that the ancestor clip is responsible.

The subsequent 21-scene run (84 pairs) added the same portal without the ClipRect as a control. At 1x, 1.5x and 2x the no-clip scenes match exactly; at 1.25x 0.01% of pixels differ by more than 16. The clipped scene retains the differences above. Thus the missing Incular popup in this scene depends on the ancestor ClipRect. Source-level mechanism is under review.

The table reports the share of pixels whose maximum channel difference exceeds 16. These measurements are comparisons, not by themselves proof of an implementation defect; scene equivalence and the respective source paths still require review.

| Scene | 1x | 1.25x | 1.5x | 2x |
| --- | ---: | ---: | ---: | ---: |
| physical_shape_triangle | 30.08% | 30.00% | 29.95% | 29.88% |
| color_filter_matrix_bias | 25.00% | 25.00% | 25.00% | 25.00% |
| rich_text_spans | 1.25% | 1.16% | 1.17% | 1.12% |
| clip_r_superellipse | 0.61% | 0.55% | 0.46% | 0.34% |
| backdrop_rrect | 0.30% | 0.20% | 0.19% | 0.15% |
| text | 0.32% | 0.26% | 0.17% | 0.16% |
| size_transition_clip | 0% | 0% | 0% | 0% |
| blur | 0% | 0% | 0% | 0% |

At 1x the largest channel difference is 255 for physical shape, color filter, and rich text; 122 for the superellipse clip. The remaining scenes include circle, rounded rectangle, path, dark and gray text, shadow, blend and hairline. The scene definitions and prior proposed explanations are in `harness/flutter/lib/main.dart`, `harness/incular/tests/pixels.rs`, and `findings/paint.md`.

## Rerun at HEAD a19bf487 (after renderer changes)

The Flutter scene list (`harness/flutter/lib/main.dart`, last changed in 20c0620f) is unchanged since its 84 captures, so the existing Flutter captures were reused with `harness/run-pixels.sh --incular-only`; only the Incular WGPU side was re-rendered at HEAD. The run exited successfully. Every scene matches the earlier table within 0.01 percentage points, so the renderer changes between a425aa2e and a19bf487 did not change these 21 scenes. Share of pixels whose largest channel difference exceeds 16:

| Scene | 1x | 1.25x | 1.5x | 2x | max channel diff |
| --- | ---: | ---: | ---: | ---: | ---: |
| image_default_fit | 16.41% | 16.25% | 16.41% | 16.41% | 255 |
| decoration_image_default_fit | 35.94% | 35.94% | 35.94% | 35.94% | 255 |
| circle | 0.00% | 0.00% | 0.00% | 0.00% | 1 |
| rrect | 0.00% | 0.00% | 0.00% | 0.00% | 2 |
| path | 0.00% | 0.00% | 0.00% | 0.00% | 31 |
| text | 0.32% | 0.26% | 0.17% | 0.16% | 137 |
| rich_text_spans | 1.25% | 1.16% | 1.17% | 1.12% | 255 |
| size_transition_clip | 0.00% | 0.00% | 0.00% | 0.00% | 0 |
| physical_shape_triangle | 30.08% | 30.00% | 29.95% | 29.88% | 255 |
| physical_model_circle | 12.66% | 12.59% | 12.51% | 12.39% | 243 |
| overlay_portal_ancestor_clip | 6.25% | 6.50% | 6.25% | 6.25% | 255 |
| overlay_portal_no_clip | 0.00% | 0.01% | 0.00% | 0.00% | 64 |
| clip_r_superellipse | 0.61% | 0.55% | 0.46% | 0.34% | 122 |
| color_filter_matrix_bias | 25.00% | 25.00% | 25.00% | 25.00% | 255 |
| text_dark | 0.27% | 0.19% | 0.21% | 0.15% | 110 |
| text_gray | 0.05% | 0.03% | 0.01% | 0.02% | 39 |
| shadow | 0.00% | 0.00% | 0.00% | 0.00% | 4 |
| blur | 0.00% | 0.00% | 0.00% | 0.00% | 3 |
| blend | 0.00% | 0.00% | 0.00% | 0.00% | 16 |
| hairline | 0.03% | 0.03% | 0.02% | 0.00% | 61 |
| backdrop_rrect | 0.30% | 0.20% | 0.19% | 0.15% | 61 |

The larger "max" values on otherwise matching scenes (path 31, overlay_portal_no_clip 64 at 1.25x, text 137) are isolated antialiasing pixels; their >16 shares stay at or below 0.32%.

## 24-scene run at HEAD with new audit scenes

Three scenes were added to both harnesses and Flutter was recaptured (`flutter run -d windows --release` wrote 96 captures, then the comparison ran with `--incular-only`):

- `linear_gradient_offset_box`: a red-to-blue `LinearGradient` (Flutter `Alignment.topLeft` to `bottomRight`; Incular box-relative `Offset(0,0)` to `Offset(100,100)`) on a 100x100 box at (30,30). **Matches** at all scales, so decoration gradients are box-relative in Incular, as in Flutter (`positionDependentBox`).
- `text_align`: `Text` rows with start, center and end alignment and a wrapping justified paragraph in Inter 14. **Matches** within the text antialiasing noise seen in the other text scenes (0.11-0.18% of pixels over 16), so line placement for these alignments agrees.
- `raw_scrollbar_thumb`: a `RawScrollbar` with `thumbVisibility: true` and a white 6 px thumb over a 20-item list. The captures are identical, but **neither side paints a thumb** in the captured first frame (pixel (157,10) is the list colour on both): Flutter learns the scroll metrics from a post-layout notification and paints the thumb a frame later. This scene therefore shows first-frame agreement only; thumb geometry and fade timing remain unchecked and need a capture after an extra frame.
- A `decoration_center_slice` scene was not added: the shared 40x20 fixture is uniformly white, so nine-slicing would be invisible. It needs a non-uniform fixture on both sides.

The other 21 scenes reproduce the table above. Full results (share of pixels over 16):

| Scene | 1x | 1.25x | 1.5x | 2x | max channel diff |
| --- | ---: | ---: | ---: | ---: | ---: |
| image_default_fit | 16.41% | 16.25% | 16.41% | 16.41% | 255 |
| decoration_image_default_fit | 35.94% | 35.94% | 35.94% | 35.94% | 255 |
| linear_gradient_offset_box | 0.00% | 0.01% | 0.00% | 0.00% | 63 |
| raw_scrollbar_thumb | 0.00% | 0.00% | 0.00% | 0.00% | 0 |
| text_align | 0.18% | 0.11% | 0.12% | 0.16% | 137 |
| circle | 0.00% | 0.00% | 0.00% | 0.00% | 1 |
| rrect | 0.00% | 0.00% | 0.00% | 0.00% | 2 |
| path | 0.00% | 0.00% | 0.00% | 0.00% | 31 |
| text | 0.32% | 0.26% | 0.17% | 0.16% | 137 |
| rich_text_spans | 1.25% | 1.16% | 1.17% | 1.12% | 255 |
| size_transition_clip | 0.00% | 0.00% | 0.00% | 0.00% | 0 |
| physical_shape_triangle | 30.08% | 30.00% | 29.95% | 29.88% | 255 |
| physical_model_circle | 12.66% | 12.59% | 12.51% | 12.39% | 243 |
| overlay_portal_ancestor_clip | 6.25% | 6.50% | 6.25% | 6.25% | 255 |
| overlay_portal_no_clip | 0.00% | 0.01% | 0.00% | 0.00% | 64 |
| clip_r_superellipse | 0.61% | 0.55% | 0.46% | 0.34% | 122 |
| color_filter_matrix_bias | 25.00% | 25.00% | 25.00% | 25.00% | 255 |
| text_dark | 0.27% | 0.19% | 0.21% | 0.15% | 110 |
| text_gray | 0.05% | 0.03% | 0.01% | 0.02% | 39 |
| shadow | 0.00% | 0.00% | 0.00% | 0.00% | 4 |
| blur | 0.00% | 0.00% | 0.00% | 0.00% | 3 |
| blend | 0.00% | 0.00% | 0.00% | 0.00% | 16 |
| hairline | 0.03% | 0.03% | 0.02% | 0.00% | 61 |
| backdrop_rrect | 0.30% | 0.20% | 0.19% | 0.15% | 61 |
