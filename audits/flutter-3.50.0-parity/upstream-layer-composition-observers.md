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
