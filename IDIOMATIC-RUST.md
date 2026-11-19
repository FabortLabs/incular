# Idiomatic Rust

A practical checklist of Rust idioms that improve **performance**, **API
flexibility**, and **developer experience**. Each entry says what to do, why,
and (where useful) shows a before/after.

Project-specific rules in [AGENTS.md](AGENTS.md) and
[the architecture contract](system-design/ARCHITECTURE.md) take precedence over
anything here. These are defaults, not laws: deviate when you have a reason,
and add to this guide when you learn something worth keeping. If an idiom
needs a newer Rust than the current MSRV, raise the MSRV (see the Rust version
policy in AGENTS.md) rather than writing worse code.

---

## Contents

0. [Worked example: the "recent orders" refactor](#0-worked-example-the-recent-orders-refactor)
1. [Function signatures](#1-function-signatures)
2. [Ownership, cloning and allocation](#2-ownership-cloning-and-allocation)
3. [Iterators](#3-iterators)
4. [Sorting, searching and selection](#4-sorting-searching-and-selection)
5. [Strings and formatting](#5-strings-and-formatting)
6. [`Option` and `Result`](#6-option-and-result)
7. [Error handling](#7-error-handling)
8. [Modeling with types](#8-modeling-with-types)
9. [Traits and generics](#9-traits-and-generics)
10. [Collections](#10-collections)
11. [Memory layout and codegen](#11-memory-layout-and-codegen)
12. [Shared state, interior mutability and concurrency](#12-shared-state-interior-mutability-and-concurrency)
13. [Async](#13-async)
14. [Control flow and readability](#14-control-flow-and-readability)
15. [API design and naming](#15-api-design-and-naming)
16. [`unsafe`](#16-unsafe)
17. [Testing and benchmarking](#17-testing-and-benchmarking)
18. [Tooling, lints and build profiles](#18-tooling-lints-and-build-profiles)
19. [Working with the borrow checker](#19-working-with-the-borrow-checker)
20. [Closures, `Drop` and RAII](#20-closures-drop-and-raii)
21. [Numerics](#21-numerics)
22. [Arrays, slices and const generics](#22-arrays-slices-and-const-generics)
23. [Hot paths, per-frame code and caching](#23-hot-paths-per-frame-code-and-caching)
24. [I/O, paths and the environment](#24-io-paths-and-the-environment)
25. [Serialization (serde)](#25-serialization-serde)
26. [Macros, `cfg`, modules and logging](#26-macros-cfg-modules-and-logging)
27. [Trait-object and generic details](#27-trait-object-and-generic-details)
28. [Self-review checklist before finishing a change](#28-self-review-checklist-before-finishing-a-change)
29. [Anti-patterns quick reference](#29-anti-patterns-quick-reference)

---

## 0. Worked example: the "recent orders" refactor

Starting point — it compiles, but it restricts callers and wastes work:

```rust
fn recent_orders(orders: &Vec<Order>, customer_id: &String, limit: usize) -> Vec<Order> {
    let mut matches: Vec<Order> = orders
        .iter()
        .filter(|o| &o.customer_id == customer_id)
        .cloned() // clones *every* match, even the ones we'll throw away
        .collect();
    matches.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    matches.truncate(limit);
    matches
}
```

Step by step:

1. **`&str` instead of `&String`** — callers can pass literals (`"c-42"`) and
   `&String` still works through deref coercion.
2. **`&[Order]` instead of `&Vec<Order>`** — callers can pass a `&Vec`, a
   sub-slice (`&orders[..100]`), an array, or a `Box<[Order]>`.
3. **Defer cloning** — collect `Vec<&Order>`, sort and truncate the
   references, clone only the survivors. With 20,000 matches and `limit = 10`
   that is 10 clones instead of 20,000 (6.57 s → 0.95 s in the video's bench).
4. **Return references** — if callers only read, tie the output to the input
   with a lifetime and don't clone at all.

```rust
use std::cmp::Reverse;

fn recent_orders<'a>(orders: &'a [Order], customer_id: &str, limit: usize) -> Vec<&'a Order> {
    let mut matches: Vec<&Order> = orders
        .iter()
        .filter(|o| o.customer_id == customer_id)
        .collect();
    matches.sort_unstable_by_key(|o| Reverse(o.timestamp));
    matches.truncate(limit);
    matches
}
```

Going further:

5. **Don't fully sort when you only need the top *k*.**
   `select_nth_unstable_by_key` partitions in O(n); then sort only the *k*
   survivors (O(k log k)) instead of all n (O(n log n)).

   ```rust
   if limit < matches.len() {
       matches.select_nth_unstable_by_key(limit, |o| Reverse(o.timestamp));
       matches.truncate(limit);
   }
   matches.sort_unstable_by_key(|o| Reverse(o.timestamp));
   ```

   For a streaming source where you can't collect everything, keep a
   `BinaryHeap<Reverse<_>>` of size `limit`.
6. **Use `sort_unstable*` unless you need ties kept in input order.** It is
   faster and does not allocate. (If ties *must* stay in input order, keep
   `sort_by_key` — that is a behavior difference, not just a speedup.)
7. **`Reverse(key)` beats `b.cmp(a)`** — the intent ("descending") is obvious
   and you can't accidentally swap `a`/`b` in only one branch of a compound
   comparison.
8. **If the caller needs owned values, let them decide:**
   `recent_orders(..).into_iter().cloned().collect::<Vec<Order>>()`.

---

## 1. Function signatures

1. **Accept borrowed views, not borrowed owners.**

   | Instead of     | Accept        |
   | -------------- | ------------- |
   | `&String`      | `&str`        |
   | `&Vec<T>`      | `&[T]`        |
   | `&mut Vec<T>`  | `&mut [T]` (unless you push/pop) |
   | `&PathBuf`     | `&Path`       |
   | `&OsString`    | `&OsStr`      |
   | `&Box<T>`      | `&T`          |
   | `&Option<T>`   | `Option<&T>` (`opt.as_ref()`) |
   | `&Option<String>` | `Option<&str>` (`opt.as_deref()`) |
   | `&Rc<T>` / `&Arc<T>` | `&T` (unless you need to clone the pointer) |

   Clippy's `ptr_arg` catches most of these.

2. **Take ownership when you're going to store or consume the value.** Taking
   `&str` and calling `.to_string()` inside forces a copy even when the caller
   already had a `String` they were about to drop. Take `String` (or
   `impl Into<String>`) and let the caller choose to move or clone.

   ```rust
   // Bad: always allocates, even for `Name::new(owned_string)`
   fn new(name: &str) -> Self { Self { name: name.to_string() } }
   // Good: moves an owned String for free, allocates only for &str
   fn new(name: impl Into<String>) -> Self { Self { name: name.into() } }
   ```

3. **Use `impl AsRef<Path>` for file-system entry points** so callers can pass
   `&str`, `String`, `PathBuf`, `&Path`. Keep this to the public boundary — see
   §9.9 for avoiding the monomorphization cost.

4. **Accept `impl IntoIterator<Item = T>` instead of `Vec<T>`** when you only
   iterate: callers can pass a `Vec`, an array, a `HashSet`, a `map()` chain…
   without collecting first.

5. **Return `impl Iterator<Item = &T>` instead of `Vec<&T>`** when callers may
   only need the first few, a count, or to chain further. Lazy beats eager.
   Return a `Vec` when callers need random access or the iterator would borrow
   something awkward.

6. **Return references tied to inputs** (`fn first<'a>(xs: &'a [T]) -> &'a T`)
   instead of cloning. Lifetime elision covers the single-input case; add
   explicit lifetimes only when there is more than one reference input.

7. **Use `Cow<'_, str>` / `Cow<'_, [T]>` for "usually borrowed, sometimes
   owned" returns**, e.g. escaping that rarely changes the input.

   ```rust
   fn escape(s: &str) -> Cow<'_, str> {
       if !s.contains('&') { return Cow::Borrowed(s); }
       Cow::Owned(s.replace('&', "&amp;"))
   }
   ```

8. **Pass small `Copy` types by value** (`u32`, `f32`, `Point { x, y }`,
   `char`, small enums). `&u32` is a pointer to a 4-byte value — more work,
   not less. Rule of thumb: ≤ 2–3 machine words and `Copy` → by value.

9. **Avoid `bool` parameters; use enums.** `draw(true, false)` is unreadable;
   `draw(Fill::Solid, Stroke::None)` is self-documenting and extensible.

10. **Many optional parameters → options struct + `Default`, or a builder.**

    ```rust
    let style = TextStyle { size: 14.0, ..TextStyle::default() };
    ```

11. **Prefer `&mut self` methods to `self -> Self` when the type is large or
    not `Copy`**, unless you're building a fluent builder.

12. **Take `&mut impl Write` / `&mut impl Read` / `&mut String` output buffers**
    in hot paths so callers can reuse allocations.

13. **Accept closures as `impl Fn…` (static dispatch) unless you store them.**
    Store as `Box<dyn Fn…>` only when you must keep heterogeneous callbacks.
    Choose the weakest bound you need: `FnOnce` > `FnMut` > `Fn` in flexibility
    for callers.

---

## 2. Ownership, cloning and allocation

1. **Defer `.clone()` / `.cloned()` / `.to_owned()` until you know you need
   the value** (see §0). Filter, sort, dedupe and truncate on references.

2. **Clone at the edge, not in the middle.** If a pipeline needs owned data at
   the end, keep borrows throughout and convert once.

3. **Preallocate:** `Vec::with_capacity(n)`, `String::with_capacity(n)`,
   `HashMap::with_capacity(n)`, `vec.reserve(n)`. `collect()` already does this
   when the iterator's `size_hint` is exact (e.g. `map` over a slice), but not
   after `filter`.

4. **Reuse buffers across iterations.** `buf.clear()` keeps the capacity; a new
   `Vec::new()` each loop iteration re-allocates.

   ```rust
   let mut line = String::new();
   while reader.read_line(&mut line)? != 0 {
       handle(&line);
       line.clear();
   }
   ```

5. **`extend` / `extend_from_slice` instead of `push` in a loop** — reserves
   once and can use `memcpy` for `Copy` types.

6. **`clone_from` reuses the destination's allocation:** `a.clone_from(&b)`
   instead of `a = b.clone()` when `a` is a `String`/`Vec` you keep around.

7. **Move out without cloning:** `std::mem::take(&mut self.items)`,
   `std::mem::replace(&mut self.state, State::Idle)`, `self.slot.take()`
   (`Option`). Great in `&mut self` methods and `Drop` impls.

8. **`Box<str>` / `Box<[T]>` / `Arc<str>` / `Arc<[T]>` for immutable data.**
   They drop the capacity word (2 words instead of 3) and `Arc<str>` avoids the
   double indirection of `Arc<String>`.

9. **Write `Arc::clone(&x)` / `Rc::clone(&x)`** instead of `x.clone()` when you
   want readers to see it's a cheap refcount bump, not a deep copy.

10. **Prefer borrowing over `Rc`/`Arc` when lifetimes are simple.** Reach for
    shared ownership only when ownership is genuinely shared or the lifetime
    can't be expressed statically.

11. **Avoid `collect()` followed immediately by `into_iter()`.** Chain the
    iterator instead; the intermediate `Vec` is pure overhead (clippy:
    `needless_collect`).

12. **Small-vector / inline storage** (`smallvec`, `arrayvec`, `tinyvec`) for
    collections that are almost always tiny — avoids a heap allocation per
    instance. Measure first; inline storage makes the struct bigger.

13. **`shrink_to_fit()` long-lived collections** after a build phase, if memory
    matters more than a possible later re-grow.

14. **`Vec::drain(..)` / `into_iter()` to move elements out**, not
    `iter().cloned()` followed by `clear()`.

15. **`String::from_utf8_lossy` returns `Cow`** — it only allocates when it
    has to replace bytes. Many std APIs do this; don't call `.into_owned()`
    reflexively.

16. **`to_owned()` vs `to_string()` vs `String::from`** — all fine for `&str`;
    prefer `to_owned()`/`String::from` when you mean "copy this string"
    and `to_string()` when you mean "format via `Display`".

---

## 3. Iterators

1. **Iterate directly, not by index.** `for x in &v` or `v.iter()` avoids
   bounds checks and off-by-one errors. When you need the index, use
   `.enumerate()`.

2. **Lockstep iteration with `zip`** instead of `for i in 0..a.len() { a[i] + b[i] }`.
   The compiler can then elide bounds checks and vectorize.

3. **`for x in &v` / `&mut v` / `v`** — the three forms map to `iter()`,
   `iter_mut()`, `into_iter()`. Use the shortest form that says what you mean.

4. **Use the purpose-built adapter** instead of a hand loop with flags:
   `any`, `all`, `find`, `find_map`, `position`, `rposition`, `max_by_key`,
   `min_by_key`, `count`, `sum`, `product`, `fold`, `last`, `nth`.

5. **`filter_map` instead of `filter` + `map` + `unwrap`;** `flat_map` /
   `flatten` instead of nested loops pushing into a `Vec`.

6. **`.copied()` for `Copy` items, `.cloned()` only when you must**, and
   always as late in the chain as possible.

7. **Collect into the right type directly:** `collect::<HashSet<_>>()`,
   `collect::<String>()`, `collect::<Result<Vec<_>, _>>()` (stops at the
   first error), `collect::<Option<Vec<_>>>()`.

8. **`partition` / `unzip`** to split one iterator into two collections in a
   single pass.

9. **`try_fold` / `try_for_each`** for early-exit accumulation with `?`.

10. **`windows(n)`, `chunks(n)`, `chunks_exact(n)`, `array_windows` (nightly),
    `split`, `rsplit`** on slices — no manual index arithmetic.
    `chunks_exact` lets the compiler drop bounds checks in the body.

11. **`peekable()`** for lookahead parsers instead of an index + "previous"
    variable.

12. **`iter::from_fn`, `iter::successors`, `iter::repeat_n`, `iter::once`**
    to build iterators without writing a struct.

13. **Implement `Iterator` (plus `size_hint`, `ExactSizeIterator`,
    `DoubleEndedIterator`, `FusedIterator` where true)** for your own
    sequences. An accurate `size_hint` lets `collect` preallocate.

14. **`.rev()` on double-ended iterators** instead of collecting and reversing;
    `.next_back()` instead of `.last()` when you just want the final element
    (`last()` walks the whole iterator for non-`DoubleEnded` impls).

15. **`.len()` over `.iter().count()`** on collections — O(1) vs O(n).

16. **Don't over-chain.** If a chain needs comments to explain, or mutates
    outside state in `map`, a `for` loop is clearer. Side effects belong in
    `for` or `for_each`, not `map`.

17. **`retain` / `retain_mut` / `dedup` / `dedup_by_key`** instead of building
    a new filtered `Vec` when you can mutate in place.

18. **`extract_if` (stable 1.87)** to remove-and-take matching elements in one
    pass.

---

## 4. Sorting, searching and selection

1. **`sort_unstable*` by default;** use stable `sort*` only when equal
   elements must keep input order.
2. **`sort_by_key` for cheap keys, `sort_by_cached_key` for expensive keys**
   (e.g. keys that allocate or lowercase a string) — the cached form computes
   each key once.
3. **`Reverse(key)` for descending order;** tuples for multi-key ordering:
   `sort_unstable_by_key(|p| (Reverse(p.score), p.name.clone()))` — or
   `sort_unstable_by(|a, b| b.score.cmp(&a.score).then_with(|| a.name.cmp(&b.name)))`
   to avoid cloning the key.
4. **`select_nth_unstable*` for top-k / median** — O(n) instead of sorting.
5. **`BinaryHeap` for streaming top-k or priority queues.** It's a max-heap;
   wrap in `Reverse` for a min-heap.
6. **`binary_search` / `binary_search_by_key` / `partition_point` on sorted
   slices** instead of linear `find`.
7. **`f32`/`f64` ordering: use `total_cmp`** —
   `v.sort_unstable_by(|a, b| a.total_cmp(b))` — never
   `partial_cmp(..).unwrap()`, which panics on NaN.
8. **`is_sorted` / `is_sorted_by_key`** in `debug_assert!`s to document and
   check invariants for binary-search code.
9. **`contains` on a slice is O(n).** If you call it in a loop, build a
   `HashSet` (or sort + `binary_search`) first.

---

## 5. Strings and formatting

1. **`write!(buf, …)` into an existing `String` instead of `buf += &format!(…)`**
   — no temporary allocation. Needs `use std::fmt::Write;`.
2. **`push_str` / `push` for literals and chars,** not `format!("{}{}", a, b)`
   for simple concatenation. Or `[a, b].concat()` / `parts.join(", ")`.
3. **Inline format args:** `format!("{name}: {value:?}")` reads better than
   positional `{}` lists (clippy: `uninlined_format_args`).
4. **Implement `Display`, not a bespoke `to_string`.** You get `to_string()`
   for free and your type works in `format!`, `write!`, logs, etc. Implement
   `Debug` (usually via derive) for every public type.
5. **Use the slicing helpers:** `split_once`, `rsplit_once`, `strip_prefix`,
   `strip_suffix`, `trim*`, `splitn`, `char_indices`, `lines`,
   `split_ascii_whitespace`.
6. **Byte-level work for ASCII data** — `as_bytes()`, `bytes()`,
   `eq_ignore_ascii_case`, `to_ascii_lowercase`, `make_ascii_lowercase` — is
   faster than `chars()` and Unicode-aware lowercase when the input is known
   ASCII.
7. **Never index strings by byte offsets you computed by hand** — `&s[..n]`
   panics on a non-char boundary. Use `char_indices`, `floor_char_boundary`
   (when stable), or offsets returned by `find`.
8. **`s.parse::<T>()`** via `FromStr` instead of ad-hoc parsing functions;
   implement `FromStr` for your own types.
9. **Avoid `to_lowercase()` for comparisons** — use `eq_ignore_ascii_case`
   or a case-folding crate; `to_lowercase` allocates.
10. **`format_args!` / `fmt::Arguments`** to pass formatted text without
    allocating, e.g. into a logger.
11. **`char::is_*` predicates** (`is_ascii_digit`, `is_whitespace`) instead of
    range comparisons.
12. **`concat!`, `stringify!`, `include_str!`** for compile-time strings.

---

## 6. `Option` and `Result`

1. **`?` everywhere you would write `match x { Ok(v) => v, Err(e) => return Err(e.into()) }`.**
   Works for `Option` in functions returning `Option`, too.
2. **Combinators over nested `match`:** `map`, `and_then`, `or_else`,
   `filter`, `zip`, `xor`, `ok_or`, `ok`, `unwrap_or`, `unwrap_or_default`,
   `map_or`, `map_or_else`, `is_some_and`, `is_none_or`, `is_ok_and`.
3. **Lazy variants for expensive defaults:** `unwrap_or_else(|| compute())`,
   `ok_or_else(|| Error::new(...))`. `unwrap_or(expensive())` evaluates
   eagerly (clippy: `or_fun_call`).
4. **`let … else` for early exit on a failed pattern:**

   ```rust
   let Some(user) = users.get(id) else {
       return Err(Error::UnknownUser(id));
   };
   ```

5. **`if let` / `while let`** for single-pattern matches. Rust 2024 also has
   **`let` chains**: `if let Some(x) = a && x > 3 { … }`.
6. **`as_ref()` / `as_mut()` / `as_deref()` / `as_deref_mut()`** to look
   inside an `Option` without moving it.
7. **`get_or_insert_with`, `take`, `replace`, `insert`** for in-place `Option`
   updates.
8. **`transpose()`** to flip `Option<Result<T, E>>` ↔ `Result<Option<T>, E>`,
   e.g. for an optional field that might fail to parse.
9. **`expect("reason this cannot fail")` over `unwrap()`** when a panic would
   indicate a bug; the message should state the invariant, not repeat the
   operation. In library code, prefer returning an error.
10. **`Option<Box<T>>`, `Option<&T>`, `Option<NonZeroU32>`, `Option<NonNull<T>>`
    cost nothing extra** — the `None` case uses the niche. Use them freely.
11. **Don't use sentinel values** (`-1`, `""`, `usize::MAX`) for "absent" —
    use `Option`.

---

## 7. Error handling

1. **Libraries: typed error enums** (hand-written or `thiserror`), so callers
   can match on variants. **Applications: `anyhow`/`eyre`** with
   `.context("while loading config")` for human-readable chains.
2. **Never `Result<T, String>` in public APIs** — strings can't be matched and
   lose sources. Implement `std::error::Error` and `Display`.
3. **`impl From<SourceError> for MyError`** (or `#[from]` with `thiserror`) so
   `?` converts automatically.
4. **Keep the source chain:** implement `Error::source()` / use
   `#[source]` rather than formatting the inner error into the message.
5. **Error messages are lowercase, no trailing punctuation** (std convention):
   `"invalid header length"`.
6. **Panics are for bugs, `Result` is for expected failure.** Document panics
   in a `# Panics` section and errors in a `# Errors` section.
7. **Make error types cheap:** box large payloads
   (`Box<ErrorKind>` inside the error) so `Result<T, E>` stays small on the
   happy path (clippy: `result_large_err`).
8. **`#[must_use]` on functions whose return value carries meaning** (builders,
   pure computations, guards). `Result` is already `#[must_use]`.
9. **Mark error paths `#[cold]` / `#[inline(never)]`** in hot code so the
   optimizer lays out the happy path first.

---

## 8. Modeling with types

1. **Newtypes for domain values:** `struct UserId(u64)`, `struct Px(f32)`.
   Zero runtime cost; prevents mixing up `user_id` and `order_id`.
2. **Enums instead of stringly-typed or boolean state.** `enum Status { Pending, Shipped, Delivered }`
   beats `status: String` — exhaustive matching catches missing cases.
3. **Make invalid states unrepresentable.** Put data inside the variant that
   needs it:

   ```rust
   // Bad: `error` is meaningful only when `state == Failed`
   struct Job { state: State, error: Option<String> }
   // Good
   enum Job { Queued, Running { started: Instant }, Failed { error: String }, Done }
   ```

4. **Typestate for protocols:** `Connection<Disconnected>` →
   `Connection<Connected>`; methods only exist on the right state, enforced at
   compile time.
5. **Derive the standard traits eagerly on public types** where they make
   sense: `Debug`, `Clone`, `Copy` (small, plain data), `PartialEq`, `Eq`,
   `Hash`, `PartialOrd`, `Ord`, `Default`. Missing derives are a common
   downstream papercut (C-COMMON-TRAITS).
6. **Conversions through std traits:** implement `From` / `TryFrom` /
   `FromStr` / `AsRef` / `Borrow`. Never implement `Into` directly — it comes
   for free from `From`.
7. **`NonZero<u32>`, `NonNull<T>`, and enums** to encode invariants and unlock
   niche optimizations.
8. **`#[non_exhaustive]` on public enums and structs** that may grow, so adding
   a variant/field isn't a breaking change.
9. **Box large enum variants** so every value isn't sized to the largest
   variant (clippy: `large_enum_variant`).
10. **`Deref` only for smart-pointer-like wrappers,** not to fake inheritance.
    Forward methods explicitly or expose `as_inner()`.
11. **`PhantomData<T>`** to carry type information (units, typestates,
    lifetimes) without storing a `T`.
12. **Prefer `struct` with named fields over tuples** once there are more than
    two fields or the meaning isn't obvious.
13. **Use `Self`** inside `impl` blocks — renaming the type is then a one-line
    change.

---

## 9. Traits and generics

1. **Static dispatch (`impl Trait` / generics) for hot paths;** `dyn Trait`
   for heterogeneous collections, plugin points, and to curb code size.
2. **`impl Trait` in argument position** for simple single-use bounds;
   **`where` clauses** when bounds get long or are reused.
3. **Associated types when there's exactly one sensible type per impl**
   (`Iterator::Item`); generic parameters when a type can implement the trait
   many ways (`From<T>`).
4. **Blanket impls for references and smart pointers** so `&T`, `&mut T`,
   `Box<T>` work wherever `T: MyTrait` does.
5. **Extension traits** to add methods to foreign types
   (`trait StrExt { fn is_identifier(&self) -> bool; } impl StrExt for str { … }`).
6. **Sealed traits** (a private supertrait) when downstream crates may use but
   not implement a trait, so you can add methods later.
7. **`Borrow<Q>` enables `HashMap<String, V>::get(&str)`** — no need to
   allocate a `String` just to look something up.
8. **Default methods on traits** to give implementors behaviour for free and
   let you grow the trait compatibly.
9. **Avoid monomorphization bloat** with the inner-function trick:

   ```rust
   pub fn read_config(path: impl AsRef<Path>) -> io::Result<Config> {
       fn inner(path: &Path) -> io::Result<Config> { /* real work */ }
       inner(path.as_ref())
   }
   ```

   Only the tiny wrapper is instantiated per caller type.
10. **Keep traits object-safe** (no generic methods, no `Self` by value in
    signatures without `where Self: Sized`) if you might want `dyn Trait`.
11. **Don't put trait bounds on struct definitions** unless the struct's
    layout needs them; put them on the `impl` blocks that require them.
12. **Implement `Default` (derive when possible)** and `IntoIterator for &T` /
    `&mut T` on your collections so `for x in &my_coll` works.

---

## 10. Collections

1. **Entry API instead of `contains_key` + `insert`/`get_mut`** — one hash
   lookup instead of two:

   ```rust
   *counts.entry(word).or_insert(0) += 1;
   groups.entry(key).or_default().push(item);
   ```

2. **Choose the right container:**
   - `Vec` — default for everything.
   - `VecDeque` — queue / ring buffer (push/pop at both ends).
   - `HashMap`/`HashSet` — unordered lookup.
   - `BTreeMap`/`BTreeSet` — ordered keys, range queries, deterministic output.
   - `BinaryHeap` — priority queue.
   - `indexmap` — hash lookup + insertion order.
   - `LinkedList` — almost never.
3. **Faster hashers for trusted keys** (`rustc-hash`/`FxHash`, `ahash`,
   `foldhash`) when HashDoS isn't a concern — SipHash's default is
   comparatively slow for small keys.
4. **Arenas + indices (`Vec<Node>` with `NodeId(u32)`, `slotmap`,
   generational indices)** instead of `Rc<RefCell<Node>>` graphs. Better
   cache locality, no refcounting, no runtime borrow panics.
5. **`swap_remove`** for O(1) removal when order doesn't matter.
6. **`HashSet::insert` returns `bool`** — use it for "first time seen?"
   instead of `contains` + `insert`.
7. **`split_at_mut`, `get_disjoint_mut` (1.86), `chunks_mut`** for multiple
   simultaneous mutable borrows into one slice/map without `unsafe`.
8. **Sort + `dedup` is often faster than a `HashSet`** for one-off
   deduplication of `Copy`/small items.
9. **`HashMap::retain` / `BTreeMap::retain`** to filter in place.
10. **`get()` instead of `[]` when the key/index may be missing** — indexing
    panics, `get` returns `Option`.
11. **`first()` / `last()` / `split_first()` / `split_last()`** instead of
    `v[0]` / `v[v.len() - 1]`.
12. **Iterate maps with `keys()`, `values()`, `values_mut()`, `into_values()`**
    rather than destructuring tuples you'll half-ignore.

---

## 11. Memory layout and codegen

1. **Measure before optimizing.** Profile (`perf`, `samply`, `cargo flamegraph`,
   Tracy), heap-profile (`dhat`, `heaptrack`), check code size (`cargo bloat`,
   `cargo llvm-lines`).
2. **Guard type sizes with compile-time assertions** on hot types:

   ```rust
   const _: () = assert!(std::mem::size_of::<Token>() == 16);
   ```

3. **Use smaller index types** (`u32` instead of `usize`) in large arrays of
   handles/indices — halves the memory on 64-bit targets.
4. **Struct-of-arrays for hot loops** that touch one field of many items —
   better cache use and auto-vectorization.
5. **Hoist bounds checks:** slice once (`let xs = &xs[..n];`) or
   `assert!(i < xs.len())` before the loop so the optimizer removes per-access
   checks.
6. **`#[inline]` on small, hot, cross-crate non-generic functions.** Generic
   functions are already inlinable across crates. Don't sprinkle
   `#[inline(always)]` without a benchmark.
7. **Avoid large values on the stack** (big arrays, large structs moved many
   times). `Box` them or pass by reference.
8. **`const` for compile-time values, `static` for a single address,
   `LazyLock` / `OnceLock` (std) for lazily-initialized globals** — no
   `lazy_static!` needed anymore.
9. **`const fn`** for constructors and pure helpers so they can be used in
   `const` contexts.
10. **Avoid `dyn` in tight inner loops** — vtable calls block inlining.
11. **Prefer `u8`/`u16` enums with `#[repr(u8)]` only when you need a fixed
    ABI/layout;** otherwise let the compiler pick and use niches.
12. **Don't read `Vec::capacity()` as a performance proxy** — growth is
    amortized; preallocate when the size is known, otherwise trust `push`.

---

## 12. Shared state, interior mutability and concurrency

1. **Ownership > borrowing > `Cell` > `RefCell` > `Mutex`.** Pick the
   simplest tool that works.
2. **`Cell<T>` for `Copy` values** — no runtime borrow flag, can't panic.
3. **`Rc<RefCell<T>>` is a design smell** in most code; prefer passing
   `&mut`, restructuring ownership, or arenas.
4. **`std::thread::scope`** to borrow stack data in threads without `Arc`.
5. **Message passing (`std::sync::mpsc`, `crossbeam-channel`) vs shared
   state (`Arc<Mutex<T>>`):** use channels for pipelines, mutexes for shared
   caches/maps.
6. **Keep critical sections short.** Clone/extract what you need, drop the
   guard, then do the work. Use a block `{ let g = m.lock(); … }` or
   `drop(guard)` to make the scope explicit.
7. **Atomics for counters/flags:** `AtomicUsize::fetch_add(1, Relaxed)` for a
   statistic; `Acquire`/`Release` pairs for publishing data. Don't default to
   `SeqCst` without thinking, and don't use `Relaxed` to publish data.
8. **`RwLock` isn't automatically faster than `Mutex`** — it only helps for
   long, read-heavy critical sections.
9. **`rayon`'s `par_iter()`** for data-parallel loops — often a one-line
   change from `iter()`.
10. **Make types `Send + Sync` where possible and assert it** in tests:
    `fn assert_send_sync<T: Send + Sync>() {}`.
11. **Don't `.lock().unwrap()` blindly in library code** — decide what
    poisoning means for your invariants (or use `parking_lot`, which has no
    poisoning).
12. **Platform constraints matter:** on `wasm32-unknown-unknown`,
    `std::thread`, `std::time::Instant::now` and blocking calls panic or hang
    — in this repo, use `incular_core::time` (see [AGENTS.md](AGENTS.md)).

---

## 13. Async

1. **Never block in async code** (no `std::thread::sleep`, blocking I/O, or
   long CPU loops) — use the runtime's async equivalents or
   `spawn_blocking`.
2. **Don't hold a `std::sync::Mutex` guard across `.await`.** Restructure, or
   use an async-aware mutex if the lock genuinely must span an await.
3. **`join!` / `try_join!` / `FuturesUnordered` for concurrency;** sequential
   `.await`s run one after another.
4. **Be cancellation-safe:** any `.await` may be the last thing that runs.
   Don't leave state half-updated across await points.
5. **Avoid boxing futures unnecessarily;** `async fn` in traits is stable
   (1.75). Use `Box::pin` only for recursion or `dyn` dispatch.
6. **Keep futures small** — large locals held across `.await` bloat the
   future's size; box big buffers or scope them before the await.

---

## 14. Control flow and readability

1. **Early returns / `let … else` / `?`** keep the happy path unindented.
2. **Expression-oriented code:** `let x = if cond { a } else { b };`,
   `let v = match … { … };`. No trailing `return` at the end of a function.
3. **`matches!(x, Pat | Pat2)`** for boolean pattern checks.
4. **Exhaustive `match` without `_ =>`** on your own enums so adding a
   variant forces every site to be revisited.
5. **Destructure in patterns:** `let Point { x, y } = p;`,
   `for (i, (a, b)) in xs.iter().zip(ys).enumerate()`, `|&(k, v)|`.
6. **Shadowing for successive transforms** (`let input = input.trim();`)
   instead of `input2`, `trimmed_input`, etc.
7. **Labeled `break 'outer` / `break 'block value`** instead of flag variables.
8. **Keep `unsafe`, `unwrap`, `as` casts, and `clone` visible and
   intentional;** prefer `u32::try_from(x)?` / `From` over `as` for lossy or
   sign-changing conversions.
9. **Prefer `impl` methods and small free functions over deep nesting;** a
   function that needs section comments wants to be split.
10. **Use `todo!()` / `unimplemented!()` / `unreachable!("why")`** to mark
    intent — and `debug_assert!` for internal invariants that are too
    expensive to check in release builds.

---

## 15. API design and naming

Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).
Highlights:

1. **Conversion prefixes:** `as_` (free, borrowed → borrowed), `to_`
   (expensive or owned output), `into_` (consumes `self`).
2. **Getters have no `get_` prefix:** `fn len(&self)`, `fn name(&self) -> &str`.
   `get` is for keyed/indexed lookup returning `Option`.
3. **Iterator methods:** `iter()`, `iter_mut()`, `into_iter()`; iterator types
   named after the method (`Iter`, `IterMut`, `IntoIter`).
4. **Constructors:** `new`, `with_capacity`, `with_*`, `from_*`; implement
   `Default` when `new()` takes no arguments (clippy: `new_without_default`).
5. **`is_*` / `has_*` for boolean queries.**
6. **Keep visibility minimal:** `pub(crate)` by default, `pub` only for the
   real API. Re-export the public surface from the crate root (or a
   `prelude`) so users don't depend on internal module paths.
7. **Feature flags must be additive** — enabling a feature must never remove
   API or change behaviour incompatibly.
8. **Document every public item.** Include a runnable example, plus
   `# Errors`, `# Panics`, and `# Safety` sections where they apply.
   `#![warn(missing_docs)]` keeps this honest.
9. **Intra-doc links** (``[`Widget::build`]``) instead of hard-coded URLs.
10. **Don't expose dependency types in your public API** unless you're willing
    to tie your semver to theirs.
11. **Make it hard to misuse:** take `NonZeroUsize` instead of documenting
    "must not be zero"; take an enum instead of a magic string.

---

## 16. `unsafe`

1. **Minimize and encapsulate.** Wrap `unsafe` in a small safe abstraction
   whose public API cannot cause UB no matter how it's called.
2. **Every `unsafe` block gets a `// SAFETY:` comment** explaining why the
   preconditions hold; every `unsafe fn` gets a `# Safety` doc section
   (clippy: `undocumented_unsafe_blocks`, `missing_safety_doc`).
3. **`#![deny(unsafe_op_in_unsafe_fn)]`** (default in edition 2024) so
   `unsafe fn` bodies still mark their individual unsafe operations.
4. **Run `cargo +nightly miri test`** on code with `unsafe`.
5. **Prefer safe, zero-cost alternatives first:** `split_at_mut`,
   `chunks_exact`, `bytemuck`/`zerocopy` for casts, `MaybeUninit` over
   `mem::uninitialized`.
6. **`get_unchecked` only with a benchmark proving the bounds check matters**
   and a `debug_assert!` guarding the index.

---

## 17. Testing and benchmarking

1. **In this repository, all tests live in `tests/` directories** — never
   `#[cfg(test)]` modules under `src/` (see [AGENTS.md](AGENTS.md)). Test the
   public API like a real caller would.
2. **Doc tests double as documentation** — every public example should
   compile and run.
3. **`assert_eq!(actual, expected, "context {x}")`** — include context so
   failures are self-explanatory. `assert!(matches!(…))` for enum shapes.
4. **Property tests (`proptest`, `quickcheck`)** for parsers, layout math,
   serialization round-trips, and invariants.
5. **Snapshot tests (`insta`)** for large structured output (render commands,
   layout trees, diagnostics).
6. **Benchmark with `criterion` or `divan`,** use `std::hint::black_box` to
   stop the optimizer from deleting the work, and compare medians across runs
   — like the 6.57 s → 0.95 s measurement in §0.
7. **Test the edge cases the type system can't:** empty inputs, `limit = 0`,
   `limit > len`, NaN, non-ASCII strings, ties in sorting.
8. **Static assertions for auto traits and sizes**
   (`const _: () = assert!(size_of::<T>() <= 32);`) so regressions fail the
   build.

---

## 18. Tooling, lints and build profiles

1. **Run the standard checks** (repo commands are in [AGENTS.md](AGENTS.md)):
   `cargo fmt`, `cargo check`, `cargo clippy --all-targets --all-features -- -D warnings`,
   tests.
2. **Workspace-wide lints** in `Cargo.toml` (`[workspace.lints.rust]`,
   `[workspace.lints.clippy]`) instead of per-crate attributes.
3. **`#[expect(lint, reason = "…")]` over `#[allow(lint)]`** (1.81) — it
   warns when the suppression is no longer needed, and the reason documents
   why.
4. **Useful opt-in clippy lints:** `clippy::pedantic` (selectively),
   `needless_pass_by_value`, `redundant_clone`, `inefficient_to_string`,
   `implicit_clone`, `manual_let_else`, `semicolon_if_nothing_returned`,
   `unnested_or_patterns`, `missing_errors_doc`, `missing_panics_doc`.
5. **Release profile knobs** (measure each):

   ```toml
   [profile.release]
   lto = "thin"        # or "fat" for max cross-crate inlining
   codegen-units = 1   # slower builds, better optimization
   debug = "line-tables-only"  # usable profiles without full debuginfo
   # panic = "abort"   # smaller/faster, but no unwinding
   ```

6. **Speed up dev builds** with `[profile.dev.package."*"] opt-level = 1..3`
   for heavy dependencies (image decoding, shaping, GPU) while keeping your
   own crates fast to recompile.
7. **`cargo deny` / `cargo audit`** for licenses and advisories;
   `cargo machete` / `cargo udeps` for unused dependencies;
   `cargo semver-checks` before publishing.
8. **`cargo tree -d`** to find duplicate dependency versions bloating builds.
9. **`cargo expand`** to debug macros; **`cargo asm`** / Compiler Explorer to
   confirm that an "optimization" actually changed the generated code.

---

## 19. Working with the borrow checker

Patterns that resolve the most common borrow errors *without* cloning or
`RefCell`.

1. **Split borrows by destructuring `self`.** A method borrowing `&mut self`
   locks the whole struct; borrowing fields separately does not.

   ```rust
   fn update(&mut self) {
       let Self { items, cache, .. } = self;
       for item in items.iter() {
           cache.insert(item.id, item.size); // ok: disjoint fields
       }
   }
   ```

   Helper methods taking `&self` can't do this — pass the specific fields
   instead (`fn measure(cache: &mut Cache, item: &Item)`).
2. **Closures capture disjoint fields** (edition 2021+): `|| self.a.len()`
   only borrows `self.a`, so you can mutate `self.b` meanwhile.
3. **Hold indices or keys across mutation, not references.** Look up again
   after the mutation; it's cheap and keeps the borrow short.
4. **Compute, then mutate.** Split "read phase" (collect what to change into a
   small `Vec` of ids/commands) from "write phase" (apply them). This is the
   standard fix for "cannot borrow as mutable because it is also borrowed as
   immutable" in loops.
5. **Move out of `&mut` with `Option::take`, `mem::take`, `mem::replace`,**
   then put the value back. Common for state machines:

   ```rust
   self.state = match std::mem::take(&mut self.state) {
       State::Loading(req) => State::Ready(req.finish()),
       other => other,
   };
   ```

   (Requires `State: Default`; otherwise use `mem::replace` with a cheap
   placeholder variant.)
6. **Reborrow instead of moving `&mut`:** pass `&mut *r` (or just `r` in
   argument position, which reborrows implicitly) when you need `r` again
   afterwards. For `Option<&mut T>`, use `.as_deref_mut()`.
7. **Return owned data from short-lived locks/borrows**, then work outside
   them (`let snapshot = guard.items.clone(); drop(guard);`) — only when the
   data is small.
8. **Don't reach for `'static` bounds to silence errors.** `T: 'static`
   forbids borrowed data; add it only if you actually store `T`
   indefinitely (threads, global registries, `Any`).
9. **Use `'_` for elided lifetimes in paths:** `Formatter<'_>`, `Iter<'_, T>`,
   `impl Trait + '_`. Write named lifetimes only when you must relate two of
   them.
10. **Higher-ranked bounds (`for<'a> F: Fn(&'a T) -> &'a U`)** are what you
    need when a closure must work for any borrow, not one specific lifetime.
    Plain `F: Fn(&T) -> &U` desugars to this already.
11. **If a struct holding references fights you everywhere, store indices or
    owned data instead.** Self-referential structs are not idiomatic; use
    arenas, offsets, or a crate like `ouroboros`/`self_cell` as last resort.

---

## 20. Closures, `Drop` and RAII

1. **RAII guards for cleanup and paired operations** (lock/unlock,
   push/pop clip, begin/end scope, restore previous value). A guard's `Drop`
   runs on every exit path, including `?` and panics.

   ```rust
   struct ClipGuard<'a> { canvas: &'a mut Canvas }
   impl Drop for ClipGuard<'_> { fn drop(&mut self) { self.canvas.pop_clip(); } }
   ```

2. **`Drop` should not panic and should be cheap.** Expose an explicit
   `close() -> Result<…>` when cleanup can fail; `Drop` is the best-effort
   fallback.
3. **Implementing `Drop` blocks moving fields out of the struct.** Keep
   `Drop` on a small inner type if you need destructuring elsewhere.
4. **Fields drop in declaration order; locals drop in reverse.** Order
   fields deliberately when one must outlive another (e.g. a GPU surface
   before its window).
5. **`let _ = expr;` drops immediately; `let _guard = expr;` lives to the end
   of scope.** A guard bound to `_` is a classic bug.
6. **`move` closures** when the closure outlives the current scope (threads,
   callbacks, tasks). Clone `Arc`s into a block right before the `move`:

   ```rust
   let handle = { let state = Arc::clone(&state); move || state.tick() };
   ```

7. **Return closures as `impl Fn…`**, `Box<dyn Fn…>` only for
   heterogeneous storage.
8. **`ManuallyDrop` / `mem::forget`** only for FFI or unsafe ownership
   transfers — never to "fix" a double-borrow.
9. **Edition 2024 shortened temporary lifetimes** in `if let` scrutinees and
   block tail expressions — a lock guard created in an `if let` condition is
   now dropped before the `else`. Bind guards to named locals when their
   scope matters.

---

## 21. Numerics

1. **Choose overflow behaviour explicitly** when it can happen:
   `checked_*` (→ `Option`), `saturating_*`, `wrapping_*`, `overflowing_*`.
   Plain `+` panics in debug and wraps in release.
2. **Use the helpers:** `abs_diff`, `div_ceil`, `rem_euclid` (always
   non-negative modulo), `clamp`, `min`/`max`, `pow`, `leading_zeros`,
   `count_ones`, `next_power_of_two`, `is_power_of_two`.
3. **`TryFrom` for narrowing integers, `From` for widening;** `as` silently
   truncates/saturates. Float→int `as` saturates and maps NaN to 0 — fine
   when intended, so say so in a comment.
4. **Floats:** never `==` on computed values — compare with an epsilon suited
   to the domain; use `total_cmp` for ordering; `f32::EPSILON` is not a
   general tolerance. `clamp` panics if `min > max` or either bound is NaN.
5. **Floats can't be `Eq`/`Hash`/`Ord`.** For map keys use `to_bits()`
   (after normalizing `-0.0` and NaN) or a wrapper like `ordered_float`.
6. **Keep units in types** (`Px`, `Dp`, `Seconds(f64)`, `Radians`) to make
   unit mix-ups compile errors.
7. **`f32` for geometry/graphics, `f64` for time accumulation and long sums**
   — accumulated `f32` time loses precision after hours of runtime.
8. **Compare to zero with `is_sign_negative`/`== 0.0` carefully** — `-0.0 == 0.0`
   is `true` but they have different bits and behave differently under
   division.

---

## 22. Arrays, slices and const generics

1. **Fixed sizes → arrays, not `Vec`:** `[f32; 4]` lives inline with no
   allocation. `std::array::from_fn(|i| …)` builds one; `arr.map(f)`
   transforms one.
2. **Slice → array:** `let [a, b, c]: [u8; 3] = slice.try_into()?;` or
   `<[u8; 4]>::try_from(&bytes[..4])`.
3. **Slice patterns:** `match xs { [] => …, [one] => …, [first, .., last] => … }`
   and `if let [x, y] = parts[..]`.
4. **Bulk slice ops** beat element loops: `fill`, `copy_from_slice`,
   `clone_from_slice`, `swap`, `reverse`, `rotate_left`, `split_off`,
   `append`, `resize`, `truncate`, `concat`, `join`, `contains`,
   `starts_with`.
5. **Flatten `Vec<Vec<T>>` grids into one `Vec<T>` with a stride** — one
   allocation and contiguous memory.
6. **Const generics (`struct Matrix<const N: usize>`)** when the size is a
   compile-time property; don't over-generalize when a runtime length works.
7. **Bit sets (`u64` masks, `fixedbitset`) instead of `Vec<bool>` /
   `HashSet<small int>`** for dense small-integer membership.

---

## 23. Hot paths, per-frame code and caching

Especially relevant to a UI framework, where build/layout/paint run every
frame.

1. **Zero allocations in the steady state.** Per-frame work should reuse
   buffers owned by long-lived structs (`clear()` + reuse), not create
   `Vec`/`String`/`Box` each frame. Verify with a counting allocator or
   `dhat`.
2. **Dirty flags / generation counters** to skip work whose inputs didn't
   change. Recompute only what's invalidated; cache the last result with its
   key.
3. **Cache by cheap keys.** Hash an id + generation, not a deep structure.
   If you must hash content, hash once and store the hash.
4. **Intern repeated strings** (`Arc<str>` or a symbol table returning
   `u32` ids) — comparisons become integer compares and memory drops.
5. **Batch work:** group commands by state (texture, pipeline), sort keys
   once, process in tight loops over contiguous data.
6. **Avoid `dyn` and `HashMap` lookups in the innermost loop** — resolve to
   indices/pointers before the loop.
7. **Use `memchr` / byte-slice search** for scanning text; avoid per-char
   allocation in shaping/parsing loops.
8. **Specialized hashers for integer keys** (`FxHash`, or `nohash` for
   already-unique ids).
9. **Pick algorithms first, micro-optimizations last.** An O(n²) relayout
   dwarfs every `#[inline]`.
10. **Benchmark the realistic size distribution** (many tiny lists *and* the
    occasional huge one); optimize the common case without making the rare
    case pathological.
11. **Global allocator swap (`mimalloc`, `jemalloc`)** is an application-level
    decision for binaries, never set in a library.

---

## 24. I/O, paths and the environment

1. **Buffer I/O:** wrap files and sockets in `BufReader`/`BufWriter`; call
   `flush()` (and handle its error) before dropping a `BufWriter` you care
   about. Lock `stdout()` once for many writes.
2. **Use the convenience functions:** `fs::read_to_string`, `fs::read`,
   `fs::write`, `io::copy`, `BufRead::lines`.
3. **Paths are not strings.** Use `Path::join`, `extension`, `file_stem`,
   `parent`, `components`, `with_extension`. Don't concatenate with `/` or
   assume UTF-8 (`to_string_lossy()` only for display).
4. **`env::var_os` when the value may be non-UTF-8;** `env::var` returns an
   error otherwise.
5. **Add context to I/O errors** — a bare "No such file or directory" doesn't
   say *which* file.
6. **Use `tempfile` for temp files** in tests and tools, not hand-built paths
   in `env::temp_dir()` (which also panics on `wasm32`).

---

## 25. Serialization (serde)

1. **Derive and configure, don't hand-write:** `#[serde(rename_all = "camelCase")]`,
   `#[serde(default)]`, `#[serde(skip_serializing_if = "Option::is_none")]`,
   `#[serde(tag = "type")]` for enums.
2. **`#[serde(deny_unknown_fields)]`** on config files to catch typos —
   but not on wire protocols that must tolerate newer peers.
3. **Version wire formats** (a `version` field or enum of versions) — a
   protocol crate like `incular-devtools-protocol` must stay compatible.
4. **Zero-copy deserialization** with `&'de str` / `Cow<'de, str>` /
   `#[serde(borrow)]` when the input buffer outlives the value.
5. **Gate serde behind a feature** in libraries
   (`#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]`).
6. **Validate after deserializing** (`TryFrom<RawConfig>`), so invalid states
   still can't exist in your domain types (§8.3).

---

## 26. Macros, `cfg`, modules and logging

1. **Functions and generics first, macros last.** Use `macro_rules!` for real
   syntax needs (repetitive impls, DSLs, variadics). Inside exported macros,
   refer to items as `$crate::path` and std as `::core::…`.
2. **Derive macros over hand-written boilerplate** when a well-known one
   exists (`thiserror`, `serde`, `bytemuck`, `strum`).
3. **Platform code behind `cfg`, in its own module:**
   `#[cfg(target_os = "macos")] mod macos;` — not `cfg` sprinkled through
   function bodies. Keep the cross-platform API identical across targets.
   (In this repo, platform APIs belong in the matching native crate.)
4. **`cfg!(…)` in expressions** when both branches compile on every target;
   `#[cfg]` when they don't.
5. **Module layout:** `foo.rs` + `foo/` (modern) or `foo/mod.rs` — pick one
   per crate consistently. Group `use` statements: std, external crates,
   `crate::`.
6. **Structured logging with `tracing`/`log`:** fields, not formatted
   strings (`debug!(widget = %id, "relayout")`); log macros are lazy, so
   formatting only happens if the level is enabled. Spans for timing.
7. **Debug impls must not leak secrets** — hand-write `Debug` for types
   holding tokens; use `finish_non_exhaustive()` for partially shown structs.
8. **`#[doc(hidden)]` for public-for-macros items** that aren't part of the
   stable API.

---

## 27. Trait-object and generic details

1. **Take `&dyn Trait` / `&mut dyn Trait` parameters, not `Box<dyn Trait>`,**
   unless you take ownership.
2. **Add `+ Send + Sync` to stored trait objects** that cross threads:
   `Box<dyn Fn() + Send + Sync>`.
3. **`T: ?Sized`** on generics that should accept `str`, `[T]` or
   `dyn Trait` behind a reference (`fn show<T: Display + ?Sized>(x: &T)`).
4. **`Any` + `downcast_ref` for type-erased maps** (context values, extension
   slots), keyed by `TypeId`.
5. **Return-position `impl Trait` captures all in-scope lifetimes in edition
   2024;** use precise capturing `impl Trait + use<'a, T>` to narrow it.
6. **`PhantomData<fn() -> T>`** if you need a type marker without making the
   struct inherit `T`'s `Send`/`Sync`/drop-check behaviour.
7. **Keep `Hash`, `Eq`, `Ord` consistent:** `a == b` ⇒ `hash(a) == hash(b)`,
   and `Ord` must agree with `PartialEq`. A `Borrow<Q>` impl must hash and
   compare identically to `Q`.
8. **`#[derive(PartialEq)]` on floats** gives NaN ≠ NaN — fine for most
   values, but be aware in change-detection ("did this value change?") code:
   a NaN field always reports "changed".
9. **Enum `Default` via derive:** `#[derive(Default)] enum Align { #[default] Start, Center, End }`.
10. **Deterministic output:** `HashMap` iteration order is random — sort or
    use `BTreeMap`/`IndexMap` for anything printed, serialized, snapshotted,
    or compared in tests.

---

## 28. Self-review checklist before finishing a change

Run through this before calling Rust work done:

1. Signatures take the most general borrowed form (§1) and ownership only
   where stored.
2. No `clone`/`to_string`/`to_vec`/`collect` that exists only to satisfy the
   borrow checker — try §19 first.
3. No allocation inside loops that could be hoisted; capacity reserved
   where the size is known.
4. Every `unwrap` is either impossible to hit (with `expect` saying why) or
   replaced by `?`.
5. Error types are typed, carry context, and preserve sources.
6. New public items have docs, sensible derives, and `#[must_use]` where
   ignoring the result is a bug.
7. Edge cases tested: empty, one, many, huge; NaN; non-ASCII; `0`/`MAX`.
8. Platform constraints respected (`wasm32` clocks/threads, native crate
   boundaries, dependency allow-list in `specs/architecture.json`).
9. `cargo fmt`, `cargo check --workspace`, `cargo test-constrained`, and
   `cargo clippy --workspace --all-targets --all-features -- -D warnings` pass.
10. Any performance claim is backed by a benchmark or profile, not intuition.

---

## 29. Anti-patterns quick reference

| Smell | Prefer |
| --- | --- |
| `fn f(s: &String)` | `fn f(s: &str)` |
| `fn f(v: &Vec<T>)` | `fn f(v: &[T])` |
| `fn f(o: &Option<T>)` | `fn f(o: Option<&T>)` |
| `.iter().cloned().filter(..)` | `.iter().filter(..)` then clone survivors |
| full sort for top-k | `select_nth_unstable_by_key` / `BinaryHeap` |
| `sort_by(\|a, b\| b.k.cmp(&a.k))` | `sort_unstable_by_key(\|x\| Reverse(x.k))` |
| `partial_cmp(..).unwrap()` on floats | `total_cmp` |
| `for i in 0..v.len() { v[i] }` | `for x in &v` / `zip` / `enumerate` |
| `.collect::<Vec<_>>().into_iter()` | keep chaining |
| `.iter().count()` on a `Vec` | `.len()` |
| `if map.contains_key(k) { map.get_mut(k) }` | `map.entry(k)` |
| `s += &format!(..)` | `write!(s, ..)` |
| `Result<T, String>` | typed error enum / `anyhow` in apps |
| `unwrap()` without a reason | `?`, or `expect("invariant")` |
| `x.unwrap_or(expensive())` | `x.unwrap_or_else(expensive)` |
| `status: String` | `enum Status` |
| `fn f(flag: bool)` | `fn f(mode: Mode)` |
| `Arc<String>` / `Arc<Vec<T>>` (immutable) | `Arc<str>` / `Arc<[T]>` |
| `Rc<RefCell<Node>>` graph | arena + `NodeId` indices |
| `lazy_static!` | `std::sync::LazyLock` |
| `#[allow(lint)]` | `#[expect(lint, reason = "…")]` |
| `as` for narrowing/sign casts | `TryFrom` / `From` |
| `Vec::new()` inside a hot loop | hoist and `clear()` |
| `impl Into<X> for Y` | `impl From<Y> for X` |
| `get_name()` getter | `name()` |
| `#[cfg(test)] mod tests` in `src/` (this repo) | `crates/<crate>/tests/` |
| `.clone()` to dodge a borrow error | split borrows / indices / read-then-write (§19) |
| `let _ = lock.lock();` | `let _guard = lock.lock();` |
| `T: 'static` to silence an error | fix the ownership; add `'static` only when storing |
| `a + b` that may overflow | `checked_add` / `saturating_add` / `wrapping_add` |
| `a % b` for wrap-around indices | `a.rem_euclid(b)` |
| `x == 0.1 + 0.2` | epsilon compare |
| `Vec<Vec<T>>` grid | flat `Vec<T>` + stride |
| `Vec<bool>` membership | bitset |
| allocating per frame | reuse buffers, dirty flags |
| `format!("{}/{}", dir, file)` paths | `dir.join(file)` |
| unbuffered `File` writes | `BufWriter` (+ `flush()`) |
| `HashMap` order in output/tests | `BTreeMap` / sort first |
| `fn f(x: Box<dyn Trait>)` (no ownership needed) | `fn f(x: &dyn Trait)` |
| macro where a function works | function / generic |
