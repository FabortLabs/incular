# focus_input: Focus and input

## Summary

Seven deterministic frame scenarios cover basic focus traversal, a typed shortcut/action, PageDown scrolling, mouse enter/hover/exit, pointer down/move/up, and the default IgnorePointer/AbsorbPointer cases. Five scenarios match. Two differences are confirmed: `FocusScope` defaults to widget order in Incular while Flutter resolves to reading order, and PageDown does not scroll a focused Incular list. The pre-existing childless `SizedBox` hit-test finding remains supported by code reading. Scope combinations and secondary input phases remain untested; they are listed below.

## Coverage

| Flutter item | Incular counterpart | Method | Scenarios | Result |
| --- | --- | --- | --- | --- |
| Focus, FocusScope, FocusNode; autofocus and one Tab transition | `Focus`, `FocusScope`, `FocusNode` | scenario + reading | `focus_input__focus_scope_default_traversal` | divergent |
| Default traversal policy and next-focus order | `FocusScope` / `FocusTraversalGroup` | scenario + reading | `focus_input__focus_scope_default_traversal` | divergent |
| Shortcuts, Actions, Intent through a focused action detector | `Shortcuts`, `Actions`, `FocusableActionDetector` | scenario + reading | `focus_input__shortcuts_action_intent` | match |
| CallbackShortcuts activator registration and callback routing | `CallbackShortcuts` | reading + existing tests | `action_focus.rs`, `action_wrappers.rs` | partial |
| Keyboard-driven vertical page scrolling | WidgetsApp shortcut map / `ScrollAction` | scenario + reading | `focus_input__keyboard_page_scroll` | divergent |
| MouseRegion enter, hover, exit | `MouseRegion` | scenario + reading | `focus_input__mouse_region_enter_hover_exit` | match |
| Listener pointer down, move, up | `Listener` | scenario + reading | `focus_input__listener_pointer_lifecycle` | match |
| IgnorePointer and AbsorbPointer default hit testing | `IgnorePointer`, `AbsorbPointer` | scenario + reading | `focus_input__ignore_pointer_falls_through`, `focus_input__absorb_pointer_blocks_sibling` | match |
| Hit testing of childless SizedBox and stacked pointer regions | `SizedBox`, retained hit testing | reading + existing tests | none; see `focus_input-001`, `pointer_hover.rs`, `pointer_hit_test.rs` | partial |

This covers 7 groups with a frame twin and 2 by source/test reading only. Several parameter and lifecycle combinations within those groups remain unchecked, as detailed under Open questions.

## Findings

### focus_input-001 — Empty SizedBox claims hits

- Severity: P1
- Category: input
- Flutter: `widgets/basic.dart:2732`, `rendering/proxy_box.dart:214`, `rendering/box.dart:2953` and `rendering/box.dart:2975` — SizedBox uses a constrained proxy box, which hits only a child or a self-hit; the default self-hit is false.
- Incular: `crates/incular-widgets/src/layout/basic/sizing_constraints.rs:130` replaces an absent child with a transparent Box; `crates/incular-widgets/src/tree/painting.rs:121` claims leaf hits.
- Evidence: code reading of the references above.
- Impact: in a Stack, a childless sized box above an interactive sibling can intercept a pointer that Flutter would pass through.
- Likely cause: a layout placeholder is represented as a hit-testable transparent painted box.
- Fix direction: keep the placeholder layout-only or distinguish its hit testing from a colored box.
- Confidence: high

### focus_input-002 — FocusScope traverses widget order by default

- Severity: P2
- Category: default-value
- Flutter: `widgets/focus_traversal.dart:63` uses `ReadingOrderTraversalPolicy` when no traversal group policy applies; `widgets/focus_traversal.dart:2041` also defaults FocusTraversalGroup to reading order.
- Incular: `crates/incular-widgets/src/focus_keyboard.rs:105` sets `FocusScope`'s default policy to `WidgetOrder`; `:140` applies that policy to its child.
- Evidence: scenario `focus_input__focus_scope_default_traversal`: frame 0 geometry and initial focus match. After Tab, first divergence at frame 2 (32 ms), `value.first_focused`: Flutter `1.0`, Incular `0.0`; `value.last_focused`: Flutter `0.0`, Incular `1.0`.
- Impact: when visual reading order differs from widget construction order, Tab moves to a different control.
- Likely cause: Incular's `FocusScope` installs WidgetOrder even when the caller supplies no policy; Flutter inherits its reading-order default.
- Fix direction: make the default policy follow Flutter's reading order, while retaining the explicit widget-order option.
- Confidence: high

### focus_input-003 — PageDown does not scroll a focused list

- Severity: P1
- Category: input
- Flutter: `widgets/app.dart:1334` maps PageDown to a page `ScrollIntent`; `widgets/app.dart:1413` installs `ScrollAction`; `widgets/scrollable_helpers.dart:437` calculates a page as 80% of the viewport and `:503` animates to it over 100 ms with easeInOut.
- Incular: `crates/incular-runtime/src/frame.rs:1881` dispatches key events, then handles escape, menu arrows, activation, Tab and text editing through `:1964`; this path has no default PageDown-to-scroll action.
- Evidence: scenario `focus_input__keyboard_page_scroll`: frame 0 row geometry, focus, and offset match (`row0` rect `[0,0,800,50]`, focused `1`, offset `0`). First divergence at frame 3 (48 ms), `value.offset`: Flutter `24.4631767`, Incular `0`; offsets diverge for 9 frames, max difference `480` px. Flutter reaches `480` px at 144 ms; Incular remains at `0`.
- Impact: keyboard users cannot page through a focused Incular scroll view with PageDown.
- Likely cause: Flutter's WidgetsApp provides the PageDown shortcut and contextual ScrollAction; Incular runtime has no matching default mapping/action.
- Fix direction: add the default PageDown intent/action path for a focused scroll view, using Flutter's 0.8 viewport increment and 100 ms easeInOut motion.
- Confidence: high

## Pixel scene proposals

None. These input behaviors are not static paint comparisons.

## Harness requests

None.

## Open questions

- FocusNode ownership, external focus requests, focus callback order, reparenting, and disposal were not frame-compared. Existing widget/runtime tests cover some lifecycle contracts, but not their Flutter frame behavior.
- Focus traversal with Shift+Tab, Ctrl+Tab, explicit FocusOrder, nested groups, traversal exclusions, and edge behavior was not frame-compared. Existing tests cover some Incular-only ordering cases.
- CallbackShortcuts, shortcut priority across nested scopes, key-up/repeat activators, disabled/ignored Actions, and action fallthrough were not exercised in the frame harness. Existing tests cover portions of these APIs.
- Keyboard scrolling was tested only for a vertical PageDown with a focused list child. Arrow/line increments, horizontal and reverse axes, custom increments, controller fallback, and rejected physics remain unchecked.
- MouseRegion was tested for one mouse entering, moving within, and leaving a single region. Overlapping opaque/translucent ordering and cursor fallback have Incular unit tests but no matching frame twin; touch/pen transitions and multiple mouse devices were not compared.
- Listener was tested for down/move/up only. Hover, cancel, pointer signal, trackpad gestures, and non-default hit-test behaviors remain unchecked.
- IgnorePointer and AbsorbPointer were tested with their default `true` settings over a sibling. The `false` settings and semantics changes are unchecked.
- The frame recorder does not report hit-test targets directly. Pointer callback counters were used for the tested hit-test cases; other transformed/clipped hit-test combinations were not compared.
