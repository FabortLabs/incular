# RawTooltip presentation animation

Reference: pinned Flutter 3.50.0 `ab2912035e160d9cad6306df307576da6128ebdb`,
`packages/flutter/test/widgets/raw_tooltip_test.dart`.

| Registration line | Verified behavior |
| ---: | --- |
| 2762 | A linear show and a separately curved hide both produce their expected midpoint values. |
| 2810 | Without a reverse curve, hiding samples the forward curve. |
| 2851 | Changing the style between shows updates the duration of the retained tooltip. |
| 2916 | Omitted durations are 150 ms to show and 75 ms to hide. |
| 2957 | A custom show duration does not override the 75 ms default hide duration. |

Each exact upstream registration passed. The five corresponding native
runtime tests also passed, as did 19 related placement, trigger and
Escape checks. These are mounted controls sampled on a fake frame
clock, not API-name or descriptor comparisons. The first native check
also verifies the actual compositor opacity command, checks that the
overlay content is not rebuilt by animation ticks, and confirms the
bubble stays mounted through reverse until the animation settles
strictly after its end time.

The renderer remains wgpu; no fade is forced onto a custom builder.
`RawTooltip::with_animated_builder` supplies the animation to the
builder, and explicit animation style also enables timed presentation.
The older Rust builder without animation input or an explicit style
keeps its immediate behavior. This pass is not claiming that every
tooltip descriptor now fades, nor that the anchor descriptor never
rebuilds when the portal is mounted or unmounted.
