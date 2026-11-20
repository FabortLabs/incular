# Incular maintainer guide

Incular is a retained-mode Rust UI framework that aims to match Flutter's
behavior and fidelity while being faster and leaner. It runs on desktop
(Windows, macOS, Linux), the web (`wasm32`) and mobile semantic adapters.

You maintain this repository. Treat it as your own codebase, not someone
else's code you are visiting.

## How to work here

- **Own the outcome.** Fix root causes rather than patching symptoms. If you
  find a bug, a slow path or a confusing API while doing something else, fix
  it when it's small. Otherwise write it down (`ROADMAP.md`,
  `INCULAR-GAPS.md`, or `.tmp/` notes) instead of walking past it.
- **Rules serve the code, not the other way round.** Every rule in this file
  has a reason. If a rule, test, lint, architecture boundary or tool setting
  stands in the way of a clearly better design, change it on purpose:
  - Make the change together with the code that needs it: this file,
    `system-design/ARCHITECTURE.md`, `specs/architecture.json`, CI or
    `Cargo.toml`.
  - Give the reason in the commit and, if users notice, in the CHANGELOG.
  - Never work around a rule silently, and never leave code worse just to
    satisfy an outdated rule.
- **Prefer the better design over compatibility.** The project is pre-1.0:
  breaking changes are fine when they make the API clearer, safer or faster.
  Record them in the CHANGELOG with migration notes. Don't keep shims,
  deprecated aliases or dead code around "just in case".
- **Simplify relentlessly.** Delete unused code, merge duplicates, and keep
  modules and functions small enough to read in one sitting. A smaller
  codebase that does the same thing is a better codebase.
- **Verify, don't assume.**
  - Run the checks.
  - Run an example when behavior changes.
  - Back performance claims with a benchmark or profile.
  - Compare against Flutter with the parity harness when fidelity is the
    point.
- **Use modern Rust.** Develop on the latest stable toolchain and use new
  language and std features when they make code clearer, faster or safer
  (see [Rust version policy](#rust-version-policy)).

Reference material:

- [Architecture contract](system-design/ARCHITECTURE.md) and
  [API design](system-design/API_DESIGN.md): the current design. Change them
  when the design changes.
- [Idiomatic Rust guide](IDIOMATIC-RUST.md): the default way to write Rust
  here. Run its self-review checklist (§28) before finishing. Deviate when
  there's a reason, and say why in a comment. Add to the guide when you learn
  something worth keeping.
- [CONTRIBUTING.md](CONTRIBUTING.md): the error and panic policy, and
  checklist for contributors.
- [Parity harness guide](harness/README.md): Flutter comparison tooling.

## Rust version policy

- `rust-toolchain.toml` tracks `stable`. Always develop and test on the
  latest stable release.
- MSRV (`rust-version` in the workspace `Cargo.toml`) is a setting we maintain,
  not a ceiling to code around. Raise it whenever a newer stable feature makes
  the code meaningfully better. A bump is one change that updates:
  - `rust-version` in the workspace `Cargo.toml`;
  - the Rust matrix in `.github/workflows/build.yml`;
  - the "Rust X or newer" lines in `README.md`, `crates/incular/README.md`
    and the docs installation page;
  - a CHANGELOG entry;
  - and removes workarounds that existed only for the old MSRV, such as the
    workspace `deprecated = "allow"` lint and old-API comments.
- Don't write worse code to keep an old compiler working.

## Architecture

The crate boundaries below are the current design. They exist to keep
dependencies one-directional, renderer and platform code isolated, and the
public facade small. If a boundary is wrong, update it here,
`system-design/ARCHITECTURE.md` and `specs/architecture.json` in the same
change. `crates/incular/tests/architecture_contract.rs` enforces the
dependency allow-list and support matrix in that JSON file. Historical parity
plans and audits are input, not authority.

- `incular`: Feature-controlled public facade and application preludes.
- `incular-accessibility`: AccessKit and mobile semantic projections and action translation.
- `incular-android`: Android semantic adapter for a native host.
- `incular-animation`: Curves, interpolation, tweens and animation physics.
- `incular-assets`: Font identities, shared font bytes and asset bundles.
- `incular-config`: Shared constraints, alignment, insets, localization and application policies.
- `incular-controls`: Themed Incular controls and visual slots over neutral widgets.
- `incular-core`: Geometry, identity, input, interpolation and lower-level context values.
- `incular-desktop`: Shared Winit host, WGPU frame coordination and native service integration.
- `incular-devtools`: Opt-in diagnostics transport and runtime command bridge.
- `incular-devtools-protocol`: Versioned diagnostics messages shared with tooling.
- `incular-gestures`: Pointer recognition, gesture arbitration, keyboard and focus mechanisms.
- `incular-image`: Raster image identities, decoding and CPU image caches.
- `incular-ios`: iOS semantic adapter for a native host.
- `incular-layout`: Layout algorithms over measured sizes; config values are exact re-exports.
- `incular-linux`: Linux desktop entry and native services over the shared desktop host.
- `incular-macos`: macOS desktop entry and native services over the shared desktop host.
- `incular-material`: Material presentation composed from controls and neutral widgets.
- `incular-navigation`: Application route stacks, routing, restoration and route composition.
- `incular-platform`: Portable native IDs, events, capabilities, commands, metrics and errors.
- `incular-rendering`: Renderer-neutral commands, paths, canvas and retained compositor layers.
- `incular-runtime`: Application lifecycle, scheduling, reactive dispatch, tasks and frame coordination.
- `incular-scroll`: Scroll controllers, physics, metrics, extent indexing and geometry.
- `incular-semantics`: Platform-neutral semantic nodes, roles, labels, state and actions.
- `incular-text`: Font selection, shaping, metrics, text layout and editing values.
- `incular-wgpu`: Owned native surfaces, GPU resources and execution of rendering commands through WGPU or native Windows D3D11.
- `incular-web`: Browser entry and page services over the shared desktop host.
- `incular-widgets`: Neutral widget composition and retained build/layout/paint/input/semantics.
- `incular-windows`: Windows desktop entry and native services over the shared desktop host.
- `incular-devtools-ui`: Standalone desktop DevTools application over the public facade and diagnostics protocol.
- `incular-hot-reload-example`: Standalone hot reload example application over the public facade.
- `incular-parity-harness`: Frame-by-frame and pixel parity harnesses comparing Incular with Flutter.

## Repository layout

- Framework crates live under `crates/*`; tools live under `tools/*`; the root
  is a virtual workspace. Each crate has `Cargo.toml`, `README.md` and
  `src/lib.rs`.
- The Flutter parity harnesses live under `harness/`: the Flutter reference
  project in `harness/flutter` and the `incular-parity-harness` package in
  `harness/incular`.
- Examples are targets of the `incular` package under `examples/<example>/`.
  `examples/hot_reload` is its own binary package because the hot-patch build
  driver only patches and watches a served package.
- Built-in widgets live in `incular-widgets`, not a separate crate.
- Platform-specific APIs live in their matching native crate.
  Renderer-independent APIs stay out of `incular-wgpu`. No dependency cycles
  between crates.
- **Tests live only in `tests/` directories:**
  - `crates/<crate>/tests/` for crate tests;
  - `crates/incular/tests/` for workspace integration tests;
  - `examples/<example>/tests/` for example tests.

  Never add `#[cfg(test)]` modules, test helpers or test-only implementations
  under `src/` or beside an example's `main.rs`. Tests exercise the public
  surface the way a real caller would.

## Platform facts

These are constraints of the platforms themselves, not project preferences:

- The web target (`wasm32-unknown-unknown`) runs on the shared Winit host.
  `incular-web` only binds page canvases and page services. There is one
  event loop, one input path and one renderer path for every platform.
- Code reachable on `wasm32` must not block the page thread. It must not call
  `std` APIs that panic there: `std::time::Instant::now`,
  `SystemTime::now`, `std::thread`, `std::env::temp_dir`, `std::process::id`.
  Use `incular_core::time` for clocks. A successful wasm compile proves
  nothing at runtime; load the example in a browser.

## Engineering standards

- **Errors vs panics** (full policy in CONTRIBUTING.md):
  - Mistakes by application authors return structured errors or diagnostics.
  - Broken framework invariants may `expect("<invariant>")`.
  - Public APIs panic only as a documented contract, paired with a
    `maybe_*`/`try_*` form.
- **Performance is a feature.**
  - Build, layout, paint, input and frame submission must not allocate in the
    steady state.
  - Skip unchanged work with dirty flags and generations.
  - Watch the size of types that are stored in bulk: paint commands, layers,
    render kinds, semantic nodes.
  - Measure before and after with `benchmarks/` or a profile.
- **`unsafe`** is allowed where it pays for itself. Encapsulate it, write a
  `// SAFETY:` comment, and test it. The workspace denies
  `unsafe_op_in_unsafe_fn`.
- **Public APIs** follow `system-design/API_DESIGN.md` and are documented
  (`rustdoc::broken_intra_doc_links` is denied). When the API or ownership
  changes, update the crate `README.md` and the relevant `docs/` page.
- **CHANGELOG:** record user-visible changes under `Unreleased`. Breaking
  changes start with `**Breaking (`crate-name`):**` and say how to migrate.

## Validation

Before calling a change done:

```text
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
cargo test -p <crate> [--test <file>]   # targeted tests for what changed
```

CI (`.github/workflows/build.yml`) runs the format, clippy, wasm32 clippy,
feature and doc checks on every push, so keep them green locally. A broken
doc link fails CI even when everything else passes. Changes under `docs/`
also need `npm run lint` and `npm run types:check` in `docs/`, which the
Documentation workflow runs.

**Prefer targeted tests.** The full suite (`cargo test-constrained`) can take
more than 30 minutes, so run it only when you truly need it.

- **Targeted tests:** test every crate you changed:
  `cargo test -p <crate>`, or `--test <file>` for one test file. Also test the
  crates that directly depend on an API whose behavior changed. Use
  `crates/incular/tests/` when facade or cross-crate wiring changed.
- **Full suite:** run `cargo test-constrained` only when:
  - shared behavior many crates rely on changed (core types, runtime
    scheduling, the widget tree, layout or rendering semantics);
  - you're preparing a release; or
  - targeted runs can't reasonably cover the blast radius.

  Run it in the background and keep working meanwhile.
- **Mechanical changes:** renames, lint fixes and formatting that clippy and
  `cargo check` fully cover only need the tests of the crates they touch.

- **Windows:** `cargo fmt --all` fails with os error 206 because the command
  line gets too long. Check with `python scripts/check-format.py` and fix with
  `cargo fmt -p <package>`.
- `.cargo/config.toml` limits builds to 4 jobs, turns off incremental builds
  and runs tests on one thread. This keeps the machine's memory in check.
  Use `cargo test-sequential` if the linker still runs out of memory. Run long
  builds in the background.
- Run these when they apply:
  - Facade feature flags: `python scripts/check-features.py`.
  - Doc-site snippets: `python scripts/check_doc_examples.py --compile`.
  - Web-reachable code: `python scripts/web-smoke.py --examples <name>`.
  - Dependencies: `cargo deny check -W unmaintained` and
    `cargo machete --with-metadata`.
- If a check is wrong or flaky, fix the check; don't skip it. Report anything
  you couldn't run, such as other platforms.

## Conventions

- Put scratch notes, audit data, throwaway scripts and generated reports in
  the git-ignored `.tmp/` directory. Keep `git status` free of untracked
  clutter.
- Commit only when asked. Subjects use `type(scope): summary` in the
  imperative mood (`perf(text): …`, `feat(motion): …`, `fix(widgets): …`,
  `docs(changelog): …`). The scope is the crate name without `incular-`. Keep
  unrelated changes in separate commits.
