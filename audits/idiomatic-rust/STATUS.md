# Audit status (keep updated; survives context compaction)

## Tools (all in audits/idiomatic-rust/)

- `units.json` / `PLAN.md`: 342 units (8 pilot U*, 334 A*). `briefs/<ID>.md` holds each agent prompt.
- Launch: a Haiku agent (model haiku, effort medium, background) with the prompt
  `Read D:/CODEBASE/PROJECTS/INCULAR/incular/audits/idiomatic-rust/briefs/<ID>.md and follow it exactly.`
  At most 8 at once.
- Find missing units: run `python - <<EOF` over `units.json` and keep units without a `findings/<ID>.json`.
- `verify.py [IDs]`: mechanical check that the evidence is at the cited line. It writes the `mech` field.
- `show.py <ctx> <finding ids>`: prints code around each finding.
- `setv.py "<id>=<verdict>[:note]" ...`: verdicts are `confirmed`, `rejected`, `needs-design` or `systemic:<name>`. They are saved to `verdicts.json`.
- `systemic.json`: systemic issues S1–S8.
- `aggregate.py`: rebuilds `REPORT.md`, including `METHOD.md`.

## Progress

- Phase 1 (plan): done.
- Phase 2 (audit): DONE 2026-11-19. 342/342 units, 286,068 lines, 1,105 findings. Every high finding outside the allocation class (§2/§11/§23) has been verified. Allocation highs were sampled: 10 sampled, 0 wrong.
- Phase 3 (report): DONE. REPORT.md was rebuilt after all units came in. Verdicts: 951 unverified, 63 systemic, 61 confirmed, 19 needs-design, 11 rejected (rejection rate 11/154 = 7%). Earlier note: REPORT.md exists and is rebuilt by aggregate.py. Re-run verify.py and aggregate.py once all units are in.
  - Then verify the new high findings with `show.py` and `setv.py`, tagging systemic ones.
- Phase 4 (convert): IN PROGRESS (2026-11-20). Done and uncommitted: probe.rs deleted, S7 x4, real-bug list x12 (assets, runtime, rendering and controls via Sonnet agents, reviewed). Tests for 10 crates were running. Next: commit, then S8 helper, then the per-crate mechanical waves from fixes/index.json (mkfix.py; fixed.json lists done finding ids). Public-API findings get a separate later pass.

## Phase 4 order (planned)

0. **A248-01 (do first): delete the committed TEMPORARY `crates/incular-widgets/src/probe.rs`.** It calls `Instant::now` in layout/reconcile hot paths, which panics on wasm32. Remove the `pub mod probe;` line, the 5 `crate::probe::span` call sites (scrolling/layout.rs, tree/layout/behavior/sliver.rs ×2, tree/reconciliation.rs ×2) and `benches/scroll_probe.rs` if it exists.
1. **S7 refcell-across-callback.** These are real latent BorrowMutError bugs: A196-04, A198-01, A200-01, A188-02.
2. **Real bugs from findings:**
   - A150-01: window clamp before validate.
   - A171-02: u16 underflow in the glyph atlas.
   - A122-02: huge blur sigma.
   - A084-01: Date month 0 index.
   - A020-01: cursor string panic.
   - A012-01: asset type panic.
   - A008-01/02, A024-01, A068-04: Duration overflow.
   - A065-01 and A234-01 (split_view min > max): clamp panics.
3. **S8 id-generator helper** in incular-core.
4. **S1 public-panic-contracts:** `# Panics` docs plus `try_`/`maybe_` forms. These are additive, not breaking.
5. **S2 listener-snapshot** helper, then **S3 restoration-clone**.
6. **Per crate, bottom-up:** core, config, platform, animation, assets, gestures, layout, rendering, scroll, text, semantics, image, navigation, runtime, wgpu, widgets, controls, material, desktop + natives, devtools, examples.
   - Haiku handles the mechanical confirmed/unverified medium/low findings (at most 4 agents, disjoint files).
   - I review the diff, run `cargo clippy -p` and `cargo test -p`, and commit once per crate.
7. **Design follow-ups** (S4, S5, S6, needs-design items) go into the REPORT only. They need benchmarks.

## Guide gaps noted so far

- §28 item 9 tells you to run `cargo test-constrained` for every change. That contradicts AGENTS.md's prefer-targeted-tests rule. Change it to targeted `cargo test -p` and point to AGENTS.md.
- Add a pattern: never hold a `RefCell` borrow while calling app-supplied closures (S7).
- Add a pattern: copy-on-write `Rc<[T]>` listener lists instead of cloning a Vec per notification (S2).
- Add a pattern: `Duration::from_secs_f32/f64` and `Instant + Duration` panic on overflow. Use the `try_from_secs_*` and `checked_add` forms with input from app authors or env vars.
- Add a pattern: `f32::clamp` panics when min > max or a bound is NaN. Validate the bounds first, or use max/min.
