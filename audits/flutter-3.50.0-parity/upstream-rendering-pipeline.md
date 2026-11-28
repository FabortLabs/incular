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

## Global demand reaches each retained view

Flutter's pinned `rendering/binding_pipeline_manifold_test.dart:36`
passed. A global handle enables semantics on both the root pipeline
and an adopted child pipeline; releasing it disables both. Incular
does not create a dummy render owner above its window roots.
`Application::ensure_global_semantics` expresses the same scope over
the actual retained roots of one application.

The native test starts with two clean opted-out views, acquires two
handles, and opens another view before any release. Every view
publishes its own label. The first release has no effect. A window
closed under demand can be replaced in the same slot; the new
generation inherits demand, the old one is unavailable, and the final
release wakes the host and removes the primary view's native node.
An independently held window request stays effective without waking
that view. A reentrant release listener can restore app-wide demand
without sending stale or duplicate state to other listeners. All ten
focused native tests passed.

## Layout dirtiness wakes the native scheduler

Flutter's pinned `rendering/binding_pipeline_manifold_test.dart:22`
passed. Marking a clean render object in an adopted pipeline as
needing layout calls the binding's visual-update callback once.

Incular's public layout delegate can change without a widget rebuild.
Before this fix its retained child view reported frame demand but
never woke an idle native host (0 wakes instead of 1). Its first
dirty dependency now wakes the scheduler without enqueueing a
no-op UI message. Two changes before the frame produce one wake
and paint the latest of the two exact requested sizes. An independent
window stays clean, the root builder runs only once, another change
after painting wakes again, and the detached delegate no longer
wakes the host. The focused pipeline, task wake, and semantics
activation targets passed all 14 tests.

## View metrics request presentation without widget subscribers

Flutter's pinned `rendering/binding_test.dart:13` passed. Before the
Incular fix, new native metrics could leave a mounted view idle if no
widget read the ambient environment. Its physical view configuration
and layout constraints still changed, so the runtime now requests
presentation independently of those subscriptions. The window fixture
checks the semantic bounds changing from 200×150 to 400×300 logical
pixels, a scale change leaving the logical size alone, unrelated and
unknown windows staying idle, and an identical payload being ignored.
All ten focused pipeline, constraint, and multi-view tests passed.

This is partial against the original registration. Flutter's binding
can register a RenderView without a child and forces a frame for a
global metrics callback with a mounted child even if the new values
are identical. Incular registers mounted roots and coalesces duplicate
per-window metric payloads.
