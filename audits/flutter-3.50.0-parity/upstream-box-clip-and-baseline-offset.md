# Box clipping defaults and missing baselines

Three exact pinned Flutter 3.50.0 registrations in `rendering/box_test.dart`
passed: clip behavior, minimum offset, and baseline translation.

The clip case uses a centered 200×200 child in a loose 100×100 viewport.
Native layout reports unclipped overflow for both omitted and explicit
None, not for the three clipping modes. The emitted render command uses
the requested viewport clip and preserves the visible child; save-layer
mode is distinguished from ordinary antialiasing. The retained follow-up
uses a fitting 80×80 child, preserves the proxy's render identity, and
checks that the obsolete clip is gone. The native diagnostic is a counter,
not Flutter's global debug error callback.

`BaselineOffset` treats a missing baseline as the identity of minimum
and translation does not synthesize one. The original arithmetic inputs
are tested and the same helper now combines measured-and-translated
baselines in OverflowBar and Stack/IndexedStack.

All 30 native cases passed across the five focused layout, constraints,
overflow and baseline targets. No full workspace or real GPU run.
