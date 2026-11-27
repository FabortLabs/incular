# Preview host theme inputs

Seven exact pinned scaffold registrations passed: `ide_theme_test.dart`
:14, :27, :40 and :49, and `theme_test.dart` :16, :34 and :52. The
IDE query keeps its original parameters but only the foreground,
background, and theme keys affect the neutral preview inputs. Missing
and invalid colors are independent of the chosen mode. Only exact
`light` opts out of the dark default.

The surface resolver accepts an override only when its RGB luminance
fits the selected mode; it preserves the supplied alpha. A matching
white or black surface is accepted and orange is rejected in either
mode. The no-override fallback is supplied by the host, not copied from
the Flutter design system. An authored pinned probe verifies encoded
CSS hash prefixes, shorthand and full alpha, and the adjacent 8-bit
gray levels straddling both 0.12 and 0.88. Eight focused and eight
adjacent preview-composition tests passed.
