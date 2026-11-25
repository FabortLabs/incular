# OverflowBox fit and retained sizing

The five selected pinned Flutter executions pass, including both
expansions of the same parameterized registration at `:37`.

- **Defer to child (`:37`, `:68`):** A 100px child makes
  the box 100px tall; a 10,000px child still measures at its full
  height, while the box ends at the 600px parent bound. Removing
  the child collapses the box to the parent's minimum. Incular
  tests all three through the same retained element, including
  removal of the old render child. Both expansions at `:37`
  count as one registration.
- **Max/default:** In a loose 100×80 parent, a 20×10 child
  remains 20×10 and is centered at (40,35). Before the fix
  Incular shrank the wrapper to the child. Both engines retain
  the same box when selecting defer-to-child and switching
  between populated and empty fits. An empty Max box still
  reserves the finite bounds. The previous tight `:10`
  control stays covered in its original evidence.
- **Baseline:** An 80px tall box centers a 100px child at
  -10px, moving its real 40px baseline to 30px. Removing the
  child clears the metric without replacing the box. The direct
  Flutter dry-baseline interface is not implemented in Incular.
- **Diagnostics (`:93`):** The public Max and centered
  defaults agree with the rendered fixture; Dart's diagnostic
  tree and string formatting are not paired.

A Max box under an unbounded parent axis still uses a finite
child extent in Incular rather than producing an infinite render
size. The API above compares bounded layout, not Flutter's debug
diagnostics for an unbounded Max wrapper.
