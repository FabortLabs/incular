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

## Resuming or extending

- Edit `coverage/<package>.jsonl`, then run `pwsh -NoProfile -File audits/flutter-parity/inventory_tool/render-inventory.ps1` from the repository root. Do **not** run `build-inventory.ps1` or `inventory_tool/build-index.ps1`: they predate the coverage files and would overwrite manual work. `seed-coverage.ps1` was a one-time migration and refuses to run again.
- Findings use the schema in `WORKER-BRIEF.md`; check with `inventory_tool/check-findings.ps1`. Record status and severity decisions in `decisions.json` and apply them with `inventory_tool/apply-decisions.ps1`, then run `consolidate.ps1`.
- Open questions and the harness capabilities needed for the 10 unchecked items are listed in `SUMMARY.md`.

Audit artifacts belong in this folder, not `.tmp`. Do not change framework behavior as part of the audit.
