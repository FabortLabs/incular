# Remaining findings pass

Second pass over the findings the first fix pass left blocked, deferred,
out of scope or invalid (switchers-004). One commit per finding.

## switchers-004: fixed

`AnimatedSize::reverse_duration` stores Flutter's `reverseDuration` and
forwards it to the size controller at creation and on every update
(`didUpdateWidget`), as `widgets/animated_size.dart:74-78` does. Flutter
3.41.7's `RenderAnimatedSize` only runs forward
(`rendering/animated_size.dart:309-374`), so the value does not change
any size run; the regression test checks that a shrink still takes
`duration`.

## gestures-013: fixed

`trackpadScrollCausesScale` was done in the first pass; this pass adds
Flutter's `DeviceGestureSettings` (`gestures/gesture_settings.dart`).

- `incular_core::DeviceGestureSettings { touch_slop }`, with `pan_slop()`
  (twice the touch slop) and `from_physical(slop, dpr)` for
  `DeviceGestureSettings.fromView`. Re-exported from `incular-config` and
  `incular-gestures`.
- `incular-gestures` has one settings-aware `compute_hit_slop(kind,
  settings)` and `compute_pan_slop(kind, settings)`, as in
  `gestures/events.dart`, replacing three private copies. Mice keep the
  precise-pointer slops; `compute_scale_slop` has no override, as in
  Flutter.
- Every slop-using recognizer takes the settings: the pointer, scale,
  tap-and-drag, multi-tap, multi-drag and force-press recognizers.
- `RuntimeEnvironment::gesture_settings` is Flutter's
  `MediaQueryData.gestureSettings`. The retained tree hands the nearest
  `MediaQuery`'s settings (or the window's) to the recognizers it creates
  for gesture detectors, scale detectors and scroll drags, as
  `GestureDetector` and `Scrollable` do with
  `MediaQuery.maybeGestureSettingsOf`.
- Hosts: Flutter's desktop and web embedders report no touch slop, so the
  desktop and web hosts leave the default. No Incular host runs on Android
  or iOS yet; a mobile host fills the value with `from_physical`.

Tests: `incular-gestures/tests/device_gesture_settings.rs` (5) and
`incular-widgets/tests/gesture_settings.rs` (4). `incular-gestures` tests
and the related widget test files pass; harness `gestures__` 11/11.

## navigation-006: fixed

The Router's `BackButtonDispatcher` (`app_shell/router.rs`) now has
Flutter's child dispatchers (`widgets/router.dart`):

- `create_child` returns a `ChildBackButtonDispatcher`. Its parent consults
  it only after `take_priority`, which calls the parent's `defer_to`.
- `dispatch_back` (`invokeCallback`) offers the request to the children
  that took priority, newest first, and then to the dispatcher's own
  callbacks.
- `BackButtonDispatcher::take_priority` clears the children, and `forget`
  removes one. Dropping a child's last callback subscription forgets the
  child, as `ChildBackButtonDispatcher.removeCallback` does.
- `defer_to` ignores a dispatcher that is not its own child, so priority
  cannot form a cycle.
- `RouterConfig::with_back_button_dispatcher` takes
  `impl Into<BackButtonDispatcher>`, so the root router passes a
  `RootBackButtonDispatcher` and a nested router passes a child.
  `RouterConfig::back_button_dispatcher` now returns the general
  dispatcher type (API change).

The navigation-layer dispatcher used by `PopScope` and `BackButtonListener`
(`navigation/back_dispatch.rs`) already had children and is unchanged.

Tests: `incular-widgets/tests/router_back_priority.rs` (8), including two
nested `Router`s. `app_shell`, `completed_module_integration` and
`incular/tests/public_api` pass.

## navigation-011: fixed

Flutter's `TransitionDelegate` for declarative page updates
(`widgets/navigator.dart:1016-1260`), in `incular-navigation`'s
`transition_delegate.rs`:

- **Records.** `Navigator::set_pages` describes each update as a
  `PageTransition`. It holds one `PageRouteRecord` per page of the new
  history, plus one `ExitingPageRoute` per removed route, with its
  `location`: the kept route it sat above (`locationToExitingPageRoute`).
- **Decisions.** `PageEntering::{Push, Add}` and
  `PageExiting::{Pop, Complete, Remove}`, with `mark_for_*` methods.
- **Default.** `DefaultTransitionDelegate` follows Flutter: an added new top
  pushes and the other added routes are added; the last removed route above
  the new top pops and the other removed routes complete. Records arrive
  prefilled with these defaults, so a custom delegate changes only what it
  cares about. This is a Rust-first shape: Flutter asserts that every record
  is decided.
- **API.** `Navigator::set_transition_delegate` (closures implement the
  trait) and `clear_transition_delegate`. `take_page_transitions` hands
  pending decisions to the presenter; nothing is recorded until a presenter
  first reads, so a navigator without one keeps nothing.
- **Outlet.** `RouteOutlet::drive_route_animations` reads the decisions.
  An `Add` starts complete; a `Pop` animates out where the route was. A new
  `ExitPlacement` (`Bottom`, `Above(route)`, `Top`) replaces the implicit
  `below`/`until` encoding. `Complete` and `Remove` leave at once (Incular
  routes have no completion result, so the two differ only in what they
  record). Imperative push, pop and replace keep their transitions, as in
  Flutter.

**Behavior change.** A `set_pages` update that added several pages
previously animated all of them, and replacing the top page kept the old
page under the new one. Both now match Flutter's default.

**Tests:**
- `incular-navigation/tests/transition_delegate.rs` (10);
- four outlet tests in `incular-runtime/tests/runtime/route_outlet.rs`.

`incular-navigation` and `incular-runtime` tests pass. Harness
`navigation__` is 6/12, the same six documented divergences as before.

## semantics-013: fixed

Flutter's `CustomPainter.semanticsBuilder` and `CustomPainterSemantics`
(`rendering/custom_paint.dart`):

- `CustomPainter::semantics_builder() -> Option<SemanticsBuilder>`, which
  defaults to `None`. `SemanticsBuilder` is `Rc<dyn Fn(Size) ->
  Vec<CustomPainterSemantics>>`.
- `CustomPainterSemantics::new(rect, Semantics)`, with an optional `key`.
  The region's properties are a `Semantics` built without a child.
- `CustomPaint` lowers a painter that has regions to a transparent overlay
  over the painting. The overlay is a pointer-ignoring `Positioned::fill`
  whose `LayoutBuilder` receives the painting's final size, runs the
  builder, and places one `Semantics` node per region at its rect.
- Order is background regions, then the child, then foreground regions, as
  `RenderCustomPaint.assembleSemanticsNode` orders them. Regions rebuild
  with the widget and whenever the size changes.

This is composed from existing widgets rather than by adding synthetic
nodes to the incremental semantics pass, so the pass, its retention and
action dispatch are unchanged. Region actions use the `Semantics`
callbacks.

Not ported:
- `shouldRebuildSemantics`: regions rebuild with the widget, which is
  correct but does not skip unchanged region lists.
- `CustomPainterSemantics.transform` and `tags`.

Tests:
- `incular-widgets/tests/custom_paint_semantics.rs` (5): rects, resize,
  order, no regions without a builder, and a tap through foreground regions
  reaching the child.
- New twin scenario `semantics__custom_paint_regions`: match. Harness
  `semantics__` is 19/20 (the known `enabled_default` divergence) and
  `paint` 9/9.

## paint-012: fixed

Flutter's image stream and `Image(image: provider)` lifecycle
(`painting/image_stream.dart`, `widgets/image.dart` `_ImageState`):

- `incular-image` gains `ImageStream`, `ImageStreamCompleter`,
  `ImageLoadState` (loading with an optional `ImageChunkEvent`, ready,
  failed) and `AnyImageProvider`, a type-erased provider compared by type
  and value. `ImageProvider::resolve(&ImageConfiguration) -> ImageStream`
  defaults to a completed stream from `load_with`, so existing providers
  keep working; asynchronous providers return a loading stream and complete
  it on the UI thread.
- Reading `ImageStream::state` inside a build tracks the stream through
  `DependencySource`, the mechanism `UiFuture` uses, so each chunk or
  completion rebuilds the image on the next frame.
- `Image::provider(p) -> ProviderImage` takes the `Image` paint options
  plus `frame_builder`, `loading_builder`, `error_builder`,
  `gapless_playback`, `semantic_label` and `exclude_from_semantics`.
- It resolves against `createLocalImageConfiguration` (device pixel ratio,
  explicit size, text direction, locale) and resolves again only when the
  provider or configuration changes. A stream that is already complete when
  the widget subscribes counts as synchronously loaded (Flutter's
  `synchronousCall`). Without gapless playback a new provider clears the old
  image at once.
- While loading, the widget is an empty box of the explicit size (a null
  `RawImage`). A failure shows the error builder, or that empty box.

Not ported: multi-frame (animated) images, since decoded handles have one
frame; `ImageStreamListener.onChunk` as a separate callback (chunks arrive
as loading states); `TickerMode` pausing.

Tests:
- `incular-image/tests/image_stream.rs` (5).
- `incular-widgets/tests/provider_image.rs` (8): loading box, chunks,
  frame builder and synchronous loads, error builder and fallback, gapless
  playback, stream reuse for an equal provider, semantic label.
- Pixel harness (`--incular-only`): image scenes unchanged. The
  `image_default_fit@1.25` diff also appears in `decoration_image_default_fit`,
  whose path this change does not touch.

## paint-014: fixed

`FadeInImage` options (`widgets/fade_in_image.dart`):

- `FadeInImage::from_providers(placeholder, image)` loads both through
  `ProviderImage`. The target fades in from its frame builder, and an image
  that loads synchronously appears without a fade, as in Flutter.
- `image_error_builder` and `placeholder_error_builder`;
  `image_color`/`image_color_blend_mode` and
  `placeholder_color`/`placeholder_color_blend_mode`; `match_text_direction`.
- `Image` itself gains `color` and `color_blend_mode`. It lowers to a
  `ColorFiltered` over the image, using the tint filter by default
  (Flutter's `srcIn`).

Tests:
- `incular-widgets/tests/fade_in_image.rs`: 5 provider tests. A loading or
  failed target leaves the stack at opacities `[0.0, 1.0]`, as in Flutter.
- `incular-widgets/tests/paint_options.rs`: Image color blend, and separate
  placeholder and image tints.

## slivers-010: fixed

Flutter's `Viewport.center` and `anchor` (`rendering/viewport.dart`
`RenderViewport._attemptLayout`, `performLayout`):

- `incular-scroll` now has a real minimum offset:
  - `ScrollController::min_offset()`;
  - `ViewportMetricsUpdate::leading` and `with_leading`;
  - `ScrollMetrics::min_scroll_extent`.
  It is used by the following, which clamped to 0 before:
  - jumps, corrections, drives and the unclamped-drive overscroll;
  - physics deltas and settling;
  - spring-back and clamped ballistics;
  - pending, restored and initial offsets;
  - range maintaining;
  - both scrollbar geometries.
  The widget side uses it for drag release ballistics, fling boundary
  ends, keyboard and semantics scrolling, semantics scroll actions and
  `ensure_visible`.
- `CustomScrollView::center(index)` / `anchor(f32)` and `Viewport::center` /
  `anchor`. Flutter names the center sliver by key; Incular slivers have no
  keys, so it is named by index.
- `SequenceViewportDelegate` ports `_attemptLayout`:
  - `centerOffset = extent * anchor - pixels`;
  - the slivers before the center lay out as a run from the center
    outward, with the reverse paint and cache budgets;
  - the center and the slivers after it lay out forward;
  - the content range is `min(0, -reverse + extent * anchor)` to
    `max(0, forward - extent * (1 - anchor))`.
- Reverse-growth children are mirrored about the center line. The viewport
  keeps their trailing (center-side) edge when a measurement differs from
  the estimate, and anchoring ignores them, so refinements move content
  away from the center as in Flutter.
- A reverse sliver that is hidden while the center line is above the
  viewport parks at the leading edge, as Flutter's clamped
  `initialLayoutOffset` places it. Centered viewports with reverse slivers
  therefore re-lay out on every scroll.
- `ShrinkWrappingViewport` takes a controller, axis and reverse, and is
  exported (with `Viewport`) from the facade.

Not ported:
- `centerOffsetAdjustment`.
- Shrink-wrapping viewports with a center (Flutter has none).
- Pinned headers before the center are positioned by the mirrored run but
  have no dedicated test.

Tests:
- `incular-widgets/tests/viewport_center.rs` (8): reverse growth, anchor
  extents and range, anchor alone, reverse list placement, prepended content
  keeping the center, reversed axis, parked reverse slivers, shrink-wrap
  options.
- incular-scroll suite and the scroll, sliver, scrollbar and
  ensure-visible widget suites pass. The full incular-widgets suite passes.

Harness:
- New twin `slivers__viewport_center__anchor`. Every probe rect, every
  presence flag, `offset` and `max` match. Only `min` diverges, by up to 18
  px, while children are unmeasured. Flutter re-estimates from the children
  it currently holds (-291.8, then -300, then -282 after scrolling away);
  Incular keeps measured extents (-303, then -300). This is the same
  retained-extent difference as the known list-estimate findings.
- `slivers__` is 15 of 16 match plus the new twin. The only other
  divergence is the known keep-alive one.
- The `scroll` filter gives the same divergences before and after the
  change.

## scroll-008: fixed

Flutter's `ScrollAwareImageProvider` (`widgets/scroll_aware_image_provider.dart`)
and the deferred-loading heuristic it consults:

- `ScrollPhysics::recommend_deferred_loading(velocity, physical_longest_side)`
  is Flutter's root `ScrollPhysics.recommendDeferredLoading`: defer while
  `|velocity|` exceeds the longest side of the view in physical pixels. No
  built-in Flutter physics overrides it. (The threshold is the view size,
  not the max fling velocity.)
- `ScrollController::recommend_deferred_loading` is
  `ScrollPosition.recommendDeferredLoading`: the activity velocity plus
  `_impliedVelocity`, through the physics the viewport last published.
  - `OwnedActivity::set_velocity` / `ScrollController::activity_velocity`
    are `ScrollActivity.velocity`. The value is tagged with the activity
    generation, so it lapses when the bracket closes or is taken over.
    The fling and `animate_to` driver records `simulation.dx(t)` every
    frame (ballistic and driven activities); drags report 0, as Flutter's
    `DragScrollActivity` does.
  - `jump_to` records the jump distance as the implied velocity and clears
    it in a post-frame callback, as `forcePixels` does.
- Every `Scroll` and `SliverViewport` element provides a `ScrollableScope`
  (Flutter's `_ScrollableScope`) with its controller. An update with the
  same controller keeps the scope value, so no dependents are invalidated.
- `Scrollable::recommend_deferred_loading_for_context` reads the nearest
  scope without depending on it, as Flutter's
  `getInheritedWidgetOfExactType` does.
- `ScrollAwareImageProvider` wraps an `AnyImageProvider` with a
  `ScrollAwareContext` (Flutter's `DisposableBuildContext`: the nearest
  controller, the physical window size and a shared disposed flag).
  - While deferral is recommended, `resolve` returns a loading stream and
    retries in a scheduler frame callback each frame.
  - Once the scroll slows, the deferred stream follows the real resolution
    through the new `ImageStreamCompleter::follow`, Flutter's
    `ImageStream.setCompleter`.
  - A disposed context never resolves.
- `ProviderImage`, and so `FadeInImage`, wraps every resolution, as
  `_ImageState._resolveImage` does. It disposes the previous context on
  re-resolution and on drop.
- `AnyImageProvider` gained `load_with` and `Eq`, so the wrapper is itself
  an `ImageProvider`.

Not ported:
- Flutter's cache shortcut (`imageCache.containsKey(key)`): Incular has no
  provider-keyed image cache. An image that keeps an equal provider keeps
  its stream and never re-resolves.
- The `axis` argument of `recommendDeferredLoadingForContext`.

Tests:
- `incular-scroll/tests/deferred_loading.rs` (3): the threshold, activity
  velocity lapse and takeover, implied velocity cleared after the frame.
- `incular-image/tests/image_stream.rs` (+3): follow, follow refusal and
  cycles, erased load.
- `incular-widgets/tests/scroll_aware_image.rs` (3): a provider swapped
  during a fast `animate_to` stays unresolved, retries each frame and
  resolves after the scroll stops. A replaced deferred provider never
  resolves. There is no deferral outside a scroll view or with a disposed
  context.
- The incular-scroll, incular-image and full incular-widgets suites pass,
  including the allocation budgets. The widgets doctest build fails to find
  `incular_rendering` and `incular_text` at the crate's untouched re-export
  lines; this is a doctest toolchain issue, not this change.

Harness: no twin. Deferral changes only when an image appears during a fast
fling, and the harness has no asynchronous image provider on either side.

## support-005: fixed

`NetworkAssetBundle` (Flutter `services/asset_bundle.dart:146`) lives in
`incular-assets`, which owns the bundles. `specs/architecture.json` gives
that crate no dependencies, so it cannot take an HTTP client. The transport
is therefore an injected `AssetFetcher`: Flutter's `HttpClient`, supplied
by the application (for example `reqwest` natively or `fetch` in the
browser).

- `AssetFetcher::fetch(url, done)` starts a GET and answers once, possibly
  later and on another thread, so it never blocks the wasm page thread.
  `fetch_blocking` is optional and defaults to `None`.
- `NetworkAssetBundle::new(base_url, fetcher)` resolves keys with
  `resolve_url`, a port of Dart's `Uri.resolve` (RFC 3986 5.2 merge and
  dot-segment removal, with illegal characters percent-encoded).
- As in Flutter, a response other than `200` fails with
  `HTTP status code: N` (`AssetError::Io`, so `AssetError` gains no
  variant). Nothing is cached, and `evict` is a no-op.
- Flutter's loads are futures, while Incular's `AssetBundle::load` is
  synchronous. The trait gains `load_async(key, done)`, Flutter's `load`:
  - by default it answers synchronously through `load`;
  - `CachingAssetBundle` forwards it;
  - the network bundle fetches without blocking.
  The network bundle's synchronous `load` works only with a fetcher that
  can block, and otherwise fails with an explanatory `Io` error.
- Exported from `incular-assets`, and so from the facade's `assets`
  module.

Tests: `incular-assets/tests/network_bundle.rs` (5) cover:
- `Uri.resolve` cases;
- asynchronous success;
- status-code and transport failures;
- the blocking path through `CachingAssetBundle`;
- the default `load_async`.
`cargo check -p incular-assets --target wasm32-unknown-unknown` passes.

Not ported: `PlatformAssetBundle` (bundles are compiled in, as the finding
notes) and `loadBuffer`.

## support-003: fixed

Flutter's platform undo manager (`services/undo_manager.dart`) and the
`UndoManagerClient` side of `UndoHistoryState` (`widgets/undo_history.dart`).
In Incular the runtime owns each text field's history
(`UndoHistoryController`), so the runtime is the client.

- `incular-platform`:
  - `UndoDirection` (with `from_name` for the `"undo"`/`"redo"` channel
    names);
  - `PlatformEvent::Undo(direction)`, Flutter's
    `UndoManagerClient.handleUndo`;
  - `UndoManagerCommand::SetUndoState { can_undo, can_redo }`, Flutter's
    `UndoManager.setUndoState`.
- `Runtime::handle_platform_undo` is `handlePlatformUndo`. The focused text
  field is `UndoManager.client`, and the command runs the same `Undo`/`Redo`
  text intent as the keyboard shortcuts: menu hide, caret reset and frame.
  Without a focused field nothing happens; Flutter asserts there.
- After each frame, `sync_undo_manager_state` sends `SetUndoState` when a
  field gains focus and whenever its availability changes. As in Flutter's
  `_updateState`, this happens only for the iOS target platform. A focus
  loss sends nothing; the next client reports again.
- `Runtime::take_undo_manager_commands` and
  `Application::take_window_undo_manager_commands` drain the queue, and
  `PlatformEvent::Undo` is routed per window.
- The shared desktop host drains the queue each frame into the new
  `DesktopPlatformServices::apply_undo_manager_command`. It defaults to
  dropping the command, since Flutter drives no desktop system undo state.

Native crates: `incular-ios` and `incular-android` are semantic adapters
with no host runner, so no native undo manager is wired. A native iOS host
applies `SetUndoState` to `UIUndoManager` and forwards its undo and redo as
`PlatformEvent::Undo`.

Tests: `incular-runtime/tests/platform_undo_manager.rs` (3) cover:
- platform undo and redo on the focused field;
- no client means no effect;
- iOS state reporting on focus and change, no duplicates, and switching
  client.
The `incular-runtime` and `incular-platform` suites pass.
