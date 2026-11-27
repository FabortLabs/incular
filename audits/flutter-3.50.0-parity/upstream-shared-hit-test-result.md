# Shared hit-test path and transform stack

All three exact pinned Flutter 3.50.0
`gestures/hit_test_test.dart` registrations passed. Native tests compare
the same three operations against the library's shared hit result:

- Both wrappers see the same live path identity and entries appended by
  either wrapper in order. Existing entries retain their captured
  transform even after the stack changes.
- A translation, one-radian rotation, and independent x/y scale compose
  in the child-from-parent order. A wrapper observes pushes and pops
  after its creation. Repeated adds do not multiply a transform twice.
  A separate point check computes the composed coordinates directly.
- Offset pushes are caller-to-child, not paint offsets to be inverted.
  The same stack interleaves offsets and matrices, and fully popping it
  restores identity.

Owned and box-scoped hit tests are unchanged and do not allocate a shared
wrapper unless requested. Shared handles require releasing read borrows
before mutation. All tested matrices are planar, as in the originals;
this does not claim perspective/3D pointer parity. Three native
comparisons and four box-coordinate regression tests passed.
