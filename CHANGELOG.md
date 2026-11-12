# Changelog

Maintainers: record user-visible changes under `Unreleased`, then move them to
a version and date when publishing.

## Unreleased

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
