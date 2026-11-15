# navigation fixes

Group `navigation`: 12 findings (navigation-001 to 011, switchers-005). Machine-readable records are in `navigation.jsonl`.

Orchestrator correction: the worker swapped the IDs of navigation-005 (hero flights on pop) and navigation-010 (covered route hidden during a fade push). The section headings and JSONL ids below are corrected to match the audit findings.

## navigation-005: partial
- Cause: popped routes were not recognised as popped by hero flights.
- Change: hero_controller.rs records Popped before ActiveRouteChanged; popped routes without a route animation skip the flight.
- Tests: `crates/incular/tests/heroes.rs` heroes_do_not_fly_without_a_route_animation_or_on_pop.
- Harness: not run.


Residual pass: the popped route still leaves one frame after Flutter. Route animations tick inside run_frame, after the outlet rebuild. A same-frame settle (tick before rebuild) was tried and reverted because it did not change the frame report. The frame harness still shows probe.top.present at frame 44 (Flutter false, Incular true) in fade_pop_timeline and pop_flight_navigator. Also, heroes.rs heroes_do_not_fly_without_a_route_animation_or_on_pop fails at line 391 (source hero not painted after the reverse completes). The hero test harness does not use RouteOutlet, so the outlet is not the cause, but the cause is not isolated.

## navigation-003: fixed
- Cause: PageRouteBuilder had no default transition or reverse duration.
- Change: presentation.rs sets 300 ms forward and reverse defaults; build() yields an Animated route when no transition is set.
- Tests: route_outlet.rs popped_fade_and_default_page_routes_stay_mounted_through_reverse.
- Limitation: the declarative Page default is still RouteTransition::None.

## navigation-010: fixed
- Cause: fade pages revealed the route below too early; exiting routes were removed too soon.
- Change: route_outlet.rs transition_settled gates occlusion; pop reverses through reverse_on_next_tick for route-animation routes.
- Tests: route_outlet.rs fading_page_reveals_the_route_below_until_its_fade_completes.
- Limitation: removal lags one frame behind Flutter.

## navigation-002: fixed
- Cause: no imperative route removal.
- Change: navigator.rs adds remove_route, remove_route_below, pop_until and push_and_remove_until; removal reports Popped and ActiveRouteChanged.
- Tests: crates/incular-navigation/tests/navigation.rs (five removal and predicate tests, plus overlay_entries_are_positioned_and_removed_by_identity).

## navigation-007: fixed (pixel pending)
- Cause: open popups were culled by ancestor clips before being lifted out.
- Change: compositor.rs flatten_layer drops the ancestor clip for surface partitions before culling.
- Tests: crates/incular-rendering/tests/compositor_culling.rs surface_partitions_are_not_culled_by_ancestor_clips.
- Harness: navigation__overlay_portal__content_outside_anchor_clip shows matching content. The pixel scene overlay_portal_ancestor_clip (1x and 1.5x) is pending the orchestrator.

## navigation-009: fixed
- Cause: the OverlayPortal anchor lacked a stable keyed element and tight constraints.
- Change: navigation_scopes.rs anchor_wrapper keys the anchor with a Passthrough-fit wrapper and keeps the caller's key.
- Outer stacks: setting both outer stacks to Passthrough fixed the anchor probe but broke 6 overlay_tooltip_lifecycle tests (nested_z_order, options_matrix, outside_tap, anchor_movement, builder_content_agrees, placement_options). They are left Loose.
- Harness: navigation__overlay_portal__direct_anchor DIVERGES (anchor 60x50 against Flutter 100x100).
- Tests: crates/incular-widgets/tests/navigation_scopes.rs (8 pass).


Residual pass: both OverlayPortal outer stacks are now Passthrough, so the anchor takes the parent's constraints as in Flutter. navigation__overlay_portal__direct_anchor matches (3 frames). The six overlay_tooltip_lifecycle tests that failed (nested_z_order, options_matrix, outside_tap, anchor_movement, builder_content_agrees, placement_options) encoded a Loose anchor: they mounted small anchors under a tight root and expected content-sized anchors. Flutter sizes such a child to the tight root, so those tests now mount under a loose root (Align topLeft). Their expectations are otherwise unchanged. All 22 tests in that file pass.

## navigation-001: blocked
- Blocker: eager scope validation needs a Navigator API change outside this group. A modal-scope default-on experiment broke two atomic-failure outlet tests and was reverted.


Residual pass: re-assessed, still blocked. The scope is an inherited node built lazily, so enabling it by default makes route content validate after the rebuild commits, which breaks the atomic duplicate-key contract. Fixing it needs eager building of environment_scope children in the widget tree, outside this group. The default stays false.

## navigation-004: fixed
- Rebuild on navigation state change is deferred; it needs a host subscription API.


Residual pass: Overlay state now carries a DependencySource. entries() and blocks_background_input() track it, and insert, insert_above/below, remove and remove_top notify it, so a host build that reads entries rebuilds on change. Test: route_outlet.rs overlay_host_rebuilds_when_entries_are_added_and_removed. Limitation: no built-in Overlay host exists, so the application's host must read entries inside a build.

## navigation-006: deferred
- Needs a Router child dispatcher API that is too large for this group.

## navigation-008: deferred
- A design boundary between navigation and widgets needs a decision.


Residual pass: re-assessed, still deferred. RouteOutlet's rejection of overlay routes is a tested contract (OutletPlacement::ExternalOverlay, UnsupportedPresentation). Hosting overlay entries in the outlet would put barrier and z-order layering inside navigation, which is a navigation/widgets design boundary decision.

## navigation-011: deferred
- Needs a TransitionDelegate API that is too large for this group.

## switchers-005: fixed
- Cause: AnimatedModalBarrier could not fade to a null color.
- Change: barrier.rs OptionalColorAnimation (sampler closure). API change: AnimatedModalBarrier::animation() now returns OptionalColorAnimation.
- Tests: navigation_scopes.rs animated_barrier_null_color_endpoint_paints_no_fill_and_still_dismisses.

## Verification run
- incular-widgets navigation_scopes: 8/8 pass.
- incular-widgets overlay_tooltip_lifecycle: 13 pass, 9 fail with the Passthrough outer stacks and 19 pass, 3 fail with Loose outer stacks. The 3 remaining failures (tooltip_constructors_agree, tooltip_content_setters_update_message, tooltip_semantics_options) also fail with the anchor wrapper removed, and the test file is unmodified in git, so they most likely come from other workers' semantics changes.
- Frame scenario navigation__overlay_portal: show_hide and content_outside_anchor_clip match; direct_anchor diverges.
- Not run: cargo clippy, rustfmt (shared files are other workers' ownership), pixel harness.
