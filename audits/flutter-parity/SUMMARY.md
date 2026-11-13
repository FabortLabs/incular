# Flutter parity audit — incomplete

**Clean stop, 2026-11-13:** The audit is incomplete. Gesture and scroll workers stopped unexpectedly during follow-on work; their new scenario files compile but their latest unreported cases require independent review. `scroll-005` was independently rerun and source-verified before the stop. The 21-scene pixel batch completed at four scales, but the requested exhaustive inventory, package coverage, final all-scenario run, and remaining pixel questions are unfinished. The Flutter runner required interruption after writing captures; `--incular-only` completed the comparisons. No framework code was modified. At the clean stop, work remained uncommitted. `cargo check -p incular-parity-harness --tests` passed; `flutter analyze` reported nine informational lint issues. Do not treat the coverage snapshot below as exhaustive or fully current.

Continued against revision `a425aa2e`: the existing pixel scenes were compared at four device pixel ratios; see [pixel-run.md](pixel-run.md). The `focus_input__` frame suite was independently rerun: five scenarios match and the traversal and PageDown scenarios diverge as recorded in that package. The earlier statements below describe the prior stop, not the latest validation. The revision includes rendering changes after the original audit, so earlier paint conclusions require revalidation against current source.

Continuation harness changes: the pixel harness gained image-fit, decoration-image-fit and physical-model circle scenes; the latter is explicitly a non-equivalent missing-API diagnostic. `incular-semantics` was added as a harness-only dev dependency to exercise semantic action dispatch. No framework diagnostic accessors or framework fixes were added in this continuation.

This is a partial audit, not a complete inventory or a parity certification. Flutter 3.41.7 was verified locally. The generated `inventory.md` indexes a subset of declarations in the four primary directories; it omits some declarations and widget-dependent libraries, and its package assignments and exact-name counterpart matches need review. All entries marked not checked remain so because the audit is incomplete. Partial reading does not imply that all behavior of an item was checked.

## Coverage

The declaration index is not complete enough for reliable whole-scope counts. An analyzer AST pass extracted 1,751 public declaration occurrences from animation, physics, scheduler, widgets, rendering, painting, gestures and services without parse diagnostics. Getter/setter pairs are grouped in `inventory.md`; the four latter directories contain candidates whose relevance to widget behavior still needs review. Package assignments and counterpart name matches are provisional, and nearly all declarations remain marked unchecked. The worker coverage tables give finer-grained scenario and reading coverage achieved. The raw extraction and parser are in `declarations.jsonl` and `inventory_tool/`.

The following snapshot counts **candidate index rows**, not fully audited APIs. “Partial or other” includes partial readings and scenarios, so it does not imply complete behavioral coverage. Some candidate declarations may be out of scope and unindexed items may be missing.

| Package | Candidate rows | Marked not checked | Partial or other |
| --- | ---: | ---: | ---: |
| anim_compose | 13 | 1 | 12 |
| anim_core | 25 | 19 | 6 |
| anim_curves | 73 | 0 | 73 |
| focus_input | 205 | 204 | 1 |
| gestures | 182 | 181 | 1 |
| implicit | 19 | 4 | 15 |
| layout | 112 | 112 | 0 |
| misc | 424 | 410 | 14 |
| navigation | 56 | 51 | 5 |
| paint | 189 | 182 | 7 |
| physics | 14 | 0 | 14 |
| scroll | 104 | 100 | 4 |
| semantics | 18 | 13 | 5 |
| slivers | 104 | 99 | 5 |
| switchers | 14 | 6 | 8 |
| text | 169 | 162 | 7 |
| transitions | 27 | 10 | 17 |

## Accepted findings

| ID | Severity | Title | Package | Evidence | Scenario |
| --- | --- | --- | --- | --- | --- |
| anim_compose-001 | P0 | Train hopping callback waits for a getter read | anim_compose | scenario | anim_compose__train_hopping__unobserved_crossing |
| misc-004 | P0 | Form.save skips callbacks when a field is invalid | misc | scenario and source review | misc__form_save_invalid_field |
| misc-007 | P0 | GenericFormField is not connected to retained field state | misc | code reading | — |
| physics-002 | P0 | Negative acceleration marks gravity simulations done immediately | physics | In physics__gravity__negative_acceleration_threshold, both positions are 0 with end=100 at frame 0. Flutter reports done=0 and Incular reports done=1. | physics__gravity__negative_acceleration_threshold |
| scroll-005 | P0 | Programmatic jumps omit start and end notifications | scroll | scenario and source review | scroll__controller__jump_activity_notifications |
| slivers-002 | P0 | Floating headers omit Flutter's default scroll-end snap | slivers | scenario and source review | slivers__floating_header__default_snap_after_wheel_scroll |
| text-006 | P0 | Edit transforms and on_changed run for programmatic controller updates | text | scenario and source review | text__editable_programmatic_controller_update |
| anim_compose-003 | P1 | CurvedAnimation can latch the wrong curve before its first running read | anim_compose | scenario and source review | anim_compose__curved_animation__unobserved_reversal |
| focus_input-001 | P1 | Empty SizedBox claims hits | focus_input | code reading | — |
| focus_input-003 | P1 | PageDown does not scroll a focused list | focus_input | scenario and source review | focus_input__keyboard_page_scroll |
| gestures-001 | P1 | Singleton scroll arena drops the first drag move | gestures | scenario-trace | sample_list_fling |
| gestures-002 | P1 | Double-tap-only detector does not retain the first tap | gestures | scenario-trace | gestures__double_tap |
| gestures-003 | P1 | onLongPress fires on release instead of at the deadline | gestures | scenario-trace | gestures__long_press |
| gestures-004 | P1 | Single-pointer scale starts at different times | gestures | scenario-trace | gestures__scale_single_pointer_pan |
| gestures-005 | P1 | LongPressDraggable maps to immediate Draggable | gestures | source-review-open | — |
| layout-001 | P1 | IntrinsicHeight removes horizontal constraints | layout | scenario | layout__box_constraints__limited_overflow_intrinsic |
| misc-001 | P1 | Expansible adds and removes its body without animation | misc | code reading | — |
| misc-002 | P1 | RotatedBox retains unrotated layout dimensions for odd turns | misc | scenario and source review | misc__rotated_box_layout_size |
| misc-005 | P1 | RawAutocomplete has no interactive options behavior | misc | code reading | — |
| misc-008 | P1 | SafeArea does not consume the insets seen by descendants | misc | scenario and source review | misc__safe_area_nested_consumption |
| navigation-002 | P1 | Imperative stack removal and predicate operations are missing | navigation | code reading | — |
| navigation-005 | P1 | Hero flights do not run on normal route pops | navigation | code reading | — |
| navigation-007 | P1 | OverlayPortal popup pictures are culled by an ancestor clip | navigation | pixel comparison and source review | overlay_portal_ancestor_clip |
| paint-002 | P1 | Image defaults scale small sources up | paint | code reading and pixel comparison | image_default_fit |
| paint-003 | P1 | DecorationImage defaults to cover instead of scale-down | paint | code reading and pixel comparison | decoration_image_default_fit |
| paint-015 | P1 | ColorFilter matrix translations use different units | paint | code reading and pixel comparison | color_filter_matrix_bias |
| semantics-001 | P1 | MergeSemantics drops descendant semantics | semantics | scenario and source review | semantics__merge |
| semantics-002 | P1 | Unspecified enabled state is projected as disabled | semantics | scenario and source review | semantics__enabled_default |
| slivers-001 | P1 | Varied extent children are measured at intrinsic size | slivers | scenario-trace-and-source | slivers__varied_extent_list__indices |
| text-001 | P1 | RichText drops styles after the first flattened run | text | scenario and source review | text__rich_span_style_metrics |
| text-007 | P1 | EditableText long press does not select a word | text | scenario and source review | text__editable_long_press_word_selection |
| anim_core-001 | P2 | Controller value listeners are change-gated | anim_core | frame parity | anim_core__controller_listener_notifications |
| anim_core-002 | P2 | First forward status notification is suppressed for an intermediate initial value | anim_core | frame parity | anim_core__controller_listener_notifications |
| anim_core-003 | P2 | Scheduler-wide time dilation is not implemented | anim_core | missing API demonstrated with a diagnostic frame trace | anim_core__scheduler_callbacks_time_dilation |
| anim_core-004 | P2 | Derived animations cannot register Flutter-style listeners | anim_core | source review | — |
| anim_core-005 | P2 | with_vsync tickers do not inherit TickerMode | anim_core | source review | — |
| anim_compose-002 | P2 | AlwaysStoppedAnimation accepts only scalar values | anim_compose | code reading | — |
| anim_curves-001 | P2 | Size and rectangle tweens clamp extrapolated extents | anim_curves | frame parity | anim_curves__tweens__typed |
| anim_curves-002 | P2 | Tween sequences choose the previous item at an exact boundary | anim_curves | frame parity | anim_curves__tween_sequence__weighted |
| anim_curves-003 | P2 | Flipped tween sequences have no direct Incular equivalent | anim_curves | source review | anim_curves__tween_sequence__weighted |
| anim_curves-004 | P2 | Color, size, and rect tweens do not accept nullable endpoints | anim_curves | source review | — |
| focus_input-002 | P2 | FocusScope traverses widget order by default | focus_input | scenario and source review | focus_input__focus_scope_default_traversal |
| implicit-001 | P2 | Sliver opacity starts one frame late | implicit | @{type=frame parity; first_divergence=; additional_evidence=At frame 5 / 80 ms Incular equals Flutter's frame-4 opacity; both reach 0.25 at frame 26.; impact=The visible opacity trajectory lags Flutter's by one frame during the fade, then converges to the same target.; cause=Retained sliver adoption schedules forward_on_next_tick after the current frame's tick; Flutter starts the controller during didUpdateWidget.; fix_direction=Establish the adopted animation before the first post-update tick so its elapsed timeline matches Flutter.; confidence=high} | implicit__sliver_opacity__opacity |
| implicit-003 | P2 | Implicit layout widgets cannot animate directional geometry | implicit | @{type=source review; impact=Callers cannot preserve start/end alignment or insets in RTL layouts through AnimatedContainer, AnimatedPadding, AnimatedAlign, or AnimatedFractionallySizedBox, and cannot interpolate mixed directional and physical geometry.; cause=Implicit widget setters and tween storage accept concrete Alignment and EdgeInsets values; Incular's separate directional types are not accepted by those APIs.; fix_direction=Accept Flutter-equivalent geometry enums and interpolate them with direction-aware geometry lerps.; confidence=high} | — |
| implicit-004 | P2 | Opacity widgets cannot match Flutter's semantic visibility option | implicit | @{type=source review; impact=At zero opacity, assistive technology may still announce or navigate Incular child content that Flutter hides by default; callers cannot choose Flutter's alwaysIncludeSemantics behavior.; cause=Incular opacity is paint-only for semantic exposure and has no always-include flag.; fix_direction=Add the flag to box and sliver opacity widgets and propagate it into semantics projection, defaulting to Flutter's false behavior.; confidence=high} | — |
| implicit-005 | P2 | AnimatedPhysicalModel has no circular shape option | implicit | @{type=source review; impact=Callers cannot create Flutter's circular physical model or match its clip and shadow silhouette.; cause=The Incular widget models corner radii but has no rectangle/circle shape field.; fix_direction=Add shape to the widget and renderer-neutral physical-model descriptor, then support the matching paint and clip behavior.; confidence=high} | — |
| layout-002 | P2 | OverflowBox lacks Flutter's centered child alignment | layout | scenario | layout__box_constraints__limited_overflow_intrinsic |
| misc-003 | P2 | MediaQuery readers rebuild for unrelated environment changes | misc | scenario and source review | misc__media_query_size_aspect |
| misc-009 | P2 | KeyedSubtree replaces the child key instead of adding an outer key | misc | code reading | — |
| navigation-001 | P2 | Modal route lookup is disabled by default | navigation | code reading | — |
| navigation-003 | P2 | PageRouteBuilder does not create Flutter's default route animations | navigation | code reading | — |
| navigation-004 | P2 | Overlay entries lack per-entry removal and rebuild lifecycle | navigation | code reading | — |
| navigation-006 | P2 | Router back dispatch cannot transfer priority to nested Routers | navigation | code reading | — |
| paint-001 | P2 | BackdropFilter cannot accept general image filters | paint | code reading | — |
| paint-004 | P2 | PhysicalShape drops fill, elevation, shadow and clip behavior | paint | code reading and pixel comparison | physical_shape_triangle |
| paint-005 | P2 | PhysicalModel ignores its shape and clipBehavior | paint | code reading; non-equivalent pixel diagnostic | physical_model_circle |
| paint-006 | P2 | ClipRSuperellipse is rendered as a rounded rectangle | paint | code reading and pixel comparison | clip_r_superellipse |
| semantics-003 | P2 | Scroll actions remain advertised at exhausted edges | semantics | scenario and source review | semantics__scrollable |
| text-002 | P2 | Text editing offsets use UTF-8 bytes instead of UTF-16 code units | text | scenario and source review; API-unit difference | text__editing_selection_offsets |
| text-003 | P2 | WidgetSpan children are replaced by plain text during layout | text | scenario and source review | text__widget_span_layout |
| text-004 | P2 | DefaultTextStyle.maxLines does not constrain descendant Text | text | source review; non-equivalent API diagnostic | text__default_text_style_max_lines |
| text-005 | P2 | Text controllers with initial text begin in different selection states | text | scenario and source review | text__editable_initial_selection_focus |
| text-008 | P2 | Incular paints a caret during non-collapsed selection | text | code reading | — |
| physics-001 | P2 | Spring simulations estimate velocity with a forward difference | physics | In physics__spring__damping_regimes, frame 0 / 0 ms reports dx=30 for all Flutter regimes and 34.8282 (under), 34.6680 (critical), 34.3399 (over) in Incular. The direct scroll spring also reports 0 vs 4.9438 at frame 0. Position remains within tolerance. | physics__spring__damping_regimes; physics__scroll_spring__snap_when_done |
| physics-003 | P2 | Bounded friction changes velocity and omits the bound-distance tolerance | physics | In physics__bounded_friction__position_and_velocity, frame 3 / 48 ms has clamped x=4: Flutter dx=90.8356, Incular dx=0. In physics__bounded_friction__distance_tolerance, start=3.9995 is 0.0005 from max=4 under the 0.001 default distance tolerance; Flutter is done at frame 0, Incular is not, and at frame 1 Flutter dx=96.8468 while Incular dx=0. | physics__bounded_friction__position_and_velocity; physics__bounded_friction__distance_tolerance |
| physics-004 | P2 | Clamped simulations cannot clamp velocity | physics | Source review; the diagnostic twin sets Flutter dxMax=50 while Incular has no equivalent option. Frame 0 reports Flutter 50 vs Incular 200; configurations are intentionally not equivalent. | physics__clamped_simulation__position_and_velocity |
| physics-008 | P2 | SpringDescription duration and bounce getters are missing | physics | Source review. The duration/bounce frame twin reads Flutter's getters and reconstructs the same formulas in test code because the Incular type has no equivalent accessors. | physics__spring_description__duration_bounce |
| physics-009 | P2 | SpringSimulation lacks snapToEnd | physics | Source review. Direct frame twins use Flutter's default snapToEnd=false; Incular has no equivalent configuration for the true case. Its ScrollSpringSimulation snaps position but leaves the inherited velocity unchanged. | — |
| physics-010 | P2 | FrictionSimulation omits Flutter's desktop and endpoint helpers | physics | Source review. The ordinary zero-constant-deceleration curve is frame-tested and matches; Flutter's constantDeceleration, through constructor, finalX, and timeAtX have no public counterpart on Incular's animation simulation. | physics__friction__ordinary |
| physics-011 | P2 | Simulation tolerance is not available through the trait | physics | Source review. Concrete Incular constructors may accept a tolerance, but generic simulation users cannot inspect or update a shared tolerance through the Simulation trait. | — |
| scroll-001 | P2 | Out-of-range jump is clamped instead of springing back | scroll | scenario | scroll__controller__jump_clamp |
| scroll-002 | P2 | PageController page-based navigation API is missing | scroll | code reading | scroll__page_view__fraction_initial_page |
| scroll-003 | P2 | ScrollController cannot configure an initial offset | scroll | code reading | — |
| scroll-004 | P2 | Scrollable.ensureVisible has no Incular equivalent | scroll | code reading | — |
| switchers-001 | P2 | AnimatedCrossFade outgoing child keeps its narrow width | switchers | frame parity | switchers__cross_fade_reversed_different_sizes |
| switchers-002 | P2 | AnimatedSize retarget uses the previous animation tick | switchers | frame parity | switchers__animated_size_retarget |
| switchers-003 | P2 | AnimatedSwitcher transition builder receives a scalar | switchers | code reading | switchers__switcher_overlapping_children |
| switchers-005 | P2 | AnimatedModalBarrier cannot animate to null color | switchers | code reading | switchers__animated_modal_barrier_color_and_dismiss |
| switchers-006 | P2 | SliverAnimatedList builder omits its item animation | switchers | code reading | switchers__sliver_animated_list_initial_layout |
| transitions-001 | P2 | SlideTransition cannot resolve its horizontal offset from text direction | transitions | Flutter negates the horizontal offset for RTL when textDirection is rtl; Incular samples the Offset directly into FractionalTranslation and exposes no direction parameter. The paired scenario uses LTR, where values match. | — |
| transitions-002 | P2 | SlideTransition cannot disable hit-test transforms | transitions | Flutter exposes transformHitTests and passes it to FractionalTranslation; Incular's transition constructs FractionalTranslation with its default true and exposes no setter. | — |
| transitions-003 | P2 | Matrix, scale and rotation transitions omit filterQuality | transitions | The frame scenario transitions geometry and matches without filtering. Flutter's constructors carry filterQuality to Transform while active; the Incular transition and Transform descriptors have no such field. | transitions__paint__decoration_matrix |
| transitions-004 | P2 | RelativePositionedTransition cannot represent a nullable rectangle | transitions | Flutter accepts Animation<Rect?> and maps null to Rect.zero; Incular requires TweenAnimation<Rect>. The non-null twin matches. | transitions__positioned__relative_rect |
| transitions-005 | P2 | DecoratedBoxTransition only accepts BoxDecoration | transitions | Flutter accepts Animation<Decoration>; Incular requires TweenAnimation<BoxDecoration>. The BoxDecoration color interpolation twin matches. | transitions__paint__decoration_matrix |
| transitions-006 | P2 | SliverFadeTransition requires an OpacityController | transitions | Flutter accepts Animation<double>; Incular accepts OpacityController. Direct opacity assignments produce matching traces, but an arbitrary derived animation cannot be supplied. | transitions__sliver_fade__opacity |
| transitions-008 | P2 | SizeTransition clamps negative fixed cross-axis factors instead of asserting | transitions | The valid fixedCrossAxisSizeFactor twin matches. Flutter asserts the factor is non-negative; Incular clamps a negative factor to zero. | transitions__layout__size_clip |
| implicit-002 | P3 | AnimatedDefaultTextStyle has intermittent one-pixel line-box drift | implicit | @{type=frame parity; first_divergence=; additional_evidence=Frame 0 matches. Horizontal widths differ by less than 0.00002 px while top and bottom differ by 0.5 px on five intermediate frames; adjacent frames sometimes match.; impact=A text widget's layout bounds can shift by one logical pixel during a font-size animation and move nearby content.; cause=Tween progress and text width agree; the observed difference is in line-box metrics at fractional font sizes, likely text-engine metric rounding.; fix_direction=Compare Flutter and Incular line metrics for the same Inter font sizes and align fractional-size line-box rounding.; confidence=medium} | implicit__default_text_style__text_style |
| implicit-006 | P3 | AnimatedDefaultTextStyle omits Flutter paragraph options | implicit | @{type=source review; impact=Callers cannot configure inherited paragraph wrapping, truncation, alignment, or line-height behavior through Incular's animated style widget.; cause=Incular's AnimatedDefaultTextStyle stores only TextStyle, child, and timing.; fix_direction=Add Flutter's paragraph options to AnimatedDefaultTextStyle and pass them through to the default-style scope.; confidence=high} | — |
| layout-003 | P3 | Baseline-aligned small text is shifted by 0.625 px | layout | scenario | layout__baseline__cross_axis |
| physics-005 | P3 | Completion comparisons are inclusive at Flutter's strict tolerance boundary | physics | physics__friction__tolerance_boundary uses velocity=tolerance=0.001 and reports done=0 in Flutter vs 1 in Incular at frame 0. physics__spring__tolerance_boundary starts exactly one distance tolerance from equilibrium and reports done=0 vs 1 at frame 0. | physics__friction__tolerance_boundary; physics__spring__tolerance_boundary |
| physics-006 | P3 | Spring descriptions normalize inputs Flutter preserves | physics | physics__spring_description__damping_regimes at frame 0: raw (mass=0, stiffness=0, damping=-1) remains unchanged in Flutter and becomes (1,100,10) in Incular. A damping ratio of -0.25 gives damping=-5/type=under in Flutter and damping=20/type=critical in Incular. | physics__spring_description__damping_regimes |
| physics-007 | P3 | Duration and bounce spring creation retains sub-millisecond precision | physics | physics__spring_description__duration_bounce passes 300500 µs and bounce=0.3. Flutter reports stiffness=438.6491, damping=29.3215, duration=300 ms; Incular computes stiffness=437.1906, damping=29.2727, and duration=301 ms from its spring fields. | physics__spring_description__duration_bounce |
| physics-012 | P3 | nearEqual and nearZero have no public Incular equivalent | physics | Source review; the inventory marked both helpers unresolved. Incular has only a private scroll-local near_zero helper. | — |
| switchers-004 | P3 | AnimatedSize reverseDuration API surface differs | switchers | code reading | switchers__animated_size_retarget |
| transitions-007 | P3 | AnimatedWidget has no direct public base equivalent | transitions | Flutter provides a subclassable listening widget base; Incular provides an internal retained helper and public closure-based AnimatedBuilder/ListenableBuilder instead. The builder twins match, while custom subclass lifecycle is not represented. | transitions__builders__cached_child |
| transitions-009 | P3 | RelativeRectTween has no named public counterpart | transitions | Flutter exposes RelativeRectTween; Incular uses RelativeRect with a generic TweenAnimation. The positioned twin uses this generic replacement. | transitions__positioned__relative_rect |

Other paint, focus/input and semantics findings are pending independent review and are included in the merged JSONL but excluded from this accepted table until verified.

See individual findings for detailed evidence. The `timeDilation` scenario uses non-equivalent settings because the scheduler option is absent; it illustrates a missing API rather than a same-input timing result. The `AnimatedSize.reverseDuration` observation is API-only, not a demonstrated reverse-timing difference.

## Harness and existing tests

`sample_` ran successfully: four scenarios matched, and `sample_list_fling` first differed at frame 3 (48 ms), offset 40 vs 0. The generated report is `harness/out/frames/report-sample_.md`. Flutter closes the arena on pointer down (`gestures/binding.dart:530`) and resolves a sole member by default (`gestures/arena.dart:254`); Incular's arena registration and drag-start code are at `crates/incular-gestures/src/arena.rs:86` and `crates/incular-gestures/src/pointer.rs:643` (gestures-001). The full existing suite ran, generating `harness/out/frames/report.md`. The three populated audit packages were independently rerun: anim_core had two matching and two divergent scenarios (one divergent diagnostic uses non-equivalent scheduler configuration), gestures had two matches and three divergences, and switchers had six matches and two divergences.

The existing `flutter_ballistics.rs` includes carried momentum and bouncing spring hand-off cases. Animation parity tests cover selected curves, fling, repeat counts, directional completion, curved-animation latching, Catmull-Rom, and ticker futures. Gesture timing and recognizer tests cover selected timeouts, slop, multi-tap and force press. These tests are samples, not complete coverage of the corresponding APIs.

Scenario modules were registered for all packages; several packages have verified twins, and additional package work is in progress. A filtered empty package exits with `No tests match`, so those placeholders are not passing package tests. The harness uses `incular-animation` and `incular-scroll` as dev-dependencies. Both recorders now expose a second touch pointer for pinch and rotation scenarios, and Flutter fill recording recognizes PhysicalModel and PhysicalShape to match Incular's solid fill recording. After these recorder changes, `sample_` reran with the same four matches and known list-fling divergence. The independently run two-pointer pinch delivers two contacts in both systems and exposes the same late scale-arena acceptance as gestures-004. No framework changes were made. The existing pixel scenes were later compared; see the continuation note above.

## Themes

- Gesture arena default resolution explains both the first scroll move and singleton scale-start differences. Other gesture callback problems have independent mechanisms.
- Several wrappers reduce Flutter's animation or rich-text objects to sampled scalar values or a single style, limiting custom composition.
- Retained layout and child observation produce discrepancies when animated children change size or occupy differently constrained slots.

## Known-gap status

| Claim | Status |
| --- | --- |
| AnimatedSwitcher transition builder receives f32 | Confirmed: switchers-003. |
| Vsync controller requires manual TickerMode muting; implicit animations follow automatically | Confirmed by source review: anim_core-005; retained animation ticker mode handling is at `crates/incular-widgets/src/tree/interaction.rs:2146`. |
| OverlayPortal is clipped by ancestors | Confirmed for the tested ClipRect scene by pixel comparison: Flutter paints the 40×40 popup outside the clip, while Incular does not. The corresponding no-clip control matches at 1x. The precise source mechanism and other ancestor effects remain under review. See `pixel-run.md`. |
| RichText uses first span style | Confirmed: text-001. |
| ModalRoute lookup requires outlet opt-in | Confirmed for default RouteOutlet: navigation-001. |
| Childless SizedBox is a hit-test leaf | Confirmed: focus_input-001. |
| BackdropFilter is blur only | Refuted: matrix constructor at `crates/incular-widgets/src/compositing.rs:171`; general filters remain missing (paint-001). |
| Bouncing has no spring hand-off or momentum carry/hold | Refuted as a missing-implementation claim: hand-off in `crates/incular-scroll/src/ballistic.rs:333`, carry in `crates/incular-scroll/src/physics.rs:582`, and hold in `crates/incular-widgets/src/tree/interaction.rs:785`; numerical parity beyond existing tests was not audited. |
| ExpansionTile and AnimatedIcon are out of scope, Expansible in scope | Scope confirmed; Expansible has an immediate expansion divergence (misc-001). |
| Sample list drag loses one move | Confirmed by baseline and source review: gestures-001. |

## Validation

The audit was stopped on request. All workers were terminal when stopped; their final turns for text, semantics and focus/input ended with a workspace-credit error. At that stop, paint and focus/input findings remained pending orchestrator review, and the pixel batch had not been run. At that stop, `cargo test -p incular-parity-harness --test frames --no-run` and `flutter analyze` passed, the harness package was formatted, and the work was uncommitted.

Earlier in the audit, `cargo check --workspace`, `cargo test-constrained`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt -p incular-parity-harness -- --check`, and `flutter analyze` passed. After the latest edits, `cargo fmt -p incular-parity-harness` and `cargo check --workspace` passed. The implicit filtered suite was independently rerun and showed 15 matches and two divergences. The physics suite was independently rerun: four scenarios match and ten report differences; the missing velocity-clamp option is documented as a non-equivalent diagnostic configuration. The gravity completion P0 was checked against both source implementations. The multi-touch gesture and composition scenarios were independently rerun. Most package-specific runs, pixel comparison, complete inventory and exhaustive review remain incomplete. `cargo fmt --all -- --check` was not used because the supplied environment notes report a path-length problem; the modified Rust package was formatted separately.
