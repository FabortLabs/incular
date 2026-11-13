# navigation: routes, overlays, heroes and routing

Revalidated at HEAD `a19bf487`. Earlier findings were recorded at `a425aa2e`.

## Summary

Twelve frame scenarios now run; 5 match. Navigator-hosted twins (`host::mount_navigator`) cover fade push and pop timelines, a pushReplacement and observer order log, a Hero push and pop, and a route-level PopScope blocked by the back action. Results:

- Match: observer order (counters and order), PopScope callback count and popped value, earlier Navigator counters, Router, overlay portal show/hide and clip geometry.
- Divergence, real: navigation-010 (new, P1) covered route hidden from the first frame of a fade push; navigation-003 and navigation-005 (route removed at pop instead of reverse transition).
- Divergence, not a behaviour finding: the top route's push opacity is about 1.3 ms ahead of Flutter (harness base instant captured before runtime mount; the harness does not expose its start instant), which exceeds the 0.002 opacity tolerance; Hero probe rects (Flutter flight copies the probed child, so the probe reports destination geometry); PopScope root opacity under an opaque covered route (not visible).
- Unresolved: the Flutter below-route probe reads 1.0 where the secondary-animation fade should lower it toward 0.5.

## Coverage

56 rows in `audits/flutter-parity/coverage/navigation.jsonl`. Status counts: out-of-scope 15, partial-reading 18, partial-scenario 12, scenario+reading 3, scenario 2, unchecked 5, missing 1 (HeroControllerScope, open question). Unchecked rows: BackButtonListener, PlatformRouteInformationProvider, RouteInformationReportingType (browser history harness), PopNavigatorRouterDelegateMixin (Router twin), RestorableRouteFuture (restoration not built this pass). Two traversal constants and two selection overlays are out-of-scope with reassign (focus_input, text).

## Scenarios

| Scenario | Frames | Result | Notes |
| --- | --- | --- | --- |
| `navigation__navigator__push_pop_can_pop` | 6 | match | Counters only (Incular twin mounts a placeholder, not the Navigator). |
| `navigation__router__provider_route_change` | 5 | match | External provider update. |
| `navigation__pop_scope__blocked_root_route` | 3 | match | Root route, blocked maybePop, dispatcher path. |
| `navigation__overlay_portal__content_outside_anchor_clip` | 5 | match | Tap outside clip. Pixel result in navigation-007. |
| `navigation__overlay_portal__show_hide` | 7 | match | Centered anchor in a 100x100 box. |
| `navigation__overlay_portal__direct_anchor` | 3 | DIVERGES | Key override, navigation-009. |
| `navigation__navigator__fade_push_timeline` | 25 | DIVERGES | below opacity 1.0 vs 0.0 from frame 3 (navigation-010, real); top opacity 0.0045 residual (harness start, not a finding). |
| `navigation__navigator__fade_pop_timeline` | 47 | DIVERGES | Push as above; top present true vs false from frame 25 (navigation-003, navigation-005 evidence). |
| `navigation__hero__push_flight_navigator` | 25 | DIVERGES | Probe-based only; flight geometry not comparable (see harness requests). hero_from opacity 1.0 vs 0.0 not decided. |
| `navigation__hero__pop_flight_navigator` | 47 | DIVERGES | hero_to present true vs false from frame 25 (navigation-005 evidence). |
| `navigation__navigator__observer_order` | 9 | match on `log` | Only the covered root opacity (invisible under an opaque route) differs. |
| `navigation__pop_scope__route_back_blocked` | 7 | match on callbacks and did_pop | Covered root opacity differs (invisible under an opaque route). |

Twin notes: Flutter's `handlePopRoute` does not reach a bare Navigator (no Router or WidgetsApp registers it), so the Flutter PopScope twin uses `maybePop`; the Incular twin dispatches through its own dispatcher. Both are labelled as a dispatch-path difference. Incular twins push after mount where Flutter pushes after mount, so frame structure matches.

## Findings

### navigation-003 — PageRouteBuilder does not create Flutter's default route animations (P2, timing)
Source (unchanged): Flutter defaults durations to 300 ms and keeps the popped route for its reverse duration (`widgets/pages.dart:96-97`, `widgets/routes.dart:234-246`); Incular defaults to `RouteTransition::None` (`crates/incular-navigation/src/presentation.rs:351`) and has no reverse transition.
Scenario: `navigation__navigator__fade_pop_timeline`. Forward fade matches within the harness residual. At 400 ms Flutter still has the popped route (reverse 300 ms); Incular has removed it (`probe.top.present` diverges from frame 25). Status: accepted (SUMMARY); evidence is scenario and source.

### navigation-005 — Hero flights do not run on normal route pops (P1, behaviour)
Source and existing test unchanged (`crates/incular-navigation/src/hero_controller.rs:14-26`; `crates/incular/tests/heroes.rs:351-380`).
Scenario: `navigation__hero__pop_flight_navigator`. The destination route is present at 400 ms in Flutter and absent in Incular, so the route that carries the flight is removed at the pop. Hero flight geometry is not compared (see harness requests). Status: accepted (SUMMARY).

### navigation-006 — Router back dispatch cannot transfer priority to nested Routers (P2)
Unchanged; source only. The new PopScope twin uses the widget dispatcher, not Router back priority, so it does not test this finding.

### navigation-010 — Covered route is hidden from the first frame of a fade push (P1, visible-motion) — new, pending-review
Scenario: `navigation__navigator__fade_push_timeline`, first divergence frame 3 (t 48 ms), `probe.below.opacity`, Flutter 1.0, Incular 0.0; the value stays 0.0 through the transition.
Source: `crates/incular-runtime/src/route_outlet.rs:894-907` treats a route as settled when `route_animation()` is `None`; `crates/incular-navigation/src/presentation.rs:539-543` returns `None` for `RouteTransition::Fade`, so a fading page occludes the route below immediately. The comment at `route_outlet.rs:894-907` says the route below shows while it animates in.
Impact: a fade page transition blanks the page underneath for the whole fade.
Likely cause: fade transitions have no `RouteAnimation`, so occlusion ignores their controller.
Fix direction: treat a route as unsettled while its fade controller runs.
Confidence: high on the cause. Limitation: the Flutter secondary-animation target is unresolved (below probe reads 1.0 where the transitionsBuilder should give about 0.5), so only the occlusion timing is claimed.

### Earlier findings 001, 002, 004, 007, 008, 009
Unchanged. Status: 001, 002, 004, 007 accepted (SUMMARY); 008 and 009 pending-review.

## Pixel questions

- Re-run `overlay_portal_ancestor_clip` at 1x and 1.5x at HEAD before closing navigation-007 (pixel harness is orchestrator-owned).
- Verify the covered-route fade in pixels (navigation-010): a Flutter fade over a coloured page, and the Incular equivalent.

## Harness requests

1. Expose the harness start instant (`FrameHarness::start()` or a clock accessor). The current base is captured before `Runtime::new` inside `mount_inner`, which leaves a 1.3 ms residual that exceeds the 0.002 opacity tolerance for fade starts. Blocks an exact transition twin.
2. Hero flight geometry through probes: Flutter's flight copies the probed child, so `hero_to` reports the destination rect, and `hero_from` stays visible. A flight-only probe (or a pixel comparison of flights) is needed before hero geometry can be compared.
3. Router back-event twin (Router `popRoute` and BackButtonListener), and a browser history harness for PlatformRouteInformationProvider and RouteInformationReportingType.
4. Restoration (not built this pass by instruction).

## Open questions

- Flutter secondary animation of the route below: the probe reads 1.0 where the transitionsBuilder should lower it toward 0.5. Confirm with a Flutter-only check before claiming a secondary-animation finding.
- Is HeroControllerScope needed (is a HeroController attached by default in an app shell)?
- Is the overlay-route rejection (navigation-008) a deliberate boundary?
- Is WillPopScope intentionally absent (deprecated)?

## Validation run

- `FLUTTER="C:/Manual Installs/flutter/bin/flutter" harness/run-frames.sh navigation__`: 12 scenarios, 5 match (see Scenarios). Report `harness/out/frames/report-navigation__.md`.
- `cargo check -q -p incular-parity-harness --tests`: passes (0 errors after the new host twins).
- `rustfmt --edition 2024 harness/incular/tests/frames/navigation.rs`: applied.
- `flutter analyze test/frames/navigation_test.dart`: no issues.
- `render-inventory.ps1`: validates; inventory rebuilt.
- Not run: the pixel harness (orchestrator-owned).

### navigation-011 — Navigator has no TransitionDelegate for declarative page changes

- Severity: P2. Category: missing-api (missing-feature). Added by the orchestrator from the coverage sweep; status accepted.
- Flutter: `widgets/navigator.dart:1016 (TransitionDelegate), :1182 (DefaultTransitionDelegate), Navigator.transitionDelegate`. Incular: `MISSING: rg -n -i "transition_delegate|TransitionDelegate|RouteTransitionRecord" crates returns nothing`.
- Evidence (source): Flutter lets declarative (pages-based) navigation decide per route whether page additions and removals animate (markForPush/markForAdd/markForPop/markForComplete/markForRemove). Incular exposes no such hook.
- Impact: Apps cannot control which declarative page-list changes animate, for example to add pages without a push animation or remove them without a pop transition.
- Cause: Not implemented.
- Fix direction: Add a transition-delegate hook to page-based navigation with Flutter's default decisions.
- Confidence: high. Limitations: Source only; Incular's default animation decisions for page-list diffs were not compared.
