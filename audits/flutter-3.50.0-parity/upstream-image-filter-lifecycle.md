# ImageFiltered Gaussian paint lifecycle

The pinned Flutter originals for layer reuse, enabled/disabled, a
zero-area filter, and avoiding repaint of a child during animation
passed. Native retained tests verified one layer ID across sigma 5 to
10, no flattened blur when disabled and one when enabled, child element
and render IDs that survive both toggles, and a zero-size filter that
lays out and paints without error. A separate custom painter is called
once across sigma 5 to 6 while the scene carries the new value.

The enable-state fix retains the Gaussian widget when detaching its
effect. Mixed matrix/color compositions are not credited with the same
retention, and these cases do not claim parity for the pixel goldens.
The adjacent RawImage builder assertion distinguishes an omitted fit
(serving contextual scaleDown or fill) from an explicit override.
