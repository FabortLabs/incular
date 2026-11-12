# navigation: Routes, overlays, heroes and routing

## Summary

The exercised Navigator push/pop/canPop flow, blocked-root PopScope callback, and OverlayPortal popup geometry/hit test all match in the frame harness. Code reading found missing imperative route-stack operations, PageRouteBuilder route animations default to no controller rather than Flutter's 300 ms primary/secondary animation, and Hero flights do not run on normal pops. The existing ModalRoute scope default gap remains. Overlay clipping was not accepted as a divergence: the frame test compares geometry and hit testing, while the pixel test is needed to confirm paint clipping.

## Coverage

| Flutter item | Incular counterpart | Method | Scenarios | Result |
| --- | --- | --- | --- | --- |
| Navigator push/pop/canPop, replace, pushAndRemoveUntil/removeRoute/popUntil | `incular_navigation::Navigator` | scenario + reading | `navigation__navigator__push_pop_can_pop` (push/pop/canPop); other methods checked by reading | partial; imperative stack operations missing |
| ModalRoute lookup and route status | `incular_navigation::ModalRoute`, runtime RouteOutlet scope | reading | none | divergent; see navigation-001 |
| PageRoute/PageRouteBuilder, primary and secondary route animations, opaque/maintainState/barrier | `Route`, `RoutePresentation`, `PageRouteBuilder`, `RouteAnimation` | reading + existing Incular tests | none | partial; presentation defaults are covered by existing tests, route animation defaults differ |
| PopScope blocked and allowed pop callbacks | `incular_widgets::PopScope`, navigation back dispatcher | scenario + reading | `navigation__pop_scope__blocked_root_route` | match for blocked root pop |
| Overlay/OverlayEntry insertion, ordering, removal and rebuild | `incular_navigation::Overlay`, `OverlayEntry` | reading | none | partial; generic entry removal/rebuild lifecycle is missing |
| OverlayPortal show, layout builder and input outside anchor | `incular_widgets::OverlayPortal` | scenario + reading | `navigation__overlay_portal__content_outside_anchor_clip` | match for recorded geometry and tap; paint not compared |
| Hero/HeroController/HeroMode flight geometry, custom tween/shuttle, pop and user gestures | `incular_widgets::Hero`, `HeroMode`; `incular_navigation::HeroController` | reading + existing `crates/incular/tests/heroes.rs` | none | partial; normal pop flights are missing; user-gesture route flights are not available |
| Router, RouterConfig, RouteInformation provider/parser, RouterDelegate and back dispatch | `incular_widgets::Router` and router module | reading | none | partial; platform route-information and restoration timelines not run |

Three scenarios were compared and re-run. They cover ordinary push/pop values, a blocked root pop, and an open portal with a popup hit outside its 100x100 anchor clip. All three pass, including frame 0. Existing `crates/incular-navigation/tests/navigation.rs` covers route presentation defaults and route stack behaviors; `crates/incular/tests/heroes.rs` covers Hero-specific mechanisms, but neither replaces Flutter twin comparisons for the listed flows.

Not checked by a frame twin: `pushAndRemoveUntil`, `removeRoute`, `removeRouteBelow`, `popUntil`, route transition timing/secondary animation values, arbitrary OverlayEntry removal/rebuild, Hero pop and user-gesture flights, and Router platform/back/restoration event timelines. Reasons: corresponding imperative APIs are absent for the route and overlay operations; no current scenario records route animation controller values or platform route-information events; Incular has no user-gesture route-pop API. The missing APIs and Hero pop behavior are recorded below; other timing/Router variants remain reading-only.

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

## Pixel scene proposals

Check whether an OverlayPortal child painted at (200,200) escapes a 100x100 ancestor ClipRect. Use a 160x160 black canvas and place the clipped anchor at the top-left, with an 80x60 orange popup and a blue 60x50 anchor. Flutter should use `Align(alignment: Alignment.topLeft, child: ClipRect(child: SizedBox(width: 100, height: 100, child: OverlayPortal.overlayChildLayoutBuilder(...))))`; the Incular twin should use `Align::new(Alignment::TOP_LEFT, ClipRect::new(SizedBox::new().width(100.).height(100.).child(OverlayPortal::new(...).overlay_child_layout_builder(...).show(true))))`. The popup's child is a `Positioned(left: 200, top: 200)` / `Positioned::new(...).left(200.).top(200.)` colored box. Compare the pixel at the popup location and the clipped 100x100 region.

## Harness requests

None.

## Open questions

- The frame scenario confirms popup geometry and hit testing outside the anchor clip, but it does not capture paint; run the proposed pixel scene before closing the ancestor-clip known gap.
- Router platform route-information reporting, restoration and back-button event order were read but not exercised by twins.
- Hero user-gesture transitions remain untested; Incular currently has no user-gesture route transition path.
