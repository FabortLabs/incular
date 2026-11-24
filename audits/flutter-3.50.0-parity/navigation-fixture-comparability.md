# Navigation fixture comparability review

These five existing fixtures were run, but are not full valid pairs. The
original detailed traces are retained; these are fixture dispositions,
not evidence that a behavior mismatch has been fixed.

| Fixture | Diagnostic |
| --- | --- |
| `navigation__hero__push_flight_navigator` | Flutter fixture does not attach a HeroController. Incular runs a flight and Flutter does not. Add equivalent controllers and a unique actual shuttle probe before comparing flight frames. |
| `navigation__navigator__fade_push_timeline` | Flutter explicitly animates the route below from 1 to .5; Rust installs an ordinary unanimated below route. The tiny top-phase mismatch begins at push and also has unequal clock initialization. |
| `navigation__navigator__fade_pop_timeline` | Same below-route and push clock problems. The upper route's observed **reverse** opacity and removal match on all sampled frames. |
| `navigation__navigator__observer_order` | The captured observer sequence matches; only the covered route's opacity probe differs. |
| `navigation__pop_scope__route_back_blocked` | The retained top and blocked-back result/callback probes match. Only the covered route's opacity differs; Flutter `maybePop` and the explicit Rust dispatcher are not universal back-dispatch surface equivalents. |

Flutter `harness/flutter/test/frames/navigation_test.dart:407–441`
wires the unequal Hero/Fade inputs; compare Rust
`harness/incular/tests/frames/navigation.rs:287–345`. Flutter's overlay
skips covered retained children in actual painting
(`widgets/overlay.dart:893–917,1157–1160,1425–1437`), but the recorder
reads local ancestor opacity; Rust explicitly changes retained visibility
(`crates/incular-runtime/src/route_outlet.rs:959–984`). Thus 1 vs 0
does not mean Flutter drew a visible root below an opaque route.

Other source findings remain open: Flutter's `Hero.curve` and `Hero.reverseCurve`
options (`widgets/heroes.dart:174–184,266–274`) have no matching
public controls in Incular's hardwired trajectory
(`crates/incular-navigation/src/hero.rs:159–178,502–506`).
Likewise compare equivalent custom transition pairs before evaluating a
separate secondary-animation pairing gap.
