# Widget preview zoom behavior

Four exact pinned `dev/integration_tests/widget_preview_scaffold/test/zoom_test.dart`
registrations passed: :22, :50, :84 and :106. The scaffold widget-test
runner copies the pinned sources unchanged and omits the independent
CLI dev dependency, which the selected widget tests do not use.
Flutter uses its own controls for the reference; Incular uses its
neutral controls without a Material or Cupertino dependency.

A real mounted toolbar has three tooltips and a slider. It starts at
100% with Zoom out and Reset inert. Actual button presses step through
125%, 150%, 125%, and 100%, reset restores the identity transform
(including translation), and a host transform change to 2.5
immediately updates both the slider and the 250% label. Additional
native cases tap the track, clamp at 400%, and keep the continuous
slider value while using a separate meaningful accessibility step.
The slider value and focus node are retained across controller updates.
Six focused runtime cases passed.
