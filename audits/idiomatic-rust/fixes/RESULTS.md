# Phase 4 batch results

Format: batch: finding ids fixed / skipped (reason). "reviewed" means Opus
read the diff. "committed" names the commit.

- incular-assets-01: A012-04 fixed. reviewed.
- incular-semantics-01: A156-02 fixed. reviewed.
- incular-platform-01: A113-01, A113-02, A114-02 fixed. reviewed.
- incular-scroll-01: A153-03/05/07, U102-02/03 fixed. reviewed. Skipped for the cross-file pass: A155-03/04/05 (signature changes in controller.rs, coordinator.rs and activity.rs) and A154-01 (S3 restoration Rc).
  Check: ballistic.rs dropped `#[allow(clippy::too_many_arguments)]`; verify that clippy still passes.
- incular-text-02: A160-01, A160-02 fixed. reviewed.
- incular-wgpu-01: reviewed OK (FrameTally single pass, advance_stencils in place, #[expect]); skipped A179-02/03, A178-02 (new renderer fields), A178-03, A177-02 (no smallvec)
- incular-text-01: reviewed OK (update_ranges equivalent for selection-only moves; placeholder cursor relies on per-line extend order)
- incular-text-01 note: agent iterated `&Arc<[FontFamily]>`; fixed to `.iter()` by orchestrator.
- incular-wgpu-02: A182-02/03/04/05, A175-03/04, A180-02/03, A181-01 fixed; A175-02 skipped (needs ImageFilter::stages to stop returning Vec, rendering crate). Reviewed: lower_commands_into appends nested groups in place; removed blend_fixed_function_draws bump was dead (non-destination modes continue earlier); matrix_params identical.
- s1-widgets (Sonnet): A188-01, A191-01, A192-05, A193-01/02/03, A205-02, A209-03, A237-02, A245-03, A267-01, A332-01 (material) fixed as try_/maybe_ pairs with documented panicking wrappers. Reviewed: checks equivalent incl. NaN.
- incular-devtools-02: A051-01..07, A052-01/02 fixed. reviewed.
- incular-devtools-03: A045-02/03/04/05, A037-01/05 fixed; A045-01 skipped (pub fn filter signature), A037-02/03 skipped (need threaded scratch buffer). reviewed.
- incular-devtools-01: A041-04/05/06, A042-01/02/04/06 fixed; A042-05 skipped (needs protocol TargetEvent variant), A042-03 skipped (send return type across callers). Reviewed: RequestFrame mirrors Message::Request serde attrs exactly; size checks count bytes without buffering; retry note now carries the connect error text.
- incular-windows-01: A313-01/02/03, A314-01..07 fixed. Reviewed: pen frame scratch reuse (reserve before set_len), menu install borrows disjoint fields instead of cloning snapshot, SAFETY comments added.
- incular-macos-01: A074-01/02/03, A075-01 fixed. Reviewed: SAFETY comments; notification lock now dropped before the stale/denied completion callbacks (granted branch keeps its scope).
- incular-web-01: A164-01..04, A166-01/02/03/05, A167-01 fixed; A166-04 skipped (listener ownership, needs browser check). Reviewed: stale-child removal keyed by owner id (replacements use replace_with, so equivalent); style rewrite skipped when cached bounds unchanged (nothing else writes node style); write_system is a no-op for empty contents. Needs wasm32 clippy in Phase 5.
- examples-02: A318-01/03, A321-03/04, A322-02, A326-08/09, A327-03 fixed; A327-01/02, A322-01 skipped (need new pub accessors). reviewed.
- examples-01: A320-01/02/04/05, A323-01/02, A326-02/03 fixed; A320-03 skipped (per-char lowercasing changes final-sigma search), A323-03 skipped (test signature), A326-01 skipped (pub field type). Reviewed: issue_tracker borrow now spans the build but every borrow_mut is inside an event closure.
