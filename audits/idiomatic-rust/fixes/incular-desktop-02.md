# Fix brief incular-desktop-02

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-desktop/src/lib.rs`, `crates/incular-desktop/src/display.rs`, `crates/incular-desktop/src/transients.rs`, `crates/incular-desktop/src/devtools_overlay.rs`, `crates/incular-desktop/src/global_shortcuts.rs`, `crates/incular-desktop/src/file_dialogs.rs`, `crates/incular-desktop/src/pointer.rs`. Other agents are editing other files at
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
### A027-02 - crates/incular-desktop/src/lib.rs:355 (§2, low)
Evidence:
```
let single_instance = if let Some(policy) = application.single_instance_policy().cloned() {
    match acquire_single_instance(&policy, launch_activation.clone())
```
Suggested fix: let single_instance = if let Some(policy) = application.single_instance_policy() {
    match acquire_single_instance(policy, launch_activation.clone())
// The borrow ends with the if-let block: nothing inside it touches `application` mutably, so the clone only copies the policy once at startup for no reason.

### A027-03 - crates/incular-desktop/src/lib.rs:679 (§19, low)
Evidence:
```
let drops = self
    .windows
    .values_mut()
```
Suggested fix: Iterate directly, splitting the borrow between the two fields:
for state in self.windows.values_mut() {
    if let Some(event) = state.external_file_drag.take_drop() {
        let _ = self.application.handle_external_drag_event(state.id, event);
    }
}
Disjoint field borrows (self.windows vs self.application) make the intermediate Vec unnecessary.

### A028-01 - crates/incular-desktop/src/lib.rs:1457 (§2, medium)
Evidence:
```
let mut failed = Vec::new();
let mut ready = HashSet::new();
... ready.insert(partition);
```
Suggested fix: Hoist `ready` into a runner field (for example `ready_transient_partitions: HashSet<SurfacePartition>`, created in the constructor). At the top of this block call `self.ready_transient_partitions.clear();`, then insert into it, and pass `&self.ready_transient_partitions` to `detach_surface_partitions`. Keep `failed` as is, since it only allocates on the failure path. Borrow note: take the set out with `std::mem::take` and restore it after `detach_surface_partitions` if the `transient_context()` borrow conflicts with `&self` access.

### A029-01 - crates/incular-desktop/src/lib.rs:2626 (§2, medium)
Evidence:
```
fn platform_menu_key(event: &winit::event::KeyEvent) -> Option<String> {
    match &event.logical_key {
        winit::keyboard::Key::Character(value) if !value.is_empty() => Some(value.to_string()),
```
Suggested fix: Return a borrowed-or-owned key so character keys (the common case) do not allocate per key press: `fn platform_menu_key(event: &winit::event::KeyEvent) -> Option<std::borrow::Cow<'_, str>>` with `winit::keyboard::Key::Character(value) if !value.is_empty() => Some(Cow::Borrowed(value.as_str()))` and `winit::keyboard::Key::Named(value) => Some(Cow::Owned(format!("{value:?}")))`. The caller at lib.rs:1687-1689 passes `&key`, which deref-coerces to `&str`, so it needs no change. The Named arm still formats per press; a `match` returning `&'static str` for the handled named keys would remove that too if the menu lookup only needs the known names.

### A025-05 - crates/incular-desktop/src/display.rs:25 (§1, medium)
Evidence:
```
monitors: Vec<MonitorHandle>,
primary: Option<MonitorHandle>,
```
Suggested fix: Both parameters are only read: `monitors` is used only as `&monitors` and `primary` only through `.as_ref()`. Change to `monitors: &[MonitorHandle], primary: Option<&MonitorHandle>` and drop the `.as_ref()` at line 32; callers pass `&monitors` and `primary.as_ref()`.

### A025-06 - crates/incular-desktop/src/display.rs:70 (§4, low)
Evidence:
```
snapshots.sort_by_key(|display| display.id);
```
Suggested fix: DisplayId values are unique by construction (index plus generation), so stability is irrelevant: `snapshots.sort_unstable_by_key(|display| display.id);`.

### A032-02 - crates/incular-desktop/src/transients.rs:189 (§10, medium)
Evidence:
```
let desired = snapshots
.iter()
.copied()
```
Suggested fix: synchronize_transient_hosts runs once per painted frame (lib.rs, transient block). Avoid building a HashMap each frame: iterate `snapshots` with a filter predicate and use `snapshots.iter().find(|s| s.id == key.transient)` (or `any`) where `desired.get` / `desired.contains_key` are used today. Transient counts per owner are small, so a linear scan is cheaper than hashing and allocates nothing. If a set is still needed for the stale-key pass, keep a scratch buffer on the owner state and clear it rather than allocating per frame.

### A032-01 - crates/incular-desktop/src/transients.rs:571 (§2, medium)
Evidence:
```
state.display_list = list.translated(Offset::new(
-state.snapshot.content_offset().x,
-state.snapshot.content_offset().y,
```
Suggested fix: Add an in-place variant in incular-rendering, e.g. `pub fn translate_from(&mut self, source: &DisplayList, offset: Offset)` that clears `self` and reuses its command storage (pushing PushTransform, extend_from(source), PopTransform, or a plain copy when the offset is zero). Then replace the assignment with `state.display_list.translate_from(list, Offset::new(-state.snapshot.content_offset().x, -state.snapshot.content_offset().y));`. This removes the new DisplayList allocation and the full command copy that DisplayList::translated performs on every transient frame. The retained state.display_list buffer is already there to be reused.

### A023-01 - crates/incular-desktop/src/devtools_overlay.rs:209 (§2, low)
Evidence:
```
Some(us) => format!("{name} {:.1} ms", us as f32 / 1000.),
None => format!("{name} n/a"),
```
Suggested fix: performance_overlay runs once per frame while the opt-in DevTools overlay is on, and each call allocates a fresh String per row (plus the label_commands Vec built in draw_text). Keep a reusable String per row in the overlay state and rewrite it with `use std::fmt::Write; buf.clear(); write!(buf, "{name} {:.1} ms", us as f32 / 1000.)` (or `write!(buf, "{name} n/a")`), passing `&buf` to draw_text. Only re-format when `latest` changes. Measure with the overlay enabled before and after.

### A026-01 - crates/incular-desktop/src/global_shortcuts.rs:195 (§2, medium)
Evidence:
```
let Ok(native_code) = NativeCode::from_str(&format!("{code:?}")) else {
    return false;
};
let native_modifiers = native_modifiers_from_winit(modifiers);
```
Suggested fix: Compare modifiers first and return before the allocation and string parse when no registration can match:
let native_modifiers = native_modifiers_from_winit(modifiers);
if !self.registrations.values().any(|r| r.hotkey.mods == native_modifiers) {
    return false;
}
let PhysicalKey::Code(code) = event.physical_key else { return false; };
let Ok(native_code) = NativeCode::from_str(&format!("{code:?}")) else { return false; };
self.registrations.values().any(|r| r.hotkey.key == native_code && r.hotkey.mods == native_modifiers)

### A026-02 - crates/incular-desktop/src/file_dialogs.rs:171 (§2, low)
Evidence:
```
.iter()
.map(|extension| extension.as_str().to_owned())
.collect::<Vec<_>>()
```
Suggested fix: Borrow instead of allocating a String per extension; native_dialog's add_filter takes V: AsRef<[U]> with U: ToString, so &str works:
let extensions = filter
    .extensions_list()
    .iter()
    .map(|extension| extension.as_str())
    .collect::<Vec<&str>>();

### A030-01 - crates/incular-desktop/src/pointer.rs:22 (§10, low)
Evidence:
```
if let Some(id) = self.ids.get(&device).copied() {
    return id;
}
let id = self
```
Suggested fix: Use the entry API so the map is probed once. Borrow the counter and map as disjoint fields and share one helper with native_id (lines 37-48), which currently duplicates the same logic:

fn next_public_id(next: &mut u64) -> u64 {
    *next = next.checked_add(1).expect("pointer device id space exhausted");
    *next
}

pub(crate) fn id(&mut self, device: DeviceId) -> u64 {
    *self.ids.entry(device).or_insert_with(|| next_public_id(&mut self.next))
}

fn native_id(&mut self, device: u64) -> u64 {
    *self.native_ids.entry(device).or_insert_with(|| next_public_id(&mut self.next))
}

The same id sequence is preserved, so behavior is unchanged. The lookup on a hit becomes one hash instead of get plus a second hash on miss.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
