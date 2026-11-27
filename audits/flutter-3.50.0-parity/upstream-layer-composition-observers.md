# Composition callbacks on retained layers

Five exact pinned Flutter 3.50.0 layer originals passed. They cover
callback presence after registration, cancellation and detachment
from a parent, and actual notification for both a freshly built
scene and a clean retained one.

Incular's UI-thread subscriptions fire on retained scene publication,
even when the previous command buffer is reused. Observers receive
the original layer identity; dropping or cancelling a subscription
removes it, and a detached retained layer no longer contributes to
the previous ancestor's observer query. The observer pass skips
unobserved branches and does not dirty an otherwise clean scene.

All 26 focused native observer, topology, flatten reuse and
annotation tests passed. These are scene-publication observers,
not notifications that a GPU texture was presented to a screen.

## Detachment and observer lifetime

Five more pinned lifecycle registrations passed. In the three native
pairs, explicitly detaching a scene root notifies children before the
parent and preserves subscriptions for the next attachment; two observers
keep a branch observed until both are removed; and cancellation after
destroying the layer is safe. All 20 focused native observer and topology
regressions passed.

Double cancellation is a deliberate difference: the Rust subscription
treats it as idempotent so explicit cancellation and scope drop compose
safely. Flutter's second cancellation asserts. The runtime mutation-
assertion original remains reference-only: the Rust observer receives a
layer ID rather than a mutable layer. These two registrations add no
paired-match credit. The native detachment fixture exercises the explicit
root lifecycle, not a window-system presentation event.
