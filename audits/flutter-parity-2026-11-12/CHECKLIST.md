# Flutter parity audit checklist

Status at the clean stop on 2026-11-13: **incomplete**. See `SUMMARY.md`, `inventory.md`, and each package's findings for evidence and current limits. Candidate-row counts in the summary are provisional, and partial coverage does not mean an item is fully audited.

## Inventory and package coverage

- [ ] Reconcile the declaration index with every in-scope Flutter file and public item; remove out-of-scope candidates and find omitted declarations.
- [ ] Verify each item's package assignment and Incular counterpart, including unresolved names; record missing APIs only after searching.
- [ ] Record for each item what was checked by scenario or reading, and explicitly explain any unchecked behavior.
- [ ] Complete animation core, curves, composition and scheduler behavior, including interruptions, listeners, ticker mode and timing.
- [ ] Complete physics simulations and scroll physics, including edge cases and existing Flutter-value test coverage.
- [ ] Complete implicit animations, transitions, switchers, animated collections and render-object animations.
- [ ] Complete navigation, routes, overlays, heroes, router and back-dispatch behavior.
- [ ] Complete scroll positions and activities, scrollable widgets, page and wheel views, notifications and overscroll effects.
- [ ] Complete sliver layout, cache behavior and pinned, floating and resizing headers.
- [ ] Complete gestures, recognizers, drag and drop, dismissal, interactive viewer and reordering. Review the latest gesture scenarios left by the interrupted worker before accepting findings.
- [ ] Complete static layout, paint, text and editing, focus and keyboard input, semantics, and miscellaneous widgets.
- [ ] Reconcile cross-package duplicates and check all known-gap claims against the current code.

## Evidence and validation

- [ ] Independently rerun at least three divergent scenarios per package, or all when fewer exist; confirm matching frame 0 or explain the intended difference.
- [ ] Review every P0 and P1 finding against both source implementations; reject twin mismatches and unsupported claims.
- [ ] Review the latest scroll scenarios left by the interrupted worker. `scroll-005` was independently reproduced and source-checked.
- [ ] Add and compare worthwhile remaining pixel scenes. The completed 21-scene, four-scale run and its limitations are in `pixel-run-2026-11-12.md`; caret timing and other paint questions remain open.
- [ ] Run each package's filtered frame suite without harness errors, then the complete frame suite once when the user permits a larger run.
- [ ] Resolve the nine informational issues reported by `flutter analyze` at the clean stop, and run appropriate targeted formatting and error checks.
- [ ] Update `SUMMARY.md` with verified coverage counts, deduplicated findings, themes, harness changes, limitations and known-gap status; regenerate the merged `findings.jsonl`.

Audit artifacts belong in this folder, not `.tmp`. Do not fix framework behavior as part of the audit. Do not run a large general test suite without reconciling the user's instruction to avoid huge tests.
