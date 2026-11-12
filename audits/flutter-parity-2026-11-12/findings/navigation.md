# navigation: Routes, overlays, heroes and routing

## Summary

The exercised Navigator push/pop/canPop flow, external Router route-information update, blocked-root PopScope callback, and OverlayPortal popup geometry/hit test all match in the frame harness. A pixel pair/control confirms an ancestor ClipRect hides the 40x40 OverlayPortal popup in Incular while Flutter paints it outside the anchor clip. Code reading found missing imperative route-stack operations, a Router-specific nested back-priority gap, PageRouteBuilder route animations default to no controller rather than Flutter's 300 ms primary/secondary animation, and Hero flights do not run on normal pops. The existing ModalRoute scope default gap remains.

## Coverage

| Flutter item | Incular counterpart | Method | Scenarios | Result |
| --- | --- | --- | --- | --- |
| Navigator push/pop/canPop, replace, pushAndRemoveUntil/removeRoute/popUntil | `incular_navigation::Navigator` | scenario + reading | `navigation__navigator__push_pop_can_pop` (push/pop/canPop); other methods checked by reading | partial; imperative stack operations missing |
| ModalRoute lookup and route status | `incular_navigation::ModalRoute`, runtime RouteOutlet scope | reading | none | divergent; see navigation-001 |
| PageRoute/PageRouteBuilder, primary and secondary route animations, opaque/maintainState/barrier | `Route`, `RoutePresentation`, `PageRouteBuilder`, `RouteAnimation` | reading + existing Incular tests | none | partial; presentation defaults are covered by existing tests, route animation defaults differ |
| PopScope blocked and allowed pop callbacks | `incular_widgets::PopScope`, navigation back dispatcher | scenario + reading | `navigation__pop_scope__blocked_root_route` | match for blocked root pop |
| Overlay/OverlayEntry insertion, ordering, removal and rebuild | `incular_navigation::Overlay`, `OverlayEntry` | reading | none | partial; generic entry removal/rebuild lifecycle is missing |
| OverlayPortal show, layout builder, input and paint outside anchor clip | `incular_widgets::OverlayPortal` | frame scenario + pixel pair/control + source review | `navigation__overlay_portal__content_outside_anchor_clip`; `overlay_portal_ancestor_clip` | frame geometry/tap match; pixel confirms Flutter paints outside the anchor clip, but Incular culls the popup picture; see navigation-007 |
| Hero/HeroController/HeroMode flight geometry, custom tween/shuttle, pop and user gestures | `incular_widgets::Hero`, `HeroMode`; `incular_navigation::HeroController` | reading + existing `crates/incular/tests/heroes.rs` | none | partial; normal pop flights are missing; user-gesture route flights are not available |
| Router, route-information provider/parser and delegate | `incular_widgets::Router` and router module | paired scenario + reading | `navigation__router__provider_route_change` | match for an external `/home` → `/details` provider update; platform engine/history reporting and restoration not exercised |
| Root/child back-dispatcher priority for Router | app-shell `RootBackButtonDispatcher` and `BackButtonDispatcher` | reading | none | partial; Router's app-shell dispatcher has no child priority transfer; the separate navigation dispatcher is not accepted by RouterConfig, see navigation-006 |

Four scenarios were compared and re-run. They cover ordinary push/pop values, an external route-information update, a blocked root pop, and an open portal with a popup hit outside its 100x100 anchor clip. All four pass, including frame 0. In the Router pair, both providers begin at `/home`, publish `/details` after two frames, and update a top-left 120x80 route probe and sampled provider/delegate state on the same frame. Flutter uses a notifying `RouteInformationProvider` and an async path parser; Incular uses `MemoryRouteInformationProvider::set_value` and `StringRouteInformationParser`.

The Router path is supported by Flutter `widgets/router.dart:618-640` (provider subscription and initial route application), `:770-803` (parse and provider-notification dispatch), and Incular `crates/incular-widgets/src/app_shell/router.rs:1107-1114`, `:1188-1205`, and `:1208-1215` (provider subscription, initial route initialization, and update application). Existing `crates/incular-navigation/tests/navigation.rs` covers route presentation defaults and route stack behaviors; `crates/incular/tests/heroes.rs` covers Hero-specific mechanisms, but neither replaces Flutter twin comparisons for the listed flows.

The `overlay_portal_no_clip` control matches at 1x. In `overlay_portal_ancestor_clip`, the same 40x40 orange child at (110,110) is visible outside the 80x80 blue anchor in Flutter and absent in Incular; the lost popup is 6.25% of the 160x160 image. The only scene change is the ClipRect around the anchor. Flutter documents that the overlay child extends outside the portal without clipping (`widgets/overlay.dart:1882-1889`) and attaches its deferred render box directly to `_RenderTheater` (`:2105-2110`).

Incular initially keeps the popup as a child of the portal Stack and marks its root compositor layer as a surface partition (`crates/incular-widgets/src/navigation_scopes.rs:176-208`; `crates/incular-widgets/src/tree/retained.rs:526-543`). During `compositor.flatten`, the ancestor ClipRect passes its active clip to descendant layers (`crates/incular-rendering/src/compositor.rs:1609-1632`), and the popup's Picture layer unconditionally drops pictures outside that clip (`:1582-1590`). `WidgetTree::paint` calls `lift_surface_partitions` only after flattening (`crates/incular-widgets/src/tree/painting.rs:151-165`), so the orange picture commands are already gone before the lift can remove the clip. This is compositor culling-before-lift, not a placement mismatch.

Not checked by a frame twin: `pushAndRemoveUntil`, `removeRoute`, `removeRouteBelow`, `popUntil`, route transition timing/secondary animation values, arbitrary OverlayEntry removal/rebuild, Hero pop and user-gesture flights, nested Router back-priority handoff, browser/engine history reporting, restoration, and Router back-event timelines. The corresponding imperative route and overlay APIs are absent; Incular's default `PageRouteBuilder` has no animation controller to sample, so a duration timeline would not be an equivalent twin; Incular has no user-gesture route-pop API. The missing APIs and Hero pop behavior are recorded below; remaining timing and Router variants are reading-only.

## Findings

### navigation-001 — Modal route lookup is disabled by default

- Severity: P2
- Category: default-value
- Flutter: `widgets/routes.dart:1188` installs the modal scope around route content, and `:1315` finds that scope.
- Incular: `crates/incular-runtime/src/route_outlet.rs:465` defaults the scope setting to false, and `:982` installs the scope only when enabled by `set_modal_route_scopes`.
- Evidence: code reading of the references above.
- Impact: route content using ModalRoute lookup sees no route under a default RouteOutlet until the application explicitly enables scopes.
- Likely cause: modal route scope installation is an opt-in outlet policy.
- Fix direction: align the default scope behavior with route lookup expectations while preserving the outlet's validation contract.
- Confidence: high

### navigation-002 — Imperative stack removal and predicate operations are missing

- Severity: P1
- Category: missing-api
- Flutter: `widgets/navigator.dart:5247` provides `pushAndRemoveUntil`; `:5635`, `:5677`, and `:5702` provide `popUntil`, `removeRoute`, and `removeRouteBelow`.
- Incular: `crates/incular-navigation/src/navigator.rs:421`, `:818`, and `:826` provide push, pop, and replace; the public Navigator API has no matching removal-by-id/route or predicate-based stack operation.
- Evidence: code reading of Flutter NavigatorState's public methods and Incular Navigator's full public stack-operation surface.
- Impact: applications cannot use these common imperative flows directly, including replacing a login flow with a new root route or removing a specific route under an active route.
- Likely cause: Incular exposes stack reconciliation with `set_pages`, but not Flutter's imperative route predicate/removal methods.
- Fix direction: add route-identity and predicate-based removal operations with explicit lifecycle and observer semantics.
- Confidence: high

### navigation-003 — PageRouteBuilder does not create Flutter's default route animations

- Severity: P2
- Category: timing
- Flutter: `widgets/pages.dart:96-97` defaults both transition durations to 300 ms; `widgets/routes.dart:234-246` creates the route controller and animation from those durations even when the default transitions builder is a jump cut.
- Incular: `crates/incular-navigation/src/presentation.rs:335-351` defaults `PageRouteBuilder` to `RouteTransition::None`; `:510-514` returns no animation for that transition. `RouteAnimation::new` requires explicit durations.
- Evidence: code reading of the defaults and controller construction on both sides.
- Impact: callers cannot observe Flutter's default primary/secondary animation progress, and code driven by those animations (including Hero flights with default page transitions) has no equivalent default timeline.
- Likely cause: the Incular descriptor models a transition as optional paint composition, while Flutter's PageRoute always owns a transition controller independently of the default jump-cut builder.
- Fix direction: retain primary and secondary route animation controllers with Flutter-compatible default and reverse durations even when the route transition builder paints no motion.
- Confidence: high

### navigation-004 — Overlay entries lack per-entry removal and rebuild lifecycle

- Severity: P2
- Category: missing-api
- Flutter: `widgets/overlay.dart:225` removes a specific `OverlayEntry`; `:247` marks that entry for rebuilding. `OverlayState` also supports `insert` relative to another entry and `rearrange` (`:718`, `:789`).
- Incular: `crates/incular-navigation/src/presentation.rs:666-680` appends entries and can remove/dismiss only the top entry; `OverlayEntry` is a child/barrier value without its own builder, removal handle, or rebuild method.
- Evidence: code reading of the public Flutter OverlayEntry/OverlayState methods and Incular Overlay surface.
- Impact: callers cannot update or remove a particular persistent overlay entry while preserving other entries, or request that one entry rebuild from changed builder state.
- Likely cause: Incular's Overlay is a simple stack value rather than a retained entry lifecycle matching Flutter's OverlayState.
- Fix direction: provide stable entry identity and per-entry lifecycle operations, including rebuild and ordering controls.
- Confidence: high

### navigation-005 — Hero flights do not run on normal route pops

- Severity: P1
- Category: behavior
- Flutter: `widgets/heroes.dart:886-912` starts flights for a reverse route animation and selects `HeroFlightDirection.pop` at `:901-902`.
- Incular: `crates/incular-navigation/src/hero_controller.rs:18-22` states flights run on pushes only; `crates/incular-navigation/src/navigator.rs:770-816` removes the popped route immediately and ends its lifetime.
- Evidence: code reading of Flutter's reverse-animation path and Incular's route-removal path. Existing Hero tests cover flight mechanics but do not restore a popped route for a reverse flight.
- Impact: shared-tag heroes disappear with the popped route instead of flying back to their destination during ordinary back navigation.
- Likely cause: pop commits discard the source route immediately instead of keeping it mounted while its reverse transition drives the flight.
- Fix direction: retain the popped route through its reverse transition and let HeroController observe the reverse animation before final disposal.
- Confidence: high

### navigation-006 — Router back dispatch cannot transfer priority to nested Routers

- Severity: P2
- Category: missing-api
- Flutter: `widgets/router.dart:1021-1037` creates child dispatchers; `:1040-1084` exposes `takePriority`, `deferTo`, and `forget`; `:1125-1149` implements the child-parent relationship.
- Incular Router integration: `crates/incular-widgets/src/app_shell/router.rs:866` accepts only an app-shell `RootBackButtonDispatcher`; `:648-685` and `:727-746` implement that app-shell type as a flat callback list and root wrapper without child creation or priority-transfer operations.
- Separate Incular API: `crates/incular-widgets/src/navigation/back_dispatch.rs:413-428` has nested `NavigationBackButtonDispatcher` children and `defer_to`; `crates/incular-navigation/src/navigator.rs:1006-1079` also has attach/active-child operations. Neither type is the app-shell dispatcher accepted by `RouterConfig`.
- Evidence: code reading of Flutter Router's hierarchical dispatcher and the distinct Incular Router and navigation dispatcher types.
- Impact: nested Routers cannot use the explicit child-priority handoff through `RouterConfig`; the separate navigation dispatcher does not plug into that Router API.
- Likely cause: app-shell Router back dispatch and navigation/pop-scope back dispatch use separate dispatcher types.
- Fix direction: add child dispatchers with explicit priority transfer and fallback through parent callbacks.
- Confidence: high

### navigation-007 — OverlayPortal popup pictures are culled by an ancestor clip

- Severity: P1
- Category: behavior
- Flutter: `widgets/overlay.dart:1882-1889` documents that the overlay child can extend beyond the portal un-clipped; `:2105-2110` attaches the deferred child directly to `_RenderTheater`.
- Incular: `crates/incular-widgets/src/tree/retained.rs:526-543` marks the popup root layer as a partition; `crates/incular-rendering/src/compositor.rs:1582-1590` culls a Picture outside the active clip before `crates/incular-widgets/src/tree/painting.rs:151-165` lifts the surface partition.
- Evidence: 1x pixel pair/control. `overlay_portal_no_clip` matches. With the only scene change being `ClipRect` around the 80x80 anchor, `overlay_portal_ancestor_clip` omits the 40x40 orange popup at (110,110) in Incular while Flutter shows it; the popup area is 6.25% of the 160x160 image.
- Impact: an OverlayPortal popup disappears when it extends outside an ancestor ClipRect, including clips used to bound an anchor or viewport.
- Likely cause: compositor flattening applies the ancestor clip to the popup's unmarked Picture descendants before partition lifting can detach them.
- Fix direction: preserve surface-partition descendants through ancestor-clip culling, then apply clipping from the popup's own subtree after lifting.
- Confidence: high

## Pixel scene results

The `overlay_portal_ancestor_clip` scene pair/control confirms the ancestor-clip paint gap at 1x. The existing frame scenario checks geometry and hit testing only; pixel results establish the paint divergence.

## Harness requests

None.

## Open questions

- The Router provider update twin covers external route changes only. Browser/engine history reporting, restoration and back-button event order remain unchecked.
- Hero user-gesture transitions remain untested; Incular currently has no user-gesture route transition path.
