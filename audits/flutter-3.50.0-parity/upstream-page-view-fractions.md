# PageView fractions, controller replacement, and visible touch routing

## Current recheck

The pinned :591 original and all eight native fraction/controller checks pass. This confirms the controller handoff notes below, including the retained logical page rather than the incoming controller’s initial page.

All eight associated pinned Flutter originals and seven of eight focused Rust tests pass. The intentional Rust failure is controller replacement.

| Registration | Outcome |
| --- | --- |
| :591 | The 7/8 fraction works (x=50/750 around the initial item; x=-650/50/750 around page 10). Flutter keeps page 10 when the controller is replaced with a fresh 39/40 controller and repositions at -770/10/790. Incular goes to page 0; page 0 is physically at x≈10 with width 780, confirming the new fraction took effect but the active page did not transfer. |
| :696 | All initial and page-10 neighbor coordinates match at the 1/8 fraction. |
| :739 | The 5/4 fraction places an 800px viewport's 1000px page at x=-100 on both page 0 and after a jump to page 10. |
| :771 | Replacing 5/4 with 4 then explicitly jumping to page 10 puts its 3200px page at x=-1200, as expected. |
| :805 | At 5/4, page 0 and final page 2 both center at (400,300) after settling. |
| :840 | Real normalized taps activate the first three and last three partially visible pages, including the manually inset edge tap. |
| :886 | Final page remains centered at (400,300) through a 300px-to-200px width change when the existing controller is shared. |
| :928 | An out-of-range jump past the final page emits no change callback, both immediately and after a frame. |

Run cargo test --locked -p incular-runtime --test flutter350_upstream_page_fraction -- --nocapture.

## Preserve the live page when changing controllers

The pinned `:591` replacement now matches. At page 10 the incoming 39/40
controller takes over the retained viewport at page 10; the neighboring
pages start at x=-770, x=10 and x=790. Position transfer occurs on the
first successful metrics publication, after acquiring the incoming
controller's lease. It cannot overwrite the position of a controller
whose live attachment rejected the claim.

An authored comparison also switches controllers with snapping disabled.
At viewport width 800, page 1.8 is 1260px at fraction 0.875 and 1080px
at fraction 0.75. Both frameworks preserve that fractional page instead
of applying the incoming controller's configured initial page 4. This
does not cover a controller swap while the viewport is collapsed to zero
or while a ballistic motion is active.
