# LayoutBuilder constraints and rebuild invalidation

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

Five selected originals fully match. LayoutBuilder receives
100×200 parent limits and returns 50×100 parent and child
geometry (:10). Text in a tight zero area completes (:41).
When only MediaQuery changes, a retained non-reader does
not rebuild (:260), a whole-scope reader rebuilds once
(:289), and an inner Builder with both changed constraints
and MediaQuery still rebuilds once in the frame (:319).
See `flutter350_layoutbuilder_dependencies`.

The two layout-skipped-subtree originals (:906,:956) passed
on Flutter: a previously laid-out hidden builder can rebuild
using its last constraints; a never-laid-out hidden builder
cannot. These are reference-only. We did not find a Rust
equivalent to mounted opaque OverlayEntry with maintainState
that also skips layout and supports the same explicit mark-
needs-build. Visibility or IndexedStack would change the
layout contract and would not be equivalent substitutes.
