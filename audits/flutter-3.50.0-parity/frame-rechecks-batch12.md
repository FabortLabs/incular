# Focused frame rechecks, batch twelve

Flutter 3.50.0 `ab2912035e` vs Incular `c647cb85`, fresh strict frames.
Cumulative: **63/253 valid paired outcomes (24.90%), 55 match, eight
differ**; one fixture not equivalent.

| Scenario | Result | Frames |
| --- | --- | ---: |
| `navigation__overlay_portal__content_outside_anchor_clip` | match, repeat | 5 |
| `navigation__navigator__fade_push_timeline` | mismatch | 25 |
| `navigation__hero__push_flight_navigator` | fixture-not-equivalent | 25 |
| `navigation__pop_scope__route_back_blocked` | mismatch | 7 |
| `semantics__properties_state` | match | 2 |

- Fade push: top opacity first differs at 48 ms (0 vs about 0.009);
  below-route opacity at 64 ms is 0.9733 vs 1, with a maximum differing
  opacity of 0.5. This is distinct from the existing fade-pop check.
- Hero push is a **non-equivalent fixture**, not a product finding.
  The raw destination probe at 64 ms is [100, 150, 180, 210] vs
  about [10.83, 21.20, 51.20, 61.39], source alpha 1 vs 0. The
  Flutter twin creates a bare Navigator without a HeroController
  (`harness/flutter/test/frames/navigation_test.dart:407–420`;
  recorder root `recorder.dart:97–109`). Flutter requires a
  controller to drive the flight (`widgets/heroes.dart:827–841`).
  The Rust twin explicitly attaches a controller and mounts the
  overlay (`harness/incular/tests/frames/navigation.rs:322–345`).
  These therefore sample a Flutter route endpoint versus an active
  Incular shuttle. The intended next comparison must attach a
  controller to both and give the two actual shuttles a separate,
  uniquely identified probe, or compare frame pixels.
- Route back blocking: only underlying root opacity differs
  (first 48 ms, 1 vs 0, four frames total). The strict report does
  not show a difference in the recorded blocked-back values.

Scratch batch `20261124-153306-124`; [focused runner](run-focused-frames.ps1).
