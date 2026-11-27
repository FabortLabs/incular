# Widget preview Rust source generation

Three exact pinned tooling originals in `preview_code_generator_test.dart`
passed: :466, :574, and :599.

- The allocator retains the two scaffold helper modules, package
  namespaces and SDK paths, but omits local source-file imports. It
  still reserves the greatest observed alias, even for an omitted file.
  The original seed retains aliases 1, 2, 3, 5 and 6 and reserves 7;
  allocating another library starts it at 8. Reusing a library does not
  allocate another alias.
- Seeding after a first real allocation fails without replacing the
  previously allocated symbol. Invalid paths, invalid symbols, and
  colliding seed aliases do not partially modify a registry.
- The analyzer-to-source test receives one library preview and one
  package-less root preview. Native generation preserves the first
  preview's factory, label, source URI, line and column, and neither
  names nor imports the root-only preview. A focused smoke test compiles
  the emitted Rust with stub facade/factory types, executes it, and checks
  the bound factory and emitted registration count.

Four focused native generator tests and five manifest controls passed.
The Rust generator accepts portable, trusted analysis records carrying
ASCII identifiers and zero-argument widget factories; this does not
discover Rust previews, generate arbitrary Dart annotation expressions,
run project code, or launch a browser/IDE scaffold.
