# Source-verified findings

Flutter paths below are relative to `.tmp/flutter/packages/flutter/lib/src/`
unless stated otherwise. Incular paths are relative to the checkout. References
are to the snapshots in [README.md](README.md). A finding can be an API gap,
behavior difference, or deliberate architecture choice; none requires a Dart
inheritance copy, an engine change, or a design-system implementation. Examples
with numbers are calculations from source, not observed test runs.

## Animation

| ID | Finding | Source evidence |
| --- | --- | --- |
| A01 | **Behavior:** AnimationStyle selects the entire old or new style at the midpoint. Flutter interpolates each duration (rounding to microseconds) and curve (null treated as linear). At t = 0.25, 100→300 ms is 150 ms in Flutter, 100 ms in Incular. | Flutter `animation/animation_style.dart:87–113,146–161`; Incular `crates/incular-animation/src/style.rs:71–79`. |
| A02 | **Partial API:** TweenSequence Flutter segments accept arbitrary Animatable values and require nonempty positive-weight input; intervals are computed once. Incular segments accept TweenValue, discard invalid weights, allow empty sequences and sum weights per sample. Only its f32 sequence implements Animatable. | Flutter `animation/tween_sequence.dart:45–91,121–138`; Incular `crates/incular-animation/src/tween.rs:237–301`, `value.rs:19–30`. |
| A03 | **Extension API:** No public controller resync equivalent was found, though Rust Ticker can absorb another ticker. Rust also lacks a one-dimensional caller-defined Curve route on its closed Curve enum, and a public ValueListenable-to-animation adapter. Rust's generic Animatable trait remains extensible. | Flutter `animation/animation_controller.dart:321–345`, `animation/curves.dart:26–116`, `animation/animation.dart:198–201,380–410`; Incular `crates/incular-animation/src/scheduler.rs:453–462`, `curves.rs:7–106`, `value.rs:8–11`. |

## Foundation

| ID | Finding | Source evidence |
| --- | --- | --- |
| F01 | **Partial API:** Flutter still exports ChangeNotifier, Listenable, ValueListenable and ValueNotifier via package:listen. Incular has signal reads and RAII dependency subscriptions; it does not provide the same general public listener, value-read and merge lifecycle through its Listenable trait. Treat signals as a useful existing primitive, not as evidence the APIs match. | Flutter `lib/foundation.dart:14`, `packages/flutter/pubspec.yaml:17`; Incular `crates/incular-widgets/src/explicit_animations.rs:166–209`, `crates/incular-core/src/reactivity.rs:49–90`, `crates/incular-runtime/src/environment.rs:582–643`. |
| F02 | **Architecture/API:** Foundation's neutral license registry is unavailable through a neutral Incular crate; a registry exists in a design-system crate. The finding is neutral access, not Material parity. | Flutter `foundation/licenses.dart:300`; Incular `crates/incular-material/src/catalog/about.rs:43–81`. |
| F03 | **Behavior:** Flutter web defaults TargetPlatform from the browser OS and allows override. Incular compile-target detection falls back to Android on wasm, with no host injection found; desktop browser defaults can therefore pick mobile conventions unless overridden. | Flutter `foundation/_platform_web.dart:12–17,44–59`; Incular `crates/incular-config/src/environment.rs:20–50`. |
| F04 | **Partial diagnostics:** Flutter diagnostics nodes/properties, structured error details, allocation listeners and web console error-dump capture lack an equivalent common public model. DevTools inspection and tracing are related, not an API match. | Flutter `foundation/diagnostics.dart:1463,2560,3030`, `foundation/assertions.dart:374`, `foundation/memory_allocations.dart:118–155`, `foundation/_error_dumper_web.dart:10–32`; Incular `crates/incular-devtools-protocol/src/lib.rs:114,203,567,644`. |

## Gestures

| ID | Finding | Source evidence |
| --- | --- | --- |
| G01 | **Behavior:** ScrollBehavior exposes multitouch strategies but production recognition does not consume that selection. Flutter applies latest-pointer, average-boundary-pointers and sum-all-pointers policies in the drag recognizer. | Flutter `gestures/monodrag.dart:88,130–142,451–547`; Incular `crates/incular-widgets/src/scroll_behavior.rs:22–33,137–140,185–195`. |
| G02 | **Partial API:** Flutter's scale recognizer accepts native pan/zoom streams and optional trackpad scroll-as-scale (default scale factor (0, −1/200)). Incular's recognizer handles regular contacts; InteractiveViewer's wheel path is not recognizer-level support. | Flutter `gestures/scale.dart:24–31,72–86,345–355,403–427,527–589`; Incular `crates/incular-gestures/src/scale.rs:276–325`, `crates/incular-widgets/src/interactive_viewer.rs:689–719`. |
| G03 | **Missing public API:** Resampling, transformed/reentrant per-pointer and global PointerRouter entries, and recognizer grouping/captain in GestureArenaTeam lack equivalents found in neutral source. Flutter resampling is off by default, with −38 ms sampling offset and 16,667 μs interval. | Flutter `gestures/resampler.dart:36`, `gestures/binding.dart:224,232,599–616`, `gestures/pointer_router.dart:17–75,94–140`, `gestures/team.dart:139+`; Incular internal pointer dispatch and arena are narrower. |
| G04 | **Visibility:** Velocity tracking exists internally but not as Flutter's public tracker, velocity estimate and least-squares solver API. | Flutter `gestures/velocity_tracker.dart:87,138,295,416`, `gestures/lsq_solver.dart:56,88`; Incular `crates/incular-gestures/src/velocity.rs:35`. |

Sampled timings/slop match: press 100 ms, long press 500 ms, double tap 300 ms
timeout and 40 ms minimum, touch slop 18 logical px, pan slop 36 logical px, and
fling limits 50–8,000 px/s (Flutter `gestures/constants.dart:14–109`; Incular
`crates/incular-gestures/src/constants.rs:9–99`). This is not a
gesture-behavior blanket pass.

## Painting

| ID | Finding | Source evidence |
| --- | --- | --- |
| P01 | **Behavior:** HSL/HSV hue interpolation crosses the long arc. Hue 350°→10° at t = 0.5 yields 0° Flutter vs 180° Incular. HSL also clamps t before interpolation: 20°→60° at 1.5 yields 80° vs 60°. | Flutter `painting/colors.dart:30–38,188–203,371–385`; Incular `crates/incular-core/src/geometry.rs:730–754`. |
| P02 | **Partial image API:** DecorationImage takes an already decoded handle; lacks provider-backed placeholder/error behavior. ResizeImage lacks provider/cache-key resizing equivalent to Flutter's logical-size mode (false by default, ceil per dimension after multiplying by DPR, default DPR 1). | Flutter `painting/decoration_image.dart:48–85`, `painting/image_provider.dart:1268–1321,1476–1496`; Incular `crates/incular-widgets/src/painting_effects/decoration.rs:415–439`, `crates/incular-image/src/lib.rs:117–137`. |
| P03 | **Partial cache API:** 1,000 entries and 100 MiB are nominally aligned; provider-stream pending/live/completed state is absent from the decoded-byte LRU contract. | Flutter `painting/image_cache.dart:18–41,86–94`; Incular `crates/incular-image/src/lib.rs:654–692,726–775`. |
| P04 | **Partial text API:** No manual/hidden soft-hyphen control (manual: glyph only at wrap; hidden: preserve break without glyph; Flutter has a web caveat). TextScaler supports only no-scaling and linear modes, not custom nonlinear scaling and composable clamps. | Flutter `painting/text_painter.dart:1063–1079`, `painting/text_scaler.dart:20–74,117–147`; Incular `crates/incular-text/src/style/text_style.rs:10–43`, `style/text.rs:199–255`. |
| P05 | **Partial geometry API:** ShapeBorder directional paths and hit testing are absent. Gradient.fromColor is absent; sweep gradients have fixed TAU rather than configurable end angle (Flutter defaults 0 and 2π). | Flutter `painting/borders.dart:542–580`, `painting/gradient.dart:253,480,786,1009–1010,1108`; Incular `crates/incular-widgets/src/painting_effects/shape_borders.rs:14–26`, `crates/incular-rendering/src/gradients.rs:181–258,417–419`. |

## Physics

| ID | Finding | Source evidence |
| --- | --- | --- |
| PH01 | **Behavior:** withDurationAndBounce(500 ms, 0) is critical in Flutter. Incular narrows damping and stiffness before classifying, so the discriminant is about +0.00005374 instead of zero; Rust reports overdamped. | Flutter `physics/spring_simulation.dart:70–81,139–153,284–294`; Incular `crates/incular-animation/src/physics.rs:137–153,179–187,223–240`. |
| PH02 | **Behavior/API:** ClampedSimulation permits inverted bounds and propagates NaN on current Rust path; Flutter asserts ordered bounds and its engine clamp chooses the upper bound for NaN. Simulation tolerance is mutable in Dart, not via the Rust trait. | Flutter `physics/clamped_simulation.dart:33–40,58–65`, `physics/simulation.dart:36–56`, engine `lib/ui/math.dart:13–24`; Incular `crates/incular-animation/src/physics.rs:62–72,193–209,720–753`. |

Default tolerance matches: 0.001 distance, 0.001 velocity, 1 ms time (Flutter
`physics/tolerance.dart:14–23`; Incular `physics.rs:18–23`).

## Rendering

| ID | Finding | Source evidence |
| --- | --- | --- |
| R01 | **Architecture/API:** Flutter has caller-defined RenderObject/RenderBox protocols and independent PipelineOwners for offscreen work. Incular exposes domain delegates and sliver geometry, but its retained render node and arena are private. Decide which extensibility contracts to expose in Rust without opening private internals or replacing wgpu. | Flutter `rendering/object.dart:1004–1045,1973–2010`, `rendering/box.dart:1577`; Incular `crates/incular-widgets/src/scrolling/geometry.rs:130–192`, `render_object/mod.rs:372–392`, `tree.rs:1316–1338`. |
| R02 | **Partial sliver clipping:** SliverClipRect and SliverClipRRect now provide retained overlap-aware rectangular and rounded clips and pointer rejection. Live clipper notifications update paint and hit tests without relayout. A separate approximate accessibility clip controls semantic membership and bounds. Remaining work includes elliptical corners and expanded nested/overlap comparisons. | [Sliver clip comparisons](upstream-sliver-clipping.md); pinned Flutter `widgets/sliver_clip.dart` and `rendering/sliver_clip.dart`. |
| R03 | **Cache units and page semantics verified:** Viewports accept logical pixels or viewport multiples. SliverFillViewport defaults to including cached offscreen pages in semantics; disabling implicit scrolling excludes those pages without evicting them. Further nested, shrink-wrap, and performance combinations remain to verify. | [SliverFillViewport comparisons](upstream-sliver-fill-viewport.md) and [cache geometry](upstream-sliver-cache-geometry.md). |
| R04 | **Partial custom paint:** Painter hit policies, separate semantic invalidation, and listenable repaint are retained. Shared-canvas diagnostics and raster-cache hint parity remain open. | [Custom paint comparisons](upstream-custom-paint.md); pinned Flutter `rendering/custom_paint.dart`. |
| R05 | **Destination blend API paired; pixel cases open:** RawImage now defaults to source-over and updates a separate destination blend without remeasuring or repainting the image. Advanced destructive blends still need native pixel verification. | [Pinned RenderImage and RawImage blend cases](upstream-raw-image-blend.md). |
| R06 | **Missing capture API:** RepaintBoundary isolates repaint but does not expose subtree capture or its independent pixel ratio (Flutter default 1.0). Full-window next-frame capture is not equivalent. | Flutter `rendering/proxy_box.dart:3481–3494,3540,3606–3609`; Incular `crates/incular-widgets/src/layout/basic/painting.rs:57–74`, `crates/incular-runtime/src/simulation.rs:582–590`. |

## Scheduler

| ID | Finding | Source evidence |
| --- | --- | --- |
| SC01 | **Behavior:** Flutter runs persistent callbacks before snapshotting post-frame callbacks and keeps the current frame timestamp live. Incular snapshots post callbacks and clears frame time first. A post callback registered by a persistent callback can run in the same Flutter frame but be delayed in Incular. | Flutter `scheduler/binding.dart:1341–1378`; Incular `crates/incular-animation/src/scheduler.rs:289–311`. |
| SC02 | **Partial API:** No public SchedulerPhase, endOfFrame future or UI-priority task scheduler found. Flutter idle/animation/touch priorities are 0/100000/200000. Incular transient callback registration always requests a frame and cannot opt out as Flutter's scheduleNewFrame:false does. | Flutter `scheduler/binding.dart:468–531,610–623,1463–1473`, `scheduler/priority.dart:20–26`; Incular `crates/incular-animation/src/scheduler.rs:173–180,235–244`, `crates/incular-runtime/src/tasks.rs:348–380`. |

## Semantics

| ID | Finding | Source evidence |
| --- | --- | --- |
| SE01 | **Partial API:** Plain semantic strings and fixed action enums omit UTF-16 attributed ranges (including locale/spell-out) and custom semantic actions. Flutter's label builder also inserts bidi embedding marks when mixing direction. | Flutter `semantics/semantics.dart:789–815,872–922,1014–1088`; Incular `crates/incular-semantics/src/lib.rs:45–76,168–234,276–286`, `crates/incular-widgets/src/semantics.rs:12–38`. |
| SE02 | **Partial API:** Sibling sort keys and identifiers do not support cross-subtree traversal relations or independent accessibility focus blocking. Flutter focus blocking merges subtree > node > none and does not change keyboard focus. | Flutter `semantics/semantics.dart:119–159,2106–2149`; Incular `crates/incular-semantics/src/lib.rs:168–234`, `crates/incular-widgets/src/semantics.rs:12–38`. |
| SE03 | **Partial lifecycle:** Flutter collection uses independently disposable ensureSemantics handles. Incular has a window-level flag and debugger exception. Incular has an announcement capability query and live regions, but no callable, view-specific announcement found (Flutter defaults polite). | Flutter `semantics/binding.dart:140–169,253–263`, `semantics/semantics_service.dart:35–40,76–94`; Incular `crates/incular-runtime/src/application.rs:1743–1755`, `context.rs:389–393`, `crates/incular-widgets/src/tree/semantics.rs:93–118`. |

## Services

| ID | Finding | Source evidence |
| --- | --- | --- |
| SV01 | **Partial IME API:** TextInputStyle and live updateStyle send direction/alignment, font/spacing and line-height data. Incular host commands have no style update; the web textarea uses fixed 16px/line-height 1. Also no enableInlinePrediction via Flutter's EditableText options, or LiveText camera entrypoint found. | Flutter `services/text_input.dart:1588–1624,1851–1853`, `widgets/editable_text.dart:943,2138–2139,5440`, `services/live_text.dart:18–33`; Incular `crates/incular-platform/src/lib.rs:604–699`, `crates/incular-web/src/text_input.rs:28–33`. |
| SV02 | **Behavior:** AutofillGroup returns its child unchanged. No equivalent grouping of clients with scoped register/unregister and topmost commit/cancel; per-field hints do not cover the lifecycle. | Flutter `widgets/autofill.dart:168–215,233–244`; Incular `crates/incular-widgets/src/forms.rs:1140–1159`, `crates/incular-web/src/text_input.rs:197–204`. |
| SV03 | **Partial assets:** Bundle byte/text loading and native filesystem DPR discovery exist. Manifest enumeration, metadata and variant discovery through compiled/provider bundles are absent; fallback root bundle is private and empty. NetworkAssetBundle is already implemented. | Flutter `services/asset_manifest.dart:24–66,89–119,137`; Incular `crates/incular-assets/src/bundle.rs:47–106`, `network.rs:40–78`, `crates/incular-image/src/lib.rs:155–235`, `crates/incular-widgets/src/asset_bundle.rs:29–44`. |
| SV04 | **Missing general API:** No BinaryMessenger or BasicMessageChannel, MethodChannel, EventChannel and StandardMessageCodec capability equivalents found. Parity here is portable request/reply/event behavior, not automatically Flutter wire compatibility. | Flutter `services/binary_messenger.dart:66–80`, `services/platform_channel.dart:202,291,650`, `services/message_codecs.dart:263`. |
| SV05 | **Partial keyboard:** Per-event dispatch and last modifiers do not provide a global physical/logical pressed-key and lock-state registry, with synthesized reconciliation before handler dispatch. | Flutter `services/hardware_keyboard.dart:348–399,411–442`; Incular `crates/incular-widgets/src/shortcut_registry.rs:125–130`, `crates/incular-gestures/src/keyboard.rs:34–68`. |
| SV06 | **Platform gaps:** Default spell checker returns None on the Incular hosts vs Flutter native Android/iOS. SystemChrome has overlay styling but not orientation lock, task-switcher description, or full visibility modes. No matching browser context menu toggle found. | Flutter `services/spell_check.dart:114–129`, `services/system_chrome.dart:547–568+`, `services/browser_context_menu.dart:23–39,54–76`; Incular `crates/incular-platform/src/spell_check.rs:99–115,172–176`, `crates/incular-runtime/src/frame/system_chrome.rs:13+`, `application.rs:1529`. |
| SV07 | **Partial metadata:** Cargo package version is an idiomatic analog for app build name, but no separate portable build number/override identified. | Flutter `services/app_version.dart:17–31`; no neutral Incular counterpart for the distinct build number found. |
| SV08 | **Partial binary assets:** Flutter shares in-flight structured-binary loads, caches successes, and retries failures. Incular structures UTF-8 text but provides no equivalent structured-binary cache. Flutter native offloads UTF-8 decode from 51,200 bytes; Incular defaults to synchronous decode, an unmeasured throughput/latency risk. | Flutter `services/asset_bundle.dart:91–100,117–127,250–299`; Incular `crates/incular-assets/src/bundle.rs:54–69,82–105,216+`. |

## Widget previews

| ID | Finding | Source evidence |
| --- | --- | --- |
| WP01 | **Missing API/tooling family:** No equivalent preview metadata/registration, discovery/scanner, renderer wrapper, per-preview variants and localization/theme inputs found in Incular crates/tools. General gallery/demo widgets aren't the same API. | Flutter `widget_previews/widget_previews.dart:66,244,264,329,397,407` and `lib/widget_previews.dart:13`; no matching neutral public module/tool in the checkout. |

## Widgets

| ID | Finding | Source evidence |
| --- | --- | --- |
| W01 | **State retention:** Flutter updates compatible unkeyed pages and preserves attached pageless routes. Incular always remounts unkeyed pages and keyed reconciliation can retire unmatched imperative entries. Rebuilding a declarative route list can reset page state or remove a pageless push. | Flutter `widgets/navigator.dart:763,4165–4190,4201–4253,4413–4418`; Incular `crates/incular-navigation/src/navigator.rs:718–740,751–783,809–839`. |
| W02 | **Partial paging:** PageView has explicit scrollCacheExtent and allowImplicitScrolling; Incular fixes cache extent to zero and does not expose the flag. See also R03. | Flutter `widgets/page_view.dart:710–723,836–862`; Incular `crates/incular-widgets/src/scrolling/scroll_views.rs:1279–1282,1573,1598`. |
| W03 | **Partial animated widgets:** AnimatedCrossFade lacks selectable clipBehavior (Flutter default hardEdge) and onEnd. FadeInImage lacks fadeInOver, where the placeholder remains opaque beneath the target while it fades in, instead of the sequential 300 ms/700 ms Incular path. | Flutter `widgets/animated_cross_fade.dart:129–143,231–237,313`, `widgets/fade_in_image.dart:17–35,751–759`; Incular `crates/incular-widgets/src/animated_cross_fade.rs:44–56,228–231`, `fade_in_image.rs:48–63,87–94,334–373`. |
| W04 | **Partial layout:** IndexedStack does not expose a null selection or direction-aware top-start alignment. Incular does already exclude unselected children from focus; do not mark that part missing. | Flutter `widgets/indexed_stack.dart:43–81,90–118`; Incular `crates/incular-widgets/src/layout/stack.rs:173–185,197–227`, `tree/focus.rs:41–46,609–612`. |
| W05 | **Partial forms/focus/menu:** No onUnfocus/onUserInteractionIfError autovalidation, validateGranularly or forceErrorText. FocusableActionDetector lacks skipTraversal on its descriptor (FocusNode setter is a workaround). Context-menu item kind is a closed legacy enum without Flutter's extensible translate kind. | Flutter `widgets/form.dart:287–291,376–387,550–562,733–734,966`, `widgets/actions.dart:1180,1212–1213,1401`, `widgets/context_menu_button_item.dart:58–77,137–140`; Incular `crates/incular-widgets/src/forms.rs:23–35,422–435`, `focus_keyboard.rs:494–504,571–608`, `crates/incular-gestures/src/focus.rs:203–211`, `crates/incular-widgets/src/context_menu.rs:24–39,61–68`. |
| W06 | **Partial overlays/dialogs:** No entry-level opaque, maintainState, canSizeOverlay or overlay alwaysSizeToContent equivalents found. Neutral showRawDialog selects root/nearest navigator and returns the pop result; Incular show_dialog inserts an overlay and returns unit. | Flutter `widgets/overlay.dart:111–195,493,534–546,1476,1505–1515`, `widgets/dialog.dart:16,58–82`; Incular `crates/incular-navigation/src/presentation.rs:651–694,750–769,818–820`. |
| W07 | **Rust design/API:** GlobalKey supports identity/reparenting; no imperative typed current context/widget/state lookup. Borrowed BuildContext and Rust stateful builders suggest providing a safe equivalent only if the application use case needs it. | Flutter `widgets/framework.dart:160–193`; Incular `crates/incular-widgets/src/tree/values.rs:142–167`, `tree/context.rs:147–181`, `tree/widget/constructors.rs:1030`. |
| W08 | **Native view/texture API:** Generic PlatformViewSurface/controller and host command types exist, but no production host was found advertising AboveSurface. Flutter has AndroidView, UiKitView, HtmlElementView and PlatformViewLink, plus a separate Texture widget with producer-driven repaint and freeze/sampling options. Internal GPU handles are not a Texture widget. Respect wgpu and platform composition limits. | Flutter `widgets/platform_view.dart:70,313,653,1239`, `widgets/texture.dart:17–25,36–44`; Incular `crates/incular-widgets/src/platform_view.rs:49–75,194–230`, `crates/incular-platform/src/platform_view.rs:10–22,41–51`. |
| W09 | **Behavior:** Flutter TapRegionSurface dispatches inside/outside for semantic tap/longPress. Incular sends region events on the raw pointer path, but semantics handlers directly invoke target callbacks. | Flutter `widgets/tap_region.dart:240–274`; Incular `crates/incular-widgets/src/tree/raw_input.rs:90–110`, `crates/incular-runtime/src/frame.rs:922–936,1012–1019`. |
| W10 | **API placement:** WidgetStateProperty, WidgetStateMapper, and WidgetStatesController are neutral Widgets exports in Flutter. Incular equivalents are currently reachable only through its Material crate. The gap is neutral placement only. | Flutter `lib/widgets.dart:188`, `widgets/widget_state.dart:821,1000,1140`; Incular `crates/incular-material/src/theme/state.rs:8,176,301,318`. |
| W11 | **Partial overlay transform:** Flutter returns child-local size, the full child-to-target-overlay Matrix4, and target overlay size. Incular returns an axis-aligned anchor bounding box and owning viewport size. A rotated or skewed child’s local edges cannot be recovered from that rectangle. | Flutter `widgets/overlay.dart:39–53,2905–2915`; Incular `crates/incular-widgets/src/transient.rs:585–594`, `tree/retained.rs:238–266,987–996`. |

Positive spot check: StreamBuilderBase reconnects and exposes state hooks in
`crates/incular-widgets/src/async_builders.rs:478–519,552–585`. Sliver clipping,
hyphens, autofill, and IME gaps are recorded once in Rendering, Painting, or
Services instead of being independently counted in Widgets.
