# Flutter parity audit checklist

Status on 2026-11-13: **complete**. Details are in `SUMMARY.md`.

## Inventory and package coverage

- [x] Reconcile the declaration index with every in-scope Flutter file and public item (1,753 items; four widgets files have no public declarations). Coverage lives in `coverage/<package>.jsonl`; `inventory.md` is generated from it.
- [x] Verify package assignments and counterparts; record `MISSING` only after a search (202 items).
- [x] Record for each item what was checked by scenario or reading, or why it is out of scope (548) or unchecked (10, each blocked on a named harness capability).
- [x] Animation core, curves, composition and scheduler.
- [x] Physics simulations and scroll physics.
- [x] Implicit animations, transitions, switchers and animated collections.
- [x] Navigation, routes, overlays, heroes, router and back dispatch (Navigator-hosted twins via `tests/frames/host.rs`).
- [x] Scroll positions and activities, scrollable widgets, page and wheel views, notifications, sheets and nested scroll views.
- [x] Slivers, cache behavior, pinned, floating and resizing headers.
- [x] Gestures, recognizers, drag and drop, dismissal, interactive viewer and reordering (the earlier interrupted worker's scenarios were reviewed).
- [x] Layout, paint, text and editing, focus and keyboard input, semantics, misc widgets, and widget-relevant rendering/services (`support` package).
- [x] Reconcile cross-package duplicates (scroll-006, semantics-016 and text-018 rejected as duplicates) and check the known-gap claims against HEAD.

## Evidence and validation

- [x] Independently rerun every package's divergent scenarios: the orchestrator reran all filtered suites after the final edits (`VERIFICATION.md`).
- [x] Review every P0 and P1 finding against both implementations. All P0 scenarios were rerun; source-only findings were checked by a separate verifier (`verification-source-findings.md`), and its corrections were applied through `decisions.json`.
- [x] Review the latest scroll and gesture scenarios left by the interrupted workers.
- [x] Pixel comparisons: 24 scenes at four scales at HEAD, with Flutter recaptured (`pixel-run.md`). Limits recorded: no in-motion captures (scrollbar thumb, fades), and no hero flights, obscureText or sliver decorations.
- [x] Run each package's filtered frame suite without harness errors.
- [x] `flutter analyze` (no issues); harness `cargo fmt --check` and `cargo clippy -D warnings`; `cargo check --workspace`.
- [x] Update `SUMMARY.md`, `findings.jsonl` (`consolidate.ps1`), `inventory.md` (`render-inventory.ps1`).
- [x] Complete all-scenario frame run (with the user's permission): 251 scenarios, 146 match; the 105 divergences equal the union of the package suites, with no structural problems.

## Fix pass (branch `fix/flutter-parity`)

Status on 2026-11-15: every accepted finding has a recorded outcome (`FIXES.md`). After the remaining pass below, 176 are fixed, 1 is partial (support-001), 2 were already fixed and 3 are invalid. None is blocked, deferred or out of scope.

- [x] Reassess each accepted finding against current code before fixing it (`fixes/<group>.md`).
- [x] Fix the P0 findings (7/7) and P1 findings (38/38).
- [x] Fix or classify the P2 and P3 findings, recording the reason for each that is blocked, deferred, invalid or out of scope.
- [x] Add regression tests in the owning crates' `tests/` directories, and rerun the related frame scenarios package by package.
- [x] Compare failing tests against a clean checkout of `e25e45bc` to separate regressions from tests that encoded non-Flutter behavior. Fix the regressions; rewrite the outdated tests with Flutter citations.
- [x] Pixel check with `run-pixels.sh --incular-only` (paint fixes and text baseline; see `SUMMARY.md`).
- [x] `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt -p <pkg> -- --check` on changed packages, and `flutter analyze`.
- [x] Full `cargo test-constrained` (2026-11-15, with the user's permission). The first run had 29 failing tests in 16 targets. Triage against the baseline (`fixes/suite-triage-input.md`, `suite-triage-widgets.md`, `suite-triage-alloc.md`) found:
  - real regressions, which are fixed: semantics allocations, text engine allocations, transparent-editor semantics in autocomplete/OTP/number fields, and modal barrier semantic dismissal;
  - the rest were tests encoding non-Flutter behavior, rewritten with citations.
  The rerun passes all 540 test binaries, and workspace clippy is clean.
- [x] All-scenario frame run: 232/252 match. The audit run was 146/251. The 20 divergences are the documented residuals in navigation, scroll, slivers, text editing, focus and semantics. `run-pixels.sh --incular-only` covers all 96 scene/scale pairs: text is unchanged from the earlier text-only run, and the fixed paint scenes stay fixed at 1x, 1.5x and 2x. At 1.25x, small residuals remain (`image_default_fit` 0.25% of pixels over 16 levels); no earlier all-scene run exists to compare them with.
- [ ] Pixel twins for the new scenarios (ShaderMask, paint order, BackdropFilter compose): need a Flutter pixel capture.

## Remaining pass (branch `fix/flutter-parity-remaining`)

The 15 findings left blocked, deferred, out of scope or invalid, one commit each (`fixes/remaining.md`):

- [x] switchers-004, gestures-013, navigation-006, navigation-011, semantics-013.
- [x] paint-012 and paint-014 (image streams, provider images, FadeInImage options).
- [x] slivers-010 (Viewport center and anchor, negative minimum offset).
- [x] scroll-008 (ScrollAwareImageProvider).
- [x] support-005 (NetworkAssetBundle over an injected fetcher).
- [x] support-003 (platform undo manager).
- [x] support-004 (spell check service and misspelled-word styling).
- [x] support-002 (predictive back).
- [x] semantics-009 (mobile adjusted values; AccessKit has no property for desktop).
- [~] support-001 (platform views): partial. Native compositing on WGPU is infeasible; see `SUMMARY.md`.
- [x] Per-crate suites for the touched crates, workspace clippy, and `run-pixels.sh --incular-only` after the text field paint change (no regressions).
- [ ] Full `cargo test-constrained` for the remaining pass (not run: needs the user's go-ahead).

To resume: edit or add `fixes/<group>.jsonl` records (schema in `FIX-BRIEF.md`; the pass order is in `consolidate-fixes.ps1`), then run `pwsh -NoProfile -File audits/flutter-parity/inventory_tool/consolidate-fixes.ps1`.

## Resuming or extending

- Edit `coverage/<package>.jsonl`, then run `pwsh -NoProfile -File audits/flutter-parity/inventory_tool/render-inventory.ps1` from the repository root. Do **not** run `build-inventory.ps1` or `inventory_tool/build-index.ps1`: they predate the coverage files and would overwrite manual work. `seed-coverage.ps1` was a one-time migration and refuses to run again.
- Findings use the schema in `WORKER-BRIEF.md`; check with `inventory_tool/check-findings.ps1`. Record status and severity decisions in `decisions.json` and apply them with `inventory_tool/apply-decisions.ps1`, then run `consolidate.ps1`.
- Open questions and the harness capabilities needed for the 10 unchecked items are listed in `SUMMARY.md`.

Audit artifacts belong in this folder, not `.tmp`. The audit changed no framework behavior; framework fixes belong to the fix pass and are recorded in `fixes/`.
