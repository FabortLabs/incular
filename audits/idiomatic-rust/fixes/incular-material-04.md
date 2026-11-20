# Fix brief incular-material-04

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: `crates/incular-material/src/menus/controls.rs`, `crates/incular-material/src/theme/theme_data.rs`, `crates/incular-material/src/app_shell/navigation.rs`. Other agents are editing other files at
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
### A098-03 - crates/incular-material/src/menus/controls.rs:184 (§2, low)
Evidence:
```
let children = if let Some(style) = value.item_style.clone() {
value
    .children
```
Suggested fix: Borrow the style; the per-child style.clone() inside the map already produces the owned value Button needs:
let children = if let Some(style) = &value.item_style {
    value.children.iter().cloned().map(|child| ControlButton::with_child(child).style(style.clone()).into()).collect::<Vec<Widget>>()
}

### A098-01 - crates/incular-material/src/menus/controls.rs:408 (§2, medium)
Evidence:
```
let mut children = Vec::with_capacity(3);
if let Some(icon) = &self.leading_icon {
    children.push(icon.clone());
```
Suggested fix: Build the Vec only in the icon branch and drop the unused allocation in the common case:
let mut content: Widget = if self.leading_icon.is_none() && self.trailing_icon.is_none() {
    self.child.clone()
} else {
    let mut children = Vec::with_capacity(3);
    if let Some(icon) = &self.leading_icon { children.push(icon.clone()); }
    children.push(self.child.clone());
    if let Some(icon) = &self.trailing_icon { children.push(icon.clone()); }
    Row::new(children).spacing(8.0).cross_axis_alignment(CrossAxisAlignment::Center).into()
};

### A098-02 - crates/incular-material/src/menus/controls.rs:459 (§2, low)
Evidence:
```
.and_then(|menus| menus.menu_button_theme.style.clone());
let mut style = self
```
Suggested fix: Borrow instead of cloning, since the value is only passed to merge(&Self):
let inherited_button = menus.as_ref().and_then(|menus| menus.menu_button_theme.style.as_ref());
...
if let Some(inherited) = inherited_button {
    style = style.merge(inherited);
}

### A098-04 - crates/incular-material/src/menus/controls.rs:690 (§2, low)
Evidence:
```
let mut row = Row::new([value.child.clone()]).spacing(8.0);
if let Some(icon) = value.leading_icon.clone() {
    row = Row::new([icon, value.child.clone()]).spacing(8.0);
```
Suggested fix: Build the row once with a match on the owned value instead of allocating a Row that is immediately discarded when a leading icon exists:
let row = match value.leading_icon {
    Some(icon) => Row::new([icon, value.child]),
    None => Row::new([value.child]),
}.spacing(8.0);

### A107-04 - crates/incular-material/src/theme/theme_data.rs:450 (§14, low)
Evidence:
```
if patch.brightness.is_some()
    || patch.color_scheme.is_some()
    || patch.text_theme.is_some()
```
Suggested fix: The ten-clause condition is copied verbatim from global_control_projection_changed (lines 439-448), which is already computed above. Replace the copy with the existing flag:
if global_control_projection_changed {
    let core = Rc::make_mut(&mut inner.core);
    ...
}
This removes the duplicated list that must otherwise be kept in sync by hand.

### A107-05 - crates/incular-material/src/theme/theme_data.rs:528 (§26, low)
Evidence:
```
macro_rules! patch_button {
    ($field:ident) => {
        if let Some(value) = patch.$field {
```
Suggested fix: The same macro is defined five times (patch_button, patch_navigation, patch_selection, patch_surface, patch_menu at lines 528, 555, 578, 597, 625), each doing `if let Some(value) = patch.$field { group.$field = value; }`. Use one helper function and drop the macros:
fn set_if_some<T>(slot: &mut T, value: Option<T>) {
    if let Some(value) = value {
        *slot = value;
    }
}
Then call set_if_some(&mut group.elevated_button_theme, patch.elevated_button_theme); and so on for each field.

### A107-03 - crates/incular-material/src/theme/theme_data.rs:891 (§2, low)
Evidence:
```
themes.elevated_button_theme.elevation = Some(ControlTheme::default().elevation.popup.min(6.0));
```
Suggested fix: Avoid building a full dark ControlTheme (palette, density, motion) just to read one elevation value. Read the popup elevation from a shared constant or a small accessor that does not construct the theme, for example a `const` next to the elevation definition in incular-controls. The same pattern appears at line 905 (card_theme.elevation). Keep the resulting values identical.

### A107-01 - crates/incular-material/src/theme/theme_data.rs:1072 (§2, medium)
Evidence:
```
let fallback = previous.target.clone();
let displayed = previous.displayed(&fallback);
```
Suggested fix: Move both lines into the else branch, where displayed is used. In the equal-target branch the value is discarded, but previous.displayed() runs a full ThemeData::lerp, which rebuilds the ControlTheme and reallocates several Rc groups:
} else {
    let fallback = previous.target.clone();
    let displayed = previous.displayed(&fallback);
    replacement.start.replace(Some(displayed));
    replacement.controller.reset();
}

### A077-01 - crates/incular-material/src/app_shell/navigation.rs:168 (§2, medium)
Evidence:
```
let inherited = inherited.cloned().unwrap_or_default();
```
Suggested fix: Borrow instead of cloning the whole ComponentThemeData (it carries an Option<ButtonStyle> and about 30 fields): `let default = ComponentThemeData::default(); let inherited = inherited.unwrap_or(&default);`. The later field reads (`inherited.label_color.or(..)`, `inherited.minimum_size.map(..)`) work unchanged on a reference because the fields are Copy Options. The same pattern repeats in NavigationDrawer::build_with_theme at line 416; fix both together.

### A077-03 - crates/incular-material/src/app_shell/navigation.rs:214 (§2, low)
Evidence:
```
let mut children = Vec::new();
```
Suggested fix: The final child count is known up front (leading + destinations.len() + trailing), so write `let mut children = Vec::with_capacity(self.destinations.len() + 2);`.

### A077-02 - crates/incular-material/src/app_shell/navigation.rs:424 (§2, medium)
Evidence:
```
children.extend(self.children.clone());
```
Suggested fix: Iterate the borrowed slice instead of cloning the Vec first: `children.extend(self.children.iter().cloned());`. The current form allocates a temporary Vec<Widget> that is dropped right after the extend. Also size the vector up front: `let mut children = Vec::with_capacity(self.children.len() + self.destinations.len() + 1);` (line 420).


## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
