# Changelog

Maintainers: record user-visible changes under `Unreleased`, then move them to
a version and date when publishing.

## Unreleased

### Added

- `incular-widgets`: Focus exposes focus-change callbacks and independent
  descendant focus and traversal permissions.

- `incular-runtime`: wheel hosts can be notified when no retained
  listener or viewport claims a pointer signal.

- `incular-runtime`: dispatch timestamped raw pointer samples without
  dropping synthesized motion metadata before scale recognition.

- `incular-widgets`: box-relative linear gradients resolve logical
  endpoints with text direction and reuse shaders across retained paints.

- `incular-widgets`: directional decoration radii follow inherited text
  direction in both painting and hit testing.

- `incular-widgets`: box borders expose inner and outer paths, with
  directional insets resolved before building the inner outline.

- `incular-widgets`: physical and directional borders can paint directly
  to a canvas at a supplied rectangle.

- `incular-widgets`: `BorderSide` can copy selected fields and convert to
  renderer-neutral stroke paint.

- `incular-widgets`: physical and directional borders support optional
  interpolation endpoints, including extrapolation.

- `incular-widgets`: physical and directional borders can merge compatible
  sides; directional borders expose per-side scaling and uniformity.

- `incular-widgets`: `CallbackShortcuts` accepts `CharacterActivator`
  alongside logical and physical key bindings.

- `incular-animation`: added opt-in debug frame banners and a check that
  their switches were reset.

- `incular-runtime`: added ordered window-wide hardware key handlers.
  Handled events still reach focused listeners and other handlers.

- `incular-runtime`: exposes pressed physical positions, portable logical
  keys, and lock toggles observed from the delivered keyboard stream.

- `incular-core`: added Flutter-compatible C0/C1 control classification
  for single logical keyboard labels.

- `incular-platform`: added Flutter-style overlay copies, value hashing,
  and ordered diagnostics for status and navigation bar styles.

- `incular-core` and `incular-widgets`: added merged dependency listeners
  with owned detach and `ListenableBuilder` support.

- `incular-core`: added Flutter-style signed duration interpolation in
  microseconds, including negative durations and extrapolation.

- `incular-core`: added `CachingIterable` for lazy, repeatable iteration over
  a single source, with separately cached derived sequences.

- `incular-core`: added ordered observer lists with Flutter's two duplicate
  policies and a `clamp_double` helper that maps NaN to the upper bound.

- `incular-animation` and `incular-runtime`: the UI scheduler can run
  short, prioritized tasks separately from background jobs. The default
  policy waits for animations before running idle-priority tasks.

- `incular-animation`: added a debug assertion and a recoverable check
  for a time-dilation scale that has not been reset to 1.

- `incular-animation`: `Priority` provides Flutter's idle, animation, and
  touch bands with clamped relative offsets.

- `incular-animation`: `schedule_forced_frame()` and
  `Ticker::set_force_frames` can explicitly request frames during a
  lifecycle pause when the host still supplies them.

- `incular-animation`: `current_system_frame_time()` reads the latest
  raw host timestamp without animation dilation or epoch adjustment.

- `incular-widgets`: `Container::transform_alignment` selects a physical
  or direction-relative transform pivot without changing layout.

- `incular-widgets`: `BoxDecoration::clip_path` and `hit_test` expose the
  outline and local hit geometry for rectangles, rounded boxes, and circles.

- `incular-config`: `Constraints::try_normalized` clamps negative bounds to
  zero and raises an inverted maximum to its minimum, while rejecting NaN
  and positive-infinite minimums.

- `incular-widgets`: `Semantics::accessibility_focus_block_type` blocks
  accessibility focus for a node or subtree without changing keyboard
  focus or removing the node's other actions.
- `incular-android`: mouse and stylus hover reach widgets, and mouse wheels and
  trackpads scroll the viewport under the pointer. Mouse buttons and stylus
  orientation and barrel buttons reach raw pointer listeners.
- `incular-widgets`, `incular-material`: touch text selection handles on text fields, as Flutter's `MaterialTextSelectionHandleControls`. `TextSelectionHandlesState` presents the start and end (or collapsed) handles through an overlay portal; `TextSelectionHandles::hit_test` returns the handle under a point. The Material `TextField` paints 22 px teardrops in the primary colour. A touch long press or double tap shows the range handles, and an Android touch tap in a focused field shows the caret handle. Typing, a mouse press, losing focus and a read-only field close them. A touch or stylus press on a visible handle drags it, with the Android and Apple rules of Flutter's `TextSelectionOverlay`: the handles never cross or collapse on Android, and the drag snaps to whole lines. The handle hit area is 48 px. Tapping the collapsed handle without moving past touch slop toggles the touch toolbar, and the toolbar hides while a handle moves and shows again after a range drag.
- `incular-widgets`: `RemovePadding` hides chosen safe-area sides from a subtree, as Flutter's
  `MediaQuery.removePadding`. Use it to stop a nested `SafeArea` or bar from padding a
  side that an outer widget already handles.
- `incular-material`: `AppBar::primary` and `Scaffold::primary` (default `true`), as Flutter.
- `incular-material`: `AppBar` annotates the system bars from its background, as Flutter's
  `systemOverlayStyle` does. The status-bar icons are dark on light backgrounds and light on
  dark ones, and Material 3 leaves the status bar transparent. `AppBar::system_overlay_style`
  overrides the derived style, and `ThemeData::estimate_brightness_for_color` follows Flutter's
  luminance threshold. `SliverAppBar` inherits this through its app bar. `AnnotatedRegion` and
  `SystemUiOverlayStyle` are in the facade prelude.
- `incular-platform`: `SystemUiOverlayStyle`, Flutter's status and navigation bar
  style with optional fields and the `LIGHT` and `DARK` constants. A window takes it
  imperatively with `Application::set_window_system_ui_overlay_style` (Flutter's
  `SystemChrome.setSystemUIOverlayStyle`), or an `AnnotatedRegion` over the bar sets it
  per frame, merged as `RenderView` does. On Android the icon brightness and bar colours
  follow the style; iOS, desktop and web do not apply it yet.
- `incular-android`: the shell publishes the `JavaVM` and the application
  context through [`ndk-context`](https://docs.rs/ndk-context) before the
  app's `main` runs. Crates that call into Android read them from there, so
  `webbrowser` can open links, for example.
- `incular`/`incular-android`: a `platform-verifier` feature initializes
  `rustls-platform-verifier` with the app context. HTTPS clients that verify
  through it, such as reqwest's default `rustls` backend, then use Android's
  trust store; without it they panic on the first request. Other platforms
  need no setup.
- `incular-cli`: when the app depends on `rustls-platform-verifier`,
  `incular build android` adds its Kotlin component, at the version in the
  app's lockfile, to the generated Gradle project.
- Android text input, following Flutter's `TextInputPlugin`:
  - `incular-android`: tapping a field opens the soft keyboard. An
    `InputConnection` reports the input method's whole editing state
    (composing text, autocorrect, swipe typing, backspace), and the field's
    keyboard type and action key follow its input options.
  - `incular-android`: hardware keys reach key handlers and text fields.
  - `incular-android`: copy, cut and paste use the system clipboard.
  - `incular-android`: the system back button (and the Android 13+
    `OnBackInvokedCallback`) first reaches the focused key handlers as
    `Code::BrowserBack`, then closes popups, then asks the pop-route
    observers. If none of them takes it, the activity finishes, as with
    Flutter's `SystemNavigator.pop`.
  - `incular-runtime`: `Runtime::handle_text_editing_state` and
    `Runtime::handle_back_button`, with matching `Application` and
    `MobileEngine` entry points, give embedders those two paths.
  - `incular-text`: `TextEditingDelta::between` builds the smallest edit
    between two values, so input formatters see platform edits as user
    edits.
  - `incular-mobile`: `text_offsets` converts between Android's UTF-16
    offsets and Incular's UTF-8 ones.
- Touch text selection toolbars, as in Flutter. On Android, iOS and Fuchsia,
  a long press shows a horizontal toolbar above the selection, or below it
  when there is no room above:
  - `incular-material`: `TextSelectionToolbar` and
    `TextSelectionToolbarTextButton`.
  - `incular-controls`: the themed panel.
  - `incular-widgets`: `TextSelectionToolbarLayoutDelegate` places either
    toolbar.

  The Paste item appears only when the clipboard holds text
  (`Clipboard::has_text`, Flutter's `hasStrings`).
- `incular-widgets`: `BackGestureObserver::handle_pop_route` and
  `dispatch_pop_route`, Flutter's `didPopRoute`. `Navigator` pops on it.

### Changed

- **Breaking (`incular-semantics`):** exhaustive `SemanticState` literals
  need `accessibility_focus_block_type: AccessibilityFocusBlockType::None`
  unless the annotation blocks accessibility focus. Struct literals using
  `..SemanticState::default()` keep the previous behavior.
- **Breaking (`incular-gestures`):** match `GestureAction::LongPress { .. }`
  instead of the unit variant. A release that catches up a missed deadline
  carries start and end details so the arena can dispatch both phases in
  order after accepting the recognizer. Existing widget callbacks are unchanged.
- **Breaking (`incular-mobile`):** `TouchSample` now requires `device`, `buttons`,
  `button`, `stylus`, and `age`. Custom embedders should pass the native device
  ID (or `0`), pressed-button mask, changed button and stylus metadata when
  available, and elapsed time since the sample was recorded (`Duration::ZERO`
  for delivery time).
- **Breaking (`incular-core`, `incular-platform`):** exhaustive matches on
  `InputEvent` and `PlatformEvent` should handle `ScrollAt` and `InputAt`,
  respectively.
- **Breaking (`incular-material`):** `MaterialScrollBehavior::wrap` takes the theme, so the overscroll indicator can follow the Material version and colour scheme. Migrate by passing `&theme` as the first argument. Added `MaterialScrollBehavior::scroll_behavior_for(&ThemeData)`, which picks the Material 3 Android stretch or the Material 2 glow in the secondary colour, as Flutter's `MaterialScrollBehavior` does. `incular-widgets` `ScrollBehavior` gains `overscroll_indicator_style` and `glow_color`.
- **Breaking (`incular-platform`, `incular-widgets`, `incular-material`):** `TextInputType::Number` and
  `TextInputTypeHint::Number` carry `{ signed, decimal }` so a numeric keyboard can offer a decimal
  point or a minus sign, as Flutter does. Migrate `TextInputType::Number` to
  `TextInputType::NUMBER` (integers only, as before) or `TextInputType::number_with_options(signed,
  decimal)`; the same for `TextInputTypeHint` (`NUMBER` / `number_with_options`).
- `incular-platform`, `incular-runtime`: `TextInputCommand::SetClient` attaches and reconfigures the field without
  showing the keyboard; `TextInputCommand::Show` shows it. Adapters that showed the keyboard on
  `SetClient` must handle `Show`.
- **Breaking (`incular-navigation`):** `Navigator::observe_back_gestures` is
  removed because a navigator now registers itself when it is created, so the
  system back pops its routes and predictive back follows the top route. Migration:
  delete the call. A navigator that is dropped stops following back. Nested
  navigators: back goes to the newest one first and falls through to the
  outer ones when it cannot pop.
- **Breaking (`incular-platform`):** `PlatformEvent` gained
  `TextEditingState` and `TextInputCommand` gained `Show` (Flutter's
  `TextInput.show`, sent when the focused field is tapped again). Add arms
  for them to exhaustive matches. Hosts without a soft keyboard can treat
  `Show` like `Update`.
- `incular-runtime`: on Android and iOS a touch outside the focused text
  field leaves it focused, with the keyboard up, as Flutter's
  `EditableText` does there. Mouse and stylus presses still unfocus it.

### Fixed

- `incular-runtime`: windows opened while the app is suspended no longer
  reenable animation ticks before the app resumes.

- `incular-runtime`: app lifecycle observers now receive supported
  intermediate states when a native event skips directly across them.

- `incular-runtime`: desktop app deactivation parks focus and restores it
  on resume only if the target is still mounted and nothing else took focus.

- `incular-runtime`: focus requests made before attachment now settle during
  the first layout that mounts their target, before competing autofocus.

- `incular-runtime`: replacing text programmatically repairs the selection
  immediately in a focused editor, but leaves it unset when unfocused.

- `incular-runtime`: desktop text fields select all on external focus by
  default without overwriting pointer, autofocus, or resume caret placement.

- `incular-gestures`: a focus request made before attachment stays
  pending without publishing a primary-focus grant.

- `incular-runtime`: a newly mounted autofocus control can take focus
  after the previously focused control is removed.

- `incular-gestures`: a late rejection can no longer revoke an already
  accepted arena member.

- `incular-gestures`: synthesized pointer samples no longer distort
  release velocity for retained drags.

- `incular-gestures`: synthesized scale moves update the focal point
  without creating release velocity.

- `incular-gestures`: release velocity estimation no longer allocates
  temporary buffers for its bounded history.

- `incular-rendering`: public path bounds enclose Bézier control points
  like Flutter; layout, paint damage and wgpu culling retain tight bounds.

- `incular-rendering`: rectangular paths now contain their exact edges
  and corners, including the right and bottom boundaries.

- `incular-widgets`: debug painting rejects unsupported non-uniform
  border geometry instead of silently drawing a square inside border.

- `incular-widgets`: omitted directional border sides no longer paint
  a default line. Physical and directional border dimensions count only
  the inset portion of each stroke.

- `incular-widgets`: mounted shortcut registrars apply Num Lock policies
  using the runtime hardware keyboard state.

- `incular-widgets`: character shortcuts now distinguish the case of
  the produced character, as Flutter does.

- `incular-widgets`: callback shortcuts preserve single-activator Num Lock
  policies and run every distinct binding that matches.

- `incular-runtime`: irregular keyboard transitions still update and reach
  handlers; per-window opt-in diagnostics show the event and before/after state.

- `incular-text`: malformed IME replacement ranges no longer silently
  clamp into a different edit. Checked APIs expose the rejection.

- `incular-mobile`: invalidate the system bar style delivery cache when
  the host surface detaches or is replaced, so equal styles can be resent.

- `incular-core`: a failing dependency observer no longer prevents other
  live observers from being notified. Callers can collect the original
  panic payloads instead of resuming the first failure after delivery.

- `incular-animation`: ticker callbacks run in scheduling order with
  other frame callbacks. Unmuting during dispatch waits for the next
  frame, and a ticker started muted begins elapsed time with its first tick.

- `incular-widgets`: a childless Container with only one fixed dimension
  fills the other bounded axis. Alignment is ignored for childless
  Containers unless their explicit constraints are tight.

- `incular-widgets`: containers accept taps on background or foreground
  decoration geometry without implicitly clipping the child. Layout-only
  containers remain non-interactive.

- `incular-widgets`: interpolating a `BoxDecoration` with itself retains
  its blend mode. Its validity check now requires a color or a gradient
  for background blending; a decoration image alone does not qualify.

- `incular-widgets`: `Container` paints ordinary decorations supplied by
  its typed builder. With an explicit `BoxDecoration`, an opt-in clip
  follows the decoration's shape and leaves shadows and outside strokes
  unclipped.
- **Breaking (`widgets`):** `Container::decoration` no longer copies
  decoration properties into `color`, `background`, `border`, or
  `radius`. Inspect the `decoration` field for those properties instead.

- `incular-gestures`: long-press callbacks run in Flutter order: start,
  long press, end, then up.
- `incular-mobile`: Android and iOS gesture timing uses native touch timestamps.
  Batched Android samples keep their original spacing for fling velocity.
- `incular-widgets`: the Android overscroll indicator is now selected from the ambient `ScrollBehavior` and the Material theme (the stretch under Material 3, the glow otherwise) as Flutter does, and scroll views (`ListView`, `SingleChildScrollView` and other viewports) now paint it: dragging past an edge on Android stretches the content under Material 3 or draws the glow, which recedes on release.
- `incular-runtime`: a touch or stylus drag on a text field follows Flutter's gesture rules. iOS never extends the selection with a touch drag, and Android extends it only from a field that already had focus when the press went down. Mouse drags are unchanged.
- `incular-material`: the touch text selection toolbar has its Flutter surface again. Without an ambient `Theme` it fell back to a transparent card with black labels; it now uses the light default (white, or `#424242` on dark, as Flutter). The corner radius is the 22-pixel pill of Flutter, the content padding above the anchor matches `TextSelectionToolbar.build`, and the end button padding is 14.5 as in Flutter.
- `incular-runtime`: a touch or stylus double tap on a text field shows the selection toolbar for the selected word. A mouse double click still does not show it, as Flutter `shouldShowSelectionToolbar`.
- `incular-runtime`: changing a text field's text, by typed text or by an IME edit, hides its open selection toolbar. A selection-only change keeps it open, as Flutter `updateEditingValue`.
- `incular-runtime`: on Android and iOS a finger tap on another text field moves focus to it. Only a
  tap outside every text field keeps the focused field focused, as Flutter's `onTapOutside` does.
- `incular-android`: changing a focused field's configuration, such as a password field's visibility toggle, no longer reopens a keyboard the user dismissed. Focusing a field still shows it.
- `incular-android`: after Rust pushes a selection or text change, the next input method edit that lands on the same state is reported again instead of being dropped as a duplicate.
- `incular-android`: hardware keys without a physical-key mapping, and numpad digits and operators, type their characters as Flutter does.
- `incular-material`: `Scaffold` and `AppBar` apply the safe insets like Flutter. The app bar
  pads its toolbar by the status-bar inset and its background covers it; the body drops the
  top inset under an app bar and the bottom inset when a navigation bar is present; the FAB
  sits 16 pixels above the bottom region and the bottom inset. `NavigationBar`,
  `BottomNavigationBar` and `BottomAppBar` add the bottom inset to their height. Slivers
  do not change yet: their extents need a `BuildContext` that `Sliver::build` lacks.
- `incular-material`: `Material` tints its surface with Flutter's Material 3 elevation opacity table
  (none at elevation 0, 5 % at 1 up to 14 % at 12). A transparent `surface_tint_color` no
  longer greys the surface, so the app bar matches the status bar behind it.
- `incular-android`: safe insets that arrive before the surface reports its density are
  rescaled when the density arrives, instead of staying in physical pixels.
- `incular-widgets`: `Text` and editable text follow the system text scale by
  default (Flutter's `MediaQuery.textScalerOf`); pass `text_scaler` to opt out.
- `incular-android`: the back button pops `Navigator` routes and `WidgetsApp`
  and `Router` stacks instead of closing the app, and the predictive back
  gesture follows the top route. `incular-widgets`: `RootBackButtonDispatcher`
  registers with the back gesture observers when its first callback is added.
- `incular-android`: the app is no longer active from `onStart`, before the
  activity resumes. It becomes active in `onResume` only, as in Flutter.
- `incular-android`: the app reports dark mode, font scale, bold text,
  reduced motion and locales, as Flutter does, and updates them when the
  configuration changes (`onConfigurationChanged`) or the activity resumes.
  `incular-mobile`: `MediaQuery`-style `safe_insets` now subtract the soft
  keyboard from the persistent padding (`max(0, view_padding - view_insets)`)
  instead of adding it, so a shown keyboard reduces the safe bottom edge.
- `incular-widgets`: touch-drag scrolling now matches Flutter's gesture arena
  in two cases that left pages unscrollable on phones:
  - A `GestureDetector` above a scroll view (an app-wide tap or keyboard
    listener, say) no longer hides it. The scroll views that join a touch
    are now found from the hit widget, not from the innermost gesture
    detector.
  - A drag that starts on a scroll view nested inside one on the other axis
    (a horizontal poster row in a vertical page) scrolls whichever view
    matches the drag's direction. Before, only the innermost view could
    take the drag.
- `incular-wgpu`: debug builds no longer crash at startup in the Android
  emulator with host-GPU rendering. wgpu's debug object labels
  (`vkSetDebugUtilsObjectNameEXT`) segfault in the emulator's gfxstream
  Vulkan driver, so Android instances leave `InstanceFlags::DEBUG` off.
- `incular-android`: the app owns `tracing` again. The shell installed a
  global subscriber before the app's `main` ran, so an app's own
  `tracing_subscriber` setup failed with "a global default trace dispatcher
  has already been set". Like iOS, the shell now installs its logcat
  subscriber only as a fallback, after `main` returns.
- `incular-cli`: Android builds of apps whose dependencies compile C with
  CMake (opus, aws-lc, …) work on Windows. The tool exports the NDK it found
  as `ANDROID_NDK_HOME`/`ANDROID_NDK_ROOT`, sets `ANDROID_PLATFORM` to the
  minimum API level, and uses the SDK's newest CMake package with its Ninja
  instead of the Visual Studio generator. Variables you set still win.
- `incular-cli`: apps that enable Incular's `mobile` feature through a
  non-default feature of their own (`mobile = ["incular/mobile"]` beside a
  default `desktop`) no longer fail with "incular-android is not in the
  dependency graph".
- `incular-widgets`: choosing an item in a text field's context menu no
  longer unfocuses the field first. The menu counts as part of the field
  (`TextFieldTapRegion`), so after Paste the caret and keyboard stay.
- `incular-widgets`: a single-line field brings its caret back into view
  when the field narrows, as Flutter's `didChangeMetrics` does when the
  keyboard resizes the view. Before, text typed into a field that a sibling
  then squeezed stayed scrolled to the old width, with its end hidden.
  Scrolling also keeps the caret out of the right content padding, and the
  glyphs are clipped to the area inside the padding.

## 0.1.5 — 2026-11-22

This release adds Android and iOS app hosting, the mobile CLI, and Direct3D 11
rendering on Windows. It requires Rust 1.99.

### Added

- `incular-host`, the shell-independent host core. `HostSurface` owns one
  window's renderer, layout metrics and presentation retry pacing, and
  `HostSurface::present` settles presentation, render metrics and simulation
  captures with the runtime. The Winit shell and the Android and iOS
  embedder present through it.
- Android and iOS run Incular apps (mobile phase 1, see
  [system-design/MOBILE.md](system-design/MOBILE.md)). Apps keep their
  `fn main() { incular::run(app) }`; build with the facade's new `mobile`
  feature (`--no-default-features --features mobile,controls,material`).
  - `incular-mobile`: `MobileEngine`, the FFI-free engine both shells drive.
    It binds the application's first window to the native view, presents
    through `incular-host`, and forwards touches, insets and lifecycle.
  - `incular-android`: a Kotlin `IncularActivity` over a `SurfaceView`, JNI
    entry points registered from `JNI_OnLoad`, `AChoreographer` frames and
    logcat logging. Requires Android 8.0 (API 26). Application data, such
    as `RestorationConfig::file_backed` snapshots, lives under the app's
    `Context.getFilesDir()`.
  - `incular_platform::set_application_data_root` lets an embedder choose
    where `application_data_local_directory` places application data.
  - `incular-ios`: Rust-defined UIKit app and scene delegates and a
    `CAMetalLayer`-backed view driven by `CADisplayLink`. Requires iOS 16.
  - `incular` CLI (`tools/incular-cli`): `incular build|run android|ios
    --example <name>` builds APKs and simulator bundles, installs them, and
    saves screenshots and logs. Install it with
    `cargo install incular-cli --version 0.1.5`.
- The `incular::motion` API adds state-driven animations, enter and exit
  transitions, layout animations, typed drag and drop with keyboard
  reordering, shared elements, and scoped view transitions.
- Accessibility projections now publish all semantic roots, including under
  layout-only widgets with multiple semantic children.
- CI is two workflows. `Build` builds, lints and documents every platform:
  desktop on stable and the MSRV, wasm32, Android APKs and iOS bundles.
  `Screenshots` runs every example on Linux (Xvfb), Windows and macOS, in
  headless Chrome (WebGL2), on an Android emulator and on an iOS simulator,
  and uploads a screenshot and log per example as one artifact per platform (`scripts/screenshots.py`, `scripts/web-smoke.py`).
- `incular_core::IdGenerator`, a `static`-friendly counter that hands out
  unique `u64` identities (`next`, `try_next`). The framework's own identity
  counters now use it instead of about 25 hand-written `AtomicU64` copies.
- Fallible forms for public panic contracts, each paired with the panicking
  form (which now documents `# Panics`): `CatmullRomSpline::try_new` and
  `try_with_handles` (`CatmullRomSplineError`), `try_set_time_dilation`
  (`InvalidTimeDilation`), `ForcePressGestureRecognizer::maybe_pressures`,
  `Canvas::try_restore` and `try_finish` (`UnbalancedCanvas`),
  `Memo::try_get`, `try_get_untracked`, `try_with` and `try_with_untracked`
  (`MemoCycleError`), `Size::maybe_new`, and
  `reactivity::try_assert_mutation_allowed` (`ReactiveMutationError`).
  `Rect::intersection` no longer panics on infinite rects.
- Fallible forms for `incular-widgets` and `incular-material` panic
  contracts, each paired with the panicking form (which now documents
  `# Panics`): `DraggableSheetExtent::try_new` and
  `DraggableScrollableSheet::try_extents` (`DraggableExtentError`);
  `TwoDimensionalViewport::try_new`, `try_set_axis_directions` and
  `try_set_cache_extent`, and `TwoDimensionalScrollable::try_set_axis_directions`
  (`TwoDimensionalConfigError`); `WheelProjection::try_new`,
  `ListWheelViewport::try_new`, `try_set_magnifier`,
  `try_set_over_under_center_opacity`, `try_set_rendering_policy` and
  `FixedExtentScrollController::try_with_item_extent` (`WheelConfigError`);
  `MultiChildLayoutContext::try_layout_child` and `try_position_child`
  (`MissingLayoutChild`); `FlowPaintingContext::try_paint_child`
  (`FlowPaintError`); `RectDragBoundary::try_nearest_position_within_boundary`;
  `RawMagnifier::try_magnification_scale` (`MagnifierScaleError`);
  `StarBorder::try_points`, `try_inner_radius_ratio`, `try_point_rounding`,
  `try_valley_rounding` and `try_squash` (`StarBorderError`);
  `SliverVisibility::try_build` and `try_create_render_sliver`
  (`SliverVisibilityFlagError`); and `StateProperty::maybe_resolve`.

### Changed

- **Breaking (`incular-desktop`):** `PresentationRetry` moved to `incular-host`.
  Import it as `incular_host::PresentationRetry`.
- **Breaking (`incular`):** The default desktop renderer uses Direct3D 11 on Windows,
  Metal on macOS and Vulkan on Linux. The Windows renderer submits the same
  display lists through the system D3D11 driver and DirectComposition. Enable
  `dx12` to select the previous Windows renderer. To restore all previous fallbacks, add
  `features = ["dx12", "metal", "vulkan", "gl"]` to the Incular dependency.
  The `incular::linux` re-export is now Linux-only. Gate platform imports with
  `#[cfg(target_os = "linux")]`.
- **Breaking (`incular-text`):** Windows builds now use the system ICU word
  and line dictionaries. Enable `bundled-dictionaries` for deployment before
  Windows 10 version 1903 or to use the same dictionary version across systems.
  Native builds with bundled data share the layout dictionaries for editing.
  Word selection can change with the dictionary version.

- **Breaking (all crates):** the minimum supported Rust version is now 1.99
  (was 1.89). Update your toolchain with `rustup update stable`.

- **Breaking (`incular-rendering`):** `mask_transform` in
  `PaintCommand::PushShaderMask` and `LayerKind::ShaderMask`, and `stage` in
  `PaintCommand::PushMatrixFilter`, are now `Arc<Transform>`. This shrinks
  every paint command from 160 to 128 bytes, and an unchanged effect reuses
  its transform when the scene is flattened again. Code that builds these
  variants directly passes `transform.into()`; code that reads them
  dereferences (`*mask_transform`).

- **Breaking (`incular-rendering`):** the `image_filter` field of
  `PaintCommand::PushBackdropFilter` and `LayerKind::BackdropFilter` is now
  `Option<Arc<ImageFilter>>`, which shrinks every paint command and layer
  slot. Code that builds these variants directly writes
  `Some(filter.into())`; code that reads the field uses `as_deref()`.
  `LayerTree::update_backdrop_image_filter` still takes an `ImageFilter`.

- **Breaking (`incular-assets`):** `CachingAssetBundle::load_structured_data`
  no longer panics when the key is cached as another type; it returns the new
  `AssetError::TypeMismatch { key }`. `AssetError` is now `#[non_exhaustive]`,
  so `match`es on it from other crates need a wildcard arm.

- `RenderKind` changes below are in `incular_widgets::internal`, which is not
  public API; they are listed for code that builds render kinds directly.

- `RenderKind::Text` and `RenderKind::SelectableText` accept a shared style.
  Pass `Arc::new(style)` when constructing either variant directly.

- Direct constructors of `RenderKind::BackdropFilter` can pass
  `Some(filter.into())` to its shared `image_filter` field.

- **Breaking (`incular-controls`):** `ButtonStyle::mouse_cursor`, the Material
  `ButtonStyle`, `MenuStyle`, `MenuButtonStyle` and icon-button theme
  `mouse_cursor` fields and setters now take a typed `MouseCursor` instead of
  a string, and `incular_controls::styles::mouse_cursor_from_name` is removed.
  An unknown cursor name used to panic at build time; it is now a compile
  error. Migrate `.mouse_cursor("grab")` to `.mouse_cursor(MouseCursor::Grab)`
  (`"pointer"`/`"click"` is `MouseCursor::Click`, `"default"`/`"basic"` is
  `MouseCursor::Basic`, `"forbidden"`/`"not-allowed"` is
  `MouseCursor::NotAllowed`, `"ew-resize"` is `MouseCursor::ResizeHorizontal`,
  and so on).

### Fixed

- `incular run ios` retries the simulator launch when FrontBoard has yet
  to register a freshly installed app.
- macOS window-state observation now leaves undecorated and fixed-size
  windows' native styles intact. For these windows the observed maximized
  state is unavailable.
- macOS apps run from a bare executable (such as `cargo run --example`)
  no longer abort at startup: UserNotifications raises an Objective-C
  exception outside an app bundle, so notifications now report
  `Unsupported` there instead of touching the notification center.
- macOS apps no longer panic in winit (`tried to get a delegate that was
  not the one Winit has registered`) on the first event. Open-file,
  open-URL and reopen handling now extends winit's application delegate
  instead of replacing it.
- Browsers that expose WebGPU without an adapter (blocklisted or GPU-less
  machines) now render with WebGL2 when both backends are enabled.
  `incular-wgpu` checks browser adapter availability before creating the
  surface so the canvas is still available for WebGL2.
- `incular-wgpu` clamps the requested device limits to what the adapter
  supports, so renderer startup no longer fails on adapters just below WGPU's
  defaults (the iOS simulator allows 15 inter-stage shader variables, not 16).
- Scroll layout and tree reconciliation no longer call `std::time::Instant::now`
  from a leftover timing probe, which panicked on `wasm32` and added a timer to
  every frame. The probe module and its `scroll_probe` bench are removed.
- App back dispatch, route-provider subscription, `DraggableScrollableSheet`
  inner scrolling and `TreeSliver` row building no longer hold a `RefCell`
  borrow while application callbacks run, which could panic with
  `BorrowMutError` when a callback called back into the same object. A
  `TreeSliverController` mutation made from inside a row builder is now
  ignored.
- Panics on out-of-range application input now saturate or clamp instead:
  `SplitView` and fling limits with a minimum above the maximum,
  `motion::secs` and `Transition::scaled` with durations too large for a
  `Duration`, a `TapAndDragGestureRecognizer` throttle frequency that
  overflows the deadline, `INCULAR_MEMORY_LOG` set to a huge value, and a
  Material `Date` with `month` outside 1 to 12 when formatted.
- `AtlasEntry::allocation_rect` no longer underflows for zero-area glyph
  entries (spaces).

- `incular-rendering`: `gaussian_kernel_weights` no longer allocates billions
  of weights or overflows `i32` (a debug-build panic) for a very large finite
  blur sigma. Sigma is now clamped to 512 physical pixels, the largest the GPU
  renderer can distinguish (1/16 downsample x 32-pixel kernel radius cap);
  results for smaller sigmas are unchanged.

- Opening a restorable window whose `WindowOptions` have a minimum size larger
  than the maximum now returns `WindowOptionsError::MinimumExceedsMaximum`
  instead of panicking when a saved window state matches. Options are
  validated before restored state is applied.

### Performance

- Retained UI updates skip unchanged subtrees, index callback and semantics
  lookups, skip idle tickers, and reclaim completed route scopes.
- Native RGB8 and RGBA8 image decoding writes directly into the shared
  pixel buffer. Text layout keeps short-line cluster data inline and streams
  cluster offsets for prefix matching.

## 0.1.4 — 2026-11-16

### Added

- Memory breakdowns in DevTools: the Memory view shows process totals (private
  and resident bytes, Rust heap, GPU memory in use and reserved) and what
  holds the memory by owner: GPU allocations by resource label, renderer
  caches, the largest retained layer textures and CPU-side owners. The A/B
  comparison lists the owners that changed most.
- `INCULAR_MEMORY_LOG=<seconds>` writes the same breakdown to stderr from a
  target built with the `devtools` feature.
- `incular::core::memory::CountingAllocator`, an opt-in global allocator that
  lets memory snapshots report the live Rust heap, and
  `incular_image::live_image_stats` for decoded pixels alive on the heap.
- `GpuResourceSummary::backdrop_filters_rendered` and
  `backdrop_filters_reused`.

### Changed

- Backdrop filters keep their result while nothing they read changes, so a
  repaint elsewhere in the window no longer captures and blurs the backdrop
  again or splits the frame into extra render passes.
- Rectangles without corner radii draw through a lighter pipeline, and solid
  fills that are fully transparent are not drawn.
- An opacity layer over a single image fades the image directly instead of
  rendering it into a layer texture.
- The renderer holds less GPU memory: recycled render targets are reused by
  size and released by age and budget, retained layer textures expire after
  30 unused frames, GPU memory blocks are a fixed 4 MiB, a frame with backdrop
  filters composes in its presentation target, and an idle window releases
  the targets only the next frame would use.
- DevTools protocol: `ResponsePayload::MemorySnapshot` boxes its snapshot,
  and `MemorySnapshot` gains `process` and `buckets` (both optional on the
  wire; the protocol version is unchanged).
- Native text layout enables dictionary and model line breaking for Chinese,
  Japanese, Thai, Lao, Khmer and Burmese.

## 0.1.3 — 2026-11-15

This release corrects widget and animation behavior against Flutter 3.41.7,
adds image-loading and host-integration APIs, and reduces retained rendering
and semantics work. The [parity summary](audits/flutter-parity/SUMMARY.md)
records the coverage and remaining behavioral differences.

### Added

- Image streams and completers, plus provider-backed images through
  `Image::provider`, with frame, loading and error builders, gapless playback
  and color blending. `FadeInImage` also accepts providers, error builders
  and per-image colors.
- `ScrollAwareImageProvider` and deferred image loading based on the nearest
  scrollable's velocity, with resolution resumed on later frames as it slows.
- Viewport `center` and `anchor`, including slivers growing before the center
  and negative minimum scroll offsets. Clamping, ballistics, range changes
  and scrollbars honor the minimum; `ShrinkWrappingViewport` accepts a
  controller, axis and reverse direction.
- Child back-button dispatchers with explicit priority for nested routers,
  and `PageTransition` / `TransitionDelegate` decisions for declarative page
  updates. The default delegate animates the new top page and the removed
  route above it.
- `DeviceGestureSettings` for platform-provided touch and pan slops, inherited
  from the nearest `MediaQuery` by widget gesture recognizers.
- `CustomPainter::semantics_builder` and `CustomPainterSemantics` regions,
  ordered around the painted child and rebuilt when its size changes.
- `NetworkAssetBundle`, asynchronous asset loading and an injected
  `AssetFetcher` transport, including relative URL resolution and HTTP status
  errors. Applications supply the network client.
- Spell-check services, suggestion spans and `SpellCheckConfiguration` for
  editable text, with stale-result reconciliation and configurable
  misspelled-word styling. A native checker requires a service implementation;
  the default service returns no results.
- Host-reported predictive back gestures that drive the current route's
  animation, commit a pop or animate back on cancellation.
- Platform-view identities, controllers, surfaces, pointer forwarding and
  per-window placement/command reports. Displaying and compositing native
  views requires host integration; current desktop services discard these
  placement commands.
- Platform undo/redo events routed to the focused text field's history, and
  per-window undo-state commands for host adapters. Native undo-manager
  integration requires the host to handle those commands.
- `AnimatedSize::reverse_duration`, forwarded to its size controller.
  Following Flutter's animated-size behavior, size changes still run forward
  and use the forward duration.
- `GpuCounters` adds `backdrop_capture_texels`, `blur_output_texels` and
  `effect_pass_submissions`.

### Changed

- Text input formatters and `on_changed` callbacks apply to user edits.
  Programmatic controller updates preserve the supplied text without running
  user-edit transforms or callbacks.
- Floating headers snap at scroll end by default; `.snap(false)` opts out.

### Fixed

- Animation graph notifications and direction latching, ticker-mode
  inheritance, implicit animation geometry and transition timing. Gravity
  simulations with negative acceleration now complete by absolute distance.
- Gesture-arena resolution, drag slop and local coordinates, drag feedback,
  and reorderable-list long-press, gap, proxy and drop-animation behavior.
- Directional keyboard focus traversal and page/line scrolling of focused
  views, while text fields retain their editing key behavior.
- Intrinsic and constrained layout, table sizing and alignment, odd-turn
  `RotatedBox` dimensions, consumed `SafeArea` insets, clipping and shape-border
  painting behavior.
- Rich-text run styles and embedded spans, editable-text geometry and
  selection, long-press selection and selectable plain text in selection
  areas. Forms save field state and callbacks even when a field is invalid.
- Scroll-jump activity notifications, floating-header timing, varied-extent
  sliver sizing, kept-alive children, shrinking `SliverFillRemaining` content
  and draggable-sheet handoff to its inner list.
- Hero flights on route pops and overlay portals escaping ancestor clips.
- Merged and label-only semantics, the default enabled state and semantic
  actions exposed by gesture detectors. Android/iOS adjustment projections
  immediately show the increased/decreased value after an accepted action.

### Performance

- **Backdrop blurs cost about what their clip shows.** A `BackdropFilter`
  blur produces only the pixels its parent clip can show and copies only
  those plus the kernel's reach from the target, as Impeller's coverage hint
  does; without a clip it still covers the whole target. Stages sharing a
  `BackdropGroup` share one capture covering all of them.
- **Gaussian blurs downsample as Impeller does.** Every blur (backdrops,
  `ImageFiltered`, box shadows) runs at the power-of-two scale that brings its
  physical sigma near 4 (at most 1/16), with a box-filtered downsample and a
  bilinear upsample. The blur shader loops only to the kernel radius and
  merges adjacent taps into one bilinear fetch, and each blur is one queue
  submission. The pixel harness stays within its previous error against
  Flutter at 1, 1.25, 1.5 and 2x, and the clipped backdrop scene gets closer.
- Retained instance staging and batched GPU uploads, fewer redundant scissor
  and stencil changes, cached rectangle/oval paths and faster resource-map
  lookups. Lifted popup scenes avoid unnecessary flattening.
- Incremental semantics invalidation follows geometry changes, including
  moving sliver children, with less shared-metadata copying. Text-field input
  paths avoid copying text when only editing state is needed.

## 0.1.2 — 2026-11-12

This release rebuilds animation on Flutter's scheduler and controller model,
adds route and Material page transitions and button ink, and brings layout,
hit testing, popups and rasterization closer to Flutter 3.41.7. It also
fixes the problems found while building an application on 0.1.1: tooltips
painted under later widgets, clicks swallowed by transparent layers and
outer gesture detectors, panics on infinite constraints and on signal writes
during a build, and off-centre button labels.

### Upgrading from 0.1.1

- **Proxy widgets pass their constraints through, as in Flutter.**
  `DecoratedBox` (and `Container` with a colour or decoration), `Transform`,
  `FractionalTranslation`, `Opacity`, `BackdropFilter` and the other effects,
  `GestureDetector`, `Visibility`, selection and semantics wrappers and
  `RawScrollbar` used to loosen their child's constraints. Under a tight
  parent the child is now sized tightly too. Layouts that relied on a child
  keeping its natural size inside a tightly sized wrapper change; place the
  child with `Align` (or `Center`) to keep it at its own size.
- **Empty layouts no longer take hits.** A childless sized `SizedBox` or
  `Container`, and an empty `Column`, `Row`, `Stack` or `IndexedStack`, pass
  pointer events to what is below them, as Flutter's render boxes whose
  `hitTestSelf` is false do. To make an empty area catch taps, give its
  `GestureDetector` `HitTestBehavior::Opaque` or give the area a colour.
- **A button inside a `GestureDetector` wins the tap.** The outer detector's
  `on_tap`, `on_tap_down`, `on_tap_up` and `on_tap_cancel` no longer fire for
  taps on an enabled button below it, as Flutter's `InkWell` wins the tap
  arena. The outer detector's drag, long-press and scale callbacks still
  compete, and a drag that wins cancels the button press.
- **A signal written during a build no longer panics.** The write is reported
  once per widget on stderr and through `tracing`, the value changes at once,
  and the widgets that depend on it rebuild in the next frame. Builders should
  still not write state; move the write to an event callback, `Effect`,
  `Action` or `scheduler::add_post_frame_callback`.
- `AnimationController` follows Flutter's runs. An interpolation ends on the
  first frame strictly past its duration; `animate_to` and `animate_back` set
  the direction as Flutter's do; `repeat` starts from the current value; and
  `fling` uses a critically damped spring.
- `PageTransitionsTheme` gains a `builders` field, so struct literals need
  `..PageTransitionsTheme::default()`.
- `SimulationError` gains `AppPanicked`; exhaustive matches need an arm for it.

### Added

#### Animation

- A Flutter-style frame scheduler (`incular_animation::scheduler`): `Ticker`,
  frame, post-frame and persistent callbacks, and the current frame time.
  Tickers started during a frame use that frame's time, and the runtime ticks
  animations before the build, so an animation started or read in a build
  loses no time.
- `AnimationController` rebuilt on Flutter's semantics: `animate_to` and
  `animate_back`, `repeat(count, reverse)`, `fling`, `toggle`, reverse
  duration, velocity (a central difference, as in Flutter), status listeners,
  and vsync controllers that tick themselves. Every run returns a
  `TickerFuture` that completes when the run ends and is cancelled when it is
  stopped or replaced; `or_cancel` reports either outcome.
- The remaining Flutter curves (`decelerate`, the elastic curves, `Interval`,
  `Threshold`, `SawTooth`, `Split`, `ThreePointCubic`, ...), `Curve::flipped`,
  and `Curve2D`, `CatmullRomSpline` and `CatmullRomCurve`, whose samples match
  Flutter's to 1e-8.
- The animation graph: `CurvedAnimation` with its direction latch, `Reverse`,
  compound, proxy and train-hopping animations, `AlwaysStopped`, curve,
  constant, step, reverse and chained tweens, and `AnimationStyle`.
  `incular-widgets` re-exports these types.
- `AnimatedBuilder`, `ListenableBuilder` and `Listenable`; fade, scale,
  rotation and slide transitions driven by an animation on the compositor
  alone; `AlignTransition`, `SizeTransition`, `PositionedTransition`,
  `RelativePositionedTransition`, `MatrixTransition` and `RelativeRect`.
- `TickerMode` publishes an inherited scope; `TickerMode::of` reads it, and
  implicit animations under a disabled mode are muted.
- `ScrollController::animate_to` runs a curved scroll in the widget tree; a
  zero duration jumps, and reduced motion jumps.

#### Navigation and Material

- Route push and pop animations: `RouteTransition::Animated` carries a
  route's controller, secondary animation and transition builder.
  `RouteOutlet` runs pushed routes forward, keeps popped and replaced routes
  on top while they reverse, and hides the routes below only once a page's
  animation completes.
- Material page transitions: `PageTransitionsBuilder` (zoom, fade-forwards,
  fade-upwards and Cupertino) with Flutter's per-platform defaults, through
  `PageTransitionsTheme` and `ThemeData::page_route_transition`.
- Buttons show Flutter's `InkRipple` or `InkSplash` press feedback, chosen by
  the style's splash factory (sparkle falls back to the ripple), for pointer
  presses and keyboard activation.

#### Layout, input and tooling

- `Constraints::at_most(max_width, max_height)` and `with_min_width`,
  `with_max_width`, `with_min_height` and `with_max_height` build constraints
  with unbounded maximums ("at most this wide, any height"), which a `Size`
  cannot hold.
- `PointerSignalResolver` lets a listener claim a wheel event away from the
  scroll views below it, as in Flutter.
- `Simulation` reports a panic on the application's UI thread: the waiting
  call and every later one return `SimulationError::AppPanicked` with the
  panic message and location.
- `Transform::coefficients`, the inverse of `Transform::from_coefficients`.
- DevTools has a reworked workbench: a grouped sidebar with error badges, a
  Ctrl+K command palette, Overview insights, console level counts and repeat
  grouping, a network waterfall with request details, and memory snapshot
  history. Streamed frames redraw only the views that changed.
- The repository's `harness/` folder holds the Flutter parity harnesses: a
  frame-by-frame comparison of geometry, transforms, opacity, colour and
  sampled values against the same scenario in Flutter, and the pixel
  comparison, moved from the `incular-desktop` tests. Both are opt-in and
  the `incular-parity-harness` package is not published.

### Changed

- The WGPU renderer rasterizes like Skia: glyphs are hinted with the TrueType
  interpreter, oversampled and preblended with Skia's mask gamma; rectangles,
  rounded rectangles and borders get Skia's analytic edge coverage; and blurs
  up to sigma 32 device pixels run at full resolution.
- Button state overlays fade as `InkWell`'s highlight does (50 ms, 200 ms when
  pressed), and elevation and shadow animate over
  `ButtonStyle::animation_duration`.
- `AppBar` puts its toolbar and `bottom` slot in one `Material`, as Flutter's
  does, so the bottom slot gets the bar's background, elevation and hit
  surface.
- `Container` sizes as Flutter's does: its `width` and `height` are clamped
  into its `constraints`, and an infinite `width` or `height` fills the
  parent's maximum.
- The modal barrier fades through an animated widget, and `Stack` lays out the
  unconfigured axes of positioned children with unbounded constraints
  (Flutter's `layoutPositionedChild`).

### Fixed

- Tooltips, menus and other `OverlayPortal` popups shown inside the window
  painted in tree order, so the widgets after their anchor covered them. They
  now paint above everything else, as entries of Flutter's `Overlay` do, and
  an ancestor's clip no longer cuts them off.
- Showing or hiding an `OverlayPortal` remounted its anchor, losing the
  anchor's state.
- A `Button` with a `height`, a `minimum_size` or a tight parent drew its label
  at the top-left of its content box instead of centring it.
- A `Text` or other shrink-wrapping child under a tightly sized
  `GestureDetector` or decoration only took taps on its own natural bounds.
- A transparent layer above a click-away barrier swallowed clicks outside its
  content, and a full-size `Align` or `Padding` swallowed taps meant for the
  widgets behind it.
- `Container::width(f32::INFINITY)` and `height(f32::INFINITY)` panicked.
- Each scroll view reads the mouse wheel on its own axis, as Flutter's do: a
  vertical wheel over a horizontal list scrolls the page around it, and Shift
  turns the wheel horizontal.
- Box shadows were clipped to the decorated box, cutting off offset shadows.
- Blurs were shifted by one pixel and dropped the first source row and column.
- Text shadows did not paint, and aligned text in a shrink-wrapped box could
  sit outside the box.
- `Icon` maps an icon set's path by its authoring canvas (`view_box`), keeping
  the set's built-in padding, and outline icons scale their stroke with the
  icon.

## 0.1.1 — 2026-11-10

This release brings Incular much closer to Flutter 3.41.7. It adds a browser
target and about a hundred Flutter widgets, gesture recognizers and Material
components. Framework defaults now match Flutter's values. It also adds a
chainable developer-experience layer and a large round of performance work.

Several changes are not source compatible with 0.1.0 (see
[Upgrading from 0.1.0](#upgrading-from-010)). Most of them follow Flutter's
behaviour where 0.1.0 differed from it.

### Upgrading from 0.1.0

- **A text direction is required, as in Flutter.** `Text`, `EditableText` and
  `Banner` with no `text_direction` and no ancestor `Directionality` now fail
  to mount with `TreeError::MissingDirectionality`. Horizontal `Row`/`Flex`
  and `Wrap` need one, in debug builds only, wherever Flutter asserts. 0.1.0
  silently assumed left-to-right. Wrap a bare `Application` root in
  `Directionality::new(TextDirection::Ltr, ...)`. `MaterialApp` and
  `WidgetsApp` provide a direction already: MaterialApp takes it from the app
  locale, then the platform locale, then LTR.
- `BuildContext::text_direction()` returns `Option<TextDirection>` (Flutter's
  `Directionality.maybeOf`). `TextAlign` gains the physical `Left` and `Right`;
  `Start` and `End` resolve against the paragraph direction.
- `AnimationStatus` has Flutter's four values (`Dismissed`, `Forward`,
  `Reverse`, `Completed`); `Idle` and `Stopped` are gone. Controller values
  clamp to their bounds, `animate_to` scales its duration by the remaining
  distance, and `Curve` output is no longer clamped, so `easeInBack`,
  `easeOutBack` and the elastic curves overshoot as in Flutter.
- `GestureDetector::on_pan_update` receives `DragUpdateDetails` instead of an
  `Offset`. Use `details.delta`. Gesture details now carry positions in the
  detector's local space; in 0.1.0 local positions equalled global ones.
- `LinearGradient`, `RadialGradient` and `SweepGradient` gain a public
  `tile_mode` field, and `RadialGradient` gains `focal` and `focal_radius`.
  Struct literals must set them; `LinearGradient::new` sets a clamped gradient.
- `DecoratedBoxTransition::new` and `DefaultTextStyleTransition::new` take a
  `TweenAnimation`, and `DualTransitionBuilder::new` takes a controller and
  forward and reverse builders. In 0.1.0 they only passed their child through.
- `Dismissible::threshold(f32)` is replaced by
  `dismiss_threshold(direction, f32)`. The per-direction default is Flutter's 0.4.
- `Semantics::toggled` sets the toggled state (`hasToggledState`) instead of a
  checked state.
- `TextClusterSpan::start` and `end` are `u32` byte offsets.
- Hand-written error types now derive `thiserror`. `ControlState`,
  `Invalidation`, `TextDecoration` and `ShortcutModifiers` are `bitflags` types,
  and their `Debug` output lists flag names.
- **Mouse drags no longer scroll.** As with Flutter's `ScrollBehavior`, scroll
  views drag only for touch, stylus, trackpad and unknown pointers. The mouse
  scrolls with the wheel and the scrollbar. `ScrollConfiguration` with a
  custom `ScrollBehavior` restores mouse dragging.
- **Default values now match Flutter's** and change layout and appearance:
  - The default font size is 14.
  - `TextHeightBehavior` applies the height to both ends of a line.
  - `Shadow` and `BoxShadow` default to black.
  - Pan and drag slops depend on the pointer kind.
  - Scrollbars default to 6 px with an 18 px minimum thumb, and are hidden at
    rest.
  - `PageView` builds only the visible pages.
  - Material components use Material 3 metrics (see *Changed*).
- Colours are composited in encoded sRGB, as in Flutter, instead of linear
  light. Translucent colours over other content therefore look darker than in
  0.1.0, and now match Flutter.

### Added

#### Browser target

- `incular-web` runs applications on `wasm32-unknown-unknown` through the
  same Winit event loop, input, renderer and accessibility paths as the
  desktop hosts. The first window fills the viewport (or a canvas the page
  provides); later windows float over the page. Enable it with the facade's
  `web` feature, which turns on `web-host` and both the `webgpu` and `webgl`
  backends. `desktop-host` and `web-host` share a `runner` feature.
- In the browser:
  - Input methods and on-screen keyboards edit through a hidden textarea at
    the caret.
  - Screen readers get ARIA elements mirrored over the canvas once the hidden
    "Enable accessibility" button is activated.
  - `BrowserRouteInformationProvider` connects routers to session history.
  - The clipboard keeps HTML and images as well as text.
  - Restoration snapshots persist to `localStorage`.
  - Noto fonts for scripts the bundled fonts do not cover load on demand.
  - `Intl.Segmenter` supplies word boundaries.
- Inter Regular and Bold are bundled behind the default `default-font` feature.
  Browsers expose no system fonts.
- WGPU gains `webgpu` and `webgl` backend features.
- `incular_core::time` (also `incular::time`) provides clocks that work on
  wasm32 as well as native. `incular_runtime::sleep` is a portable timer.
- Every example runs in the browser except `issue_tracker_memory_profile`.
  `scripts/web-smoke.py` loads each example in headless Chrome with WebGPU and
  WebGL2.
- Text hooks for hosts without system fonts:
  - Layouts report characters drawn with the missing glyph.
  - Registering a font lays the tree out again.
  - Hosts can supply their own word segmenter
    (`incular_text::set_word_segmenter`).
- Desktop platform services can apply text-input commands, deliver input
  events from outside Winit, and present semantics through the stable-ID
  projection.

#### Widgets (Flutter parity)

- Implicit animations: `AnimatedContainer` (including
  `foreground_decoration`), `AnimatedPadding`, `AnimatedAlign`,
  `AnimatedFractionallySizedBox`, `AnimatedPositioned`,
  `AnimatedPositionedDirectional`, `AnimatedScale`, `AnimatedRotation`,
  `AnimatedSlide`, `AnimatedOpacity`, `AnimatedDefaultTextStyle`,
  `AnimatedPhysicalModel`, `TweenAnimationBuilder` and `SliverAnimatedOpacity`.
- `AnimatedSwitcher`, `AnimatedSize`, `AnimatedCrossFade`, `FadeInImage`,
  `RepeatingAnimationBuilder` and `SliverFadeTransition`.
- `DecoratedBoxTransition`, `DefaultTextStyleTransition` and
  `DualTransitionBuilder` now animate, through `TweenAnimation`.
- Stateful builders keep their state across descriptor updates, like
  Flutter's `State` across `didUpdateWidget`.
- Tweens:
  - Animation: `ColorTween`, `IntTween`, `RectTween`, `SizeTween`,
    `Matrix4Tween` and the generic `LerpTween`.
  - Widgets: `AlignmentTween`, `BorderRadiusTween`, `BorderTween`,
    `BoxConstraintsTween`, `DecorationTween`, `EdgeInsetsTween`,
    `TextStyleTween` and the directional variants.
  - `Lerp` for `Transform` and `Constraints`.
- `Hero`, `HeroMode` and `HeroController`. Heroes fly between page routes on
  push.
- `FutureBuilder`, `StreamBuilder` and `StreamBuilderBase`. They read
  UI-thread `UiFuture`/`UiCompleter` and `UiStream`/`UiStreamController`
  values that executor work completes through `spawn_into`.
- Custom layout:
  - `CustomSingleChildLayout` with `SingleChildLayoutDelegate`.
  - `CustomMultiChildLayout` with `MultiChildLayoutDelegate` and `LayoutId`.
  - `Flow` with `FlowDelegate`.
  - `PositionedDirectional` and `IgnoreBaseline`.
- Inherited data:
  - `SharedAppData` and `InheritedModel`, with per-aspect dependencies.
  - `InheritedNotifier`.
  - `CapturedThemes`.
- `NotificationListener::on::<T>`: notifications of any type bubble from
  descendants (`NotificationDispatcher`).
- Size and layout utilities:
  - `SizeChangedLayoutNotifier` and its notification.
  - `DragBoundary`.
  - `MetaData` (`WidgetTree::hit_test_meta_data`).
  - `DisplayFeatureSubScreen` and display features on the environment.
  - `View::of` and `View::maybe_of`.
- `FlutterLogo`, in its mark, horizontal and stacked styles.
- Shapes:
  - `ShapeDecoration`, `DecoratedShapeBox` and `ClipPath::shape`.
  - `StadiumBorder`, `StarBorder`, `RoundedSuperellipseBorder` and
    `RoundedRectangleBorder`, behind a `ShapeBorder` trait.
  - `BorderSide` stroke alignment.
  - `PathBuilder::conic_to`, `rounded_rect` and `rounded_superellipse`.
- Backdrop filters:
  - `BackdropFilter` blur and matrix filters now run on WGPU.
  - Backdrop keys and `BackdropGroup` share one backdrop capture.
  - `RawMagnifier`, `MagnifierInfo`, `TextMagnifierConfiguration` and
    `ToolbarOptions`.
- Menus and overlays:
  - `RawMenuAnchor`, `RawMenuAnchorGroup` and `MenuController`, with Flutter
    3.41's focus and close behaviour.
  - `OverlayPortal::overlay_child_layout_builder`.
- `PhysicalModel` and `PhysicalShape` are exported from `incular-widgets` and
  the prelude.
- `SliverSemantics` and `SliverEnsureSemantics`. `SliverChildBuilderDelegate`
  and `SliverChildListDelegate`, with delegate constructors on `SliverList`
  and `SliverGrid`.
- `StretchEffect`, `StretchingOverscrollIndicator` and
  `GlowingOverscrollIndicator`. Clamping physics report overscroll.
- `ScrollBehavior` and `ScrollConfiguration` follow Flutter's per-platform
  policies: physics, drag devices, scrollbars, overscroll indicators and
  velocity trackers. `TargetPlatform` moved to `incular-config` and gained
  `Fuchsia`.
- Scroll physics:
  - Flutter-exact ballistics: `ClampingScrollSimulation` and
    `BouncingScrollSimulation`, with spring hand-off at the extents.
  - `ScrollDecelerationRate`.
  - Tolerance scaled by device pixel ratio.
  - Momentum carried into caught flings.
  - Presets `ScrollPhysics::CLAMPING`, `BOUNCING`, `NEVER_SCROLLABLE` and
    `ALWAYS_SCROLLABLE`.
- Widget options from Flutter 3.41:
  - Drag start, clipping and hit testing:
    - `drag_start_behavior` on gesture detectors, scroll views and
      `Dismissible`.
    - `clip_behavior` and `hit_test_behavior` on scroll views.
    - `PageStorageKey` scroll offsets.
  - Pages and visibility:
    - `PageView::pad_ends`.
    - `Visibility::maintain_interactivity` and `Visibility::maintain`.
  - `Dismissible` is a full port of Flutter's.
  - `Draggable` and `DragTarget` gain affinity, axis, anchor strategies and
    drag callbacks.
  - `InteractiveViewer` with `TransformationController` and `PanAxis`.
  - Gradient tile modes and two-point conical gradients.
  - `AnimationController` gains `animation_behavior`, `unbounded()` and
    `animate_with(simulation)`.
  - Raw pointer events gain deltas, local positions and scroll deltas.
- Text and text input:
  - `TextHeightBehavior` is applied, and `DefaultTextHeightBehavior` is added.
  - `EditableText` keyboard traits: `autocorrect`, `enable_suggestions`,
    `text_capitalization`, `keyboard_appearance`, `autofill_hints` and
    others, through `TextInputTraits`.
  - `scroll_padding` caret reveal and `cursor_opacity_animates`.
- `MediaQueryData` accessibility and text values: bold text, inverted
  colours, 24-hour time, accessible navigation, system gesture insets,
  navigation mode and the line-height, letter, word and paragraph spacing
  overrides. `Text` and `EditableText` apply them.
- Semantics gains toggled, hidden, live region, tooltip, link URL and
  identifier, all exported to AccessKit.
- `AssetBundle` (in-memory, directory and caching) and `DefaultAssetBundle`.

#### Gestures, keyboard and focus

- New recognizers:
  - `MultiTapGestureRecognizer` and `SerialTapGestureRecognizer`.
  - `TapAndHorizontalDrag` and `TapAndPan`.
  - `ForcePressGestureRecognizer`.
  - The immediate, horizontal, vertical and delayed multi-drag recognizers.

  Each follows Flutter's arena protocol, and `RawGestureDetector` accepts
  them through `ArenaGestureRecognizer`.
- `ScaleGestureDetector` ports Flutter's scale state machine and accepts past
  `kScaleSlop`. Its details gain horizontal and vertical scale, rotation and
  velocities.
- A `constants` module with every value from Flutter's
  `gestures/constants.dart`.
- Timing follows Flutter:
  - `onTapDown` waits for `kPressTimeout`.
  - Long press starts on its timeout.
  - Double tap uses Flutter's slops and timings.
  - Velocity uses Flutter's least-squares tracker.
- `FlingLimits`: scrollable physics set the fling velocity and distance
  limits.
- `DragStartDetails`, `DragUpdateDetails` and `DragEndDetails` are in the
  `incular` prelude.
- `ShortcutRegistry` and `ShortcutRegistrar` (included in `WidgetsApp` and
  `MaterialApp`) and `CharacterActivator`.
- `LogicalKeySet` and `FocusOrder`. `FocusTraversalOrder` accepts a lexical
  order (`FocusTraversalOrder::lexical`) as well as a numeric one.
- Per-platform text-editing shortcut tables (`DefaultTextEditingShortcuts`):
  - Word, line, paragraph and document movement.
  - Transpose.
  - On Apple platforms, Home/End/PageUp/PageDown scroll the field.

  `SelectableText` uses the same tables.
- Desktop text gestures: double-click selects a word, triple-click selects a
  paragraph, and shift-click extends the selection, all per platform.
- Text context menus: `EditableText`'s `context_menu_builder`, with
  `ContextMenuButtonItem`, `ContextMenuButtonType` and
  `TextSelectionToolbarAnchors`.
  - Controls fields show a themed popup.
  - Material fields show `DesktopTextSelectionToolbar`, or
    `CupertinoDesktopTextSelectionToolbar` on macOS, chosen by
    `AdaptiveTextSelectionToolbar`.

#### Navigation and runtime

- `RouteObserver` and `RouteAware` (with `RouteAwareCallbacks`) report
  pushes and pops relative to a subscribed route. `ModalRoute::of` and
  `ModalRoute::settings_of` find the presenting route once
  `RouteOutlet::set_modal_route_scopes(true)` is set.
- `RestorableInt`, `RestorableDouble`, `RestorableBool`, `RestorableString`
  and their nullable `…N` variants.
- The route observers, shortcut registry, focus order, `LogicalKeySet`,
  inherited notifiers, text context menu types, named tweens and typed
  restorables are in the `incular` prelude.

#### Material

- Material catalog:
  - Tables: `DataTable` and `PaginatedDataTable` (with `DataTableSource`).
  - Pickers:
    - `CalendarDatePicker`, `DatePickerDialog`, `DateRangePickerDialog` and
      `TimePickerDialog`.
    - `show_date_picker`, `show_date_range_picker` and `show_time_picker`.
  - Search and selection: `SearchBar` and `SearchAnchor` (with
    `SearchController`), `SegmentedButton` and `ToggleButtons`.
  - Steps and expansion: `Stepper`, `ExpansionTile` and `ExpansionPanelList`.
  - Lists and grids: `CarouselView`, `ReorderableListView`, `GridTile`,
    `GridTileBar` and `TabPageSelector`.
  - App bars and banners: `FlexibleSpaceBar` and `MaterialBanner`.
  - Sheets and refresh: `show_modal_bottom_sheet` and `RefreshIndicator`.
  - About: `AboutDialog`, `AboutListTile`, `LicensePage` and
    `LicenseRegistry`.
  - Icons and ink: `AnimatedIcon`, and the `InkRipple`, `InkSplash`,
    `InkSparkle` and `NoSplash` splash factories.
- Menus:
  - `MenuAcceleratorLabel` (Alt mnemonics such as `&File`) and
    `MenuAcceleratorCallbackBinding`.
  - Shortcut labels on `MenuItemButton` (`shortcut_label`,
    `character_shortcut_label`).
  - `CheckboxMenuButton` and `RadioMenuButton`.
- `MaterialScrollBehavior` follows Flutter's.
- The catalog, menu accelerators, menu buttons and desktop text toolbars are
  in `incular::material_prelude`.

#### Developer-experience layer

- `WidgetExt` adds chainable modifiers to every widget:
  `Text::new("Hi").padding_all(8.0).background(color).on_tap(...)`. Each
  modifier wraps the receiver in the widget Flutter code would write by hand.
- `AnimateExt::animate()` builds effect chains, after `flutter_animate`:
  - Effects: `fade_in`, `slide_y`, `scale`, `rotate`, `blur_in`, `shake` and
    more.
  - Ordering: `then()` sequences effects, and `delay` postpones one.
  - Repeats and targets: loop and ping-pong repeats, and `target` for
    state-driven effects.
- Responsive layout, after `responsive_framework`: `ResponsiveBreakpoints`,
  `ResponsiveValue`, `ResponsiveVisibility`, `ResponsiveRowColumn` and
  `ResponsiveScaledBox`.
- All of these are in `incular_widgets::extensions` and the `incular`
  prelude.

#### DevTools and tools

- `devtools_playground` example: one app that exercises every DevTools feature
  (continuous motion, jank and rebuild-storm toggles, keyed editable widgets,
  named editable signals, scroll/layout/clip/transform layers, text and
  semantics samples). Run with `--features devtools`.
- `incular::devtools::log(LogLevel, target, message)`: application log messages
  appear in the DevTools Console. Records go to a bounded, thread-safe queue
  (1024 entries, oldest dropped and reported, 4 KiB per message) that the
  desktop host forwards as `Log` events and wakes for while idle. It is a no-op
  without the `devtools` feature.
- `devtools_playground` Console page with buttons that log info, warning, error
  and a 100-message burst; `INCULAR_PLAYGROUND_LOG_DEMO=1` adds a background
  heartbeat log.
- DevTools UI: typed in-place property editors (number steppers, drag-scrub and
  arrow keys, colour swatch/palette/hex, enum segmented/menu/3x3 alignment grid,
  linked inset fields, switch, text) with override markers, per-property Undo,
  Reset all overrides and inline rejection messages; the dark-theme tree row
  hover is now a subtle accent wash instead of mid-grey.
- `DocumentEditor` in Controls: a `TextArea` for long documents such as source
  files and logs. It lays its text out line by line through the new opt-in
  `EditableText::paragraph_layout`, so an edit lays out only the lines it
  changes. Text gains `TextDocumentLayout` (`TextEngine::layout_paragraphs`)
  and the `TextGeometry` trait that it and `TextLayout` implement; `TextLayout`
  keeps its Flutter-shaped API.
- DevTools protocol 3 (older protocol 1 and 2 peers still connect): read-only
  `GetRuntimeDiagnostics` (widget pipeline, full text-cache, semantics and
  compositor counters), bounded `GetSemanticsTree` and `GetLayerTree`,
  `GetWorkCounts` (per-element rebuild ranking) and `HighlightRect`. `Hello`
  gains `take_over`, and `ErrorCode` gains `SessionBusy`.
- DevTools protocol 4 (protocols 1 to 3 still connect): `DebugProperty` gains
  `options`, `min`, `max` and `step` editor metadata, and `DebugOption` gains
  `PerformanceOverlay`; every addition decodes with a default on older peers.
- DevTools property editing: Text (text, fontSize, color), DecoratedBox and
  Container (background, borderWidth, borderColor, radius), Padding (padding),
  SizedBox and ConstrainedBox (width, height, min/max extents), Row and Column
  (mainAxisAlignment, crossAxisAlignment, mainAxisSize, spacing), Wrap spacing,
  Align (alignment), Opacity, Visibility and Box (color). Each edit reaches the
  retained element through the normal update path, so it invalidates exactly
  what the property affects. An override is re-applied when the application's
  own rebuild replaces the widget, until it is reset. Editable properties carry
  `options`, `min`, `max` and `step`.
- DevTools target overlays: phase flashes now fade out over about 500 ms
  (the host keeps redrawing the window until they finish) and draw a 2 px
  outline per changed element with a light fill on the innermost ones; Repaint
  Rainbow draws a persistent outline whose colour rotates on every repaint;
  layout bounds and the other geometry overlays are outlines instead of stacked
  fills; the selected or hovered node is highlighted Chrome-style (blue content
  box, green padding band, edge outline and a `Type  W x H` label); and a new
  Performance overlay draws a CPU and GPU frame graph in the window's top-right.
- DevTools UI connection life cycle: a single state (searching, connecting,
  connected, session busy, taken over, disconnected with a retry countdown)
  drives the header badge, status text and console. Plain disconnects reconnect
  with 1 to 8 s backoff; a busy or taken-over session never reconnects on its
  own and offers an explicit take over. Failures are logged once per change
  instead of on every dropped request. The UI also stores the v3 diagnostics,
  semantics, layer and work-count responses and polls diagnostics while the
  Rendering view is open.
- DevTools UI views for protocol 3: Rendering (text cache, widget pipeline,
  semantics and compositor counters with per-second deltas, plus a Top rebuilds
  table that opens the widget in the inspector) and searchable Semantics and
  Layers trees with details panes whose selection outlines the node in the
  target window. The offline workspace lists every live target with Connect,
  Take over and Refresh list.
- DevTools UI light theme (header toggle, `INCULAR_DEVTOOLS_THEME`), view
  shortcuts (Ctrl+1 to Ctrl+0, Ctrl+PageUp/PageDown, Ctrl+R, Ctrl+E), a Layers
  toggle that hides identity transforms, and a theme pass in the visual review.
- DevTools UI 2x and 4x animation speed presets next to slow motion; targets
  accept time scales up to 4x (`MAX_ANIMATION_TIME_SCALE`).

### Changed

- Material components use Flutter's Material 3 defaults:
  - Buttons:
    - At least 64x40 and stadium-shaped.
    - The padded tap target centers the button in a 48x48 box instead of
      growing the visible surface.
    - Icon buttons are 40x40.
  - Selection controls: the `Switch` track is 52x32, `Checkbox` corners are
    2 px, and the `Slider` track is 4 px.
  - FAB and `Card`:
    - The FAB uses per-size corner radii, and extended FABs are 56 tall.
    - `Card` has radius 12, a 4 px margin and no padding.
  - Dialogs: elevation 6, radius 28, a 280 px minimum width and Material 3
    paddings.
  - `Drawer` is 304 wide with 16 px end corners.
  - `Divider` and `VerticalDivider` take 16 px of space.
  - `Tooltip`:
    - It shows at once, for 1.5 s, 24 px from its target.
    - Padding, height and font size depend on the platform.
  - Progress indicators: the circular one is 36 px with a 4 px stroke, and
    the linear one is 4 px tall.
  - `Chip`: radius 8, 18 px icons and Material 3 padding.
  - `ListTile`: padding 16 at the start and 24 at the end, and a leading
    slot of at least 24 px.
  - `AppBar`:
    - It is 56 tall, with a 56 px leading slot.
    - The title centers by default on iOS and macOS when there are fewer than
      two actions.
- `Container` and `PhysicalModel` round each corner by its own radius.
  `Container` and `DecoratedBox` paint a whole `BoxDecoration`: shadows,
  circles, images and non-uniform borders. `Container` also pads its child by
  the border width and gains `foreground_decoration`.
  `BoxDecoration::lerp` and `BoxShadow::lerp` follow Flutter.
- Layout follows Flutter:
  - Flex and Wrap flip start and end for RTL and `VerticalDirection::Up`.
  - Baseline alignment uses ascent and descent.
  - Flex 0 is inflexible, and stretch tightens the cross axis.
  - A `Stack` of only positioned children takes the biggest size.
  - `Transform::scale` and `Transform::rotate` pivot about the center, and
    `Transform` and `FractionallySizedBox` gain `alignment`.
- Scrollbars:
  - `RawScrollbar` `thumb_visibility` is tri-state.
  - The thumb fades in over 300 ms, fades out 600 ms after activity, and
    stays while hovered or pressed.
  - Touch presses use a padded hit area around the thumb.
- Reverse viewports lay slivers out in scroll space and mirror painting. A
  reversed list or page view shows its first child at offset 0.
- Bouncing physics hold back the first 3.5 px of a drag and use Flutter's
  friction factor; drag updates are incremental (`DragStartBehavior::Start`).
- Text fields:
  - Arrow keys move in the string's logical order, even in RTL text.
  - Copy and cut are blocked for obscured fields and collapsed selections.
  - Alt+Arrow, Alt+Backspace/Delete and Shift+Delete follow Flutter's
    non-Apple table.
  - A field reveals its caret only when its value or focus changes.
- `onTapCancel` fires only after `onTapDown`, and also when another detector
  wins the arena, as in Flutter.
- `DragStartBehavior` moved to `incular-core` and `TargetPlatform` to
  `incular-config`. The old paths re-export them.
- `incular-widgets` may depend on `incular-assets`, and the architecture
  contract describes the browser host.
- DevTools Performance view redesigned: profiler mode as a segmented control
  with a prominent Record/Stop toggle, a fixed-width per-frame chart stacked by
  phase (build, layout, paint, semantics, composite, other) with a budget line,
  jank flags, legend and click-to-select frames, a selected-frame breakdown
  table, and a proportional flamegraph and ranked work list that show widget
  type names.

- Upgraded Parley from 0.7 to 0.12 and passed resolved paragraph direction into
  text shaping, including mixed-script bidirectional text.
- Trailing non-breaking spaces retain width; trailing ideographic spaces no
  longer affect alignment width, matching Flutter.
- `TextClusterSpan::start` and `end` are `u32` byte offsets, like
  `GlyphPosition::cluster`; `TextClusterSpan::range` returns them as a
  `usize` range.
- DevTools UI: header shows live FPS (trailing-second frame count that decays
  to 0 and reads "idle" when the app stops redrawing) with a frame-time
  sparkline; Performance labels its chart LIVE or RECORDING.
- DevTools UI: Overlays tab regrouped (Hover highlight, Layout, Hit test &
  semantics, Repaint & rebuild, Performance, Animation speed) with short
  descriptions, and also available from a header Overlays menu in every view.
- DevTools UI: Incular brand mark in the header, connection and empty states
  and as the window icon; accent shifted to the brand violet in both themes
  (success states stay green); hover, pressed and focus colours centralised in
  the palette with a visible keyboard-focus ring.

### Fixed

- Ctrl and Command chords no longer type their character into text fields.
  Ctrl+Alt still types, since it is AltGr on Windows layouts. Key events
  without a physical key take their code from the logical key, so shortcuts
  still match.
- In the browser:
  - Paste waits for the page's paste event.
  - Browser shortcuts (reload, zoom, F5, F11, F12) are not swallowed.
  - File drops do not navigate away.
  - A hidden page flushes restoration.
  - Closed windows remove their canvas.
- A `Stack` of only positioned children no longer collapses under loose
  constraints.
- Text baselines placed by proportional leading are compared within float
  tolerance.
- `GestureDetector` reports local positions, deltas and velocities in its
  own space instead of global coordinates.
- The overlay scrollbar hit-tests by pointer kind; only a hovering mouse
  reveals a faded scrollbar.
- Clippy and rustdoc failures on newer stable toolchains. Formatting checks
  run one package at a time (`scripts/check-format.py`), because
  `cargo fmt --all` exceeds the Windows command-line limit.
- Dispatching UI work from another thread no longer panics in debug builds
  when the UI thread drains the message before the sender counts it; the
  queue counter is incremented before sending.
- Reading-order focus traversal no longer panics with "comparison function
  does not correctly implement a total order" on staggered layouts; members
  are grouped into rows before being ordered left to right.
- DevTools: `SetAnimationSpeed` and `SetProfilerMode` replies now serialize
  (as `{"scale": …}` / `{"mode": …}`) instead of failing with
  `InternalError`, and the animation time scale accepts speeds up to 4×
  (`MAX_ANIMATION_TIME_SCALE`) as well as slow motion.
- DevTools UI: the target is outlined only while the pointer is over a row
  (with a Highlight on hover toggle); selecting a widget no longer leaves a
  persistent tint over the application. The overlay clears on leaving, view
  switches, scrolling and disconnects.
- DevTools UI: the widget picker, which makes the target swallow all clicks, now
  ends on view switches, Escape, a second click and disconnects, and is shown as
  "Picking… (Esc)" in the header, toolbar and status bar. A panic while the
  inspector model is locked no longer silently disables every later UI update.
- DevTools Select Widget mode no longer lets an unpaired pointer release reach
  the application after a pick, swallows every pointer event while active,
  keeps selecting after a press over empty space, ends when the window loses
  focus, and draws a border around the captured window.

- DevTools UI polish: janky frames are marked on their own bar instead of a
  full-height band, profiler modes are Basic/Timings/Deep so "Performance" is
  unambiguous, a cold text cache is no longer shown as a red 0.0% hit rate,
  and the compositor card reads "Live layers".
- DevTools UI prunes stale discovery records on Windows with a real process
  liveness check (`OpenProcess` + exit code) instead of probing the recorded
  loopback port; macOS keeps the port probe.
- DevTools UI no longer sits on "connecting" next to an OFFLINE badge with a
  stream of `request admission failed` errors when the target is busy or gone;
  discovery records of exited processes are also pruned on Windows and macOS by
  probing the recorded loopback port.
- DevTools targets no longer leave a second inspector hanging: a concurrent
  authenticated client is told immediately that the session is busy (or takes
  it over with `take_over`). Stale discovery records for exited processes are
  skipped and removed, and the memory view now reports compositor layers.
- EditableText now honors its resolved direction for alignment, painting,
  caret and pointer geometry, selection, and native input snapshots.
- Hover follows content that moves under a still pointer. After a frame whose
  layout or scroll offsets changed, the runtime hit-tests the last pointer
  position again, as Flutter's `MouseTracker` does: `MouseRegion` enter and
  exit callbacks fire and button hover moves to the new row after a
  programmatic or wheel scroll, with no hover event sent.
- Editing long multi-line text no longer retains every edited version. The
  text engine kept each composed document in its paragraph cache, which grows
  with the longest file, so every keystroke in a 5,000-line `TextArea` kept a
  full copy of the text and its layout alive. Only the four most recent
  compositions are now kept.

### Performance

- `Stack` layout no longer allocates intermediate vectors, and two-child
  offsets are stored inline. A 1 px resize over 50 text fields with their
  context-menu portals takes 562 allocations a frame.

- The semantics pass is incremental: invalidation marks render objects and
  the path to them, and the pass enters only marked subtrees, keeping every
  clean subtree's retained nodes. An idle frame of 2,000 buttons with
  semantics on allocates 13 times instead of 16,040. The AccessKit and mobile
  projections read only the nodes changed since their last sync instead of
  copying and comparing the whole tree.
- A resize frame no longer copies each render object's child list, allocates
  an offset vector per node, formats a text-call label per text layout, or
  re-records unchanged pictures: paint records into one reused buffer and a
  layer keeps its picture when the content is equal. 2,000 buttons resize
  with 4,020 allocations instead of 60,024 (semantics off).
- Text layouts of files longer than 2,048 lines no longer reshape every
  paragraph on each layout: the cache is now LRU, grows to fit the longest
  document, and returns unchanged documents whole. `TextEngine::with_shared`
  gives application measurement code a cache that persists across frames.
- Editing unwrapped multi-line text with `Clip`/`Fade` overflow and a fixed
  width reshapes only the changed paragraph instead of the whole string.
- The WGPU renderer skips glyph runs wholly outside the active clip before
  atlas lookup, instead of emitting an instance per off-screen glyph. A long
  scrolled document no longer costs GPU preparation for every line per frame.
- Caret placement in a long text layout derives caret stops for the one line
  it needs instead of every line of the document after each edit.
- A pointer event hit-tests the tree once and shares the result between
  mouse regions, listeners, tap regions, gestures, text fields, selectable
  text and buttons; it used to walk the tree up to six times. The cursor
  lookup after input walks it once instead of twice. A pointer move that
  changes nothing over 2,000 buttons in a scroll view fell from about
  0.47 ms to 0.08 ms, and the cursor lookup from 0.16 ms to 0.08 ms.
- Relayout of many labels no longer reshapes them all: text layout pools
  grow when their working set does not fit, and a single unwrapped,
  start-aligned left-to-right line is reused at any width it still fits.
  Resizing a window over 3,000 labels fell from about 90 ms to 7 ms a frame.
- Paragraphs that wrap keep their shaped layout, so a new width re-breaks
  lines instead of reshaping the text.
- Text layout pools are bounded by approximate bytes as well as entries
  (32 MiB by default, `TextEngine::set_cache_byte_budget`), and pools grown
  by a burst shrink back once a smaller working set hits steadily. After a
  burst growing the pools to 20,000 labels, the engine's retained memory with
  a 100-label working set fell from 32.8 MiB, never released, to 3.3 MiB,
  with no extra misses for that working set. A document being composed keeps
  its paragraphs regardless of the budget.
- Editing a long document carries the lines before the edit from the previous
  layout instead of rebuilding them; typing at the end of a 5,000-line field
  fell from about 9 ms to about 1 ms a keystroke.
- A multi-line `EditableText` paints only the lines inside its viewport. In a
  5,000-line field, arrow keys, drag selection and caret blinking now cost a
  quarter or less of what they did.
- Typing in a 5,000-line `DocumentEditor` costs about 0.4 ms a keystroke
  wherever the caret is, against about 10 ms near the top of a `TextArea`.
- Paint enters only the render subtrees that hold a pending repaint, and
  flattening culls a whole off-screen layer subtree by its memoized picture
  bounds instead of testing every picture inside it. A frame over 2,000
  buttons in a scroll view spent about 3.8 ms in paint whether idle or after
  a hover; it now spends about 0.08 ms idle and 0.11 ms after a hover.
- Controls and Material widgets read their theme through the new `Builder`
  (Flutter's context-only builder) and `Widget::stateful_builder` instead of
  a `LayoutBuilder`, so they rebuild when the theme or their state changes,
  not when their constraints do. Resizing a column of 308 controls rebuilt
  2,156 elements a frame and now rebuilds none; layout fell from about
  2.3 ms to 0.3 ms a frame.
- Shaped cluster spans take 20 bytes instead of 32, so a laid-out line no
  longer spends more on cluster spans than on glyphs. A 59-glyph label
  retains about 3.3 KB in the text engine instead of 4.0 KB.
- Text layout cache hits no longer allocate: lookups hash and compare a key
  that borrows the text, and only an insert copies it. A layout reused at a
  new width it still fits is no longer inserted again under that width. Over
  2,000 labels, a pass of cache hits fell from 2,000 allocations to none,
  and three resize passes from 18,005 to none.
- Undo history stores each step as the edited range instead of a full copy
  of the text. A hundred keystrokes in a 5,000-line field retained 28.8 MB of
  history and now retain 0.3 MB. A text field without a restoration binding
  no longer serializes its whole text on every commit.
- Moving, resizing or scrolling content updates only the bounds of semantic
  nodes whose role, text, state and actions did not change, instead of
  rebuilding every node below the change. A resize or scroll frame over 500
  Controls buttons fell from 3,023 allocations to 23.
- Input events no longer format a diagnostic label each time; the label is
  rendered only when a recursion report needs it. A pointer move that
  changes nothing now allocates nothing.
- Reconciliation no longer copies child lists while invalidating sliver
  measurement, validating new subtrees or materializing layout builders, and
  checks a few keyed children without a hash set. Scrolling a 10,000-row
  builder list fell from 763 to 588 allocations a frame.
- An unchanged scene no longer rebuilds its flattened display list every
  frame: display lists share their commands, and flattening reuses a retained
  buffer and returns the previous list when nothing changed. Scrollbar fade
  pumping and flex layout also stopped allocating temporary vectors. An idle
  frame over 300 buttons fell from 12 allocations to 1, a 1 px resize from 22
  to 16.
- Text fields no longer copy their whole editing value in every layout and
  paint, and `TextStyle::default` no longer allocates. A 1 px resize over 50
  text fields fell from about 712 to 512 allocations a frame. The new
  `TextEditingController::with_value` lends the value without copying it.

## 0.1.0 — 2026-10-10

First release.

- Declarative native UI with reactive state, retained layout and painting,
  input, scrolling, text, navigation, and optional Controls and Material
  presentation.
- Shared Winit/WGPU desktop host and native Windows, Linux and macOS adapters.
- Android and iOS semantic adapters. Full mobile host integration is still
  pending.
- Opt-in local DevTools and a separate, unpublished desktop inspector.
- Opt-in `hot-reload` feature: debug builds served by `dx serve --hot-patch`
  patch edited code into the running app and rebuild every window in place.
  `examples/hot_reload` is a ready-to-serve playground.
- Self-contained crate packaging, a published hello example, user guides,
  docs.rs metadata, and build-only desktop CI.
- DevTools discovery files use private exclusive temporary creation. Idle HTTP
  upgrades time out and cancel on shutdown, and browser-origin upgrades are
  rejected.
- Minimum supported Rust version is 1.89, to include the Linux notification
  stack.

The 0.1 API is experimental. Patch releases preserve compatible application
APIs; breaking changes require a new 0.x minor line.
