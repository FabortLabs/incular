# Align and SizedBox public geometry

All seven Align originals and all five SizedBox originals passed on pinned Flutter 3.50.0. The focused Rust targets pass the layout, physical view metrics, and DevTools checks.

## Align
- :9 fully matches: custom physical point, default center, literal childless Align, childless directional top-start under RTL, then physical childless Align again.
- :24 and :49 fully match: a 100×80 child occupies x=0–100 for top-start in LTR and x=700–800 in RTL, while absolute top-left remains x=0–100. Changing direction on the mounted tree preserves the same keyed child. Missing direction in a child-bearing logical Align produces the typed configuration error.
- :74 fully matches: Align inside a vertical scroll view keeps the finite 800px width, shrinks the unbounded axis to its 10px child.
- :87 and :102 fully match: width and height factors independently produce the specified 50px extent in their real Row and Column environments.
- :119 fully matches: physical metrics at 0×0 before the first Runtime frame, with the centered Align and Placeholder retained through subsequent expansion and contraction.

## SizedBox
- :10 fully matches: requested dimensions are observed before layout, including a missing size, width-only, both finite dimensions, both positive infinities, and both explicit zeros.
- :40 and :65 fully match all seven retained sizes both without a child and with a Container. Under child layout the height-only zero case preserves the full 800px width, whereas an empty box becomes 0×0.
- :118 fully matches the effective 100×100 tight constraints observed by a real child LayoutBuilder.
- :127 fully matches: 0×0 physical metrics before the first Runtime frame; the same centered SizedBox and Placeholder expand and return to zero with the view.

Focused native targets: `incular-widgets --features devtools --test flutter350_align_sized_box_parity` and `incular-runtime --test flutter350_basic_zero_area`. The runtime models the physical view change through platform events, not a real desktop window.
