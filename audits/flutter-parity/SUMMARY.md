# Flutter parity audit — widgets and animations

Reference: Flutter 3.41.7 (`C:/Manual Installs/flutter`). Incular revision: HEAD `a19bf487` plus the uncommitted audit and harness changes listed below. Status on 2026-11-13: **every in-scope inventory item is accounted for**: 1,743 of 1,753 items are checked or classified, and 10 remain unchecked, each blocked on a named harness capability. Accepted findings were independently verified by orchestrator reruns or by a separate source verifier. With the user's permission, the complete all-scenario frame run was also performed (see Validation). This is an audit; no framework behavior was changed.

How to read this folder:

| File | Contents |
| --- | --- |
| `inventory.md` | One row per Flutter public declaration: Incular counterpart, status, what was checked, linked findings. Generated from `coverage/*.jsonl` by `inventory_tool/render-inventory.ps1`. |
| `findings/<package>.md` / `.jsonl` | Each package's coverage notes, scenarios and full finding write-ups (Markdown) and machine-readable findings (shared schema in `WORKER-BRIEF.md`). |
| `findings.jsonl` | All findings merged by `consolidate.ps1`, sorted by severity. |
| `VERIFICATION.md` | The orchestrator's independent checks behind every accepted or rejected finding and every recorder fix. |
| `verification-source-findings.md` | Independent verifier's verdicts on the 44 source-only findings. |
| `decisions.json` | Status and severity decisions, applied by `inventory_tool/apply-decisions.ps1`. |
| `pixel-run.md` | Pixel comparisons (24 scenes at four device pixel ratios). |
| `CHECKLIST.md` | Completion checklist and resumption notes. |

## Coverage

Inventory: 1,753 public declarations (classes, mixins, enums, extension types, top-level functions and constants, `Curves` members) AST-extracted from `animation/`, `physics/`, `scheduler/`, `widgets/`, plus `rendering/`, `painting/`, `gestures/` and `services/` scoped to widget-observable behavior. Four `widgets/` files contribute no declarations (`_html_element_view_io.dart`, `_html_element_view_web.dart`, `platform_selectable_region_context_menu.dart`, `date.dart`: private or web/platform-only).

| Result | Items |
| --- | ---: |
| Checked by a paired scenario (frame or pixel): `scenario` 143, `scenario+reading` 70, `partial-scenario` 200 | 413 |
| Checked by reading both sources only: `reading` 185, `partial-reading` 395 | 580 |
| No Incular counterpart after a search (`missing`; covered by findings where observable) | 202 |
| Out of scope with a stated reason (Material/Cupertino-only, embedder or platform-channel plumbing, debug/inspector tooling, abstract protocol types without own behavior) | 548 |
| Unchecked, each blocked on a named harness capability | 10 |

`partial-*` means only the aspects named in the row were compared. Out-of-scope items by library: services 140, rendering 83, painting 49, gestures 33, scheduler 9, widgets 234 (largely `_window*`, inspector, debug, platform view and binding internals).

The 10 unchecked items: `KeyboardLockMode`, `KeyRepeatEvent` (no lock-key or key-repeat input), `PointerPanZoomStart/Update/EndEvent`, `PointerScaleEvent` (no trackpad input), `BackButtonListener` (needs a Router-hosted back-dispatch twin), `PlatformRouteInformationProvider` (needs a browser-history harness), `DecoratedSliver`, `RenderDecoratedSliver` (need a sliver pixel scene).

Scenarios: 246 paired frame scenarios across 19 package files, plus 24 pixel scenes. Each package's filtered suite completes without harness errors; per-package match counts are in Validation.

## Findings

187 findings recorded: 181 accepted, 6 rejected (focus_input-001 no longer reproduces; scroll-006, semantics-016 and text-018 are duplicates; misc-014 is intentional design; text-017 is an unverified twin). Accepted by severity: **P0 7, P1 37, P2 109, P3 28** (after verification corrections). Accepted by impact: missing feature 78, edge case 30, wrong final state 19, minor numeric/API 19, layout 13, visible motion 12, lost or duplicated callbacks 10.

P0 findings (each reproduced by the orchestrator):

| ID | Finding | Evidence |
| --- | --- | --- |
| [gestures-006](findings/gestures.md) | ReorderableDelayedDragStartListener never starts a reorder when the long-press deadline fires on a frame | `gestures__reorderable_list_long_press`: Flutter starts at 616 ms and drops at index 2; Incular never starts |
| [text-006](findings/text.md) | Programmatic controller updates run edit transforms and `on_changed` | `text__editable_programmatic_controller_update`: `changed_calls` 0 vs 1, text rewritten |
| [scroll-005](findings/scroll.md) | Programmatic jumps omit scroll start/end notifications | `scroll__controller__jump_activity_notifications` |
| [slivers-002](findings/slivers.md) | Floating headers omit Flutter's default scroll-end snap | `slivers__floating_header__default_snap_after_wheel_scroll` |
| [misc-004](findings/misc.md) | `Form.save` skips callbacks when a field is invalid | `misc__form_save_invalid_field` |
| [physics-002](findings/physics.md) | Negative-acceleration gravity simulations report done immediately | `physics__gravity__negative_acceleration_threshold`: done 0 vs 1 at frame 0 |
| [anim_compose-001](findings/anim_compose.md) | TrainHoppingAnimation's hop callback waits for a value read | `anim_compose__train_hopping__unobserved_crossing` |

The complete deduplicated list, ordered by severity, is generated below from the findings JSONL. Rejected findings stay listed with the reason (duplicates, intentional design, or unverified twins).

<!-- findings:begin -->
### Accepted (independently verified) (181)

| ID | Sev | Impact | Title | Evidence | Scenario |
| --- | --- | --- | --- | --- | --- |
| [anim_compose-001](findings/anim_compose.md) | P0 | lost-or-duplicate-callback | Train hopping callback waits for a getter read | scenario+source | anim_compose__train_hopping__unobserved_crossing |
| [gestures-006](findings/gestures.md) | P0 | wrong-final-state | ReorderableDelayedDragStartListener never starts a reorder when the long-press deadline fires on a frame tick | scenario+source | gestures__reorderable_list_long_press |
| [misc-004](findings/misc.md) | P0 | wrong-final-state | Form.save skips callbacks when a field is invalid | scenario+source | misc__form_save_invalid_field |
| [physics-002](findings/physics.md) | P0 | wrong-final-state | Negative acceleration marks gravity simulations done immediately | scenario+source | physics__gravity__negative_acceleration_threshold |
| [scroll-005](findings/scroll.md) | P0 | lost-or-duplicate-callback | Programmatic jumps omit start and end notifications | scenario+source | scroll__controller__jump_activity_notifications |
| [slivers-002](findings/slivers.md) | P0 | wrong-final-state | Floating headers omit Flutter's default scroll-end snap | scenario+source | slivers__floating_header__default_snap_after_wheel_scroll |
| [text-006](findings/text.md) | P0 | wrong-final-state | Edit transforms and on_changed run for programmatic controller updates | scenario+source | text__editable_programmatic_controller_update |
| [anim_compose-003](findings/anim_compose.md) | P1 | visible-motion | CurvedAnimation can latch the wrong curve before its first running read | scenario+source | anim_compose__curved_animation__unobserved_reversal |
| [focus_input-003](findings/focus_input.md) | P1 | wrong-final-state | PageDown does not scroll a focused list | scenario+source | focus_input__keyboard_page_scroll |
| [focus_input-004](findings/focus_input.md) | P1 | missing-feature | Arrow keys do not move focus directionally | scenario+source | focus_input__arrow_directional_focus |
| [gestures-001](findings/gestures.md) | P1 | wrong-final-state | Singleton arena never resolves on pointer-down; first slop-crossing move is dropped under DragStartBehavior.start | scenario+source | sample_list_fling |
| [gestures-002](findings/gestures.md) | P1 | lost-or-duplicate-callback | Double-tap-only GestureDetector never fires onDoubleTap | scenario+source | gestures__double_tap |
| [gestures-003](findings/gestures.md) | P1 | lost-or-duplicate-callback | onLongPress fires on release instead of at the 500 ms deadline | scenario+source | gestures__long_press |
| [gestures-004](findings/gestures.md) | P1 | wrong-final-state | Scale start and pinch values wait for the arena, so single-pointer start is late and two-pointer span is measured late | scenario+source | gestures__scale_single_pointer_pan |
| [gestures-005](findings/gestures.md) | P1 | wrong-final-state | LongPressDraggable converts to an immediate Draggable and ignores the long-press delay | scenario+source | gestures__long_press_draggable_quick_move |
| [gestures-007](findings/gestures.md) | P1 | missing-feature | GestureDetector has no secondary or tertiary tap or long-press callbacks | source | — |
| [gestures-009](findings/gestures.md) | P1 | missing-feature | Draggable feedback is never painted by the framework | source | gestures__draggable_drop_target |
| [layout-001](findings/layout.md) | P1 | layout | IntrinsicHeight removes horizontal constraints | scenario+source | layout__box_constraints__limited_overflow_intrinsic |
| [layout-005](findings/layout.md) | P1 | missing-feature | Table has no column width or vertical alignment API; default columns differ from Flutter | diagnostic | layout__table__default_flex_columns |
| [misc-001](findings/misc.md) | P1 | visible-motion | Expansible adds and removes its body without animation | scenario+source | misc__expansible_height_animation |
| [misc-002](findings/misc.md) | P1 | layout | RotatedBox retains unrotated layout dimensions for odd turns | scenario+source | misc__rotated_box_layout_size |
| [misc-007](findings/misc.md) | P1 | missing-feature | GenericFormField is not connected to retained field state | source | — |
| [misc-008](findings/misc.md) | P1 | layout | SafeArea does not consume the insets seen by descendants | scenario+source | misc__safe_area_nested_consumption |
| [navigation-002](findings/navigation.md) | P1 | missing-feature | Imperative stack removal and predicate operations are missing | source | — |
| [navigation-005](findings/navigation.md) | P1 | visible-motion | Hero flights do not run on normal route pops | scenario+source | navigation__hero__pop_flight_navigator |
| [navigation-007](findings/navigation.md) | P1 | wrong-final-state | OverlayPortal popup pictures are culled by an ancestor clip | pixel+source | overlay_portal_ancestor_clip (pixel); navigation__overlay_portal__content_outside_anchor_clip (frame geometry) |
| [navigation-010](findings/navigation.md) | P1 | visible-motion | Covered route is hidden from the first frame of a fade push | scenario+source | navigation__navigator__fade_push_timeline |
| [paint-002](findings/paint.md) | P1 | wrong-final-state | Image defaults scale small sources up | pixel+source | image_default_fit |
| [paint-003](findings/paint.md) | P1 | wrong-final-state | DecorationImage defaults to cover instead of scale-down | pixel+source | decoration_image_default_fit |
| [paint-015](findings/paint.md) | P1 | api-numeric | ColorFilter matrix translations use different units | pixel+source | color_filter_matrix_bias |
| [paint-022](findings/paint.md) | P1 | missing-feature | ShaderMask makes the WGPU frame fail instead of painting | source | — |
| [scroll-012](findings/scroll.md) | P1 | wrong-final-state | NestedScrollView does not scroll under a touch drag in the harness | scenario | scroll__nested_scroll_view__drag_offsets |
| [scroll-013](findings/scroll.md) | P1 | wrong-final-state | DraggableScrollableSheet does not expand under a drag from its list | scenario | scroll__draggable_sheet__drag_extent |
| [semantics-001](findings/semantics.md) | P1 | wrong-final-state | MergeSemantics drops descendant semantics | scenario+source | semantics__merge |
| [semantics-002](findings/semantics.md) | P1 | wrong-final-state | Unspecified enabled state is projected as disabled | scenario+source | semantics__enabled_default |
| [semantics-004](findings/semantics.md) | P1 | missing-feature | Label-only Semantics without a role or action is dropped | scenario+source | semantics__container_label_only |
| [semantics-007](findings/semantics.md) | P1 | missing-feature | GestureDetector onTap is not exposed as a semantic tap action | scenario+source | semantics__gesture_tap |
| [slivers-001](findings/slivers.md) | P1 | layout | Varied extent children are measured at intrinsic size | scenario+source | slivers__varied_extent_list__indices |
| [slivers-003](findings/slivers.md) | P1 | wrong-final-state | SliverFillRemaining without a scroll body keeps its largest past extent | scenario+source | slivers__fill_remaining_no_scroll_body__shrink_after_preceding_growth |
| [slivers-004](findings/slivers.md) | P1 | missing-feature | KeepAlive does not retain a scrolled-away child | scenario+source | slivers__keep_alive__retains_scrolled_child |
| [text-001](findings/text.md) | P1 | layout | RichText drops styles after the first flattened run | scenario+source | text__rich_span_style_metrics |
| [text-007](findings/text.md) | P1 | missing-feature | EditableText long press does not select a word | scenario+source | text__editable_long_press_word_selection |
| [text-009](findings/text.md) | P1 | missing-feature | EditableText never applies input formatters | scenario+source | text__editable_formatter_length_then_digits |
| [text-014](findings/text.md) | P1 | missing-feature | SelectionArea does not select on long press and has no selection-change callback | scenario+source | text__selection_area_long_press_drag |
| [anim_compose-002](findings/anim_compose.md) | P2 | missing-feature | AlwaysStoppedAnimation accepts only scalar values | source | — |
| [anim_core-001](findings/anim_core.md) | P2 | lost-or-duplicate-callback | Controller value listeners are change-gated | scenario+source | anim_core__controller_listener_notifications |
| [anim_core-002](findings/anim_core.md) | P2 | lost-or-duplicate-callback | First forward status notification is suppressed for an intermediate initial value | scenario+source | anim_core__controller_listener_notifications |
| [anim_core-003](findings/anim_core.md) | P2 | missing-feature | Scheduler-wide time dilation is not implemented | diagnostic | anim_core__scheduler_callbacks_time_dilation |
| [anim_core-004](findings/anim_core.md) | P2 | missing-feature | Derived animations cannot register Flutter-style listeners | source | — |
| [anim_core-005](findings/anim_core.md) | P2 | missing-feature | with_vsync tickers do not inherit TickerMode | source | — |
| [anim_curves-001](findings/anim_curves.md) | P2 | edge-case | Size and rectangle tweens clamp extrapolated extents | scenario | anim_curves__tweens__typed |
| [anim_curves-002](findings/anim_curves.md) | P2 | edge-case | Tween sequences choose the previous item at an exact boundary | scenario | anim_curves__tween_sequence__weighted |
| [anim_curves-003](findings/anim_curves.md) | P2 | missing-feature | Flipped tween sequences have no direct Incular equivalent | source | — |
| [anim_curves-004](findings/anim_curves.md) | P2 | missing-feature | Color, size, and rect tweens do not accept nullable endpoints | source | — |
| [focus_input-002](findings/focus_input.md) | P2 | edge-case | FocusScope traverses widget order by default | scenario+source | focus_input__focus_scope_default_traversal |
| [focus_input-005](findings/focus_input.md) | P2 | edge-case | Reading order groups rows by height tolerance, not band overlap | scenario+source | focus_input__reading_order_band_overlap |
| [focus_input-006](findings/focus_input.md) | P2 | edge-case | Ordered traversal ties use registration order, not reading order | scenario+source | focus_input__ordered_unordered_reading_order |
| [focus_input-007](findings/focus_input.md) | P2 | lost-or-duplicate-callback | Nested MouseRegion enter callbacks fire inner first | scenario+source | focus_input__mouse_region_nested_enter_order |
| [gestures-008](findings/gestures.md) | P2 | missing-feature | GestureDetector lacks onTapMove, onLongPressDown, onLongPressCancel and force-press callbacks; the force-press recognizer is unwired | source | — |
| [gestures-010](findings/gestures.md) | P2 | lost-or-duplicate-callback | DragTarget that rejects a candidate gets no onLeave or onMove, and nested targets do not fall through | source | — |
| [gestures-012](findings/gestures.md) | P2 | visible-motion | Dismissible fling slides out over a fixed movement duration instead of Flutter fling simulation | source | — |
| [implicit-001](findings/implicit.md) | P2 | visible-motion | Sliver opacity starts one frame late | scenario+source | implicit__sliver_opacity__opacity |
| [implicit-003](findings/implicit.md) | P2 | api-numeric | Implicit layout widgets cannot animate directional geometry | source | — |
| [implicit-004](findings/implicit.md) | P2 | edge-case | Opacity widgets cannot match Flutter's semantic visibility option | source | — |
| [implicit-005](findings/implicit.md) | P2 | edge-case | AnimatedPhysicalModel has no circular shape option | source | — |
| [implicit-007](findings/implicit.md) | P2 | visible-motion | Curve-only update mid-flight is ignored until the next target change | scenario+source | implicit__curve_only_update_mid_flight |
| [layout-002](findings/layout.md) | P2 | layout | OverflowBox lacks Flutter's centered child alignment | scenario+source | layout__box_constraints__limited_overflow_intrinsic |
| [layout-004](findings/layout.md) | P2 | layout | UnconstrainedBox ignores alignment; overflowing children stay at top-left | scenario+source | layout__unconstrained_box__constrained_axis |
| [layout-007](findings/layout.md) | P2 | layout | FractionallySizedBox drops the parent minimum on an axis without a factor | scenario+source | layout__fractionally_sized_box__null_factor_tight_parent |
| [misc-003](findings/misc.md) | P2 | edge-case | MediaQuery readers rebuild for unrelated environment changes | scenario+source | misc__media_query_size_aspect |
| [misc-005](findings/misc.md) | P2 | missing-feature | RawAutocomplete has no interactive options behavior | source | — |
| [misc-009](findings/misc.md) | P2 | edge-case | KeyedSubtree replaces the child key instead of adding an outer key | source | — |
| [misc-011](findings/misc.md) | P2 | missing-feature | GlobalKey reparenting (state-preserving move across parents) is missing | source | — |
| [misc-013](findings/misc.md) | P2 | missing-feature | Placeholder widget is not publicly exported | source | misc__placeholder_fallback_size |
| [navigation-001](findings/navigation.md) | P2 | missing-feature | Modal route lookup is disabled by default | source | — |
| [navigation-003](findings/navigation.md) | P2 | visible-motion | PageRouteBuilder does not create Flutter's default route animations | scenario+source | navigation__navigator__fade_pop_timeline (push timing: navigation__navigator__fade_push_timeline, matches within a 1.3 ms harness residual) |
| [navigation-004](findings/navigation.md) | P2 | missing-feature | Overlay entries lack per-entry removal and rebuild lifecycle | source | — |
| [navigation-006](findings/navigation.md) | P2 | missing-feature | Router back dispatch cannot transfer priority to nested Routers | source | — |
| [navigation-008](findings/navigation.md) | P2 | missing-feature | RouteOutlet rejects routes that present overlay entries | source | — |
| [navigation-011](findings/navigation.md) | P2 | missing-feature | Navigator has no TransitionDelegate for declarative page changes | source | — |
| [paint-001](findings/paint.md) | P2 | missing-feature | BackdropFilter cannot accept general image filters | source | — |
| [paint-004](findings/paint.md) | P2 | wrong-final-state | PhysicalShape drops fill, elevation, shadow and clip behavior | pixel+source | physical_shape_triangle |
| [paint-005](findings/paint.md) | P2 | missing-feature | PhysicalModel ignores its shape and clipBehavior | diagnostic | physical_model_circle |
| [paint-006](findings/paint.md) | P2 | edge-case | ClipRSuperellipse is rendered as a rounded rectangle | pixel+source | clip_r_superellipse |
| [paint-007](findings/paint.md) | P2 | missing-feature | ImageFiltered supports blur only | source | — |
| [paint-008](findings/paint.md) | P2 | missing-feature | ColorFiltered accepts matrix filters only | source | — |
| [paint-009](findings/paint.md) | P2 | missing-feature | CustomPaint cannot layer foregroundPainter and child | source | — |
| [paint-010](findings/paint.md) | P2 | missing-feature | Clip widgets do not accept CustomClipper delegates | source | — |
| [paint-011](findings/paint.md) | P2 | missing-feature | ShapeDecoration cannot paint a DecorationImage | source | — |
| [paint-012](findings/paint.md) | P2 | missing-feature | Image lacks Flutter's image-stream lifecycle options | source | — |
| [paint-013](findings/paint.md) | P2 | missing-feature | RawImage exposes fewer paint options than Flutter | source | — |
| [paint-014](findings/paint.md) | P2 | missing-feature | FadeInImage omits several paint and load-error options | source | — |
| [paint-016](findings/paint.md) | P2 | missing-feature | Accepted paint parameters are stored but never painted | source | — |
| [paint-017](findings/paint.md) | P2 | missing-feature | Shape border family lacks CircleBorder, OvalBorder, BeveledRectangleBorder, ContinuousRectangleBorder and LinearBorder | source | — |
| [paint-018](findings/paint.md) | P2 | missing-feature | Gradients have no transform, rotation or focal parameters | source | — |
| [paint-019](findings/paint.md) | P2 | missing-feature | AssetImage ignores device-pixel-ratio and locale variants | source | — |
| [paint-020](findings/paint.md) | P2 | edge-case | EdgeInsets, Alignment and BorderSide lerps clamp the interpolation factor | scenario+source | paint__data__edge_insets_lerp_extrapolated |
| [physics-001](findings/physics.md) | P2 | api-numeric | Spring simulations estimate velocity with a forward difference | scenario+source | physics__spring__damping_regimes |
| [physics-003](findings/physics.md) | P2 | visible-motion | Bounded friction changes velocity and omits the bound-distance tolerance | scenario+source | physics__bounded_friction__position_and_velocity |
| [physics-004](findings/physics.md) | P2 | missing-feature | Clamped simulations cannot clamp velocity | diagnostic | physics__clamped_simulation__position_and_velocity |
| [physics-008](findings/physics.md) | P2 | missing-feature | Spring description duration and bounce getters are missing | source | — |
| [physics-009](findings/physics.md) | P2 | missing-feature | SpringSimulation lacks the snapToEnd option | source | — |
| [physics-010](findings/physics.md) | P2 | missing-feature | FrictionSimulation omits Flutter's desktop and endpoint helpers | source | — |
| [physics-011](findings/physics.md) | P2 | missing-feature | Simulation tolerance cannot be read or changed through the trait | source | — |
| [scroll-001](findings/scroll.md) | P2 | visible-motion | Out-of-range jump is clamped instead of springing back | scenario | scroll__controller__jump_clamp |
| [scroll-002](findings/scroll.md) | P2 | missing-feature | PageController page-based navigation API is missing | source | — |
| [scroll-003](findings/scroll.md) | P2 | missing-feature | ScrollController cannot configure an initial offset | source | — |
| [scroll-004](findings/scroll.md) | P2 | missing-feature | Scrollable.ensureVisible has no Incular equivalent | source | — |
| [scroll-007](findings/scroll.md) | P2 | missing-feature | PageView has no onPageChanged callback (controller listener workaround exists) | scenario+source | scroll__page_view__fling_page_changed |
| [scroll-009](findings/scroll.md) | P2 | lost-or-duplicate-callback | ListWheel reports the initial selected item through onSelectedItemChanged | scenario+source | scroll__list_wheel__selected_fling |
| [scroll-010](findings/scroll.md) | P2 | layout | ListWheel children narrower than the viewport are left-aligned instead of centred | scenario | scroll__list_wheel__selected_fling |
| [scroll-011](findings/scroll.md) | P2 | edge-case | Bouncing drag past the edge reports OverscrollNotification | scenario+source | scroll__drag_overscroll__bouncing |
| [semantics-003](findings/semantics.md) | P2 | edge-case | Scroll actions remain advertised at exhausted edges | scenario+source | semantics__scrollable |
| [semantics-005](findings/semantics.md) | P2 | edge-case | onTap and onIncrease imply Button and Slider roles | scenario+source | semantics__container_label_only |
| [semantics-006](findings/semantics.md) | P2 | wrong-final-state | Hint and tooltip share one description field, and state.tooltip is never set | scenario+source | semantics__value_hint_tooltip |
| [semantics-008](findings/semantics.md) | P2 | missing-feature | No long-press semantic action | source | — |
| [semantics-009](findings/semantics.md) | P2 | missing-feature | increasedValue and decreasedValue are not supported | source | — |
| [semantics-010](findings/semantics.md) | P2 | missing-feature | No semantic sort key or ordinal traversal order | source | — |
| [semantics-011](findings/semantics.md) | P2 | missing-feature | showOnScreen action is not available | source | — |
| [semantics-012](findings/semantics.md) | P2 | missing-feature | container and explicitChildNodes have no Incular equivalent | source | — |
| [semantics-013](findings/semantics.md) | P2 | missing-feature | CustomPaint has no per-painted-region semantics | source | — |
| [semantics-014](findings/semantics.md) | P2 | missing-feature | TextSpan has no semanticsLabel | source | — |
| [slivers-005](findings/slivers.md) | P2 | edge-case | Resizing header reports pinned-header cache extent, building one trailing row too few | scenario+source | slivers__resizing_header__collapse_scroll |
| [slivers-007](findings/slivers.md) | P2 | missing-feature | SliverFillRemaining has no fillOverscroll option | source | — |
| [slivers-008](findings/slivers.md) | P2 | missing-feature | SliverFloatingHeader has no snapMode or snap configuration | source | — |
| [slivers-009](findings/slivers.md) | P2 | missing-feature | SliverPersistentHeader has no delegate with variable extents and shrinkOffset | source | — |
| [slivers-010](findings/slivers.md) | P2 | missing-feature | No public Viewport or ShrinkWrappingViewport with anchor or center | source | — |
| [slivers-011](findings/slivers.md) | P2 | missing-feature | SliverVisibility has no replacement sliver or maintain flags | source | — |
| [support-001](findings/support.md) | P2 | missing-feature | Platform views cannot be embedded in the widget tree | source | — |
| [support-002](findings/support.md) | P2 | missing-feature | Predictive back progress is not delivered to routes | source | — |
| [support-003](findings/support.md) | P2 | missing-feature | Text fields have no platform undo manager integration | source | — |
| [support-004](findings/support.md) | P2 | missing-feature | Spell check service is not integrated with text fields | source | — |
| [switchers-001](findings/switchers.md) | P2 | layout | AnimatedCrossFade outer size adopts a child size change one frame late | scenario+source | switchers__cross_fade_reversed_different_sizes |
| [switchers-002](findings/switchers.md) | P2 | visible-motion | AnimatedSize retarget uses the previous animation tick | scenario+source | switchers__animated_size_retarget |
| [switchers-003](findings/switchers.md) | P2 | api-numeric | AnimatedSwitcher transition builder receives a scalar | source | switchers__switcher_overlapping_children |
| [switchers-005](findings/switchers.md) | P2 | edge-case | AnimatedModalBarrier cannot animate to null color | scenario+source | switchers__animated_modal_barrier_color_and_dismiss |
| [switchers-006](findings/switchers.md) | P2 | api-numeric | SliverAnimatedList builder omits its item animation | source | switchers__sliver_animated_list_initial_layout |
| [text-002](findings/text.md) | P2 | api-numeric | Text editing offsets are API-incompatible with Flutter's UTF-16 units | scenario+source | text__editing_selection_offsets |
| [text-003](findings/text.md) | P2 | layout | WidgetSpan children are replaced by plain text during layout | scenario+source | text__widget_span_layout |
| [text-004](findings/text.md) | P2 | edge-case | DefaultTextStyle has no inherited maxLines policy | diagnostic | text__default_text_style_max_lines |
| [text-005](findings/text.md) | P2 | edge-case | Text controllers with initial text begin in different selection states | scenario+source | text__editable_initial_selection_focus |
| [text-008](findings/text.md) | P2 | edge-case | Incular paints a caret during non-collapsed selection | source | — |
| [text-010](findings/text.md) | P2 | layout | EditableText reports one extra line of height for maxLines and minLines | scenario | text__editable_max_lines_sizing |
| [text-012](findings/text.md) | P2 | missing-feature | Text has no textScaler parameter | diagnostic | text__text_scaler_linear |
| [text-013](findings/text.md) | P2 | missing-feature | Text has no strutStyle parameter | source | — |
| [text-015](findings/text.md) | P2 | lost-or-duplicate-callback | EditableText has no onEditingComplete callback | scenario+source | text__editable_enter_callback_order |
| [transitions-001](findings/transitions.md) | P2 | api-numeric | SlideTransition cannot resolve its horizontal offset from text direction | source | — |
| [transitions-002](findings/transitions.md) | P2 | edge-case | SlideTransition cannot disable hit-test transforms | source | — |
| [transitions-003](findings/transitions.md) | P2 | edge-case | Matrix, scale and rotation transitions omit filterQuality | scenario+source | transitions__paint__decoration_matrix |
| [transitions-004](findings/transitions.md) | P2 | api-numeric | RelativePositionedTransition cannot represent a nullable rectangle | scenario+source | transitions__positioned__relative_rect |
| [transitions-005](findings/transitions.md) | P2 | api-numeric | DecoratedBoxTransition only accepts BoxDecoration | scenario+source | transitions__paint__decoration_matrix |
| [transitions-006](findings/transitions.md) | P2 | api-numeric | SliverFadeTransition requires an OpacityController | scenario+source | transitions__sliver_fade__opacity |
| [transitions-008](findings/transitions.md) | P2 | edge-case | SizeTransition clamps negative fixed cross-axis factors instead of asserting | scenario+source | transitions__layout__size_clip |
| [gestures-011](findings/gestures.md) | P3 | missing-feature | DragTarget has no builder with candidate and rejected data | source | — |
| [gestures-013](findings/gestures.md) | P3 | missing-feature | InteractiveViewer lacks trackpadScrollCausesScale; DeviceGestureSettings slop overrides absent | source | — |
| [gestures-014](findings/gestures.md) | P3 | edge-case | DiagonalDragBehavior lacks Flutter's weightedEvent/weightedContinuous distinction | source | — |
| [implicit-002](findings/implicit.md) | P3 | layout | AnimatedDefaultTextStyle has intermittent one-pixel line-box drift | scenario+source | implicit__default_text_style__text_style |
| [implicit-006](findings/implicit.md) | P3 | api-numeric | AnimatedDefaultTextStyle omits Flutter paragraph options | source | — |
| [layout-003](findings/layout.md) | P3 | edge-case | Baseline-aligned small text is shifted by 0.625 px | scenario | layout__baseline__cross_axis |
| [layout-006](findings/layout.md) | P3 | missing-feature | No public BoxConstraints.enforce helper | scenario | layout__box_constraints__helper_values |
| [misc-006](findings/misc.md) | P3 | missing-feature | InheritedWidget has no public custom notification counterpart | source | — |
| [misc-010](findings/misc.md) | P3 | missing-feature | CustomPaint cannot request a fill-to-constraints size (Flutter Size.infinite) | source | — |
| [misc-012](findings/misc.md) | P3 | missing-feature | Restorable property types for num, DateTime, enum, Listenable and ChangeNotifier are missing | source | — |
| [navigation-009](findings/navigation.md) | P3 | edge-case | OverlayPortal replaces the caller key on its anchor child | scenario+source | navigation__overlay_portal__direct_anchor |
| [paint-021](findings/paint.md) | P3 | edge-case | HsvColor lerp wraps hue 360 to 0 before interpolating | scenario | paint__data__hsv_lerp_hue_wrap |
| [physics-005](findings/physics.md) | P3 | edge-case | Completion is inclusive at Flutter's strict tolerance boundary | scenario | physics__friction__tolerance_boundary |
| [physics-006](findings/physics.md) | P3 | api-numeric | Spring descriptions normalize inputs Flutter preserves | scenario | physics__spring_description__damping_regimes |
| [physics-007](findings/physics.md) | P3 | api-numeric | Duration and bounce spring creation retains sub-millisecond precision | scenario | physics__spring_description__duration_bounce |
| [physics-012](findings/physics.md) | P3 | api-numeric | nearEqual and nearZero have no public Incular equivalent | source | — |
| [scroll-008](findings/scroll.md) | P3 | missing-feature | No ScrollAwareImageProvider equivalent to defer image loads during scrolling | source | — |
| [semantics-015](findings/semantics.md) | P3 | edge-case | SetText is advertised on an unfocused text field | scenario+source | semantics__set_text |
| [slivers-006](findings/slivers.md) | P3 | edge-case | Children whose edge exactly meets the cache boundary are built on the opposite side | scenario | slivers__list_builder__default_cache_window |
| [slivers-012](findings/slivers.md) | P3 | missing-feature | Viewports have no paintOrder option | source | — |
| [support-005](findings/support.md) | P3 | missing-feature | NetworkAssetBundle has no counterpart | source | — |
| [switchers-004](findings/switchers.md) | P3 | api-numeric | AnimatedSize reverseDuration API surface differs | source | — |
| [switchers-007](findings/switchers.md) | P3 | edge-case | AnimatedList keeps a removed item one frame after its exit animation completes | scenario+source | switchers__animated_list_remove_shifts_items |
| [text-011](findings/text.md) | P3 | api-numeric | Text textHeightBehavior with height multiplier 2 measures 0.06 px shorter | scenario | text__text_height_behavior_first_ascent |
| [text-016](findings/text.md) | P3 | visible-motion | Caret starts hidden for about one frame after focus and its blink phase trails by about 16 ms | scenario | text__editable_caret_blink_1200ms |
| [transitions-007](findings/transitions.md) | P3 | api-numeric | AnimatedWidget has no direct public base equivalent | source | transitions__builders__cached_child |
| [transitions-009](findings/transitions.md) | P3 | api-numeric | RelativeRectTween has no named public counterpart | scenario+source | transitions__positioned__relative_rect |
| [transitions-010](findings/transitions.md) | P3 | api-numeric | StatusTransitionWidget has no Incular equivalent | source | — |

### Pending verification (not accepted) (0)

None.

### Rejected or no longer reproducing (6)

| ID | Sev | Impact | Title | Evidence | Scenario |
| --- | --- | --- | --- | --- | --- |
| [focus_input-001](findings/focus_input.md) | P1 | wrong-final-state | Empty SizedBox claims hits | scenario+source | focus_input__childless_sizedbox_passes_hits |
| [scroll-006](findings/scroll.md) | P1 | missing-feature | Keyboard paging of scroll views (duplicate of focus_input-003) | source | — |
| [misc-014](findings/misc.md) | P2 | missing-feature | No public stateful-widget lifecycle hooks (initState, didUpdateWidget, dispose and related) | source | — |
| [semantics-016](findings/semantics.md) | P2 | missing-feature | Inline WidgetSpan carries no child widget, so inline widget semantics cannot exist | source | — |
| [text-017](findings/text.md) | P2 | wrong-final-state | Undo does not restore a backspace in a bare EditableText; Incular restores it | scenario | text__editable_undo_no_programmatic_selection |
| [text-018](findings/text.md) | P3 | edge-case | Controller selection before first focus: Flutter is invalid (-1), Incular is collapsed at text end | scenario | text__editable_multiline_vertical_document |

<!-- findings:end -->

## Shared root causes

1. **Gesture arena timing.** The singleton arena does not resolve on pointer-down (gestures-001), so the first slop-crossing move is lost. It also explains the drag/fling notification-order and PageView fling divergences in scroll. Long-press, double-tap, scale and delayed-drag recognizers accept at different moments from Flutter (gestures-002..006), and the P0 reorder failure (gestures-006) arises from a long-press acceptance fired on a frame competing with a Draggable.
2. **Outgoing content is removed instead of animated out.** Popped routes and hero flights (navigation-003, -005), the covered route during a fade push (navigation-010), Expansible bodies (misc-001), removed list items (switchers-007) and cross-fade layout (switchers-001) skip or shorten Flutter's exit transitions.
3. **Text input pipeline.** Programmatic edits run user-edit transforms and callbacks (text-006), input formatters are never applied (text-009), `onEditingComplete` is absent (text-015), and long-press selection is missing in EditableText and SelectionArea (text-007, -014). Offsets use UTF-8 bytes (text-002).
4. **Flutter objects reduced to scalars or single values.** Animations passed as sampled `f32` (switchers-003, anim_compose-002, transitions-004..006), rich text flattened to one style (text-001), WidgetSpan replaced by text (text-003), DefaultTextStyle without paragraph options (implicit-006, text-004).
5. **Semantics projection.** Merging, default enabled state, label-only nodes, gesture tap actions and exhausted scroll actions differ (semantics-001..007); several properties have no API (semantics-008..014).
6. **Constraint propagation.** IntrinsicHeight, FractionallySizedBox, UnconstrainedBox, OverflowBox, RotatedBox, SafeArea, varied-extent slivers and SliverFillRemaining pass different constraints or insets than Flutter (layout-001, -002, -004, -007, misc-002, -008, slivers-001, -003).
7. **Missing APIs.** 78 accepted findings are missing features (for example imperative Navigator operations, TransitionDelegate, GlobalKey reparenting, Table column widths, scroll keyboard actions, ShaderMask in the WGPU backend, platform views).
8. **Different painting defaults.** Image and DecorationImage fit defaults, ColorFilter bias units, lerp clamping (paint-002, -003, -015, -020).

## Known-gap claims

| Claim | Status at HEAD |
| --- | --- |
| AnimatedSwitcher transition builder receives f32 | Confirmed: switchers-003. |
| Vsync controller requires manual TickerMode muting | Confirmed: anim_core-005. |
| OverlayPortal is clipped by ancestors | Confirmed by pixel comparison at HEAD: navigation-007. |
| RichText uses the first span style | Confirmed: text-001. |
| ModalRoute lookup requires outlet opt-in | Confirmed: navigation-001. |
| Childless SizedBox is a hit-test leaf | **Refuted at HEAD**: a childless SizedBox over a tappable sibling gives one tap on both sides (focus_input-001 rejected). |
| BackdropFilter is blur only | Refuted for the matrix filter; general image filters missing (paint-001). |
| Bouncing has no spring hand-off or momentum carry | Refuted as a missing implementation; ballistic dispatch twins match (physics package). |
| Expansible in scope, without animation | Confirmed: misc-001. |
| Sample list drag loses one move | Confirmed: gestures-001. |

## Harness, recorder and dependency changes

No framework code was changed and no diagnostic accessor was added (`git status` shows no changes under `crates/`, `examples/` or `tools/`). Harness changes, all mirrored on both sides where they affect recording (documented in `harness/README.md`, "Limits"):

- Non-finite numbers are recorded as the strings `NaN`, `Infinity`, `-Infinity` on both sides (previously the Flutter recorder failed on kept-alive probes).
- A fully transparent fill is skipped on both sides (Flutter recorded `[0,0,0,0]` where Incular recorded no fill).
- Flutter opacity counts an offstage `RenderSliverOffstage` and a hidden `_RenderVisibility` as 0; Incular opacity counts a probe under an inactive `IndexedStack` child as 0. The first two changes made `slivers__offstage` and `layout__visibility__maintain_flags` match; the third restored `layout__indexed_stack__active_child`.
- Dart `keyDown`/`keyUp` (modifier chords) and `back()` (`handlePopRoute`).
- Incular `FrameHarness::mount_with_setup`, `set_frame_driver`, `clock()` and `surface_constraints()`; `tests/frames/host.rs` `mount_navigator` presents Navigator twins through `RouteOutlet::present_frame_at` (the harness library keeps its architecture allow-list; the navigation dev-dependency is used only from tests).
- New `support` scenario module (empty; its rows were checked by reading).
- Pixel scenes added: `linear_gradient_offset_box`, `text_align`, `raw_scrollbar_thumb` (and earlier in the audit `image_default_fit`, `decoration_image_default_fit`, `physical_model_circle`, `overlay_portal_ancestor_clip`, `overlay_portal_no_clip`).
- After the recorder changes `sample_` was rerun: 4/5 match, with the known list-fling divergence.

## Harness limits and comparisons not completed

- No trackpad pan-zoom/pinch, lock-key or key-repeat input; no browser-history harness; no Router-hosted back-dispatch twin.
- Pixel captures are single still frames: `raw_scrollbar_thumb` shows no thumb on either side because Flutter paints the thumb a frame after layout, so thumb geometry and fade timing were not compared. Hero flight geometry, obscureText glyphs, sliver decorations and nine-slice `centerSlice` (the shared fixture is uniformly white) also lack pixel comparisons.
- The recorder sees geometry, opacity, first solid fill and sampled values, not clips, shadows, gradients, glyphs or paint order (those use the pixel harness).
- Diagnostic (non-equivalent) twins are labelled in their findings: `anim_core__scheduler_callbacks_time_dilation` (no scheduler time dilation), `physics__clamped_simulation__position_and_velocity` (no velocity clamp), `physical_model_circle` (no circle shape), `layout__table__default_flex_columns`, `text__default_text_style_max_lines`, `text__text_scaler_linear`.

## Open questions

- text-017 (rejected as unverified): in both undo twins Flutter does not restore text on ctrl+z although real Flutter apps do; the cause in `flutter test` (UndoHistory activation or key routing) is unknown.
- navigation: the Flutter below-route probe stays at opacity 1 where a secondary fade could lower it; the secondary-animation claim is unverified.
- Unconfirmed sweep candidates without findings: SliverConstrainedCrossAxis does not clamp SliverConstraints (`sliver_descriptors.rs:1434-1490` vs `rendering/proxy_sliver.dart:449-470`); FractionallySizedBox clamps negative factors where Flutter asserts; `LockState`, four `TextInputAction` values, `EdgeDraggingAutoScroller` and `LayoutChangedNotification` have no counterpart (recorded as `missing` rows).
- navigation-001: is the ModalRoute opt-in intended for embedders? misc-014 was rejected as an intentional design choice (ARCHITECTURE.md principle 8); confirm that GlobalKey reparenting (misc-011) is wanted.

## Validation

Run after the final edits:

- Filtered frame suites, all completing without harness errors: anim_ 9/15 match, physics__ 8/18, implicit__ 16/19, transitions__ 8/9, switchers__ 10/13, navigation__ 5/12, scroll__ 7/17, slivers__ 9/15, gestures__ 3/11, layout__ 31/37, paint__ 6/9, text__ 5/27, focus_input__ 12/18, semantics__ 10/19, misc__ 9/14, sample_ 4/5. Every divergent scenario maps to an accepted finding, a labelled diagnostic, or text-017.
- Pixel: 24 scenes at 1, 1.25, 1.5 and 2x (96 pairs), Flutter recaptured at HEAD; results in `pixel-run.md`.
- `cargo fmt -p incular-parity-harness -- --check`, `cargo clippy -p incular-parity-harness --all-targets --all-features -- -D warnings`, `cargo check --workspace`: pass. `flutter analyze` in `harness/flutter`: no issues.
- `inventory_tool/render-inventory.ps1` (no duplicate items) and `inventory_tool/check-findings.ps1` (every finding has the shared schema fields): pass.
- Complete all-scenario frame run (`harness/run-frames.sh` with no filter, run with the user's permission after the final edits): all 251 Flutter scenarios passed and the comparison completed with exit 0; **146/251 match**. The 105 divergent scenarios are exactly the union of the per-package divergent sets above, with no missing traces or structural (frame-count or timestamp) problems. Report: `harness/out/frames/report.md` (generated, not versioned).
- **Not run:** `cargo test-constrained`, workspace-wide clippy and `cargo fmt --all` (Windows path-length limit) were not run, because framework code was not changed and they are general suites. The filtered package suites together cover every registered scenario module.
