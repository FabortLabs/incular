# Implicit animation completion timing

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

Five exact original registrations and their Rust counterparts
match the entire callback sequence [0,0,0,1,2,3]. They compare
width and foreground decoration (:69), padding (:94), alignment
(:119), opacity (:368), and default text font style (:512).
No callback fires on mount, just after the retained update, or
at exactly 1000 ms on the first run; a tick 1 ms later and after
each subsequent toggle reports one additional completion.

The original tap is a test-fixture trigger for the property
update; Rust drives the identical retained property change
directly and checks the same elapsed samples. This compares
completion, not input or Dart widget construction. See
`flutter350_implicit_on_end_upstream_parity`.
