# Worker brief: fixing Flutter parity findings

You are a worker fixing findings from the completed Flutter parity audit of Incular (Rust UI framework). The repository root is `D:\CODEBASE\PROJECTS\INCULAR\incular`, on branch `fix/flutter-parity`.

- Reference: **Flutter 3.41.7**. Source: `C:/Manual Installs/flutter/packages/flutter/lib/src`.
- Platform: Windows. Shells are Git Bash and PowerShell 7.
- Python is not installed. Write any helper scripts to files in your scratch area, not inline heredocs.

## Rules

- **You are not alone.** Several workers edit this checkout at the same time, and all of them compile the same crates.
  - Edit only the files your assignment gives you. If a fix needs a change elsewhere, make the smallest change possible and report it.
  - Never revert, reformat or rewrite other people's code.
  - Keep your files compiling at every step. Make small edits and check them right away.
  - If the build fails in a file you do not own, wait a minute and retry. Do not edit that file.
- **Never commit.** Never delete anything under `harness/out/`. Never put files in `.tmp/`.
- **Do not run huge suites.** That means no `cargo test --workspace`, `cargo test-constrained`, `cargo clippy --workspace` or all-scenario frame runs. Run focused checks only.
- **Format only your own files**, with `rustfmt --edition 2024 <file>`. Do not run `cargo fmt --all`: it hits a Windows path limit and touches other workers' files.
- **Read before you start:**
  - `AGENTS.md`;
  - `system-design/ARCHITECTURE.md`, especially hard rule 8: "Flutter is a vocabulary/reference, not an architecture or a requirement to reproduce Dart lifecycle patterns";
  - `system-design/API_DESIGN.md` when you add public API;
  - `specs/architecture.json`, which is the dependency allow-list. Add no new crate dependency edges.
- **Tests go in `crates/<crate>/tests/`.** Never put `#[cfg(test)]` code or test helpers under `src/`.
- **Web target:** `wasm32` must not block or call `std::time::Instant`. Use `incular_core::time`.
- **Public API changes** follow the existing style. Keep existing APIs working where practical; prefer adding builder methods to changing signatures. Update the crate README or the doc comments when public behavior changes.

## For each finding you are assigned

1. **Read the finding.** It is the `### <id>` section in `audits/flutter-parity/findings/<package>.md`; the JSON line is in `audits/flutter-parity/findings.jsonl`. The finding was recorded at commit `e25e45bc`.
2. **Check that it still reproduces.**
   - If it has a scenario, run it with `FLUTTER="C:/Manual Installs/flutter/bin/flutter" harness/run-frames.sh <scenario>` from Git Bash at the repository root, then read `harness/out/frames/report-<scenario>.md`.
   - Otherwise, re-read the cited Flutter and Incular code.
3. **Understand Flutter's observable behavior**: defaults, timing, callback order, state transitions, disposal and edge cases. Then fix the root cause in Incular, not the symptom, and not just the harness comparison.
4. **Add a focused regression test** in the owning crate's `tests/` directory. Extend an existing test file when one fits. Where the test encodes a value taken from Flutter, cite the Flutter source line or the scenario in a comment.
5. **Re-run the scenario** if there is one. Confirm it now matches, or explain any remaining residual.
   - Do not edit the scenario twins to make them match. The one exception is a twin you prove was wrong, or a diagnostic twin that can now use the newly added API; report either case.
   - Shared harness files (`recorder.dart`, `harness/incular/src/lib.rs`, `tests/frames/main.rs`, the pixel scenes) belong to the orchestrator. Ask for changes in your report.
6. **Decide an outcome** for each finding:
   - `fixed`
   - `already-fixed`
   - `invalid`
   - `duplicate`
   - `blocked`, with the concrete blocker and the remaining work
   - `deferred`, only for API surface too large for this pass, with the reason

   Never force a change to satisfy a finding you believe is wrong. Record the evidence instead.

## Validation (focused)

- `cargo check -p <crate> --tests` for each crate you changed. If you changed public API that dependents use, also run `cargo check -p incular-parity-harness --tests`.
- `cargo test -p <crate> --test <file>` for the test files you added or changed, plus the directly related existing test files. Never run a crate's whole suite when the crate has more than about 20 test files; `incular-widgets` is one of these.
- `cargo clippy -p <crate> --all-targets -- -D warnings` once, at the end. Fix only warnings in code you wrote.
- The scenario reruns from step 5.

If a test fails, compare against `git stash`-free evidence: check whether the failure involves code you touched. Report pre-existing failures as such, citing the test name and the reason.

## Recording

Write `audits/flutter-parity/fixes/<group>.jsonl`, where `<group>` is your assignment name. Put one JSON object per line, one per assigned finding:

```json
{"id":"physics-002","outcome":"fixed","root_cause":"...","change":"crates/incular-animation/src/physics.rs:340-352 (what changed)","tests":["crates/incular-animation/tests/physics.rs::gravity_negative_acceleration"],"harness":"physics__gravity__negative_acceleration_threshold: match (was diverging at frame 0)","api_change":"none | description and migration","limitations":"...","note":"..."}
```

Also write `audits/flutter-parity/fixes/<group>.md`: one short section per finding, with the outcome, cause, change, tests, harness result and limitations.

## Final report (your last message, under 300 words)

- Each finding: its outcome, whether you confirmed it before fixing, and a one-line cause and fix.
- The tests and scenarios you ran, with their results.
- Any shared-file or out-of-ownership edits.
- Remaining risks or related unresolved findings.
