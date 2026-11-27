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

## Scaffold connection metadata

The exact pinned dynamic registration at :539 passed. The native writer
generates a real scaffold module containing the same four inputs: the
connection URI, service, stream, and project root. Its temporary file is
published as one replacement. The test rewrites the endpoint, verifies
the old value is gone, and compiles and runs the generated constants
with a project path containing quotes and a backslash. A Unix-only test
verifies that a non-Unicode root fails before touching the valid file.

Two dedicated native module tests and nine nearby generator/manifest
tests passed. Metadata is written to the local scaffold, not used to
open a socket or upload project paths. No running DTD connection is
claimed.

## Full analyzer generation

The exact pinned :316 registration completed successfully before the
bounded runner stopped later registrations. Flutter starts from source
discovery and checks generated factories, builders, annotated theme and
wrapper transformations, collection arguments, and dependency errors.

The six focused native generator and connection-info tests passed again
with `INCULAR_PREVIEW_RUSTC_SMOKE`, compiling and executing the generated
Rust. Only the already-discovered, importable zero-argument factory case
is implemented at this boundary. The broader analyzer registration is
now recorded as a partial match rather than adding paired credit.
