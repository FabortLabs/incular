# StrutStyle diagnostics

The exact pinned Flutter `painting/strut_style_test.dart:9` passed, and
both native `flutter350_strut_diagnostics` tests passed. A named family
and 14-point size appear in order. An absent force-height flag stays
absent, and explicit true and false get different descriptions, including
when each is the only supplied property. Even and proportional leading
remain different values with distinct descriptions when height is set.
An additional native control omits the distribution in the description
when height is absent. This is a diagnostics comparison, not layout.
