# Mounted BoxDecoration borders

Four pinned `widgets/box_decoration_test.dart` registrations pair
(:105, :118, :191, :558). The mounted circle emits a single
translucent outline. A 10 px inside border around a 25 px child
makes the actual container 45×45. Nonuniform sides, a uniform
rectangle, a single rounded corner and a circle all retain the
specified geometry, including the inner hole. Directional radii
resolve with a mounted text direction.

:132 is partial. The mounted test checks the normal outline,
explicit hairline, each independent full-width side and a mixed
hairline/solid border with no omitted side drawn. Earlier a uniform
zero-width solid border emitted a width-zero analytic-border
command; wgpu's analytic shader interprets that width as a fill.
The widget now emits the intentional width-zero path stroke instead.
The remaining renderer check is that such a stroke rasterizes at
one device pixel; the current stroke tessellator rejects that width.
This commit does not claim rendered-pixel parity for the hairline.
