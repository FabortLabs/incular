# Focused frame rechecks, batch twelve

Flutter 3.50.0 `ab2912035e` vs Incular `c647cb85`, fresh strict frames.
Cumulative: **64/253 existing registrations (25.30%), 55 match, nine
differ**.

| Scenario | Result | Frames |
| --- | --- | ---: |
| `navigation__overlay_portal__content_outside_anchor_clip` | match, repeat | 5 |
| `navigation__navigator__fade_push_timeline` | mismatch | 25 |
| `navigation__hero__push_flight_navigator` | mismatch | 25 |
| `navigation__pop_scope__route_back_blocked` | mismatch | 7 |
| `semantics__properties_state` | match | 2 |

- Fade push: top opacity first differs at 48 ms (0 vs about 0.009);
  below-route opacity at 64 ms is 0.9733 vs 1, with a maximum differing
  opacity of 0.5. This is distinct from the existing fade-pop check.
- Hero push: the destination probe reports different opacity and
  geometry. At 64 ms its rect is [100, 150, 180, 210] vs about
  [10.83, 21.20, 51.20, 61.39]; the source probe opacity is 1 vs 0.
  These are probe values, not a screenshot or proof that the two
  probes identify the same in-flight overlay child. The visible
  flight needs a separate controlled comparison.
- Route back blocking: only underlying root opacity differs
  (first 48 ms, 1 vs 0, four frames total). The strict report does
  not show a difference in the recorded blocked-back values.

Scratch batch `20261124-153306-124`; [focused runner](run-focused-frames.ps1).
