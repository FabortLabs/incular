# navigation-design fixes

Group `navigation-design`: navigation-005, 001, 008, 006 and 011. These records replace the earlier `navigation.md` records for these ids. The machine-readable records are in `navigation-design.jsonl`.

## navigation-005: fixed
- Cause: the outlet decided whether a route had finished from the controller status of the previous frame. It composes before `run_frame` ticks the controllers, so a popped route whose reverse ended on frame N was removed on frame N+1. Flutter ticks before the build, so the route is finalized on frame N.
- Change: `route_outlet.rs` `retain_unfinished_exits(state, routes, now)` treats a run that is done at `now` as finished. This covers popped routes and the replacement a replaced route waits for. Each popped route that leaves this way is ticked at `now` after the bookkeeping borrow is released, so its status listeners still hear `Dismissed` on that frame.
- New API in `crates/incular-animation/src/controller.rs` (outside ownership, additive): `AnimationController::is_done_at(now)`, a pure query.
- Rejected alternative: pre-ticking every route controller before composition. It desyncs `OpacityController`, which only updates its opacity when the inner tick reports a change.
- Tests: `route_outlet.rs` `popped_fade_and_default_page_routes_stay_mounted_through_reverse` now expects removal on the same frame. New test: `popped_route_reports_dismissed_on_the_frame_it_leaves`.
- Harness: the frame 44 `present` divergence is gone from `fade_pop_timeline` and `hero__pop_flight_navigator`. The remaining fields are other issues: the harness start residual, below-route secondary opacity, hero geometry and covered root opacity.
- Limitations:
  - `heroes.rs::heroes_do_not_fly_without_a_route_animation_or_on_pop` still fails at line 391. This was already the case, and that test belongs to the test-alignment worker.
  - Push-completion occlusion still reads the previous frame's status.

## navigation-001: blocked
- Cause: `Widget::environment_scope` is a lazily built `LayoutBuilder`. Prevalidation (`validate_widget_subtree`) stops at that node, so route content under a `ModalRoute` scope is only validated during layout, after the rebuild has applied. That breaks the atomic `DuplicateKey` contract.
- Remaining work, in incular-widgets tree internals:
  1. Give static scopes a known child that prevalidation descends into.
  2. Handle the ambient `TextDirection` below scopes: take it from a `TextDirection` scope value, and otherwise skip the direction checks.
  3. Optionally build these static scopes eagerly.
- After that, change `RouteOutlet::new` to `modal_route_scopes: true`.

## navigation-008: fixed
- Cause: `Route::overlay` entries had no host. The outlet rejected them, and nothing else read them.
- Change: `RouteOutlet` hosts overlay routes by default. A new policy controls this: `set_host_overlay_routes` and `host_overlay_routes`. For each hosted route:
  - Its entries mount in order at the route's stack position.
  - An entry with a barrier is preceded by a veil. The veil blocks input and dismisses the route.
  - The route's wrapper blocks semantics of earlier routes when an entry has a barrier.
  - The route transition and heroes wrap the entries through the new `Route::present`.
  - When hosting is disabled, the typed `UnsupportedPresentation` rejection still applies.
- Tests:
  - New: `outlet_hosts_overlay_route_entries_with_barriers`.
  - Seven rejection and atomicity tests opt out with `set_host_overlay_routes(false)` and are otherwise unchanged.
- API change: overlay routes now mount by default. To migrate, an application that hosts the entries itself calls `set_host_overlay_routes(false)`.
- Limitations:
  - Barrier veils sit inside the route transition, so a slide also moves the barrier.
  - Entries are never opaque and are not kept while covered by an opaque page.
  - The route's own child is not shown.

## navigation-006: deferred
- Cause: the Router accepts only the flat `RootBackButtonDispatcher`. The navigation `BackButtonDispatcher` has `create_child` and `defer_to`, but the Router cannot use it.
- Remaining work:
  - Make `RouterConfig` accept the navigation dispatcher, or a shared trait.
  - Add `take_priority`, `forget` and parent fallback to that dispatcher.
  - Have nested Routers take priority when they become active.
  - Adapt `RootBackButtonDispatcher`. This is a public API change.

## navigation-011: deferred
- Cause: there is no `TransitionDelegate` hook.
- Remaining work: a `TransitionDelegate` trait on `Navigator` that returns a decision per route: Push/Add for entering pages and Pop/Complete/Remove for leaving pages. Ship a default that matches Flutter: only the new top pushes, the old top pops, and the others are added or removed at once.
- The outlet's `drive_route_animations` already has both mechanisms: animating in versus starting complete, and exiting versus immediate removal. It would consume the delegate's decisions instead of its current heuristics.
