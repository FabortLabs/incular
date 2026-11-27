# EditableText configuration and zero geometry

The pinned obscured multiline and zero-area originals pair. A one-line
obscured field mounts normally. The final Rust descriptor rejects a
bounded or unbounded multiline obscured field with a typed configuration
error, both at mount and when updating an existing field. This happens
before changing the old tree. A valid unobscured multiline replacement
still works. The Flutter debug constructor asserts on the same invalid
configuration; Rust validates the assembled builder instead.

A mounted raw editor at size zero accepts a new collapsed controller
selection without turning into a nonzero field or throwing in layout
or paint. The separate caret regression remains partial: the reference
uses a fully zero 4×4 matrix with an undefined homogeneous denominator.
Incular rejects that projective matrix by design; the retained affine
zero-scale control still exposes finite local caret bounds. All three
pinned originals and 38 focused native tests passed.
