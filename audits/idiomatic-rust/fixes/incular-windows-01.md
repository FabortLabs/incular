# Fix brief incular-windows-01

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-windows/src/platform_menus.rs`, `crates/incular-windows/src/pointer.rs`, `crates/incular-windows/src/application_shell.rs`, `crates/incular-windows/src/environment.rs`. Other agents are editing other files at
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
### A314-04 - crates/incular-windows/src/platform_menus.rs:179 (§2, low)
Evidence:
```
if let Some(snapshot) = state.snapshot.clone()
            && let Err(error) = install_snapshot_for_window(&mut state, hwnd, &snapshot)
```
Suggested fix: The clone exists only to dodge the &mut borrow of `state` in install_snapshot_for_window. Either pass split fields (`&mut windows`, `&mut popup_ids`, `&registry`, `&snapshot`) so the snapshot can be borrowed from state, or store `snapshot: Option<Rc<PlatformMenuSnapshot>>` so the clone is a refcount bump. Runs once per window registration, not per frame.

### A314-01 - crates/incular-windows/src/platform_menus.rs:378 (§16, high)
Evidence:
```
unsafe {
        DrawMenuBar(hwnd);
    }
```
Suggested fix: Add a `// SAFETY:` comment directly above the block, e.g. `// SAFETY: the HWND is registered and still live; redraw only updates its non-client menu bar.` The SAFETY comment above SetMenu does not cover this call.

### A314-03 - crates/incular-windows/src/platform_menus.rs:472 (§2, medium)
Evidence:
```
let previous = state.snapshot.clone();
    for window in state.windows.values() {
```
Suggested fix: Change `update_snapshot_in_place` to take `state: &WindowsMenuState` (it never mutates state; the caller commits snapshot and registry after success) and use `let previous = state.snapshot.as_ref();`. Then `if let Some(previous) = previous { ... }` in the rollback path. This removes a whole-snapshot clone on every successful in-place update.

### A314-02 - crates/incular-windows/src/platform_menus.rs:477 (§16, high)
Evidence:
```
if !unsafe { update_menu_nodes(window.root, &snapshot.menus, registry) } {
```
Suggested fix: Add a `// SAFETY:` comment directly above each `unsafe` call site. Same gap at build_native_menu `if !unsafe { append_menu_nodes(root, ...) }` (~line 517), append_menu_nodes recursive call `if !unsafe { append_menu_nodes(submenu, ...) }` (~line 564), update_menu_nodes `ModifyMenuW` block (~line 624) and recursive `!unsafe { update_menu_nodes(submenu, ...) }` (~line 652), and `unsafe { raw_to_wndproc(previous_raw) }` (~line 140). The existing comments sit above a different statement. Justify each call with the structure-equality or detached-menu invariant it relies on.

### A314-05 - crates/incular-windows/src/pointer.rs:122 (§2, medium)
Evidence:
```
let mut infos = Vec::<POINTER_PEN_INFO>::with_capacity(count);
        // SAFETY: `infos` has capacity for exactly entries*pointers structures.
```
Suggested fix: Allocate per message: this Vec, the `captured` Vec from collect() (~line 147-155), and the `vec![...]` fallback (~line 170). Keep a reusable scratch `Vec<POINTER_PEN_INFO>` and a captured-sample buffer in the closure returned by message_hook (`let mut scratch = Vec::new();` moved into the `Box::new(move |message| ...)`). Pass `&mut scratch` into capture_pen_frame, call `clear()` and `reserve(count)`, and write samples into a caller-owned buffer instead of returning a new Vec.

### A314-06 - crates/incular-windows/src/pointer.rs:146 (§16, medium)
Evidence:
```
unsafe { infos.set_len(initialized) };
```
Suggested fix: Add a `// SAFETY:` comment directly above this statement stating that Win32 initialized `initialized` (clamped to `count`, the allocated capacity) POD POINTER_PEN_INFO entries. The existing SAFETY comment sits above the `let initialized` computation, not the set_len call.

### A314-07 - crates/incular-windows/src/pointer.rs:168 (§16, medium)
Evidence:
```
let info = unsafe { info.assume_init() };
```
Suggested fix: Add `// SAFETY: GetPointerPenInfo returned nonzero, so it fully initialized `info`.` directly above this line. The comment at lines 162-163 is attached to the GetPointerPenInfo call and does not cover this assume_init block.

### A313-02 - crates/incular-windows/src/application_shell.rs:167 (§16, high)
Evidence:
```
let coinit = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
```
Suggested fix: Keep the existing COM rationale comment (lines 162-166) and add a labelled line directly above line 167: `// SAFETY: CoInitializeEx takes no borrowed pointers; the HRESULT is checked below and only a successful result is balanced by ComUninitializeGuard.`

### A313-01 - crates/incular-windows/src/application_shell.rs:269 (§16, high)
Evidence:
```
let handle =
    unsafe { CreateIcon(None, width, height, 1, 32, and_mask.as_ptr(), bgra.as_ptr()) }
```
Suggested fix: Add a `// SAFETY:` comment directly above the `let handle =` line, e.g. `// SAFETY: and_mask and bgra outlive this call and hold at least width*height mask bytes and width*height*4 XOR bytes; CreateIcon copies both before returning, and width/height are the validated icon dimensions.`

### A313-03 - crates/incular-windows/src/environment.rs:37 (§8, low)
Evidence:
```
active: AtomicU8,
// with ACTIVE_TRUE / ACTIVE_FALSE consts (lines 27-28) encoding a bool
```
Suggested fix: The active flag is a plain bool. Use `active: AtomicBool` and delete the ACTIVE_TRUE/ACTIVE_FALSE consts (lines 27-28). Constructor: `active: AtomicBool::new(query_application_active())`; `application_active`: `self.inner.active.load(Ordering::Acquire)`; hook: `let active = message.wParam != 0; if inner.active.swap(active, Ordering::AcqRel) != active { ...push_back(if active { PlatformLifecycle::Active } else { PlatformLifecycle::Inactive }) }`.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
