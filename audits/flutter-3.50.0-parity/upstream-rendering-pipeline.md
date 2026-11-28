# Renderer binding and accessibility startup

## Semantics already enabled at initialization

The original `rendering/binding_pipeline_manifold_init_test.dart:9` passed
on Flutter `ab2912035e160d9cad6306df307576da6128ebdb`. Its renderer
binding initializes successfully while the binding already reports
semantics enabled.

Incular's `flutter350_semantics_activation` suite adopts a retained
runtime with live semantics demand into a newly initialized application.
The newly bound native consumer immediately receives `true`, the
first frame publishes one node with the expected label, and dropping
the startup demand emits `false`, wakes the idle window, and removes
the same native node on the next frame. A second test models native
host startup: two windows initially opt out, one gets an activation
before drawing, and only that window publishes a label on its first
frame. Headless runtimes retain their deliberate eager default;
native host demand is configured explicitly in this test. No desktop
screen-reader or GPU presentation is claimed. All eight native
tests passed.

## Independent native consumers

The desktop bridge previously sent both AccessKit and platform services'
activation through the same platform boolean. AccessKit deactivation could
clear the tree even when the second consumer remained active. The platform
service now holds independent demand, scoped to its native window, and
requests its initial snapshot even when the tree is already clean.
The focused desktop test starts from a live, idle AccessKit tree, activates
the second consumer, checks the current retained label, and confirms no
nodes are removed when AccessKit deactivates. Releasing the second
consumer then disables collection. This exercises the native bridge's
retained projection, not a live screen reader or browser adapter.
