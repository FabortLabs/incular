# Full audit: manual contract reviews

Baseline and scope are pinned in [README](README.md). This pass follows the
[coverage ledger](coverage-ledger.md) beyond focus and borders. The source
traces below record specific differences, matching controls, and remaining
coverage. They do not close unseen members or downstream platforms. Command
probes and paired results, where available, are recorded separately in
[focused comparisons](focused-comparisons.md).

Flutter references without an explicit tree are relative to
`.tmp/flutter/packages/flutter/lib/src/`. Incular paths are checkout-relative;
a short path following a crate path continues inside that crate. **Source**
means the actual implementations were compared, not that they ran on every
platform. IDs are local to this document.

## Animation

- **A-01 Repeat sampling/notification (source):** Flutter's repeat simulation
  is linear after a previous per-run curve override, as is Incular's.
  With Incular's persistent set_curve(Decelerate) extension, for a 1 s repeat
  from 0 to 1, at 250 ms Flutter=0.25,
  Incular=0.4375. For one reverse-enabled counted cycle, Flutter reports
  Forward→Reverse→Dismissed at the boundary; Incular's atomic terminal
  update omits Reverse. Flutter `animation/animation_controller.dart:744–749,934–947,998–1053`;
  Incular `crates/incular-animation/src/controller.rs:216–233,804–835,1087–1099`.
- **A-02 Listener lifetime (source):** Flutter ProxyAnimation retains
  detachment state, suppresses no-op parent changes, and notifies value and
  status separately. Incular snapshots but invalidates both through one
  source; same-parent/detach can notify, and changing value with unchanged
  status can incorrectly call a status listener. Both detach the old parent.
  When listener A removes B during dispatch, Flutter rechecks membership,
  while Incular has already cloned the listeners. Flutter
  `animation/animations.dart:200–239`, `listener_helpers.dart:144–161,237–243`;
  Incular `crates/incular-animation/src/animations.rs:120–135,622–635`,
  `controller.rs:1195–1235`, `crates/incular-core/src/reactivity.rs:76–90`.
- **A-03 Start sampling and fling validation (source):** custom simulation
  x(0)=0.5 sets initial value in both, but Flutter does not notify value
  listeners then; Incular deliberately notifies synchronously, a known
  contract choice documented by the code. Flutter debug rejects an
  underdamped fling override; Incular accepts it. Ordinary fling default
  spring mass 1, stiffness 500, distance tolerance .01 and accessible
  fling velocity factor 200 have matching source paths. Flutter
  `animation/animation_controller.dart:788–794,856–866`; Incular
  `crates/incular-animation/src/controller.rs:1120–1127,1142–1174`.
- **A-04 Tween boundaries (source):** a null endpoint remains null at t=0/1
  in Flutter; Incular OptionalTween can return a concrete alpha-zero color
  or zero size/rect at the missing endpoint. Interior missing-end fade/scale
  is supported in both. IntTween 16,777,217→16,777,219 at midpoint is
  16,777,218 in Flutter but 16,777,216 through the Incular f32 casts; full
  i32 end-to-end range also risks subtraction overflow. Flutter interpolates
  color channels in float and converts cross-gamut; Incular's u8 color
  can agree on 8-bit output but is not the same interpolation contract.
  Flutter `animation/tween.dart:359–369,401–485`, engine
  `lib/ui/painting.dart:420–455`; Incular
  `crates/incular-animation/src/tween.rs:38–41,73–84,109–124,154–170`.

Further review: every named curve and interval edge, CatmullRom caches and
inverse, train hopping, controller disposal and async interruption. Check
ordinary completions strictly after the duration and counted repeats at the
duration separately; the source has different boundary rules by design.

## Foundation

- **F-01 Error/diagnostics path (source):** per-runtime task-error observation
  is functional in Incular but carries TaskFailure; it is not Flutter's
  structured context, attachable diagnostics and stack-aware framework error
  funnel. Flutter also has queued print throttling at 12×1024 characters
  and a one-second pause, and named timeline operations with drain/reset.
  Incular frame/DevTools trace plumbing is a related, not equivalent API.
  Flutter `foundation/assertions.dart:374–460,922–942,970–983,1193–1194`,
  `foundation/print.dart:52–113`, `foundation/timeline.dart:24–150`;
  Incular `crates/incular-runtime/src/frame.rs:613–625,729–731`,
  `application_types.rs:50–55`.
- **F-02 Equivalent fundamentals (source):** Rust supports owned callbacks,
  typed data, UTF-8 string types and platform-aware tasks without Flutter
  class identities. Existing Signal/DependencySource and wasm deferred
  computation are actual implementations. Do not infer ChangeNotifier
  behavior/merging from the underlying mechanism; see the original F01–F03
  findings and [external boundaries](external-reexports.md) for gaps.

Further review: notification remove/add/dispose during dispatch, structured
errors from event/build/layout/paint independently, licensing observers,
comparison/sort/collection utility edge cases, foundation diagnostics fully
exposed through the facade, and isolation/thread/platform constraints.

## Gestures

- **G-01 Projected-axis arbitration (source):** with an outer tap competing
  against an inner horizontal drag rotated 90°, touch down global (50,50)
  and move to (50,70). Flutter projects into local horizontal then checks
  20 physical px > 18 px. Incular reports localized positions after acceptance
  but accumulates global x for this threshold; zero progress, no acceptance.
  Flutter `gestures/monodrag.dart:672–719,1029–1040`; Incular
  `crates/incular-gestures/src/pointer.rs:513–537,636–651,840–853`,
  `crates/incular-widgets/src/tree/interaction.rs:817–855,1145–1170`.
  Positive control: unrotated one-axis movement. The competing recognizer is
  necessary to avoid an early sole-winner bypass.
- **G-02 Synthesized velocity (source):** Flutter ScaleGestureRecognizer
  ignores synthesized Move for velocity but updates geometry. Incular samples
  every Move. With only synthesized 20 px steps at 10/20/30/40 ms and Up at
  45 ms, geometry can agree while Flutter end velocity is zero and Incular
  is about 2,000 px/s. MultiDrag exclusions are implemented on both sides
  and must not regress. Flutter `gestures/scale.dart:545–555,694–722`,
  `gestures/multidrag.dart:89–108`; Incular `crates/incular-gestures/src/scale.rs:309–316,480–501`,
  `multidrag.rs:179–195`.
- **G-03 Pressure and long press (source):** standard normalized ForcePress
  defaults 0.4 start/0.85 peak, strict thresholds, squared movement, NaN
  no-update, and end pressure 0 match on ordinary streams. Raw device
  normalization remains separate. Flutter allows a custom long-press deadline
  and post-accept slop; Incular hardcodes 500 ms and no corresponding public
  configuration. Flutter `gestures/force_press.dart:131–138,235–345`,
  `gestures/long_press.dart:262–289,325–335`; Incular
  `crates/incular-gestures/src/force_press.rs:77–94,215–320,362–376`,
  `pointer.rs:195–197,366–381,856–860,923–929`.

Further review: teams/arena cancellation and resampling, native pan/zoom and
sensor events, mixed buttons and devices, event synthesis, reentrant router
updates, hover on reordered/transformed content, multi-pointer boundaries.

## Physics

- **PH-01 Bounded friction completion (source arithmetic):** for drag .5,
  start 8191, velocity 1, bounds 8190..8192, at 1.701 s Flutter computes
  x≈8191.9989612354, leaving .0010387646 and not done. Incular checks the
  rounded f32 position 8191.9990234375, gap .0009765625, and finishes.
  At 1.700 both remain active; 1.702 both finish. Flutter
  `physics/friction_simulation.dart:117–134,158–160,186–195`; Incular
  `crates/incular-animation/src/physics.rs:570–581,627–644`.
- **PH-02 Spring/gravity controls (source):** default snap false,
  snapToEnd=true returns final position and zero velocity, ScrollSpring
  pins x but retains unsnapped velocity, and gravity completion uses
  abs(x)>=end. Flutter `physics/spring_simulation.dart:215–279`;
  Incular `crates/incular-animation/src/physics.rs:293–337,372–387`.
  These controls do not close the previously found critical-damping
  misclassification, NaN/clamp, or large-number precision cases.

Further review: numeric sweeps through every simulation, negative velocity,
near-zero mass/drag, signed zero/infinity/NaN where public inputs allow them,
duration quantization, f32 vs f64 classification and tolerance boundaries.

## Scheduler

- **SC-01 Start/stop within transient batch (source):** transient A starts
  ticker B at time 0. Flutter captures the transient batch, so B first
  callbacks next frame at elapsed 16 ms; Incular gathers active tickers
  after general transients and runs B on frame 0. Conversely if ticker A
  stops already-queued B, Flutter unschedules B; Incular can still tick
  B from a prior filtered vector. Flutter `scheduler/ticker.dart:185–207,231–249,270–319`,
  `scheduler/binding.dart:633–637,1259–1273`; Incular
  `crates/incular-animation/src/scheduler.rs:252–281,329–345,397–407`.
- **SC-02 Muting and future order (source):** a muted ticker started while
  idle, followed by unrelated frame at 0 and unmute at 16, first reports
  elapsed zero in Flutter; Incular initializes timestamp before the muted
  guard and reports 16. Muting after the first tick keeps elapsed in both.
  Flutter whenCompleteOrCancel registers a future continuation;
  Incular can execute completion inline on an already-completed ticker.
  Both support a cancel error through the secondary future. Flutter
  `scheduler/ticker.dart:447–497`; Incular
  `crates/incular-animation/src/scheduler.rs:271–280,329–345,571–614`.

Further review: warm-up/persistent/post-frame sequencing under multiple
windows, timestamp and dilation across suspension/skipped frames, reporting
frame timings, and exact task priorities.

## Painting

- **P-01 Image streams/quality (source):** Flutter animated codecs publish
  timed frames and cancel timer when last listener leaves. Incular ImageStream
  is terminal-ready/fail, and a second complete returns false. Nearest and
  bilinear policies have ordinary counterparts; Medium and High collapse to
  linear rather than mipmapped and bicubic. Probe an 8× upsample and 1/8
  downsample with contrasting texels, plus a two-frame codec.
  Flutter `painting/image_stream.dart:1050–1105,1151–1190`,
  engine `lib/ui/painting.dart:1035–1080`; Incular
  `crates/incular-image/src/stream.rs:42–50,128–205`,
  `crates/incular-rendering/src/gradients.rs:321–343`.
- **P-02 Draw path/filter (source):** Flutter sends full Paint on drawPath.
  Incular Canvas extracts brush/fill rule or stroke and drops blend mode and
  color filter. Draw a transparent source under Src onto an opaque target or
  add a tint filter; defaults alone do not expose the gap. Incular also
  rejects dilate/erode in subtree and backdrop filters even when nested
  in Compose. Existing blur/color paths and capture sharing are not missing.
  Flutter engine `lib/ui/painting.dart:4377–4385,8076–8081`; Incular
  `crates/incular-rendering/src/paint.rs:71–79,136–145`,
  `display_list.rs:84–93,625–635`, `backdrop.rs:41–112`,
  `crates/incular-widgets/src/painting_effects/images.rs:331–355`.
- **P-03 Numeric/shaping and paths (source):** wordSpacing is present in the
  Incular descriptor but absent from Parley mapping and shaping key;
  Flutter sends it to paragraph style. Probe width of `a a` with +10 px;
  letter spacing is a positive control. Foreground/background Paint
  precedence is not expressible with color-only options. Incular NonZero
  and EvenOdd containment work but one reversed contained hole is not
  arbitrary boolean Path.combine or contour metrics. Flutter
  `painting/text_style.dart:697–727,1368`, engine
  `lib/ui/painting.dart:3151–3202`; Incular
  `crates/incular-text/src/style/text_style.rs:32–33,182–184,488–514`,
  `engine.rs:495–549,2003–2034`,
  `crates/incular-rendering/src/paths.rs:169–216`.
- **P-04 Color and tiling (source):** Flutter stores float color and permits
  interpolation extrapolation. Incular's packed 8-bit color clamps t; red
  channel 20→100 at t=1.5 gives ≈140 vs 100. Preserve the compact retained
  representation by default if adding a more precise API. Flutter repeats
  all tile rects; Incular silently truncates at 16,384. A 1×1 repeated image
  in 256×256 exposes missing tiles; protective intent can be kept using GPU
  repeated sampling. All seven BoxFit geometry branches and zero-size guard
  agree on ordinary finite inputs. Flutter `painting/box_fit.dart:146–228`,
  `painting/decoration_image.dart:826–847`, engine `lib/ui/painting.dart:135–162,397–420`;
  Incular `crates/incular-core/src/geometry.rs:140–161,713–727`,
  `crates/incular-widgets/src/painting_effects/box_painter.rs:622–730`.

Further review: blend/filter matrix over paint operations, stroke joins and
zero widths, path inverse fill/metrics, font fallbacks/spacing/rendering,
images and codecs/events/caches/EXIF, tint, compositing, memory, pixel tests
at all target DPRs and buffer color spaces.

## Rendering

- **R-01 Clip and hit route (source):** Flutter separates None, hard-edge,
  AA and AA plus save layer. Incular lowers every non-None mode to a generic
  retained shape clip without AA/save-layer semantics. Flutter custom ClipPath
  checks custom path on hit; Incular culls pixels but retains the normal
  layout hit area. Probe inside the rectangular bounds but outside a triangle.
  Do not apply that rule to every default rounded rectangle: Flutter
  custom-clip hit filtering depends on a clipper. Flutter
  `painting/clip.dart:15–37`, `rendering/proxy_box.dart:1738,1997–2020`;
  Incular `crates/incular-widgets/src/render_object/layers.rs:14–26,60–77`,
  `layout/basic/clipping.rs:264–277`,
  `crates/incular-rendering/src/compositor.rs:1810–1830`,
  `display_list.rs:105–118`.
- **R-02 Transform/hit and annotations (source):** Flutter RenderTransform
  can leave hit testing untransformed; Incular public Transform always
  transforms hit tests (some other specialized transitions have a switch).
  Flutter's clip-layer annotation lookup tests the actual rounded/path
  shape; Incular tests axis-aligned transformed bounds. A corner inside the
  bounding box but outside the rounded shape disambiguates.
  Flutter `rendering/proxy_box.dart:2526–2534,2710–2727`,
  `rendering/layer.dart:1733–1739,1908–1914`; Incular
  `crates/incular-widgets/src/tree/descriptors.rs:1774–1784,1829–1870`,
  `tree/widget/constructors.rs:1315,1333`,
  `crates/incular-rendering/src/compositor.rs:1413–1435,2231–2245,2267–2283`.
- **R-03 Constraints and opacity (source):** Flutter biggest returns
  max including Infinity, Incular substitutes finite min (e.g. minWidth=12,
  maxWidth=∞ gives ∞ vs 12). Ordinary tighten/loosen/enforce are present.
  Flutter Opacity rounds 0.001 to 8-bit zero and skips painting. Incular
  semantics handles the quantized zero correctly, but wgpu still lowers a
  fractional positive opacity and traverses paint. This is an exact
  paint/optimization path difference, not proof of a visible pixel delta.
  Flutter `rendering/box.dart:363–367`,
  `rendering/proxy_box.dart:914–929,961–972`; Incular
  `crates/incular-config/src/constraints.rs:144–224`,
  `crates/incular-widgets/src/tree/semantics.rs:1302–1321`,
  `crates/incular-wgpu/src/renderer/lowering.rs:255–307`.
- **R-04 Physical shadows (source):** PhysicalShape/PhysicalModel
  already paint a surface and optional clip. Incular's offset/blur based
  on elevation are approximations (0.18× offset, max(0.55×,1) blur);
  Flutter drawShadow also takes transparent occluder and uses engine shadow
  construction. Compare contour and falloff with a transparent surface;
  don't claim current shadows are absent.
  Flutter `rendering/proxy_box.dart:2335–2359`;
  Incular `crates/incular-widgets/src/painting_effects/layers.rs:193–203,259–293`.

Further review: render dirty/repaint lifecycle, dry/baseline intrinsic behavior,
all layer/compositing operations and raster caches, shader fidelity, low-alpha
optimization and clipping, sliver geometry, and safe custom-object extension
boundaries; preserve wgpu.

## Semantics

- **SE-01 Merge action and block propagation (source + focused Incular):**
  Flutter collects descendant state and routes first reachable action for
  MergeSemantics. Incular's generic wrapper joins label and suppresses
  descendants, but loses a child's activation action. A dedicated probe on
  two actionable descendants exposes the gap despite the existing label-only
  test. Flutter bubbles a blocker through non-semantic-boundary ancestors;
  Incular checks direct child flag. Probe a blocker inside Padding with an
  earlier sibling. Flutter `widgets/basic.dart:7949–7987`,
  `rendering/object.dart:5742–5768`; Incular
  `crates/incular-widgets/src/tree/semantics.rs:278–316,810–891,1138–1154`,
  `tree/retained.rs:1337–1351`;
  label control `tests/semantics_flutter_parity.rs:32–55`.
- **SE-02 Events, focus and desktop scrolling (source):** Flutter can send
  an attached-node event and independently model accessibility gain/loss;
  iOS can request absolute scroll offset. Incular has keyboard-oriented
  Focus and relative scroll/ShowOnScreen, no equivalent per-node event
  sender or absolute scroll contract. AccessKit does map hidden, expanded,
  mixed, live and other flags; desktop lacks LongPress and maps both
  relative scrolling directions as vertical regardless of orientation.
  Flutter `semantics/semantics.dart:4415–4438,5576–5596,5802–5847`;
  Incular `crates/incular-semantics/src/lib.rs:45–76`,
  `crates/incular-runtime/src/frame.rs:914–921`,
  `crates/incular-accessibility/src/lib.rs:909–979,1092–1105`.
- **SE-03 Platform bridges (source):** Flutter creates production Android
  and iOS native bridges. Incular exports mobile adapter/projection types
  and tests them, but stock shells do not wire them into TalkBack/VoiceOver.
  Incular web renders DOM/ARIA attributes and supports click/focus/keyboard
  forwarding, but not Flutter's actual range action and polite live-region
  handling. Engine Flutter
  `shell/platform/android/io/flutter/embedding/android/FlutterView.java:1020,1220`,
  `shell/platform/darwin/ios/framework/Source/accessibility_bridge.mm:421–454`,
  `lib/web_ui/lib/src/engine/semantics/incrementable.dart:35–65`;
  Incular `system-design/MOBILE.md:1089`,
  `crates/incular-web/src/semantics.rs:336–484`. Do not equate source
  wiring with a passed assistive-technology test.

Further review: ordering, collapsed/merged flags/actions, transforms and
bounds clipping, focus/announcements across windows, TalkBack/VoiceOver,
screen reader and keyboard behavior on web, narration without refocus,
horizontal scrolling and editable/numeric accessibility actions.

## Services

- **SV-01 Restoration (source):** Incular supports versioned on-disk
  snapshots, migration, typed handles and opt-in exclusive scope leases.
  Flutter's root can be lazily supplied and replaced live, and exclusive
  child buckets can be claimed/reparented; widget restoreState receives old
  bucket and availability changes. Incular defaults to namespacing without
  live writer exclusivity or runtime rebind of retained Restorable handles.
  Flutter unregister removes data and listener; Incular remove can be
  followed by set that writes again. Incular text restoration preserving
  selection is a documented extension over Flutter text-only restoration.
  Flutter `services/restoration.dart:189–234,271–279,420–427,462–503`,
  `widgets/restoration.dart:585–624,694–743,820–835,901–929`,
  `widgets/restoration_properties.dart:495–502`; Incular
  `crates/incular-core/src/restoration.rs:164–219`,
  `crates/incular-runtime/src/restoration.rs:27–43,511–610,1134–1157,1181–1234`,
  `crates/incular-text/src/editing.rs:383–413,1099–1119`.
- **SV-02 Stock iOS input/clipboard (source):** Flutter's stock host
  implements marked text, insertion, keyboard show and UIPasteboard.
  Incular's UIKit shell constructs the MobileEngine, forwards touch, but
  does not install native clipboard, consume text-input commands, or
  provide UIKeyInput/UITextInput. Default clipboard remains in-memory.
  Runtime editing or public projection does not establish platform parity.
  Flutter engine `shell/platform/darwin/ios/framework/Source/FlutterTextInputPlugin.mm:1637,2380,2925`,
  `FlutterPlatformPlugin.mm:431–440`; Incular
  `crates/incular-ios/src/shell.rs:200–216,411–438`,
  `crates/incular-mobile/src/lib.rs:290–295`,
  `crates/incular-runtime/src/frame.rs:360`.
- **SV-03 Clipboard web and Android (source):** Android metadata-only
  hasStrings probes text MIME without triggering a content read in both.
  Flutter web awaits the native browser result and can report errors;
  Incular returns a synchronous snapshot from the latest copy/paste state
  and can report local write after a failed system write. Gesture paste and
  rich transfer exist, but do not prove fresh direct read.
  Flutter `services/clipboard.dart:26–72`, engine
  `shell/platform/android/io/flutter/plugin/platform/PlatformPlugin.java:700–714`,
  web engine clipboard implementation; Incular
  `crates/incular-android/android/src/main/kotlin/dev/incular/IncularNative.kt:121–124`,
  `crates/incular-web/src/clipboard.rs:1–17,72–102,145–176`.
- **SV-04 Keyboard and back (source):** Android bridge sends key code,
  Unicode, meta state, not physical scan code/location; several lock
  and media keys have no map and unmapped no-character keys drop.
  Flutter maps physical scans and logical Android IDs independently.
  Desktop ordinary Winit preserves physical/logical/repeat/location;
  unknown physical ASCII falls back to US guessing. Android predictive
  progress types exist in Incular core, but stock host only handles commits.
  Flutter `services/keyboard_maps.g.dart:17,281,519`;
  Incular `crates/incular-android/src/shell/text_input.rs:251–289,406–458`,
  `crates/incular-desktop/src/winit_adapter.rs:381–452`,
  `crates/incular-platform/src/back_gesture.rs:20,87`,
  `crates/incular-android/android/src/main/kotlin/dev/incular/IncularActivity.kt:78–98`.
- **SV-05 Native capability routing (source):** Flutter dynamic FontLoader
  takes an explicit family, waits for async source bytes, and handles errors
  and lifecycle. Incular has true dynamic font registration but takes bytes
  only. Android content sensitivity requires a native API call and cannot
  be inferred from Incular's policy seam. Flutter has Scribe, rich keyboard
  insertion (MIME/URI/data), process-text, deferred install, and iOS undo
  manager host behaviors. These are not provided by Incular's ordinary
  plain-text input/undo or file-transfer API. Flutter
  `services/font_loader.dart:16–76`, `services/sensitive_content.dart:97–138`,
  `services/undo_manager.dart:58–131`, `services/process_text.dart:35–61,112–133`,
  `services/deferred_component.dart:15–90`, `services/scribe.dart:29–79`,
  `services/keyboard_inserted_content.dart:15–40`; Incular
  `crates/incular-text/src/engine.rs:572–614,799–836`,
  `crates/incular-platform/src/content_sensitivity.rs:58–84`,
  `crates/incular-desktop/src/platform_services.rs:184–191`.
- **SV-06 Service positive controls (source):** stationary mouse is
  re-hit-tested after layout without synthesizing ordinary hover, and
  cursor deferral follows the route. Audit enter/exit order and annotation
  validity independently. Flutter deprecated duplicate RawKeyboard type
  identity; preserve modern per-device and physical/logical coverage.
  Basic text layout positions have an equivalent, but NEL U+0085,
  LS U+2028, PS U+2029 and vertical/affinity boundaries still need probes.
  Flutter `services/mouse_tracking.dart:49–111`,
  `services/raw_keyboard.dart:749–775`, `services/text_layout_metrics.dart:15–98`;
  Incular `crates/incular-widgets/src/tree/raw_input.rs:114–151`,
  `crates/incular-text/src/geometry.rs:15–42`.

Further review: exact binary and structured asset contracts, per-adapter
clipboard permission/error and keyboard dictionary coverage, platform text
transactions, spell check and native service stubs, accessibility, predictive
back state, platform view gestures/occlusion, and restitution/IO fault cases.

## Widget previews

- **WP-01 Toolchain contract (source):** Flutter scans annotated source and
  watches for changes, generates a manifest and launches a preview host.
  PreviewBuilder.addWrapper(new) produces new(old(child)); the first
  MultiPreviewThemeData is outermost. Incular CLI builds/runs/captures;
  manual galleries are not registration, discovery, wrappers, localization,
  per-preview options or host/reload equivalents. Flutter
  `widget_previews/widget_previews.dart:294–301,405–421`, tooling
  `packages/flutter_tools/lib/src/widget_preview/preview_detector.dart:101–124`,
  `packages/flutter_tools/lib/src/commands/widget_preview.dart:58–89,150`;
  Incular `tools/incular-cli/src/main.rs:1–11,68–77,135–144`.

Further review: tool and scaffold test registrations are catalogued separately
from the framework. Specify selected-file, hot reload/restart, errors,
non-constant annotations, widget inspector, screen sizes, theme and localization
scenarios. Do not count an unimplemented preview as passed.

## Widgets and layout

- **W-01 MainAxisSize.min with Expanded (source + focused Incular):**
  loose max 100, two equal Expanded children. Flutter measures 50 each,
  reports width 100 and child 2 x=50. Incular measures the children, then
  recomputes flex from fixed sizes with target zero, returning width 0
  and child 2 x=0. Unbounded tight Expanded also silently falls back in
  Incular where Flutter debug rejects it.
  Flutter `rendering/flex.dart:1113–1166,1249–1298,1351+`;
  Incular `crates/incular-widgets/src/tree/layout/behavior/containers.rs:77–148`,
  `crates/incular-layout/src/algorithms.rs:199–246,298–339`.
- **W-02 Table allocation and cells (source + focused Incular for widths):**
  default Table with width 100 and two natural 80 cells: Flutter computes
  flex widths 50/50 and then tight-measures. Incular keeps 80/80 and puts
  cell 2 at x=80, clamping only the reported table width. TableCell
  vertical alignment currently returns the child unchanged (bottom
  aligning height 10 next to height 30 should produce y=20). No RTL
  column order or row decoration/key contract found. Flutter
  `rendering/table.dart:197–218,1100–1229,1333–1435`,
  `widgets/table.dart:120–178,441–476`; Incular
  `crates/incular-layout/src/algorithms.rs:824–912`,
  `crates/incular-widgets/src/tree/layout/behavior/containers.rs:206–245`,
  `layout/basic/utility.rs:46–73`, `layout/table.rs:17–33`.
- **W-03 Bounds and fitting (source):** OverflowBox's Flutter max fit
  uses size 100×80 and centers a 20×10 child at (40,35) for finite loose
  bounds. Incular fits the child itself. Default Contain FittedBox keeps
  200×100 child's aspect under loose 100×100 in Flutter (100×50);
  Incular chooses 100×100 then letterboxes. Default Wrap is no clip
  in both, but custom clipping is missing and a rounding seam can
  create a second run: widths 10.1+10.3 at available 20.4. Flutter
  `rendering/shifted_box.dart:690–767`, `rendering/proxy_box.dart:2884–2931`,
  `rendering/wrap.dart:179–190`, `widgets/basic.dart:6109–6120`;
  Incular `crates/incular-widgets/src/tree/layout/behavior/core.rs:163–195`,
  `tree/layout/behavior/effects.rs:39–51`, `layout/wrap.rs:12–31`,
  `crates/incular-layout/src/algorithms.rs:701–719`.
- **W-04 Stack defaults (source):** Flutter AlignmentDirectional.topStart
  vs Incular absolute top-left. RTL width 100, non-positioned child width 30
  yields x=70 vs 0. Default hard-edge clip in Flutter activates only on
  positioned child geometry overflow; Incular unconditional clip can cut a
  fitting child's shadow. Both cover ordinary empty bounded and two-inset
  positioned cases. Flutter `widgets/basic.dart:4767–4773`,
  `rendering/stack.dart:675–723`; Incular
  `crates/incular-widgets/src/layout/stack.rs:9–24`,
  `render_object/layers.rs:14–25,248–251,855`.
- **W-05 Scroll sizing and direction (source):** loose viewport max 100×100,
  child 20×30: Flutter single-child viewport uses child size,
  Incular fills finite maxima. Flutter horizontal direction is ambient
  RTL XOR reverse: width 100 with 200 content and RTL, reverse=false
  starts at right, vs left in Incular. Flutter prototype-extent list
  measures prototype at current cross constraints; Incular uses an
  unconstrained static hint. Probe a LayoutBuilder prototype switching
  through widths. Flutter `widgets/single_child_scroll_view.dart:453–502`,
  `widgets/basic.dart:4567–4585`,
  `widgets/sliver_prototype_extent_list.dart:231–272`; Incular
  `crates/incular-widgets/src/tree/widget/geometry.rs:48–74`,
  `tree/layout/behavior/scrolling.rs:12–45`, `scrolling/scroll_views.rs:143–173,706–722`,
  `scrolling/sliver_descriptors.rs:1124–1134`,
  `scrolling/layout.rs:3116–3172`.
- **W-06 Nested scrolling and notifications (source):** Flutter provides a
  descendant primary controller eagerly and coordinates multiple inheriting
  inners plus programmatic moves; explicit inner controllers opt out.
  Incular lazily binds one weak inner selected by pointer, can bind an
  explicit controller, and programmatic jump/animate do not use the
  drive delegate. Ordinary transfer with one bound inner is implemented.
  ScrollNotificationObserver is currently pass-through, while
  NotificationListener works. Metrics publish synchronously during
  Incular layout attempts; Flutter coalesces after layout via a microtask.
  Flutter `widgets/nested_scroll_view.dart:290–295,349–373,908–954,1069–1129`,
  `scroll_notification_observer.dart:94–109,148–187,223–242`,
  `scroll_position.dart:670–680`; Incular `crates/incular-scroll/src/coordinator.rs:212–260,345–380,579–604`,
  `controller.rs:372–418,571–593`, `physics.rs:1054–1067`,
  `crates/incular-widgets/src/tree/interaction.rs:965–993`,
  `tree/layout/behavior/sliver.rs:228–264`, `scrolling/notifications.rs:31–84,145–159`.

- **W-07 Sliver safe/fill/varied extent (source):** SliverSafeArea is one
  geometry wrapper in Flutter, but Incular wraps each retained child and
  leaves geometry unchanged; two 20 px rows plus top inset 10 should yield
  extent 50 and row origins 10/30, not unchanged extent 40. Varied extent
  has current layout dimensions and nullable end in Flutter, not the
  Incular fixed-count index-only callback. Flutter fillOverscroll includes
  local scrollOffset in maximum extent to fill trailing space. At paint
  extent 100 and scrollOffset 20 with zero overlap, that max is 120
  vs 100; non-scrollable intrinsic width-dependent children and cached
  zero-paint child semantics also need targeted fixtures.
  Flutter `widgets/safe_area.dart:205–223`,
  `rendering/sliver_fixed_extent_list.dart:241–277`,
  `rendering/sliver_fill.dart:128–144,191–207,263–297`; Incular
  `crates/incular-widgets/src/scrolling/layout.rs:1405–1477,2254–2277`,
  `scrolling/sliver_descriptors.rs:1015–1071,1889–1924`.
- **W-08 SafeArea and media aspects (source):** Flutter still removes selected
  padding from descendants in maintainBottomViewPadding mode. Incular's
  maintained path does not publish reduced insets, so nested safe areas
  can double the inset (bottom 20→40). Standard path has a partial scoped
  inset layer. Flutter MediaQuery has separate width/height subscriptions,
  rich TextScaler, displayCornerRadii, and distinct reduceMotion and
  disableAnimations. Incular has real fold/accessibility/gesture metrics
  and granular dependencies, but uses whole viewport Size and f32 scale,
  has no radii and shares one reduced-motion flag. Flutter
  `widgets/safe_area.dart:113–135`,
  `widgets/media_query.dart:677–720,850,1026–1048,1680–1713,1799–1811,2274–2286`;
  Incular `crates/incular-widgets/src/safe_area.rs:203–251`,
  `tree/context.rs:70–123,397–402,479–493`,
  `crates/incular-config/src/environment.rs:161–229`.
- **W-09 Visibility vs Offstage (source):** Flutter defaults
  maintainFocusability false under hidden retained Visibility and
  separately allows opt-in. Incular deliberately retains keyboard focus
  and has no switch. A hidden retained editor can keep receiving input
  or be traversable. Offstage intentionally may retain focus and ticks
  in both; don't disable that behavior wholesale. Flutter
  `widgets/indexed_stack.dart:252–292,417–421,451–473`;
  Incular `crates/incular-widgets/src/layout/basic/visibility.rs:9–41,118–184`,
  `tree/focus.rs:27–57`.
- **W-10 Inherited dependencies (source):** Flutter InheritedModel finds
  first supporting ancestor through unsupported nearer same-type models
  and passes all subscribed aspects together. Incular uses nearest model
  and calls predicate with individual aspect singletons, breaking
  predicates requiring `A && B`. Ordinary one-aspect/all tracking exists.
  Flutter InheritedNotifier listens to arbitrary notifier and coalesces;
  Incular publishes typed value and needs explicitly tracked signal reads.
  Flutter normal BuildContext lookup crosses LookupBoundary; only
  explicit lookup limits the scope. Incular models one environment
  boundary path. Flutter `widgets/inherited_model.dart:135–258`,
  `inherited_notifier.dart:93–133`, `lookup_boundary.dart:76–156`;
  Incular `crates/incular-widgets/src/inherited_data.rs:167–253,278–310`,
  `environment.rs:681–717`.
- **W-11 Lifecycle and views (source):** Flutter synthesizes intermediate
  lifecycle transitions e.g. inactive→hidden→paused so listeners see
  hide before pause and restart/show on return. Incular one-event port
  suppresses duplicates, flushes restoration on suspend, and collapses
  some states; no equivalent hidden/detached path or async exit response.
  Flutter runApp implicitly installs View and View.maybeOf subscribes
  to identity. Incular requires explicit View and republishes metrics.
  Incular's anchored native view is not Flutter's arbitrary non-rendering
  wrapper composition and multi-view collection. Flutter
  `services/binding.dart:303–342`, `widgets/app_lifecycle_listener.dart:204–272`,
  `widgets/binding.dart:1642,1897–1899`, `widgets/view.dart:120–138,650–729`;
  Incular `crates/incular-runtime/src/application_types.rs:29–47`,
  `application.rs:1440–1468`, `frame.rs:705–727`,
  `crates/incular-widgets/src/app_shell/view.rs:138–176,494–528,885–997`.
- **W-12 Async and restore-route timing (source):** FutureBuilder pending
  replacement preserves old payload and ignores old handle; stream
  reconnect hooks exist. Already-complete Incular UiFuture is
  synchronous; ordinary Flutter Future can produce one waiting frame,
  while SynchronousFuture can skip it. Incular restoration route
  registry and typed results are present, but no RestorableRouteFuture
  bridge reconnects a new caller to a restored route's completion.
  Flutter `widgets/async.dart:486–489,633–665`,
  `widgets/navigator.dart:6315–6338,6402–6416,6442–6453`; Incular
  `crates/incular-widgets/src/async_builders.rs:156–177,283–315,478–585`,
  `crates/incular-navigation/src/registry.rs:60–164`.

- **W-13 Metrics and ListBody (source, plus focused metrics comparison):**
  Flutter exact atEdge equality and extentInside excludes overscroll. At
  pixels=-20, min=0/max=200, viewport 100, Flutter false/80 vs
  Incular true/100; before/after/total/outOfRange are separate controls.
  Incular controller snapshots also hardcode DPR 1. Flutter ListBody
  requires unbounded main with bounded cross and tightens child to the
  full cross span. Incular delegates to default Row/Column; an unbounded
  vertical layout with max width 100 and child width 20 only takes 20.
  Flutter `widgets/scroll_metrics.dart:119–162`,
  `rendering/list_body.dart:127–190,224+`; Incular
  `crates/incular-scroll/src/metrics.rs:15–69`,
  `crates/incular-widgets/src/scrolling/sliver_descriptors.rs:938–959`.
- **W-14 Retained scroll policy (source):** DraggableScrollableSheet's
  sizes/snap helper exist, but mounted release has no automatic snap caller;
  release returns None when snap is enabled. Flutter installs a snapping
  simulation on release. TwoDimensionalScrollable supports diagonal
  modes in isolation; retained view config does not carry that policy.
  Neutral RawScrollbar thickness 6 and minimum thumb 18 match, but not
  the full shape/track/mouse/predicate contract. Flutter
  `widgets/draggable_scrollable_sheet.dart:306–322,927–959`,
  `widgets/scrollbar.dart:1007–1038`; Incular
  `crates/incular-widgets/src/advanced_scrolling/draggable.rs:625–635,1298–1323`,
  `two_dimensional.rs:159–186,398–475,625–639,1326–1328`,
  `raw_scrollbar.rs:231–268`.

- **W-15 Baseline outside child (source):** Flutter Baseline sets child
  top to requested minus child baseline without clamping; an absent
  baseline falls back to child height. For non-text 20×20 with baseline
  5, Flutter gives child y=-15/root height 5; Incular clamps y=0/root
  height 20 while publishing the requested baseline. Flutter descriptor
  also takes TextBaseline, while Incular uses a scalar only. Flutter
  `rendering/shifted_box.dart:1528–1570,1595–1603`; Incular
  `crates/incular-layout/src/algorithms.rs:584–599`,
  `crates/incular-widgets/src/tree/layout/behavior/core.rs:251–265`.

Further review: layout all parent-data types, dry/intrinsic/baseline including
zero/loose/infinite constraints; sliver geometry, notifications and sequence
order, reverse and RTL, multi-position scroll, all widget lifecycle/keys,
deferred async state, focus and editing sessions, restoration ownership,
routes/modal/overlay accessibility, system view and notifications. Low profile
or rarely used neutral widgets remain in scope, including those with no
Incular lexical candidate.

## What the evidence does not establish

The broad manual reports and paired probes close only the named cases. The
upstream queue is not exhausted. API properties and defaults, callback or
geometry differences, platform integration and performance remain separate
dimensions. Use dependency/source files, official tests, maintained harness
twins, and original cases for app behavior not covered by upstream tests.
Do not reinterpret the catalogued registrations as 8,749 passed tests or the Rust
controls as a complete certification. No production implementation changes
or full-workspace test runs were made.
