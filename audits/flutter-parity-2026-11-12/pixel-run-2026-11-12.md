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
