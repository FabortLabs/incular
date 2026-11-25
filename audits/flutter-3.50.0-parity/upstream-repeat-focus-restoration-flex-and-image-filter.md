# Animation repeats, traversal, restoration, flex, and image sampling

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

- Five exact AnimationController repeat cases match bounded and
  reversing values, status switch at 1000 ms, and finite counts
  (:873, :920, :1126, :1289, :1324). Count zero (:1267) diverges:
  Flutter debug rejects it; Incular keeps reversing through 425 ms
  and its future stays pending. Count one completes at 100 ms.
  Negative count and missing duration (:1278, :541) are reference-only,
  not representable via the Rust unsigned count and required duration.
  See `flutter350_upstream_animation_repeat` and
  `flutter350_upstream_repeat_invalid_count`.
- Focus traversal matches numerical order, lexical order, and skip
  under ExcludeFocusTraversal (:1279, :1331, :3691). Rust uses
  equivalent normalized Tab/Shift+Tab because the original Dart
  FocusNode methods have no public attached-node counterpart.
  A separate pinned, verified explicit-request comparison observes
  that Flutter does let the excluded child take focus directly.
  Incular leaves the previous host target in place and both nodes
  report primary focus. The direct-adoption path wrongly searches
  the traversal-filtered list. See `flutter_focus_traversal_upstream_parity`
  and `focus_exclude_traversal_reference_test.dart`.
- Restoration reads an existing bucket without dirtying it
  (:147); writing still persists. Claiming an empty new bucket
  (:172) does not request an early save in Incular, and flushData
  (:238) starts a write immediately even with a pending visual
  frame. Flutter schedules the empty claim and waits for that
  frame before flushing. Both paths retain the values after
  writes. These compare the injected storage request timing,
  not the engine channel/serialization. See
  `flutter350_restoration_dirty_flush_parity`.
- RenderFlex spaceBetween with spacing matches exact geometry
  in both axes (:494). No-baseline fallback to top, and a zero
  Baseline frame (:914, :115) match scoped geometry, not the
  direct baseline selector/native-view APIs. Negative spacing
  (:1256) asserts on Flutter's debug constructor and becomes
  zero gap in Incular's widget adapter. See
  `flutter_flex_baseline_geometry_parity`.
- The original ImageFiltered matrix and enabled/disabled cases
  report passes (:104, :158). A different authored pinned fixture
  proves actual matrix sampling: scaling a red-blue tiled child
  2× gives Flutter default blended pixels [191,0,64] and
  [64,0,191] at its seam; actual Windows wgpu keeps [255,0,0]
  and [0,0,255], like Flutter's explicit nearest control.
  Identity, disabling and scaled extent agree. Rust lowers the
  filter to a paint-space transform rather than a filtered
  intermediate raster. No original golden or native layer
  equivalence claim. See `flutter350_image_filter_matrix_sampling`
  and `image_filter_matrix_sampling_test.dart`.

### Matrix image filter sampling recheck

The default matrix ImageFiltered now selects the retained resampling
path at Flutter's Medium quality, mapped to Incular's linear sampler.
Windows wgpu now matches the two pinned 2× seam samples within one byte
per channel. Identity, disabling and the unpainted exterior still pass.
The retained test also checks the untransformed source, scaled output
bounds, linear sampling, and removal of the filter when disabled. This
does not establish mipmap behavior for minification or high-quality
sampling, and it does not upgrade the original golden coverage.
