# Align and SizedBox public geometry

All seven Align originals and all five SizedBox originals passed on pinned Flutter 3.50.0. The focused Rust target passes all nine tests.

## Align
- :9 partial match: absolute point, default center, and a zero-size child analogue mount. There is no optional-child or ambient AlignmentDirectional setter on the Rust Align.
- :24 and :49 partial match: physical top-left puts the 100×80 child at the origin in both ambient directions. The Flutter directional topStart branch of these originals is not represented; passing physical top-right explicitly would not test ambient resolution.
- :74 fully matches: Align inside a vertical scroll view keeps the finite 800px width, shrinks the unbounded axis to its 10px child.
- :87 and :102 fully match: width and height factors independently produce the specified 50px extent in their real Row and Column environments.
- :119 partial match: zero layout area with a Placeholder, no native window resize.

## SizedBox
- :10 partial match: compared six constructors' visible geometry in the same retained environment; this does not inspect private stored nullable/infinite values as fields. The optional-size fromSize() overload is not exposed.
- :40 and :65 fully match all seven retained sizes both without a child and with a Container. Under child layout the height-only zero case preserves the full 800px width, whereas an empty box becomes 0×0.
- :118 fully matches the effective 100×100 tight constraints observed by a real child LayoutBuilder.
- :127 partial match: zero-area layout with Placeholder, not a window resize.

Run Rust with cargo test --locked -p incular-widgets --test flutter350_align_sized_box_parity -- --nocapture.
