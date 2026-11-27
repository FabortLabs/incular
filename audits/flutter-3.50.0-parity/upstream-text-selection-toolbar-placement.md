# Platform-neutral text selection toolbar placement

The pinned `widgets/text_selection_toolbar_layout_delegate_test.dart:9`
pairs against a real retained CustomSingleChildLayout. A 200 × 43 px
toolbar appears at the below anchor (y=500) with 0 or 42 px of space
above. At exactly 43 px it flips above and starts at y=0. All three
frames keep the same mounted child and use its measured size.

A separate reference probe and native control check horizontal
placement. A 775 px anchor clamps a 200 px toolbar to x=600 in an
800 px overlay. Forcing it below uses the other anchor (center x=400,
left x=300); forcing it above with an anchor near the top-left clamps
both coordinates to zero. This verifies the core delegate, not any
Material or Cupertino toolbar or their overflow visuals.
