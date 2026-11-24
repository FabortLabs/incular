# Behavior audit: neutral focus, paint, editing, and edge cases

**Reference:** Flutter 3.50.0 at `ab2912035e160d9cad6306df307576da6128ebdb`;
Incular implementation `c647cb856bda434be4657660e492c43dcee882b9` (unchanged
through the first two audit commits). Material and Cupertino are excluded.

The declaration inventory is a route map, not evidence of parity. This follow-up
traced each contract below from public defaults through event dispatch or
lowering to the observable result. **Reproduced** means the Incular side
actually ran and differed from an expected result established in the pinned
Flutter source. **Source-traced** means both implementations were read, but
the exact case has not run on both frameworks. GPU, native keyboard and
accessibility-device conclusions are not promoted to runtime verification.

## Targeted reproduction results

| ID | Scenario | Expected from Flutter 3.50 source | Observed on Incular |
| --- | --- | --- | --- |
| B01 | Focus editor A with `a`; touch down on editor B with `b`; cancel the pointer before touch-up; type `x`. | A stays the editing target: `("ax", "b")`. | `("a", "bx")`. The cancelled touch moved the editing target. |
| B02 | 40×40 Container with a uniform, tinted 2 px inside border whose style is None. | Reserve the inset, emit no stroke. | Display list contains the tinted 2 px Border. Identical absent-border and solid-border controls omitted and emitted the stroke, respectively. |
| B03 | Fixed-extent sliver, 20.3 logical px rows, cache interval [0, 60.9). | Materialize [0, 1, 2]. | [0, 1, 2, 3]. Integer control (20 px rows, end 60 px) correctly built [0, 1, 2]. |
| B04 | Apply a native selection-only update to `abc`, using a counter formatter. | Caret moves; formatter calls = 0. | Caret moves; formatter calls = 1. |
| B05 | Send native whole-state text `a\nb` into a default single-line EditableText. | `ab`, caret 2. | The LF is stored (`a\nb`). |
| B06 | Schedule transient frame callbacks A then B; A cancels B while the batch is running. | Callback trace `[A]`. | `[A, B]`. |

Existing focused controls also run on the same Incular source:

- Android/iOS touch A→B→A and Android touch outside: **2 passed**.
- Focus an undecorated neutral EditableText and inspect its display list:
  **1 passed**, no framework outline. This does not contradict B02;
  B02 is a separate route through a surrounding Container decoration.

The new probes intentionally asserted the Flutter result and failed where
shown. They ran as isolated temporary integration targets, not the workspace
suite. Scratch fixtures and logs are in `.tmp/flutter350-behavior/` on the
audit laptop; temporary targets were removed. No Flutter 3.50 runner was
started: the reference is a source clone without its cached Dart SDK.

## B01 remediation

Touching another editable on Android or iOS no longer moves focus or
publishes a new native text-input client before the tap is recognized.
Cancel, movement past slop, or loss to another gesture leaves the
current editor in place. Accepted taps still focus the second editor;
the existing long-press and desktop-click suites also pass.
The pinned authored Android test and the Rust B01 test both keep
typing in the first editor after cancellation. Rust checks the second
editor's selection and the native command queue as well.

## Focus and native editing lifecycle

Flutter references in this report are relative to
`.tmp/flutter/packages/flutter/lib/src/`; Incular references are repository
paths or continue from the preceding crate path.

- **B01: cancellation** (reproduced). Flutter RenderEditable registers a tap
  and saves down coordinates without requesting the keyboard. It requests
  keyboard on confirmed tap; a cancelled sequence does not reach that handler
  (`rendering/editable.dart:2008–2017,2045–2066`,
  `widgets/editable_text.dart:4555–4570`). Incular commits focus and native
  intent on Down; Cancel only clears captures
  (`crates/incular-runtime/src/frame.rs:1612–1669,1714–1726,1791–1823`).
- **F01: focus eligibility before side effects** (source-traced). Pointer
  dispatch identifies TextField irrespective of enabled state, then focuses it
  before the later eligibility reconciliation. Disabled or ExcludeFocus
  descendants can take logical focus transiently and displace the previous
  editor. Flutter checks canRequestFocus before requesting
  (`widgets/focus_manager.dart:541,1174–1195`). Incular:
  `crates/incular-widgets/src/tree/painting.rs:104–133,475–495`,
  `tree/text.rs:1604–1610`, `tree/focus.rs:7–57`,
  `crates/incular-runtime/src/frame.rs:1638–1669,1791–1857,2645,2686`.
  Next: while A has active composition, Down on disabled B and on B inside
  ExcludeFocus; inspect active editor and IME commands immediately and next frame.
- **F02: editor-owned FocusNode and defaults** (source-traced). Flutter attaches
  the supplied FocusNode to the editor, handles replacement and requests made
  before attachment, and schedules autofocus after layout. The neutral
  Incular descriptor lacks an external node/autofocus option; wrapping it in
  Focus mounts a separate focus target, not the editing target
  (`widgets/editable_text.dart:1004,3362,3414–3421,3544–3546,4297–4302,5961–5965`,
  `widgets/focus_manager.dart:1143–1195`; Incular
  `crates/incular-widgets/src/tree/descriptors.rs:991–1044,1577–1648`,
  `focus_keyboard.rs:59–87`, `tree/focus.rs:411–503`,
  `crates/incular-runtime/src/frame.rs:1946–1952,2200–2205`).
- **F03: select-all on external focus** (source-traced). Flutter defaults true
  on desktop/web, false on Android/iOS/Fuchsia; selects all on eligible
  single-line external focus, not internal/autofocus/resume. Incular only
  collapses an unset selection. Tab to a populated single-line field with a
  saved collapsed caret and type a character; replacement vs insertion is the
  discriminating case
  (`widgets/editable_text.dart:2164–2177,5013–5029`;
  `crates/incular-widgets/src/tree/text.rs:64–77`).
- **F04: tap group routing** (source-traced). Flutter sends outside callbacks
  for the editor's group, including up; Incular's focus-preservation check
  accepts any enabled text-field tap region. Probe grouped adjacent buttons with
  same vs different group IDs, using mouse and touch
  (`widgets/editable_text.dart:5909–5915`;
  `crates/incular-widgets/src/tree/raw_input.rs:215–227`).
- **F05: read-only transitions** (source-traced). Flutter normally does not
  create an input connection for read-only, except web/macOS where selection
  can still be applied. On going back to editable while focused it can reopen
  after layout. Incular models read-only in a runtime client but gates read-only
  inbound selection entirely; Android correctly suppresses visibility and
  only attaches on the same-client return to editable, while web deactivates
  the read-only DOM input. An attach is not a show. This is a command/adapter
  mismatch, not a tested physical keyboard outcome
  (`widgets/editable_text.dart:2651–2666,3544–3564,3720–3724,4156–4179`;
  `crates/incular-runtime/src/frame.rs:1997–2054,2461–2468,2533–2538`,
  `crates/incular-web/src/text_input.rs:64–85`,
  `crates/incular-android/android/src/main/kotlin/dev/incular/IncularInputView.kt:58–98`).
- **F06: stale native client risk** (source-traced). Flutter rejects events for
  retired client IDs; Android whole-state events in Incular do not carry a
  client ID. An unattributable desktop commit intentionally falls back to the
  current editor. After A→B, late A events need their own focused probe, not an
  assertion that every adapter exhibits the race
  (`services/text_input.dart:2267–2289`;
  `crates/incular-android/src/shell/text_input.rs:194–221`,
  `crates/incular-runtime/src/frame.rs:2477–2557`,
  `crates/incular-runtime/tests/text_input_gating.rs:162–204`).

## Neutral paint, borders and pixel edges

- **B02: invisible style dropped at Container lowering** (reproduced).
  Flutter's BorderStyle.none suppresses painting, not necessarily the
  dimensional inset (`painting/borders.dart:111–129,143–147`,
  `painting/box_border.dart:652–677`, `widgets/container.dart:378–403`).
  Incular's conversion keeps top width/color but discards style; a uniform
  inside border takes the fast DecoratedBox path and emits a border command
  (`crates/incular-widgets/src/layout/container.rs:138–152,227–269`,
  `painting_effects/decoration.rs:240–243`,
  `painting_effects/box_painter.rs:35–43,112–116`,
  `tree/painting/behavior/visual.rs:64–79`). Direct DecoratedBox's descriptor
  masks None to transparent; that is why a route-specific test matters
  (`tree/descriptors.rs:633–641`).
- **P01: zero-width solid border** (source-traced through WGSL). Flutter draws
  a one-physical-pixel hairline, not a filled shape or a missing border.
  Zero passes through the Incular analytic Border path; the square and round
  shaders select full coverage at zero stroke width. The tessellated path
  rejects zero width instead. Test square, rounded, and non-uniform variants
  at DPR 1, 1.25 and 2 with contrasting edge/interior probes
  (`painting/borders.dart:111–129`;
  `crates/incular-wgpu/src/render_plan.rs:878–899`,
  `shaders/square_rect_shader.wgsl:23`, `shaders/rrect_shader.wgsl:23`,
  `geometry.rs:65–75,168–173`;
  `crates/incular-widgets/src/painting_effects/box_painter.rs:112–139,426–449`).
- **P02: clipping belongs inside the decoration** (source-traced). Flutter
  clips the inner child by the decoration path, then installs background,
  foreground, constraints, margin and transform. Incular clips outermost and
  chooses rectangle/rounded rectangle from radius alone, ignoring circular
  shape. Use circular decoration, margin, a shadow and a child occupying
  corners to separate wrong shape from wrong layer order
  (`widgets/container.dart:400–442`;
  `crates/incular-widgets/src/layout/container.rs:246–319`).
- **P03: fast and full paint disagree** (source-traced). For both color and
  gradient supplied, Flutter paints the gradient; Incular's fast route chooses
  color while its full painter chooses the gradient. Flutter also half-insets
  opaque solid background coverage at the border; only Incular's full painter
  does, a candidate for AA fringes. None→solid BorderSide interpolation in
  Flutter fades transparent to solid; Incular switches style at 0.5. Use a
  quarter-progress sample as well as exact endpoints
  (`painting/box_decoration.dart:141–149,417–423,473–511`,
  `painting/borders.dart:255–296`;
  `crates/incular-widgets/src/layout/container.rs:138–145`,
  `tree/descriptors.rs:624–631`, `tree/painting/behavior/visual.rs:64–79`,
  `painting_effects/box_painter.rs:66–99`, `painting_effects/decoration.rs:102–115`).
- **P04: neutral editor selection/caret paint** (source-traced). Plain
  Flutter EditableText has no default selection color. Incular injects
  rgba(72,120,220,150). Flutter clips caret and selection along with overflowing
  text, intersects highlight against TextPainter bounds, and snaps the caret
  globally to physical pixels: x=10.3 at DPR 2 becomes 10.5. Incular emits
  selection/caret before the glyph viewport clip and constructs an unsnapped
  caret rect. Both default to painting below glyphs; Flutter can opt above,
  where Incular hardwires below
  (`widgets/editable_text.dart:887,6034–6063`,
  `rendering/editable.dart:495–514,1854–1855,2332–2344,2604–2625,2673–2687,2897–2921`;
  `crates/incular-widgets/src/tree/descriptors.rs:1047–1073`,
  `tree/painting/behavior/content.rs:227–315`, `tree/text.rs:1868–1889`).

## Editing transforms, defaults and Unicode

- **B04: format only real edits and composing commit** (reproduced on
  selection; composition source-traced). Flutter bypasses formatters for
  selection-only and noncollapsed composing-range-only updates; composing
  collapse/commit can format. Incular dispatches every changed native state as
  a user delta then runs transforms unconditionally. It also hides toolbar on
  range-only composition changes where Flutter does not
  (`widgets/editable_text.dart:3734–3763,4758–4779`;
  `crates/incular-runtime/src/frame.rs:2533–2560`,
  `crates/incular-text/src/editing.rs:595–599,1018–1045`).
- **B05: default single-line LF handling** (reproduced via native state).
  Flutter prepends a formatter denying LF when maxLines is 1, before user
  formatters. Incular defaults to one line but adds no implicit transform.
  Paste also routes the original string into the controller; test clipboard
  separately, rather than assume the native-state result covers every route
  (`widgets/editable_text.dart:988–995`, `services/text_formatter.dart:427`;
  `crates/incular-widgets/src/tree/descriptors.rs:1059–1062,1090–1094,1551–1576`,
  `crates/incular-runtime/src/frame/text_actions.rs:137–139`).
- **E01: filter endpoint remapping** (source-traced). Digits-only with
  `1a2`, caret 2, should yield `12`, caret 1. Incular preserves endpoint
  numbers then clamps, ending at 2. The same occurs with `1😀2`, Flutter
  incoming UTF-16 caret 3 / Incular incoming UTF-8 caret 5. The error is
  remapping after deletion, not surrogate decoding
  (`services/text_formatter.dart:392–423`;
  `crates/incular-widgets/src/forms.rs:1401–1423`,
  `crates/incular-text/src/editing.rs:1047–1048`).
- **E02: length limit can lose prior text** (source-traced). With enforced
  limit 3, old `abc` and a collapsed caret at 1, inserting X creates
  `aXbc`. Flutter returns the old value; Incular truncates to `aXb`,
  losing c. The unspecified policy is platform-dependent in Flutter, not
  always Enforced as in Incular
  (`services/text_formatter.dart:519–532,575–586`;
  `crates/incular-widgets/src/forms.rs:1450–1455,1467–1498`).
- **E03: deleting a selected partial grapheme** (source-traced). In `AéZ`,
  select only the combining mark (UTF-16 2..3, UTF-8 2..4) then Backspace:
  Flutter expands selection to the atomic boundaries and yields `AZ`;
  Incular deletes exact selected bytes and yields `AeZ`. Both already
  have ordinary collapsed-grapheme behavior and code-point movement for
  obscured text; those controls do not exercise a partial selection
  (`widgets/editable_text.dart:5557–5562,6613–6627`;
  `crates/incular-runtime/src/frame/text_actions.rs:107–123,202–225`,
  `crates/incular-text/src/editing.rs:136–144,603–605`).
- **E04: composing range and caret affinity** (source-traced). Flutter clears
  the composing range when a programmatic selection leaves it or focus is
  lost. Incular clamps it on selection change and clears transient preedit
  on blur, but can keep the controller's whole-state composing range.
  Flutter can preserve or accept upstream/downstream engine affinity;
  the Incular native-state payload carries no affinity even though internal
  code supports it
  (`widgets/editable_text.dart:358–370,3714–3717,3779–3784,4201–4207`;
  `crates/incular-text/src/editing.rs:84–90,513–523,970–992`,
  `crates/incular-platform/src/lib.rs:652–658`,
  `crates/incular-runtime/src/frame.rs:1798–1812,2435–2444`).

## Off-by-one, hit testing and time

- **B03: fractional cache edge** (reproduced). Flutter snaps within a
  tolerance of `1e-10` logical px before choosing the inclusive final
  child. Incular's f32 `60.9 / 20.3` is about 3.000000238; ceil yields
  end-exclusive 4. Left and right edges, fractional extents, and accumulated
  large positions need separate probes; do not apply a broad arbitrary epsilon
  that hides truly visible rows
  (`foundation/constants.dart:66–71`,
  `rendering/sliver_fixed_extent_list.dart:106–157`;
  `crates/incular-widgets/src/scrolling/layout.rs:165–190`).
- **G01: shared hit edge** (source-traced). Flutter box hit regions are
  half-open. Incular's Rect::contains includes right and bottom; reverse
  paint-order hit traversal then makes the last-painted left box win a shared
  edge. Test 60 px parent with right box [20,40) painted first and left
  [0,20) last: at (20,10) Flutter dispatches right, Incular left. At (40,10)
  Flutter dispatches neither child, Incular right. At x=19.999 both left
  (Flutter engine `lib/ui/geometry.dart:560–568,866–874`,
  `rendering/box.dart:2957–2963,3364–3382`;
  `crates/incular-core/src/geometry.rs:110–115`,
  `crates/incular-widgets/src/tree/painting.rs:475–554`).
- **G02: local pointer coordinates after relayout** (source-traced). Flutter
  reuses the Down hit path and recorded coordinate transform for Move/Up.
  Incular keeps recipient IDs but relocalizes against the current transform.
  Down global x=10, move the same stable listener from origin x=0 to x=40,
  release at global x=10: expected local Up x=10 vs Incular x=-30
  (`gestures/binding.dart:408–438,506–508`, `gestures/hit_test.dart:185–193`,
  `rendering/box.dart:837–849`;
  `crates/incular-widgets/src/tree/raw_input.rs:420–456,529–546`).
- **G03: single vs double tap and exact deadlines** (source-traced). With both
  callbacks installed, same-position Down/Up at 0/10 ms and 50/60 ms produces
  one double and zero singles in Flutter; Incular dispatches a single on Up 1
  then a double on Up 2. Both comparisons accept exactly 100 px double-tap
  separation and 18 px slop. A split at exactly 300 ms needs an explicit
  fake-clock ordering check: pump expiration before dispatching Down 2.
  Existing Incular tests exercise nearby values, not every exact threshold
  (`widgets/gesture_detector.dart:1055–1099`,
  `gestures/multitap.dart:102–105,223–229,265–267,328–358`,
  `gestures/arena.dart:157–166,281–291`, `gestures/tap.dart:378–384`;
  `crates/incular-widgets/src/tree/interaction.rs:817–847,1145–1171,1195–1210`,
  `crates/incular-gestures/src/pointer.rs:719–729,810–812,965–1009,1044–1051`).
- **B06: cancellation within the current frame** (reproduced). Flutter
  keeps cancelled IDs and checks them while draining. Incular detaches the
  callback vector and cancellation cannot find an item already in that local
  vector. Existing cancel-before-frame test did not cover this phase. The
  separate persistent→post snapshot order is SC01 in the first report
  (`scheduler/binding.dart:633–637,1259–1273`;
  `crates/incular-animation/src/scheduler.rs:185–193,252–265`,
  `crates/incular-runtime/tests/frame_scheduler.rs:71–90`).
- **A01: zero-duration/equal-target notifications** (source-traced). Starting
  at value 0.5, request animateTo(0.5). Flutter changes status to Completed
  without firing a value listener; Incular can emit an unchanged value
  because status changed. On a different target with zero duration Flutter
  calls the value listener before updating status; Incular has already set
  status. Existing tests assert final value/status, not notification order
  (`animation/animation_controller.dart:668–681`;
  `crates/incular-animation/src/controller.rs:1018–1056,1195–1213`,
  `crates/incular-animation/tests/public_api.rs:52–63`).

## Next verification order

1. Turn the six probes into focused regressions when their fixes are approved.
   Keep the existing ordinary-touch and no-outline controls to prevent undoing
   intentional behavior.
2. Compare paint commands and pixels at DPR 1, 1.25 and 2 (hairline,
   none/solid, clip order, fractional caret, no authored decoration). Inspect
   the center as well as the edges. Do not treat a missing or extra stroke as
   allowable antialiasing variance.
3. Run real device checks for read-only transitions and stale IME events,
   including composition and an accessibility action. Run deterministic
   headless traces just before, at, and just after hit, deadline and cache
   edges with both default and explicitly configured cases.
4. Extend the same contract-level treatment beyond these risk areas. The other
   findings in the first report and the inventory are not a sign-off on unchecked
   APIs. Maintain a record of source, focused runtime, and platform evidence
   separately; a matching name or passing ordinary case proves none of the
   adjacent cases.

No framework implementation, renderer, existing test, or older audit was
changed in this pass. No full-workspace test or benchmark was run.
