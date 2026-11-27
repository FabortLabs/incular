# Independent multi-tap contacts and deferred timer order

Both exact pinned Flutter 3.50.0 `gestures/multitap_test.dart`
registrations at :13 and :78 passed. The native replay uses the original
coordinates and IDs: finger 5 completes tap-up then tap, the other
finger stays held until its long-tap callback fires at its last position,
then movement beyond slop cancels only that finger. The touch-only replay
interleaves mouse down, small and large moves, and up; none emits a
callback or timer. Canceled or released contacts do not fire a stale
long-tap timer. These are standalone recognizer bindings, not assertions
about OS touch routing.

The authored pinned control registers IDs 40 and 7 at the same time and
ID 2 later, then drains all timers together. Flutter delivered
`40, 7, 2`; Incular previously emitted `2, 7, 40` because it iterated
by numeric pointer ID. The native fix orders the due callbacks by deadline,
then registration order. Polling again produces no duplicate.
Twenty-two focused recognizer tests and three hit-path controls passed.
