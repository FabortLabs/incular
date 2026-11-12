# anim_core: Animation controllers and tickers

## Summary

Four deterministic frame twins cover controller methods, value/status notifications, finite reversing repeats, simulations, flings, ticker futures, one-shot scheduler callbacks, and time dilation. Flutter tests pass. The comparer reports two matching scenarios and field differences in two scenarios. The `timeDilation` trace is a missing-API probe using non-equivalent settings: Flutter has global scheduler dilation, while Incular only has retained-animation scaling. Five findings remain: two same-configuration behavior differences, one missing scheduler API demonstrated by the trace, and two API integration gaps found by source review.

## Coverage

| Flutter item | Method | Scenario / evidence | Result |
| --- | --- | --- | --- |
| `forward`, `reverse` | Frame trace with 0.2–0.8 bounds and reverse duration | `anim_core__controller_listener_notifications` | Values and statuses follow the same timeline; initial status-listener notification differs (anim_core-002). |
| `animateTo` | Frame trace, including target below current value and explicit duration | `anim_core__controller_listener_notifications`, `anim_core__animate_to_repeat` | Match. |
| Finite `repeat(reverse: true)` | Three-cycle frame trace | `anim_core__animate_to_repeat` | Match, including end status and future completion. |
| `fling` | Default spring, positive velocity | `anim_core__animate_with_fling` | Match. |
| `animateWith` | Deterministic bounded step simulation | `anim_core__animate_with_fling` | Match. |
| `stop`, `reset`, `value` | Stop/cancel, reset to lower bound, same-value assignment, and out-of-range assignment | `anim_core__controller_listener_notifications` | State behavior matches; listener notification rules differ (anim_core-001). Bounds clamp to 0.2–0.8 as expected. |
| Status and controller listeners | Sampled callback counts on every frame | `anim_core__controller_listener_notifications` | Divergent (anim_core-001, anim_core-002). |
| `AnimationBehavior` | Source review and existing Incular test | `crates/incular-animation/tests/animation_behavior.rs` | No frame twin; disabled-motion scaling is covered by the existing targeted Rust test. |
| `Animation<T>` listener mixins | Source review | Flutter `animation.dart` / `listener_helpers.dart`; Incular `animations.rs` | Derived Incular animations have no listener/status-listener interface (anim_core-004). |
| `Ticker`, `TickerFuture` | Frame callback elapsed time plus controller completion/cancellation outcomes | `anim_core__scheduler_callbacks_time_dilation`, all controller scenarios | Completion and cancellation outcomes match. The trace demonstrates that Incular lacks scheduler-wide dilation; the two configurations are not equivalent (anim_core-003). |
| `TickerProviderStateMixin`, `SingleTickerProviderStateMixin` | Source review | Flutter `widgets/ticker_provider.dart`; Incular `AnimationController::with_vsync` | No state-provider equivalent; see TickerMode row. |
| `TickerMode` | Source review and existing retained-tree tests | `crates/incular-widgets/tests/ticker_mode.rs` | Existing tree-driven controller coverage; a standalone `with_vsync` controller is not automatically muted by an ambient `TickerMode` (anim_core-005). |
| `timeDilation` | Source review and diagnostic frame trace at Flutter `timeDilation = 2` | `anim_core__scheduler_callbacks_time_dilation` | Missing API: Flutter reports 8,000 µs vs Incular's raw 16,000 µs at frame 2 / 32 ms. Incular has retained-animation scaling only; these settings are not equivalent, so this is not a same-input parity conclusion (anim_core-003). |
| Scheduler callbacks | Schedule and cancel a begin-frame callback; register a post-frame callback | `anim_core__scheduler_callbacks_time_dilation` | Counts and one-shot behavior match. Persistent callbacks were not frame-tested because neither API can unregister them and they would leak into later scenarios. |
| Custom/unbounded bounds | Source review only | `AnimationController.unbounded` / `AnimationController::unbounded` | Not frame-tested; the twin covers finite custom bounds and clamping. |

## Findings

### anim_core-001 — Controller value listeners are change-gated

- Severity: P2
- Category: behavior
- Flutter: `packages/flutter/lib/src/animation/animation_controller.dart:372-375` calls `notifyListeners()` on every `value` assignment; `:941-954` calls it on every ticker tick.
- Incular: `crates/incular-animation/src/controller.rs:768-782` notifies only when the value or status changed; `:785-832` calls `notify_value()` only when the tick changed state.
- Evidence: `anim_core__controller_listener_notifications` first diverges at frame 1 / 16 ms: Flutter has 1 value notification and Incular has 0, although both still have value 0.25. After the same-value assignment at frame 5 / 80 ms, Flutter increments its count again and Incular does not.
- Impact: controller listeners miss Flutter callbacks on the first zero-elapsed tick and on equal-value assignments. Clients using callbacks for side effects or to synchronize another value observe fewer events.
- Likely cause: public listener dispatch shares the state-change guard used for reactive invalidation.
- Fix direction: preserve Flutter callback dispatch for every controller tick and value setter, while keeping dependency invalidation change-sensitive if that is required by the retained tree.
- Confidence: high

### anim_core-002 — First forward status notification is suppressed for an intermediate initial value

- Severity: P2
- Category: behavior
- Flutter: `packages/flutter/lib/src/animation/animation_controller.dart:932-938` initializes `_lastReportedStatus` to `dismissed`; `_checkStatusChanged` reports the first `forward` status when the run starts.
- Incular: `crates/incular-animation/src/controller.rs:462-469` initializes `reported_status` to the status implied by `with_value`, so a first `forward` from 0.25 is treated as already reported; `:1195-1202` suppresses the callback when the status matches that stored value.
- Evidence: `anim_core__controller_listener_notifications` first diverges at frame 1 / 16 ms: Flutter reports one status notification after `forward()`, while Incular reports none. Both report status `forward` and value 0.25.
- Impact: a status listener registered before the first run does not receive Flutter's initial `forward` transition when the controller starts at an interior value.
- Likely cause: Incular's reported-status baseline follows the initial value; Flutter's baseline remains `dismissed` until a status callback is dispatched.
- Fix direction: initialize the reported-status baseline independently from the current value to match Flutter's first-run notification behavior.
- Confidence: high

### anim_core-003 — Scheduler-wide time dilation is not implemented

- Severity: P2
- Category: missing-api
- Flutter: `packages/flutter/lib/src/scheduler/binding.dart:37-53` exposes global `timeDilation`; `:1116-1123` scales frame timestamps before tickers and callbacks receive them.
- Incular: `crates/incular-animation/src/scheduler.rs:184-200` computes ticker elapsed time directly from raw `Instant`s. `crates/incular-widgets/src/tree/interaction.rs:2037-2055` scales the retained animation clock, but that scale does not affect standalone `Ticker` callbacks or scheduler timestamps.
- Evidence: Flutter exposes `timeDilation = 2`; Incular has no equivalent scheduler setting. As a diagnostic, the twin sets Incular's retained-animation scale to 0.5 to match the controller value while sampling a direct `Ticker`: frame 2 / 32 ms reports 8,000 µs in Flutter and 16,000 µs from Incular's raw ticker. This shows the missing scheduler API; the settings are not equivalent and the elapsed values are not presented as a same-input parity failure.
- Impact: Incular callers cannot slow scheduler timestamps, ticker callbacks, and self-ticking controllers together as Flutter does.
- Likely cause: Incular scales the retained widget animation clock rather than all frame timestamps; it has no scheduler-wide time-dilation API.
- Fix direction: add scheduler-level time scaling for frame timestamps, or explicitly provide an equivalent per-runtime scheduler setting that also affects tickers and frame callbacks.
- Confidence: high

### anim_core-004 — Derived animations cannot register Flutter-style listeners

- Severity: P2
- Category: missing-api
- Flutter: `packages/flutter/lib/src/animation/animation.dart:207-230` requires value and status listener registration/removal on `Animation<T>`; `listener_helpers.dart` supplies listener mixins for derived animations.
- Incular: `crates/incular-animation/src/animations.rs:16-31` defines `AnimationSource` with value, status, and controller access only; `DoubleAnimation` at `:69-95` exposes reads and composition methods but no listener interface.
- Evidence: source review. Incular `AnimationController` itself has listeners, but curved/reversed/driven/compound `DoubleAnimation` values cannot be observed through the Flutter `Animation<T>` listener protocol.
- Impact: callers cannot attach a listener to a derived value or derived status; they must listen to underlying controllers and re-compute the graph themselves.
- Likely cause: derived Incular animations are designed as lazily sampled reactive values rather than listener-bearing `Animation<T>` objects.
- Fix direction: expose listener subscriptions on animation graph nodes, or provide a documented adapter that propagates value and status notifications through derived nodes.
- Confidence: high

### anim_core-005 — `with_vsync` tickers do not inherit `TickerMode`

- Severity: P2
- Category: missing-api
- Flutter: `packages/flutter/lib/src/widgets/ticker_provider.dart:317-405` and `:442-500` bind state-created tickers to the surrounding `TickerMode` and update mute state when it changes.
- Incular: `crates/incular-animation/src/controller.rs:494-520` creates a scheduler ticker and exposes manual muting. Automatic tree muting is in `crates/incular-widgets/src/tree/interaction.rs:2146-2170`, which finds `TickerMode` around retained animation ticker nodes.
- Evidence: source review. A controller ticking through `with_vsync()` is scheduled outside a retained `AnimationTicker` node, so an ancestor `TickerMode` cannot automatically mute it; the caller must wire `set_ticker_muted` manually. Existing `ticker_mode.rs` tests cover retained-tree tickers, not this path.
- Impact: wrapping a self-ticking controller's widget subtree in `TickerMode(enabled: false)` does not silence that controller as Flutter's widget-aware ticker providers do.
- Likely cause: Incular's self-scheduling `with_vsync` ticker has no widget/environment provider connection.
- Fix direction: provide a widget-aware ticker/provider that updates controller mute state from the active `TickerMode`, or make the ownership path explicit in the public API.
- Confidence: high

## Pixel scene proposals

None. The differences are listener and scheduler timing semantics, which scalar frame values can capture.

## Harness requests

- Add an isolated way to test `addPersistentFrameCallback` without leaking a permanent callback across all frame scenarios.
- When a widget-aware ticker provider exists, add a Flutter/Incular frame twin that toggles `TickerMode` during a running animation.

## Open questions

- Whether a global `timeDilation` equivalent belongs on Incular's scheduler or whether the existing retained-animation time scale should be extended to all scheduler consumers.
- Whether derived animation listener methods should be part of `AnimationSource` or supplied by a listener adapter.
