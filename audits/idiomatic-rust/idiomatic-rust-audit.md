# Idiomatic Rust audit and conversion: orchestrator prompt

> Paste everything below the line into a Claude Code session running **Opus**
> at the repository root. Opus orchestrates; the subagents run on
> **Haiku 5.5** (Agent tool `model: "haiku"`, model id `claude-haiku-5-5`).

---

You are the **orchestrator** for a full audit of the Incular workspace against
`IDIOMATIC-RUST.md`, followed by converting the code to follow it. You run as
Opus. You split the work, launch **Haiku 5.5 subagents** (Agent tool,
`model: "haiku"`) to do the per-area reading and the mechanical edits, check
their output, make the judgment calls yourself, and own the final result.

## 0. Ground rules (read first)

1. Read `AGENTS.md`, `IDIOMATIC-RUST.md` and the "Error and panic policy"
   section of `CONTRIBUTING.md` in full before planning. They are the
   standard. AGENTS.md wins over the guide when they disagree.
2. **Behavior must not change.** This is a quality pass, not a redesign:
   - Flutter-parity values, constants, layout math, physics and rendering
     output must stay identical.
   - If a guide rule would change behavior, it becomes a *design follow-up*,
     not an edit.
3. **Public API changes are allowed** (the project is pre-1.0) when they
   follow the guide clearly, for example `&Vec<T>` → `&[T]`, `&String` →
   `&str`, or `Option<&T>` instead of `&Option<T>`. Fix every call site in the
   workspace, examples, docs snippets and tests. Record each one in
   `CHANGELOG.md` under `Unreleased` → `Changed` as
   `**Breaking (`crate-name`):**` with a one-line migration note.
4. **Performance-motivated changes need evidence.** Hot-path rewrites (§11,
   §23) that aren't obviously free (removing a clone or a needless collect is
   free; restructuring data is not) go to the design follow-up list unless you
   benchmark them with `benchmarks/` or a profile.
5. **Respect repo structure:** tests only in `tests/` directories, no
   `#[cfg(test)]` in `src/`, platform code in its native crate, no new
   dependency without updating `specs/architecture.json`, and code reachable
   on `wasm32` must not use `std::time::Instant::now`, `std::thread` and the
   rest of the list in AGENTS.md.
6. **Memory and build limits:** the machine builds with 4 jobs. Never run
   more than one cargo command at a time across all agents. Subagents may run
   only `cargo check -p <crate>` / `cargo clippy -p <crate>` and only when
   told; you run every workspace-level command yourself. Don't run
   `cargo test-constrained` unless phase 5 truly needs it; prefer
   `cargo test -p <crate>`.
7. All scratch output goes in `audits/idiomatic-rust/` (git-ignored).
8. **Commits:** commit each validated batch as its own commit (see phase 4)
   using `type(scope): summary` subjects, e.g.
   `refactor(scroll): take slices instead of &Vec in extent index`. Never
   push.

## 1. Plan the partition (you, no subagents)

1. List every Rust source file outside `tests/`, `target/`, `vendor/`,
   `harness/flutter/`, `docs/node_modules/` and `.tmp/`. Count lines per
   crate and per module directory.
2. Split the code into **work units of about 1,500–3,000 source lines**, each
   staying inside one crate and preferably one module directory. Haiku does
   best with a narrow, concrete scope. Size snapshot (source lines, excluding
   tests) as of 2026-11-19, to be recomputed:
   - `incular-widgets` ~107k across 204 files; split by module directory,
     likely 40+ units;
   - `incular-material` ~29k, `incular-runtime` ~22k, `tools/incular-devtools`
     ~18k, `incular-wgpu` ~15k, `examples/` ~13k, `incular-controls` ~11k,
     `incular-desktop` ~10k, `incular-gestures` ~9k;
   - 3k–7k each: `incular-text`, `incular-rendering`, `incular-animation`,
     `incular-scroll`, `incular-platform`, `incular-navigation`,
     `incular-core`;
   - everything else is under 3k; bundle small crates together.
3. Mark each unit's **hot-path status**: build/layout/paint/input/frame code
   in widgets, rendering, wgpu, text, scroll, gestures, runtime frame
   scheduling and layout. Hot units get §11 and §23 checked strictly.
4. Write the plan to `audits/idiomatic-rust/PLAN.md`: a table of unit id, crate,
   files, line count and hot-path flag.

## 2. Audit (Haiku subagents, read-only)

Launch Haiku subagents in parallel batches of **at most 8 at a time**, one per
work unit. They **must not edit files or run cargo**. Give each the brief
below with the unit filled in.

### Subagent brief (audit)

````text
You are auditing part of a Rust UI framework (Incular) against the repo's
style guide. You only read and report; do not edit files and do not run
cargo or any build.

Read first:
- D:/CODEBASE/PROJECTS/INCULAR/incular/IDIOMATIC-RUST.md (the full guide;
  sections are numbered like §2.1)
- D:/CODEBASE/PROJECTS/INCULAR/incular/AGENTS.md, "Platform facts" and
  "Engineering standards" only

Your unit: <UNIT_ID>, crate <CRATE>. Files:
<FILE LIST>
Hot path: <yes/no>. If yes, apply §11 and §23 strictly; any allocation per
frame or per element in build/layout/paint/input is a finding.

Read every file in your unit completely. For each place the code departs from
the guide, record one finding. Check especially:
- §1  signatures: &String/&Vec/&PathBuf/&Option/&Box params, needless owned
      params, missing impl Into/AsRef at public boundaries, bool params
- §2  clones, to_string, to_vec, collect that could be deferred or removed;
      missing with_capacity; buffers re-created in loops; Arc<String>
- §3  index loops that should iterate; collect-then-iterate; count() on
      collections; manual loops replaceable by any/all/find/position
- §4  full sorts for top-k, stable sorts where unstable is fine,
      partial_cmp().unwrap() on floats, linear search on sorted data
- §5  format! for concatenation, push of format!, to_lowercase for compare
- §6/§7 unwrap without invariant, unwrap_or(expensive()), sentinel values,
      String errors, lost error sources. In this repo: errors made by app
      authors must not panic (see AGENTS.md Engineering standards)
- §8  stringly-typed state, bool flags that should be enums, missing
      derives on public types, missing #[non_exhaustive] on growing public
      enums
- §10 contains_key+insert instead of entry, Vec<Vec<T>> grids, HashMap with
      SipHash on hot integer keys
- §12 Rc<RefCell<>> that could be ownership/indices, long lock scopes
- §16 unsafe without // SAFETY:
- §19-§22 borrow-checker workarounds via clone, numeric overflow handling,
      float equality, rem_euclid
- §26/§27 Debug leaking secrets, HashMap order leaking into output

Do NOT report:
- formatting or naming nits that rustfmt/clippy already enforce;
- anything in tests/ directories;
- Flutter-parity constants or algorithms (don't suggest changing the math);
- speculative "could be faster" without a concrete wasteful operation.

Output: write NOTHING except a JSON array (no prose before or after). One
object per finding:
{
  "unit": "<UNIT_ID>",
  "file": "crates/…/x.rs",
  "line": 123,
  "rule": "§2.1",
  "severity": "high|medium|low",
  "category": "api|alloc|iter|error|types|unsafe|concurrency|perf|other",
  "public_api": true|false,
  "hot_path": true|false,
  "evidence": "<the offending code, max 3 lines, verbatim>",
  "fix": "<concrete replacement code or precise instruction>",
  "behavior_change": false|true,
  "confidence": "high|medium|low"
}
Severity guide: high = allocation/clone in a hot path, panic reachable from
app-author input, unsafe without SAFETY, public API forcing callers to
allocate; medium = clear guide violation with modest cost; low = readability.
If the unit is clean, output [].
````

### After each batch

1. Save each subagent's JSON to `audits/idiomatic-rust/findings/<unit>.json`.
   If the output isn't valid JSON, resend the brief once with "Output only
   the JSON array". If it fails again, audit that unit yourself.
2. **Verify** the findings yourself. Haiku over-reports and sometimes
   misreads code:
   - Open the cited line for every `high`, every `public_api: true`, and
     every `behavior_change: true` finding.
   - Spot-check at least 1 in 5 of the rest.
   - Mark each finding `confirmed`, `rejected` (with a reason) or
     `needs-design`.
   - If more than 30% of a unit's spot-checks are wrong, re-audit that unit
     yourself.
3. Watch for **cross-unit patterns**: the same anti-pattern repeated across
   crates, such as the ~20 copies of the `AtomicU64::try_update` ID
   generator. Record each as one systemic finding with a single fix (for
   example a shared helper in the lowest crate that owns the concept),
   instead of N separate local edits.

## 3. Report (you)

Write `audits/idiomatic-rust/REPORT.md`:

- Summary table: findings by crate × severity, and confirmed vs rejected
  counts (the rejection rate tells the user how much to trust Haiku output).
- **Systemic issues** first, each with the proposed workspace-wide fix.
- Then confirmed findings grouped by crate, ordered by severity, each linked
  as `path:line` with rule, evidence and fix.
- **Public API changes** as a separate list (these need CHANGELOG entries).
- **Design follow-ups**: behavior-changing or benchmark-requiring items,
  with what to measure.
- **Guide gaps**: recurring patterns in this repo that IDIOMATIC-RUST.md
  doesn't cover, or rules that proved wrong or noisy here. Propose exact text
  to add or change.

Then continue straight into phase 4. Stop and ask the user only if a
systemic fix would restructure a crate boundary or touch
`specs/architecture.json`.

## 4. Convert (Haiku for mechanical edits, you for judgment)

Order: systemic fixes first, then crates from the bottom of the dependency
graph up (`incular-core`, `incular-config`, `incular-platform` … up to
`incular-material` and `incular`), so API changes ripple upward once.

For each crate:

1. Split its confirmed findings into:
   - **Mechanical**, for Haiku: local edits that don't change a public
     signature (clone removal, entry API, iterator rewrites, with_capacity,
     SAFETY comments, `expect` messages, `sort_unstable`, format fixes).
   - **Judgment**, for you: public API changes, error-type redesigns,
     ownership restructuring, anything touching more than one file's
     contract, and every systemic fix.
2. Launch Haiku fix agents for the mechanical work, **one agent per set of
   files, never two agents on the same file**, at most 4 at a time. Brief:

   ````text
   Apply these confirmed fixes in the Incular repo
   (D:/CODEBASE/PROJECTS/INCULAR/incular). Edit only the files listed. Do not
   change behavior, public signatures, constants or formatting elsewhere.
   Do not add #[cfg(test)] code. Do not run cargo; the orchestrator builds.
   If a fix doesn't apply cleanly or would change behavior, skip it and say
   why.
   Findings (JSON): <the subset for these files>
   Reply with: for each finding id, "applied" or "skipped: <reason>".
   ````

3. Do the judgment items yourself.
4. Validate the crate (one cargo command at a time):
   - `cargo clippy -p <crate> --all-targets --all-features -- -D warnings`
   - `cargo test -p <crate>`, plus `-p` for each direct dependent if a
     public API changed
   - `cargo fmt -p <crate>`
   - Fix failures yourself; don't hand failing builds back to Haiku.
5. Review the crate's diff (`git diff -- crates/<crate>`) and reject any
   edit that changes behavior or reads worse than before. Haiku edits must
   pass this review, not just compile.
6. Commit the crate as one commit (systemic fixes get their own commits):
   - `refactor(<scope>): follow idiomatic Rust guide` with a body listing
     the main changes;
   - `perf(<scope>): …` if it removes hot-path allocations;
   - CHANGELOG entries go in the same commit as their API change;
   - end the message with
     `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## 5. Final validation and handoff (you)

1. Run the workspace checks from AGENTS.md:
   - `python scripts/check-format.py`
   - `cargo check --workspace`
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   - `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`
   - `cargo clippy -p incular --lib --examples --target wasm32-unknown-unknown --no-default-features --features web,controls,material -- -D warnings`
   - `python scripts/check-features.py`
   - `python scripts/check_doc_examples.py --compile` if public APIs changed
2. Run `cargo test-constrained` once, in the background, only if public APIs
   changed in core, runtime, widgets, layout or rendering, since those have
   the largest blast radius. Otherwise the per-crate tests from phase 4 are
   enough.
3. Apply the "Guide gaps" proposals to `IDIOMATIC-RUST.md` in a final
   `docs: …` commit.
4. Update `REPORT.md` with what was applied, skipped (and why), and left as
   design follow-ups.
5. Reply to the user with:
   - the numbers (findings found, confirmed, fixed, deferred, and Haiku's
     rejection rate);
   - the commit list;
   - the public API changes;
   - the top design follow-ups;
   - any check that couldn't run.
