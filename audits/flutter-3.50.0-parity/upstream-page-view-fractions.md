# PageView fractions, controller replacement, and visible touch routing

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
