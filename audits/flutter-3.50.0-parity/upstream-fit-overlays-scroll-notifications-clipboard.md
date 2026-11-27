# Fit, overlays, scrolling, notifications, clipboard, and color effects

Pinned Flutter 3.50.0 `ab2912035e`; Incular production `c647cb85`.

- FittedBox contain, unclipped cover paint, and transformed pointer
  delivery match (:41, :71, :396). Updating the same mounted
  FittedBox from ScaleDown to Contain and back gives heights
  [50,100,50] in Flutter and [50,50,50] in Rust (:487).
  The Rust layout branch does not use fit. This related C32
  regression tests a retained update with finite width and loose
  height. No claim about Rust's absent public clipBehavior option.
  See `flutter_fitted_box_geometry_parity`.
- FractionallySizedBox nested fractions and absolute top-right
  alignment under RTL match (:9, :32). The zero-size host case
  (:100) matches under equivalent layout constraints, not actual
  native viewport resize. Tight OverflowBox (:10) matches;
  it does not resolve C31's loose maximum constraint gap.
  See `flutter_fractional_overflow_geometry_parity`.
- OverlayPortal show/hide (:765) and inherited declaration-local
  directionality (:104) match. First builder evaluation and the
  pointer hit in moved overlay (:810, :893) match with an explicit
  Rust Builder and zero-margin anchored mount; these are partial.
  Different controller, root-overlay, and listener-order APIs
  are not implied. See `flutter_overlay_portal_upstream_parity`.
- All five exact scrolling physics behaviors match: unchanged
  initial release velocity, increasing resistance, easier easing,
  fast physics's zero easing resistance, and independence from
  content length. The gesture/scroll coordinate sign is normalized.
  See `flutter350_upstream_overscroll_offsets`.
- Manual custom notification dispatch and cancellation (:15, :43)
  match synchronous bubbling, same payload identity, and inner
  listener consumption. Flutter subtype matching is not equivalent
  to Rust's exact-type handlers and was not counted. See
  `flutter_notification_upstream_parity`.
- EditableText's clipboard gating matches the original neutral
  subsets for empty clipboard, read-only and obscured states
  (:2788, :3118, :3152), not their toolbar appearance. Read-only
  obscured Select All (:3235) disagrees on selection. The original
  uses programmatic selection; the independently verified
  `select_all_read_only_obscured_reference_test.dart` also compares
  the actual same Ctrl+A path with a non-obscured positive control.
  Flutter leaves the caret collapsed, Incular selects all 0..9.
  See `flutter350_clipboard_menu_gating_parity`.
- Original ColorFiltered sepia and zero area report passes
  (:33, :119). The custom pair tests other matrix values and
  actual presented zero-area pixels, not original golden
  equivalence. Opacity order and all paints inside the child
  match. A matrix that makes transparent black opaque should
  produce [0,32,0,255] through the enclosing clip; Windows wgpu
  only filters the child's own bounds. This is one scoped
  bounds discrepancy, not separate defects per nesting case.
  See `flutter350_color_filter_alpha_order_raster` and
  `color_filter_alpha_order_raster_test.dart`.

### Color filter bounds recheck

Windows wgpu now matches all pixels in the authored color filter probe.
When the matrix maps transparent black to a nontransparent color, that
color covers the current clip beyond the child texture. The child still
shows its own filtered pixels, and a larger offset clip correctly shows
both the translucent matte and the untouched background. Parent opacity,
zero-area content, and alpha-preserving matrices retain their controls.
The fix extends the composite quad only; it does not enlarge the cached
GPU source or effect textures. The pinned Flutter probe passes all three
registrations. This does not upgrade the original sepia golden comparison.

The original layer-reuse and zero-area cases (:97 and :119) both
pass on the pinned SDK. Incular's `paint_options` ports verify that
changing the color updates the emitted filter without replacing
its layer ID, and that a zero-area filter lays out at zero size
and paints without error. These ports initially used Screen because
BlendMode.color was missing.

### Nonseparable color blends and original layer identity

Hue, Saturation, Color and Luminosity now run on the CPU and on wgpu
through a shared WGSL implementation for ColorFiltered and direct
destination blending. The pinned Flutter and native readbacks agree
within one byte per channel for all four modes, using both opaque and
translucent destinations. The Rust test reconciles one retained tree
through these cases so layer IDs do not alias between independent trees.

The original layer-reuse port now uses the same Color blend over a
Placeholder and asserts that changing red to green keeps the same
layer identity; it is fully paired. The zero-area case was partial
at this point because Flutter omitted the child. These tests are
not the original red golden.

### ColorFilter.saturation

The pinned factory leaves alpha alone and uses its saturation argument
unclamped in the matrix. The authored pinned and Windows wgpu tests
agree for -1, 0, 1, 2 and 8, including negative extrapolation, identity,
and channel clipping at strong saturation. CPU tests also check
transparent input and preserved half alpha. This verifies the factory
and raster values but not the original FlutterLogo golden (:85).

### Clear and destination-only blending

Clear and Dst are available as matrices for ColorFiltered and as
fixed-function wgpu blends. Clear discards both colors in the painted
region; Dst keeps the original destination, including its alpha. They
do not require a copied destination texture or renumber any existing
blend code. The paired pixel checks cover both modes as isolated color
filters and direct blends over opaque and translucent destinations.

### Childless ColorFiltered

An optional child now stays genuinely absent in the retained tree.
The original zero-area case (:119) is fully paired using the same
blue Color blend and no child. Rust also verifies the smallest size
under zero, loose, and tight constraints, and reconciles a child in
and out. Pinned Flutter and Windows wgpu both leave the background
unchanged for an absent child even when the matrix would color
transparent black in a populated filter.

### Keyboard Select All honors EditableText selection policy

The pinned Windows Ctrl+A probe and Rust controls now agree: a
read-only obscured field keeps its selection unchanged by default,
an explicit false disables the shortcut in a plain editor, and
an explicit true permits the keyboard action in a read-only
obscured editor. The latter still does not enable pointer handles
or keyboard-platform selection. The focused editor can adopt the
option on rebuild without replacing its focus. Clipboard menus
and native input controls remain green. The :3235 case is still a
partial port: the pinned original calls the programmatic setter,
not the keyboard action.
## FittedBox actual baselines

An authored probe queries baselines during parent layout, as Flutter's
render API requires. A tightly sized 200 by 200 FittedBox scales its
visible child but reports the child's untransformed actual baseline.
The old Incular returned None for a visible text child with a real
baseline. Both now report the actual metric, then None when replacing
the child with a baseline-free box, then the real baseline again on the
same FittedBox render object. This does not compare font metrics between
renderers or claim a public dry-baseline API.

## FittedBox layout aspect ratio

The pinned :10, :434 and :487 originals pass. Old Incular always
clamped width and height independently. In the original 200-wide
case its default box stayed 50 high instead of 100. A larger
scaleDown child kept 200 height instead of shrinking to 100.
The corrected box adjusts the size in the same bounds order as
Flutter, using loosened minimums for scaleDown, and only then
applies the paint fit. In addition to the sizes, the permanent
Rust tests check the original world-space edge and both scaleDown
offsets, and the 50→100→50 sequence on one retained render object.
All 43 selected layout and semantics regressions pass. :487 is
a historical pair; this adds only the two new full comparisons.

## Childless FittedBox and zero area

The four pinned originals :102, :524, :537 and :559 pass. A genuinely
childless FittedBox can now be constructed. It shrinks in loose bounds
and handles a down/up input sequence safely in a tight frame. A zero
child completes both tight scaleDown and loose contain frames; a
nonempty child completes a zero viewport without giving it area.
A retained control inserts, removes, and reinserts a drawable child
on the same wrapper and observes draw submission follow its presence.
All 18 targeted Rust layout, baseline and semantics tests passed.

## FittedBox clips only overflow

Pinned :423 observes the default Clip.none and then changes the mode
on the same render object. The Rust port also verifies emitted hard,
antialiased and isolated coverage on actual overflowing content,
without relayout. Four other pinned layer tests pass. The Rust
adaptation checks contain and cover on both axes and all 81 size
combinations for BoxFit.none with a hard clip. Matching clip presence
does not equate Flutter's engine layer types to Incular's retained
commands, so those four comparisons are partial. All 19 targeted
Rust checks pass; no native raster comparison is claimed.

## FittedBox directional alignment

Pinned :115 and the Rust port agree at all five checkpoints. Bottom-end
starts on the left in RTL and moves to the right in LTR. Centering,
replacing the child size, and changing to fill match both projected
corners and the raw layout size. The wrapper keeps its render identity.
All 41 focused Rust checks passed. Every registration in
`widgets/fitted_box_test.dart` now has a disposition, including the
four partial engine-layer comparisons above.

## Explicit selection

Pinned original :3235 and the direct retained EditableText port now
match. The explicit Select All command does not move a read-only
obscured editor's caret or acquire focus, even when keyboard selection
has explicitly been enabled. For an ordinary read-only or editable
obscured field the explicit command selects the full text and focuses
the field even when its interactive shortcut is disabled. The toolbar
button uses this direct path; keyboard availability remains separate.

All four pinned authored cases, both native direct selection checks,
six keyboard/menu regressions, and five editor-focus controls passed.
The Unicode control selects the same logical string with Incular's
UTF-8 offsets without changing the committed text.

## Direct editor commands and clipboard shortcuts

The pinned :3192 and :18708 registrations passed. A mounted neutral
editable with a selected password still offers Paste and Select All
but no Copy or Cut, and calling either command directly leaves the
clipboard, text and selection unchanged. A separate editor with
interactive selection disabled still accepts typing but cannot use
Select All, Copy, Cut or Paste through keyboard shortcuts. Before this
fix Paste replaced its text despite the flag. Direct commands and menu
buttons are not blocked by the shortcut policy; ordinary direct Copy
and Cut still work when shortcut selection is disabled.

## Copy preserves selection on desktop and iOS

All four pinned :2891 platform variants and both pinned :2937 variants
passed. Native checks invoke an actual retained editor toolbar button
with a reversed selection. Copy writes the selected substring and
closes the toolbar without editing the text. It preserves the selection
on desktop and iOS; on Android it collapses at the logical end rather
than the active endpoint and hides the handles. The Android/Fuchsia
policy is shared, not a claim about a Fuchsia clipboard backend. The
Android collapse failed before the fix. All 25 focused native clipboard,
retained menu and autofill tests passed.
