# Text, implicit animation and retained scrolling

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production
`c647cb85`.

| Scratch batch | Scenario | Result | Frames |
| --- | --- | --- | ---: |
| `20261124-162324-617` | `layout__layout_builder__constraint_size` | match | 2 |
| `20261124-162324-617` | `layout__intrinsic_width__step` | match | 2 |
| `20261124-162324-617` | `layout__flex__spacing_and_fixed_gaps` | match | 2 |
| `20261124-162324-617` | `layout__unconstrained_box__constrained_axis` | match | 2 |
| `20261124-162324-617` | `layout__fractionally_sized_box__null_factor_tight_parent` | match | 2 |
| `20261124-162504-359` | `text__editable_caret_blink_1200ms` | match | 77 |
| `20261124-162504-359` | `text__editable_multiline_vertical_document` | match | 9 |
| `20261124-162504-359` | `text__selection_area_long_press_drag` | match | 5 |
| `20261124-162603-107` | `implicit__positioned_directional__direction` | match | 27 |
| `20261124-162603-107` | `implicit__tween_animation_builder__initial_and_retarget` | match | 33 |
| `20261124-162603-107` | `implicit__retarget__new_duration_mid_flight` | match | 34 |
| `20261124-162603-107` | `implicit__default_text_style__text_style` | match | 27 |
| `20261124-162603-107` | `implicit__curve_only_update_mid_flight` | match | 34 |
| `keep-alive-retention-frames-20261124-162637-095` | `slivers__keep_alive__retains_scrolled_child` | match | 6 |
| `20261124-161928-324` | `scroll__page_view__fling_page_changed` | mismatch | 68 |

The repaired KeepAlive frame fixture verifies the same retained
Element across a jump away and back, plus onstage geometry. Six frames
match. Offscreen paint is a distinct contract: with clipping disabled,
Flutter leaves the parked-only pixel transparent, while Incular
continues to emit the parked row into its compositor output. Dedicated
Rust target `flutter_sliver_keep_alive_parity` fails, pinned Flutter
`keep_alive_paint_test.dart` passes. This is not a measured Incular
wgpu pixel for this new fixture.

PageView reaches the same position and reports the same page change,
but page0 remains mounted after arrival from an intermediate partial
scroll. Direct jumps dispose and recreate the old page on both.
The standalone matched-input Flutter tests use Widget State;
the Rust `flutter350_page_view_lifecycle` test compares retained
ElementId and distinguishes exact position from lifecycle. The
public paint bounds happen to differ only because the old page
was still mounted. The suspected range-retention mechanism
(`tree/reconciliation.rs:1533–1565`) keeps extra retained children
while its cached range still covers the needed range; this is a
source localization, not a framework fix.

Raw Enter and an unseeded undo variation were also run and are
under input-lifecycle follow-up; not included in these counts.

### PageView lifecycle repair

The formerly failing authored lifecycle comparison now passes from
both a direct jump and a partial scroll. Retained viewport coverage
used to answer only “are enough pages present?”, so a page could stay
mounted after its last visible pixel left. PageView now also requests
reconciliation when a materialized page falls wholly outside its
cache. Smaller movement with the same page set remains compositor-only.

The extended Flutter probe checks normal and reversed 800px pages
with a 40px cache: the same first page survives at offsets 800 and
839; it is disposed at 840 and a fresh page mounts on returning to
839. Rust matches both directions and confirms that 800 → 839 does
not lay out any render objects. All four authored Flutter cases and
13 targeted Rust tests passed. The standalone keep-alive paint
mismatch above remains separate.
