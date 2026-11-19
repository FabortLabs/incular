# Audit checklist (condensed from IDIOMATIC-RUST.md)

Rule ids are guide sections (§1–§29). The full guide is
`D:/CODEBASE/PROJECTS/INCULAR/incular/IDIOMATIC-RUST.md`. Open a section
only if you need detail (Read with offset):
§1 L121, §2 L204, §3 L273, §4 L336, §5 L362, §6 L395, §7 L431, §8 L456,
§10 L537, §11 L578, §12 L614, §16 L718, §19 L796, §21 L895, §23 L943,
§26 L1014, §29 L1091.

## Repo facts (these override the guide)
- Errors made by *application authors* must return structured errors or
  diagnostics, never panic. Broken *framework invariants* may use
  `expect("<why it can't fail>")`. Public APIs panic only as a documented
  contract paired with a `maybe_*`/`try_*` form.
- Build, layout, paint, input and frame code must not allocate in the steady
  state (reuse buffers, dirty flags, generations).
- Code reachable on wasm32 must not call `std::time::Instant::now`,
  `SystemTime::now`, `std::thread`, `std::env::temp_dir`, `std::process::id`
  (use `incular_core::time`).
- `unsafe` needs a `// SAFETY:` comment.
- No `#[cfg(test)]` modules in `src/` (tests live in `tests/`).
- Flutter-parity constants and math are intentional. Never suggest changing
  them.

## What to look for

**§1 Signatures.** `&String`→`&str`, `&Vec<T>`→`&[T]`, `&PathBuf`→`&Path`,
`&Option<T>`→`Option<&T>`, `&Box<T>`→`&T`. Owned params (`String`, `Vec`)
that are only read. Bool params at public boundaries that read as
`f(true, false)` (suggest an enum). Returning owned clones where a
borrow/`&str`/slice would do.

**§2 Clones and allocation.** `.clone()`, `.to_string()`, `.to_owned()`,
`.to_vec()`, `.collect()` whose result is only read, or that exists to dodge
a borrow error (§19: split borrows, scope the borrow, read-then-write).
Cloning a whole collection, then filtering (clone survivors only). Missing
`Vec::with_capacity` when the length is known. A `Vec`/`String`/`HashMap`
created inside a loop or per frame that could be hoisted and `clear()`ed.
Immutable shared `Arc<String>`/`Arc<Vec<T>>`→`Arc<str>`/`Arc<[T]>`.
`Box<dyn Fn>` or `Vec` allocations where a borrowed/generic form works.

**§3 Iterators.** `for i in 0..v.len()` indexing → iterate/zip/enumerate
(unless the index itself is needed for mutation of another element).
`.collect::<Vec<_>>()` then iterate once → keep chaining. `.iter().count()` on a
slice → `.len()`. Manual loops that are `any`/`all`/`find`/`position`/`sum`/
`max_by_key`. `filter(..).next()` → `find`. `map(..).flatten()` →
`flat_map`. `.iter().cloned().filter()` → filter first.

**§4 Sorting.** `sort_by(|a,b| a.partial_cmp(b).unwrap())` → `total_cmp`.
Stable `sort`/`sort_by` on primitive keys where order of equal elements
doesn't matter → `sort_unstable*`. Full sort to take top-k → 
`select_nth_unstable*`. Linear search over sorted data → `binary_search` /
`partition_point`.

**§5 Strings.** `s += &format!(..)` / `s.push_str(&format!(..))` →
`write!(s, ..)`. `format!("{}", x)` just to get a String → `x.to_string()`.
`a.to_lowercase() == b.to_lowercase()` → `eq_ignore_ascii_case` (if ASCII).
`format!` paths → `Path::join`.

**§6 Option/Result.** `unwrap()` with no invariant (→ `?` or
`expect("why")`). `unwrap_or(expensive_call())`/`ok_or(format!(..))` → `_else`
variants. `if x.is_some() { x.unwrap() }` → `if let`. `match` that is just
`map`/`and_then`/`unwrap_or`. Sentinel values (`-1`, `""`, `usize::MAX`) that
should be `Option`.

**§7 Errors.** `Result<T, String>` or `Box<dyn Error>` in library APIs →
typed error enum. Errors that drop the source (`map_err(|_| ..)`). Panics
reachable from app-author input (see repo facts).

**§8 Types.** Stringly-typed state → enum. Several bools encoding one
state → enum. Public types missing obvious derives (`Debug`, `Clone`,
`Copy`, `PartialEq`, `Default`, `Hash`) that callers would need. Public enums
likely to grow and missing `#[non_exhaustive]` (only flag if clearly
growing). Missing `#[must_use]` on builder-style / pure functions whose
result is the whole point.

**§10 Collections.** `contains_key` then `insert`/`get_mut` → `entry`.
`Vec<Vec<T>>` grids → flat + stride. `Vec<bool>` membership → bitset.
`HashMap`/`HashSet` iteration order leaking into output, IDs or snapshots →
`BTreeMap` or sort. `.get(k).is_some()` → `contains_key`.

**§11/§23 Hot paths (only strict when the unit is hot).** Allocation per
frame or per element in build/layout/paint/input/hit-testing; repeated
lookups of the same key in a loop; recomputing unchanged values every frame
with no dirty flag; `Box`/`Rc` per small node where an index/arena exists.
Large enum variants bloating bulk-stored types (only if obvious).

**§12 Shared state.** `Rc<RefCell<..>>` used where plain ownership or
`&mut` would work. Lock or `RefCell` borrow held across a callback or a
long scope. `let _ = mutex.lock()` (drops immediately).

**§14 Control flow.** Deep nesting that `let ... else` / early return
flattens (low severity).

**§16 unsafe.** Any `unsafe` block or `unsafe impl` without `// SAFETY:`.

**§18 Lints.** `#[allow(...)]` → `#[expect(..., reason = "…")]`.
`lazy_static!`/`once_cell` → `std::sync::LazyLock`/`OnceLock`.

**§21 Numerics.** `as` casts that may truncate or change sign silently on
values that can actually be out of range (not on obviously safe values like
small loop counters or f32→f64). `a % b` for wrap-around of possibly negative
values → `rem_euclid`. `==` on computed floats. Unchecked arithmetic that can
realistically overflow (counters from user input, sizes).

**§26/§27.** `Debug` impls exposing secrets. `Box<dyn Trait>` params where
`&dyn Trait` works. Macros where a function works.

## Do NOT report
- Formatting and naming nits that rustfmt/clippy enforce.
- Anything under `tests/`.
- Flutter-parity values or algorithms.
- Speculative "could be faster" with no concrete wasteful operation.
- Clones of `Copy` types, `Rc`/`Arc` clones (cheap and intended), or clones
  that are genuinely needed because the value is stored/moved.
- `unwrap`/`expect` with a clear invariant already explained.
- Public API shape choices that `system-design/API_DESIGN.md` would govern
  unless they are clear §1 violations.

## Severity
- **high:** allocation/clone in a hot path per frame or per element; panic
  reachable from app-author input; `unsafe` without SAFETY; public API that
  forces callers to allocate (`&String`, `&Vec`, owned param only read).
- **medium:** a clear guide violation with modest cost.
- **low:** readability.

Before reporting, re-read the code around the line and make sure you are
right. False positives cost more than misses. Report each distinct issue
once (if one pattern repeats in a file, one finding listing the lines in
`evidence` is enough).
