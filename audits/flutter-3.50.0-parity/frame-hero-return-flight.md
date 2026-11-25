# Hero return flight

Reference: Flutter 3.50.0, `ab2912035e160d9cad6306df307576da6128ebdb`,
`packages/flutter/lib/src/widgets/heroes.dart` (flight manifest, `start`,
and `_maybeStartHeroTransition`).

A controller can emit repeated notifications before layout and paint
republish the route's hero boxes. The previous two-listener-tick fallback
could therefore consume the entire wait on one frame and measure the
popped hero when its old layer was gone and its replacement had not
painted yet. Flutter starts after the route's frame; Incular now takes
its geometry from that same post-frame phase. The next frame paints the
overlay over both routes. Pop drives its geometry with the reverse of
the curved route animation; the shuttle builder still receives the
curved route animation itself. The entering hero supplies the push
curve, and the departing hero supplies the pop's `reverse_curve` (or
its flipped forward curve when absent).

| Exact existing frame scenario | Strict result |
| --- | --- |
| `navigation__hero__push_flight_navigator` | 25/25 match |
| `navigation__hero__pop_flight_navigator` | 47/47 match |

The previous absence at 416 ms is now a strictly matched shuttle. No
probe or tolerance changes. Interruption, gesture-driven pop, nested
navigators, and disappearing endpoints are separate cases; this result
does not verify them.

The original Flutter `Default Hero animation is fastOutSlowIn`
(`widgets/heroes_test.dart:732`) and `Default popped hero uses fastOutSlowIn
curve` (`:3400`) pass on the pin. Their Incular twins in
`flutter350_hero_regression` assert the same initial and terminal sizes
and quarter-step size progression; neutral route and widget hosts replace
the design-system surfaces. Additional native controls check center
alignment, actual paint, a linear push, a custom flipped pop, and an
explicit reverse curve.
