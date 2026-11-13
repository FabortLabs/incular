# Worker brief — Flutter parity audit continuation

You are one of several workers auditing Incular (Rust UI framework, repo root `D:\CODEBASE\PROJECTS\INCULAR\incular`) against **Flutter 3.41.7**. Flutter source: `C:/Manual Installs/flutter/packages/flutter/lib/src`. Flutter executable: `C:/Manual Installs/flutter/bin/flutter` (export `FLUTTER` to it when running the harness). Platform: Windows; shells are Git Bash and PowerShell 7. Python is not installed; use perl, PowerShell or Dart for scripting. Inline heredocs with quotes can fail in Git Bash: write scripts to files in your scratch area instead.

**This is an audit. Never change framework code (`crates/`, `examples/`, `tools/`). Never commit. Never delete anything under `harness/out/`. Never put files in `.tmp/`.** You are not alone in the repository: other workers edit other files at the same time. Do not revert, reformat or edit files you do not own. Leave your files compile-stable at all times.

## Read first

- `AGENTS.md`, `harness/README.md` (frame harness rules: same tree, constants, timeline and probe target on both sides; Inter text; 800x600 at DPR 1; fake clock).
- Your package's existing work: `audits/flutter-parity/findings/<pkg>.md` and `.jsonl`, `harness/flutter/test/frames/<pkg>_test.dart`, `harness/incular/tests/frames/<pkg>.rs`.
- `audits/flutter-parity/SUMMARY.md` for accepted findings and themes.

## Files you own

- `audits/flutter-parity/coverage/<pkg>.jsonl` — your inventory rows (one JSON object per line).
- `audits/flutter-parity/findings/<pkg>.md` and `findings/<pkg>.jsonl`.
- `harness/flutter/test/frames/<pkg>_test.dart` and `harness/incular/tests/frames/<pkg>.rs`.

Shared files (`recorder.dart`, `harness/incular/src/lib.rs`, `tests/frames/main.rs`, `harness/flutter/lib/main.dart`, `harness/incular/tests/pixels.rs`, `Cargo.toml`) belong to the orchestrator. If you need a recorder capability, a new dev-dependency or a pixel scene, do not edit them: describe the exact need in a "Harness requests" section of your findings md and in your final report.

## Code changed since earlier findings

The audit's earlier findings were recorded at revision `a425aa2e`. HEAD has later changes in, among others: `crates/incular-widgets/src/tree/{semantics,text,interaction,painting,retained,layout}.rs`, `crates/incular-text/src/editing.rs`, `crates/incular-scroll/src/extent_index.rs`, `crates/incular-widgets/src/scrolling/layout.rs`, and the WGPU renderer. Use `git diff a425aa2e HEAD -- <path>` to check. **Revalidate each existing finding in your package against HEAD**: rerun its scenario if it has one, re-read the cited lines, and update line references. If a finding no longer reproduces, set `"status":"rejected"` with the reason; do not delete it.

## Task

1. **Inventory.** Every row of your coverage file must end in one of these statuses, with a concrete `coverage` sentence saying exactly what was checked or why not:
   - `scenario` / `scenario+reading` — checked by a paired frame scenario (list it in `scenarios`).
   - `reading` — checked by reading both sources (cite Incular `path:line`).
   - `partial-scenario` / `partial-reading` — only the named aspects were checked; name the unchecked aspects.
   - `missing` — no Incular counterpart after a real `rg -n` search (set `counterpart` to `MISSING` and say what you searched for). A missing API with observable widget behavior needs a finding (or a reference to an existing one).
   - `out-of-scope` — say why (e.g. Material/Cupertino-only, embedder/engine, debug/inspector tooling, platform channel plumbing with no widget-observable behavior, pure data class with no behavior beyond fields checked elsewhere). Rows from `rendering/`, `painting/`, `gestures/`, `services/` are in scope only where they explain observable widget behavior.
   - `unchecked` — only with a concrete reason (e.g. needs a harness capability that does not exist — name it).
   Fix wrong `counterpart` paths (they were matched by name and may be wrong; `UNRESOLVED` means not yet searched). Put finding IDs in `findings` and scenario names in `scenarios`. If a row belongs to another package, leave it in your file, keep auditing it if you can, and note `"reassign":"<pkg>"`. If you find an in-scope public declaration missing from the inventory, add a row (`file`, `line`, `name` exactly as in Flutter source). Keep valid JSON on each line; `pwsh -NoProfile -File audits/flutter-parity/inventory_tool/render-inventory.ps1` (from repo root) validates all coverage files and rebuilds `inventory.md` — run it to check your file parses; it is safe to run concurrently-ish but rerun if it errors on someone else's file.

2. **Behavior.** For each item, compare parameters and defaults, state transitions, timing and curves, callbacks and event order, updates (didUpdateWidget-equivalent), disposal, assertions on invalid input, and edge cases, as relevant. Prioritise items most likely to have user-visible differences. Review existing Incular tests (`rg -n` in `crates/*/tests`) before duplicating work, but existing tests are not Flutter comparisons unless they encode Flutter values.

3. **Scenarios.** Write deterministic paired frame scenarios where they give useful evidence. Name them `<pkg>__<area>__<case>`. Record values with `h.value(...)`. Run only your package:
   ```sh
   FLUTTER="C:/Manual Installs/flutter/bin/flutter" harness/run-frames.sh <pkg>__
   ```
   (Git Bash, from repo root.) Read `harness/out/frames/report-<pkg>__.md`. Frame 0 checks static layout: investigate any frame-0 difference and decide whether it is the tested behavior or a twin mismatch; fix twin mismatches. After every Rust edit run `cargo check -q -p incular-parity-harness --tests` before moving on. If the build fails in a file you do not own, wait a minute and retry; do not edit it. Format only your file: `rustfmt --edition 2024 harness/incular/tests/frames/<pkg>.rs`. For Dart, run `"C:/Manual Installs/flutter/bin/flutter" analyze test/frames/<pkg>_test.dart` from `harness/flutter`. Keep matching scenarios as regression evidence. If an Incular option does not exist, a diagnostic twin with different configuration is allowed only if it is labelled as non-equivalent in the md and JSONL and not presented as a same-input result.

4. **Findings.** Only record a divergence after ruling out a twin mismatch. Deduplicate against existing findings (including other packages' — grep `audits/flutter-parity/findings/*.jsonl`). Do not turn an API-shape difference into a claimed runtime failure. New IDs continue your package's numbering (`<pkg>-NNN`). Each finding appears in the md (a `### <id> — <title>` section) and as one JSON line with this schema:

   ```json
   {"id":"scroll-006","package":"scroll","severity":"P1","category":"behavior","impact_class":"wrong-final-state","title":"...","flutter_ref":"widgets/x.dart:10-20","incular_ref":"crates/incular-widgets/src/y.rs:30-40","evidence_kind":"scenario+source","scenario":"scroll__area__case","first_divergence":{"frame":3,"t_ms":48,"field":"value.offset","flutter":100.0,"incular":40.0},"evidence":"...","impact":"...","cause":"...","fix_direction":"...","confidence":"high","limitations":"...","status":"pending-review","revalidated_at":"<git short sha of HEAD>"}
   ```
   `evidence_kind`: `scenario`, `scenario+source`, `source`, `pixel`, `pixel+source`, `diagnostic` (non-equivalent configuration). `impact_class`: `crash`, `hang`, `wrong-final-state`, `lost-or-duplicate-callback`, `missing-feature`, `visible-motion`, `layout`, `edge-case`, `api-numeric`. Severity: P0 crash/hang/wrong final state/lost callbacks in common use; P1 missing common feature or clearly visible behavior difference; P2 less common feature, edge case or visible numeric drift; P3 minor numeric/API differences. `scenario` and `first_divergence` are `null` when not applicable. Use `"status":"pending-review"` for new findings; the orchestrator verifies and accepts. Normalize existing findings in your JSONL to this schema too (keep their ids and content; set `status` to `accepted` for ones listed in SUMMARY.md's accepted table that you revalidated, `rejected` if they no longer reproduce).

   Each md finding section must state: severity/category; Flutter and Incular references; evidence (scenario name and first divergence, or source explanation); user-visible impact; likely cause; fix direction (no implementation code); confidence and limitations.

5. **Package md.** Keep `findings/<pkg>.md` current: summary, a coverage table of what was checked and how, scenarios with match/divergence results, findings, open questions, pixel questions, harness requests, and validation run.

## Final report (your last message)

Under 300 words: rows by final status; new and revalidated finding IDs with one-line titles and severity; scenarios added and their match/diverge results; harness/pixel requests; anything left unchecked and why; checks you ran. Do not paste file contents.
