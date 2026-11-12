# misc: Other neutral widgets

## Summary

Expansible changes its body immediately rather than animating its height. The remaining miscellaneous widgets have not yet been audited.

## Coverage

| Flutter item | Incular counterpart | Method | Scenarios | Result |
| --- | --- | --- | --- | --- |
| Expansible expansion animation | incular_widgets::Expansible | reading | none | divergent |
| Other miscellaneous APIs | various | not checked | none | not checked: audit incomplete |

## Findings

### misc-001 — Expansible adds and removes its body without animation

- Severity: P1
- Category: behavior
- Flutter: `widgets/expansible.dart:193` describes the animated height, and `:340` creates the animation controller; defaults are 200 ms and Curves.ease at `:230` and `:235`.
- Incular: `crates/incular-widgets/src/utilities.rs:39` builds a Column when expanded and only the header when collapsed.
- Evidence: code reading of the references above.
- Impact: expanding or collapsing the body jumps instantly instead of moving smoothly, and animation style and builder behavior cannot be reproduced through this API.
- Likely cause: expansion is represented as an immediate conditional composition.
- Fix direction: retain expansion state and animate body height with the configurable lifecycle and builders.
- Confidence: high

## Pixel scene proposals

None.

## Harness requests

None.

## Open questions

Other Expansible lifecycle details and miscellaneous widgets need independent review.
