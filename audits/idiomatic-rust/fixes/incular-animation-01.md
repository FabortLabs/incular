# Fix brief incular-animation-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-animation/src/scheduler.rs`, `crates/incular-animation/src/animations.rs`, `crates/incular-animation/src/controller.rs`. Other agents are editing other files at
  the same time.
- No behavior change. Flutter-parity constants and math stay exactly as they are.
- Do not change any `pub` signature, public type or public field. If a fix
  needs that, skip it.
- Re-read the code around each finding first. The line numbers may have
  drifted a little. Apply the fix only if the finding is still correct and
  the fix is clearly behavior-preserving. Otherwise skip it with a reason.
  Skipping is better than a risky edit.
- Keep fixes minimal and idiomatic. Match the surrounding style. For hot-path
  scratch buffers, prefer a reusable field that is cleared and reused. Keep
  comments short.
- Never add `#[cfg(test)]` code under src/. Do not run cargo; the
  orchestrator builds and tests after the wave. Do not commit.

## Findings
### A010-02 - crates/incular-animation/src/scheduler.rs:236 (§2, high)
Evidence:
```
let tickers: Vec<Rc<TickerState>> = SCHEDULER.with(|scheduler| {
        let mut scheduler = scheduler.borrow_mut();
        scheduler.tickers.retain(|ticker| ticker.strong_count() > 0);
```
Suggested fix: Collect into a scratch Vec kept in `Scheduler` (new field `ticking: Vec<Rc<TickerState>>`, initialised empty) instead of a fresh Vec each frame. `let mut tickers = SCHEDULER.with(|s| std::mem::take(&mut s.borrow_mut().ticking)); SCHEDULER.with(|s| { let mut s = s.borrow_mut(); s.tickers.retain(|t| t.strong_count() > 0); tickers.extend(s.tickers.iter().filter_map(Weak::upgrade).filter(|t| t.active.get())); }); for ticker in tickers.drain(..) { ticker.tick(now); } SCHEDULER.with(|s| s.borrow_mut().ticking = tickers);`. The take/restore keeps re-entrant ticks safe (they touch `tickers`, not the scratch), and the buffer keeps its capacity across frames.

### A010-01 - crates/incular-animation/src/scheduler.rs:264 (§2, high)
Evidence:
```
(
            scheduler.persistent_callbacks.clone(),
```
Suggested fix: Do not clone the Vec per frame. Snapshot only the count and clone each Rc inside the loop, so no Vec is allocated: `let count = SCHEDULER.with(|s| s.borrow().persistent_callbacks.len()); for index in 0..count { let callback = SCHEDULER.with(|s| s.borrow().persistent_callbacks.get(index).cloned()); if let Some(callback) = callback { callback(now); } }`. The count snapshot keeps the current semantics (callbacks added during this loop run next frame). Remove the `(persistent, post)` tuple accordingly and keep the `post_frame_callbacks` take as is.

### A003-02 - crates/incular-animation/src/animations.rs:322 (§6, medium)
Evidence:
```
let reverse =
self.direction.get().unwrap_or(self.parent.status()) == AnimationStatus::Reverse;
```
Suggested fix: self.direction.get().unwrap_or_else(|| self.parent.status()) == AnimationStatus::Reverse;

### A005-02 - crates/incular-animation/src/controller.rs:716 (§21, medium)
Evidence:
```
let cycles = ((elapsed + *initial_t) / *period).floor() as u32 + 1;
```
Suggested fix: Saturate the final increment so a very long run with a very short period cannot overflow: `let cycles = (((elapsed + *initial_t) / *period).floor() as u32).saturating_add(1);`. The float-to-int `as` cast already saturates at u32::MAX, so only the `+ 1` can overflow (debug panic, release wrap to 0 which would end the repeat early).


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
