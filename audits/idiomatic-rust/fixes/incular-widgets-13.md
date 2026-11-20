# Fix brief incular-widgets-13

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-widgets/src/platform_widgets/menu.rs`. Other agents are editing other files at
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
### A247-01 - crates/incular-widgets/src/platform_widgets/menu.rs:521 (§12, medium)
Evidence:
```
        self.target
            .borrow()
            .as_ref()
```
Suggested fix: Clone the Rc out of the RefCell before calling the delegate so no borrow is held across the callback:
    fn forward(
        &self,
        call: impl FnOnce(&dyn PlatformMenuDelegate) -> PlatformMenuUpdate,
    ) -> PlatformMenuUpdate {
        let delegate = self.target.borrow().clone();
        delegate.map_or(PlatformMenuUpdate::NoOpUnsupported, |delegate| call(delegate.as_ref()))
    }
Apply the same pattern to handle_shortcut (lines 555-560), which also calls the delegate while the Ref guard is alive: `let delegate = self.target.borrow().clone(); delegate.is_some_and(|delegate| delegate.handle_shortcut(key, modifiers))`. The Rc clone is a refcount increment, not an allocation.

### A248-06 - crates/incular-widgets/src/platform_widgets/menu.rs:622 (§2, low)
Evidence:
```
.field("installed", &self.snapshot().is_some())
```
Suggested fix: Read the flag without cloning the whole menu tree: `.field("installed", &self.inner.state.borrow().installed.is_some())`.

### A248-04 - crates/incular-widgets/src/platform_widgets/menu.rs:664 (§11, medium)
Evidence:
```
let (snapshot, callbacks) = build_snapshot(menus)?;
let had_installed = self.inner.state.borrow().installed.is_some();
if self.inner.state.borrow().installed.as_ref() == Some(&snapshot) {
```
Suggested fix: install() rebuilds the full snapshot (BTreeSet, Strings, Vec nodes, format! paths) on every call, including every widget rebuild via reconcile_from -> install_if_bound (tree/reconciliation.rs:516 and :436), and only then compares with the installed snapshot. Skip the rebuild when the declarative model is unchanged: have PlatformMenuBar::update/reconcile_from compare the incoming menus with the current ones (or set a generation flag when they differ) and call install only on change. Callback-only changes still need the callback rebuild, so keep a path that rebuilds only callbacks.

### A248-05 - crates/incular-widgets/src/platform_widgets/menu.rs:970 (§2, medium)
Evidence:
```
let incoming = incoming.state.borrow();
(
incoming.menus.clone(),
```
Suggested fix: Move instead of cloning the menu Vec: `let menus = std::mem::take(&mut incoming.state.borrow_mut().menus);` (the incoming binding is a freshly built widget state that is discarded after reconciliation). Verify at tree/reconciliation.rs:516 that the incoming binding is not reused before relying on this.

### A248-07 - crates/incular-widgets/src/platform_widgets/menu.rs:1044 (§12, low)
Evidence:
```
.install(self.state.borrow().menus.as_slice())
```
Suggested fix: The shared RefCell borrow of binding state is held while install() calls delegate methods (acquire, set_event_handler, set_menus). Hold the menus as an owned handle instead, e.g. store `menus: Rc<[PlatformMenu]>` and do `let menus = Rc::clone(&self.state.borrow().menus); self.controller.install(&menus)`. The same pattern appears in install_if_bound (line 1012).

### A248-03 - crates/incular-widgets/src/platform_widgets/menu.rs:1068 (§5, medium)
Evidence:
```
build_menu_node(menu, &format!("menu.{index}"), &mut ids, &mut callbacks)
.map(|(group_index, item)| {
&format!("{path}.group.{index}.item.{group_index}"),
```
Suggested fix: Stop formatting a path String for every node (also at lines 1098, 1104, 1115, 1122), even when the node has an explicit id. Thread one reusable `path: &mut String` through build_menu_node/build_item_node: record `let len = path.len();`, `write!(path, ".menu.{index}")`, recurse, then `path.truncate(len)`. Only call MenuItemId::generated(path) when menu.id/item.id is None.

### A248-02 - crates/incular-widgets/src/platform_widgets/menu.rs:1095 (§3, medium)
Evidence:
```
.flat_map(|(index, entry)| match entry {
PlatformMenuEntry::Menu(menu) => vec![build_menu_node(
PlatformMenuEntry::Item(item) => vec![build_item_node(
```
Suggested fix: Replace the flat_map/vec![]/collect chain with a loop over a Vec::with_capacity(menu.entries.len()): Menu => children.push(build_menu_node(..)?), Item => children.push(build_item_node(..)?), Group => for (group_index, item) in group.items.iter().enumerate() { children.push(build_item_node(item, &format!(..), ids, callbacks)?) }, Divider => children.push(PlatformMenuSnapshotNode { .. }). This removes one single-element Vec per entry, the intermediate Vec per group and the Result collect.


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
