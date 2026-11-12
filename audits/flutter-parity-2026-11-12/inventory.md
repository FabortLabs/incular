# Flutter parity inventory — candidate declarations

AST-extracted public declarations from Flutter 3.41.7. This index is not yet a completed inventory: entries from rendering, painting, gestures and services require widget-dependency relevance review, and package assignments and counterpart name matches need manual verification. UNRESOLVED means not yet searched, not a confirmed missing API. Every not-checked entry is pending the audit; partial coverage notes do not assert full behavioral review. Getter and setter declarations of one name in a file are grouped as one item.

Selected rows were manually corrected after the initial generation. `inventory_tool/build-index.ps1` regenerates this file and does not preserve those corrections; reconcile them before using the generator again.

| Flutter file:line and item | Candidate Incular counterpart | Package | Scope / coverage |
| --- | --- | --- | --- |
| `animation/animation_controller.dart:59` AnimationBehavior | crates/incular-animation/src/controller.rs:73 | anim_core | partial reading: anim_core |
| `animation/animation_controller.dart:221` AnimationController | crates/incular-animation/src/controller.rs:367 | anim_core | partial scenarios and reading: anim_core |
| `animation/animation_style.dart:30` AnimationStyle | crates/incular-animation/src/style.rs:10 | anim_compose | partial reading: values and interpolation source-aligned; widget integration unchecked |
| `animation/animation.dart:145` Animation | crates/incular-animation/src/value.rs:63 | anim_core | not checked: pending audit |
| `animation/animation.dart:22` AnimationStatus | crates/incular-animation/src/controller.rs:32 | anim_core | not checked: pending audit |
| `animation/animations.dart:91` AlwaysStoppedAnimation | crates/incular-animation/src/animations.rs:337 | anim_compose | partial reading: anim_compose-002; other behavior unchecked |
| `animation/animations.dart:706` AnimationMax | crates/incular-animation/src/animations.rs:460 | anim_compose | partial reading: finite scalar value and status; generic and edge cases unchecked |
| `animation/animations.dart:693` AnimationMean | crates/incular-animation/src/animations.rs:478 | anim_compose | partial reading: finite scalar value and status; edge cases unchecked |
| `animation/animations.dart:721` AnimationMin | crates/incular-animation/src/animations.rs:469 | anim_compose | partial reading: finite scalar value and status; generic and edge cases unchecked |
| `animation/animations.dart:133` AnimationWithParentMixin | UNRESOLVED | anim_compose | not checked: pending audit |
| `animation/animations.dart:624` CompoundAnimation | crates/incular-animation/src/animations.rs:389 | anim_compose | partial reading: scalar status and formulas; extension and notifications unchecked |
| `animation/animations.dart:379` CurvedAnimation | crates/incular-animation/src/animations.rs:211 | anim_compose | partial scenario and reading: anim_compose-003; other behavior unchecked |
| `animation/animations.dart:54` kAlwaysCompleteAnimation | crates/incular-animation/src/animations.rs:377 | anim_compose | partial reading: value and status aligned; listener API covered by anim_core-004 |
| `animation/animations.dart:86` kAlwaysDismissedAnimation | crates/incular-animation/src/animations.rs:383 | anim_compose | partial reading: value and status aligned; listener API covered by anim_core-004 |
| `animation/animations.dart:174` ProxyAnimation | crates/incular-animation/src/animations.rs:488 | anim_compose | partial reading: default and detach values/status aligned; listener API covered by anim_core-004 |
| `animation/animations.dart:273` ReverseAnimation | crates/incular-animation/src/animations.rs:295 | anim_compose | partial reading: value/status aligned; listener API covered by anim_core-004 |
| `animation/animations.dart:493` TrainHoppingAnimation | crates/incular-animation/src/animations.rs:563 | anim_compose | partial scenario and reading: anim_compose-001 |
| `animation/curves.dart:923` CatmullRomCurve | crates/incular-animation/src/curve2d.rs:324 | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:706` CatmullRomSpline | crates/incular-animation/src/curve2d.rs:172 | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:365` Cubic | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:75` Curve | crates/incular-animation/src/controller.rs:921 | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:539` Curve2D | crates/incular-animation/src/curve2d.rs:28 | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:667` Curve2DSample | crates/incular-animation/src/curve2d.rs:13 | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1460` Curves | crates/incular-animation/src/curves.rs:474 | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1869` Curves.bounceIn | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1879` Curves.bounceInOut | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1874` Curves.bounceOut | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1477` Curves.decelerate | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1513` Curves.ease | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1520` Curves.easeIn | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1616` Curves.easeInBack | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1606` Curves.easeInCirc | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1559` Curves.easeInCubic | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1595` Curves.easeInExpo | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1726` Curves.easeInOut | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1845` Curves.easeInOutBack | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1831` Curves.easeInOutCirc | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1760` Curves.easeInOutCubic | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1772` Curves.easeInOutCubicEmphasized | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1817` Curves.easeInOutExpo | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1746` Curves.easeInOutQuad | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1792` Curves.easeInOutQuart | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1803` Curves.easeInOutQuint | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1735` Curves.easeInOutSine | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1548` Curves.easeInQuad | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1572` Curves.easeInQuart | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1582` Curves.easeInQuint | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1537` Curves.easeInSine | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1527` Curves.easeInToLinear | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1623` Curves.easeOut | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1718` Curves.easeOutBack | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1708` Curves.easeOutCirc | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1664` Curves.easeOutCubic | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1697` Curves.easeOutExpo | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1651` Curves.easeOutQuad | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1677` Curves.easeOutQuart | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1687` Curves.easeOutQuint | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1640` Curves.easeOutSine | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1884` Curves.elasticIn | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1894` Curves.elasticInOut | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1889` Curves.elasticOut | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1500` Curves.fastEaseInToSlowEaseOut | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1486` Curves.fastLinearToSlowEaseIn | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1858` Curves.fastOutSlowIn | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1468` Curves.linear | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1630` Curves.linearToEaseOut | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1864` Curves.slowMiddle | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1330` ElasticInCurve | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1386` ElasticInOutCurve | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1358` ElasticOutCurve | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:1230` FlippedCurve | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:158` Interval | crates/incular-animation/src/curves.rs:286 | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:26` ParametricCurve | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:130` SawTooth | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:213` Split | crates/incular-animation/src/curves.rs:320 | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:444` ThreePointCubic | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/curves.dart:270` Threshold | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/listener_helpers.dart:71` AnimationEagerListenerMixin | UNRESOLVED | anim_core | not checked: pending audit |
| `animation/listener_helpers.dart:18` AnimationLazyListenerMixin | UNRESOLVED | anim_core | not checked: pending audit |
| `animation/listener_helpers.dart:92` AnimationLocalListenersMixin | UNRESOLVED | anim_core | not checked: pending audit |
| `animation/listener_helpers.dart:184` AnimationLocalStatusListenersMixin | UNRESOLVED | anim_core | not checked: pending audit |
| `animation/tween_sequence.dart:106` FlippedTweenSequence | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/tween_sequence.dart:45` TweenSequence | crates/incular-animation/src/tween.rs:143 | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/tween_sequence.dart:121` TweenSequenceItem | UNRESOLVED | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/tween.dart:34` Animatable | crates/incular-animation/src/value.rs:8 | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/tween.dart:401` ColorTween | crates/incular-animation/src/value.rs:319 | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/tween.dart:516` ConstantTween | crates/incular-animation/src/animations.rs:670 | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/tween.dart:554` CurveTween | crates/incular-animation/src/animations.rs:660 | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/tween.dart:473` IntTween | crates/incular-animation/src/value.rs:323 | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/tween.dart:447` RectTween | crates/incular-animation/src/value.rs:325 | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/tween.dart:377` ReverseTween | crates/incular-animation/src/animations.rs:694 | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/tween.dart:426` SizeTween | crates/incular-animation/src/value.rs:321 | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/tween.dart:501` StepTween | crates/incular-animation/src/animations.rs:681 | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `animation/tween.dart:267` Tween | crates/incular-animation/src/tween.rs:8 | anim_curves | partial scenarios and reading: anim_curves; see package coverage and open cases |
| `gestures/arena.dart:43` GestureArenaEntry | crates/incular-gestures/src/arena.rs:72 | gestures | not checked: widget relevance and behavior pending |
| `gestures/arena.dart:117` GestureArenaManager | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/arena.dart:31` GestureArenaMember | crates/incular-gestures/src/arena.rs:70 | gestures | not checked: widget relevance and behavior pending |
| `gestures/arena.dart:15` GestureDisposition | crates/incular-gestures/src/arena.rs:63 | gestures | not checked: widget relevance and behavior pending |
| `gestures/binding.dart:611` FlutterErrorDetailsForPointerEventDispatcher | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/binding.dart:276` GestureBinding | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/binding.dart:41` SamplingClock | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:39` kDoubleTapMinTime | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:49` kDoubleTapSlop | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:35` kDoubleTapTimeout | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:44` kDoubleTapTouchSlop | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:26` kHoverTapSlop | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:20` kHoverTapTimeout | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:100` kJumpTapTimeout | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:29` kLongPressTimeout | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:95` kMaxFlingVelocity | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:90` kMinFlingVelocity | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:72` kPagingTouchSlop | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:76` kPanSlop | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:103` kPrecisePointerHitSlop | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:106` kPrecisePointerPanSlop | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:109` kPrecisePointerScaleSlop | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:14` kPressTimeout | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:80` kScaleSlop | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:65` kTouchSlop | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:85` kWindowTouchSlop | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/constants.dart:53` kZoomControlsTimeout | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/converter.dart:54` PointerEventConverter | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/debug.dart:77` debugAssertAllGesturesVarsUnset | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/debug.dart:40` debugPrintGestureArenaDiagnostics | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/debug.dart:20` debugPrintHitTestResults | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/debug.dart:29` debugPrintMouseHoverEvents | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/debug.dart:54` debugPrintRecognizerCallbacksTrace | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/debug.dart:68` debugPrintResamplingMargin | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/drag_details.dart:25` DragDownDetails | crates/incular-gestures/src/details.rs:245 | gestures | not checked: widget relevance and behavior pending |
| `gestures/drag_details.dart:205` DragEndDetails | crates/incular-gestures/src/details.rs:423 | gestures | not checked: widget relevance and behavior pending |
| `gestures/drag_details.dart:62` DragStartDetails | crates/incular-gestures/src/details.rs:234 | gestures | not checked: widget relevance and behavior pending |
| `gestures/drag_details.dart:116` DragUpdateDetails | crates/incular-gestures/src/details.rs:407 | gestures | not checked: widget relevance and behavior pending |
| `gestures/drag.dart:21` Drag | crates/incular-gestures/src/multidrag.rs:18 | gestures | not checked: widget relevance and behavior pending |
| `gestures/eager.dart:19` EagerGestureRecognizer | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:2552` computeHitSlop | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:2566` computePanSlop | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:2580` computeScaleSlop | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:224` isSingleButton | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:148` kBackMouseButton | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:154` kForwardMouseButton | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:131` kMiddleMouseButton | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:43` kPrimaryButton | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:70` kPrimaryMouseButton | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:101` kPrimaryStylusButton | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:59` kSecondaryButton | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:81` kSecondaryMouseButton | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:142` kSecondaryStylusButton | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:90` kStylusContact | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:119` kTertiaryButton | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:163` kTouchContact | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:173` nthMouseButton | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:182` nthStylusButton | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:895` PointerAddedEvent | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:2515` PointerCancelEvent | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:1500` PointerDownEvent | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:1213` PointerEnterEvent | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:251` PointerEvent | crates/incular-gestures/src/details.rs:19 | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:1361` PointerExitEvent | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:1097` PointerHoverEvent | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:1613` PointerMoveEvent | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:2417` PointerPanZoomEndEvent | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:2181` PointerPanZoomStartEvent | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:2293` PointerPanZoomUpdateEvent | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:988` PointerRemovedEvent | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:2094` PointerScaleEvent | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:1880` PointerScrollEvent | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:2000` PointerScrollInertiaCancelEvent | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:1792` PointerSignalEvent | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:1727` PointerUpEvent | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/events.dart:203` smallestButton | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/force_press.dart:47` ForcePressDetails | crates/incular-gestures/src/force_press.rs:12 | gestures | not checked: widget relevance and behavior pending |
| `gestures/force_press.dart:117` ForcePressGestureRecognizer | crates/incular-gestures/src/force_press.rs:44 | gestures | not checked: widget relevance and behavior pending |
| `gestures/gesture_details.dart:19` PositionedGestureDetails | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/gesture_settings.dart:23` DeviceGestureSettings | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/hit_test.dart:20` HitTestable | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/hit_test.dart:34` HitTestDispatcher | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/hit_test.dart:50` HitTestEntry | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/hit_test.dart:111` HitTestResult | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/hit_test.dart:40` HitTestTarget | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/long_press.dart:113` LongPressDownDetails | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/long_press.dart:221` LongPressEndDetails | crates/incular-gestures/src/details.rs:268 | gestures | not checked: widget relevance and behavior pending |
| `gestures/long_press.dart:262` LongPressGestureRecognizer | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/long_press.dart:176` LongPressMoveUpdateDetails | crates/incular-gestures/src/details.rs:259 | gestures | not checked: widget relevance and behavior pending |
| `gestures/long_press.dart:148` LongPressStartDetails | crates/incular-gestures/src/details.rs:252 | gestures | not checked: widget relevance and behavior pending |
| `gestures/lsq_solver.dart:88` LeastSquaresSolver | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/lsq_solver.dart:56` PolynomialFit | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/monodrag.dart:81` DragGestureRecognizer | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/monodrag.dart:995` HorizontalDragGestureRecognizer | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/monodrag.dart:1058` PanGestureRecognizer | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/monodrag.dart:931` VerticalDragGestureRecognizer | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/multidrag.dart:574` DelayedMultiDragGestureRecognizer | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/multidrag.dart:422` HorizontalMultiDragGestureRecognizer | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/multidrag.dart:371` ImmediateMultiDragGestureRecognizer | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/multidrag.dart:220` MultiDragGestureRecognizer | crates/incular-gestures/src/multidrag.rs:55 | gestures | not checked: widget relevance and behavior pending |
| `gestures/multidrag.dart:38` MultiDragPointerState | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/multidrag.dart:474` VerticalMultiDragGestureRecognizer | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/multitap.dart:122` DoubleTapGestureRecognizer | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/multitap.dart:476` MultiTapGestureRecognizer | crates/incular-gestures/src/multitap.rs:36 | gestures | not checked: widget relevance and behavior pending |
| `gestures/multitap.dart:687` SerialTapCancelDetails | crates/incular-gestures/src/multitap.rs:283 | gestures | not checked: widget relevance and behavior pending |
| `gestures/multitap.dart:621` SerialTapDownDetails | crates/incular-gestures/src/multitap.rs:272 | gestures | not checked: widget relevance and behavior pending |
| `gestures/multitap.dart:836` SerialTapGestureRecognizer | crates/incular-gestures/src/multitap.rs:317 | gestures | not checked: widget relevance and behavior pending |
| `gestures/multitap.dart:719` SerialTapUpDetails | crates/incular-gestures/src/multitap.rs:289 | gestures | not checked: widget relevance and behavior pending |
| `gestures/pointer_router.dart:17` PointerRouter | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/pointer_signal_resolver.dart:62` PointerSignalResolver | crates/incular-widgets/src/raw_input.rs:968 | gestures | not checked: widget relevance and behavior pending |
| `gestures/recognizer.dart:48` DragStartBehavior | crates/incular-core/src/input.rs:213 | gestures | not checked: widget relevance and behavior pending |
| `gestures/recognizer.dart:129` GestureRecognizer | crates/incular-gestures/src/raw.rs:14 | gestures | not checked: widget relevance and behavior pending |
| `gestures/recognizer.dart:568` GestureRecognizerState | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/recognizer.dart:64` MultitouchDragStrategy | crates/incular-widgets/src/scroll_behavior.rs:24 | gestures | not checked: widget relevance and behavior pending |
| `gestures/recognizer.dart:808` OffsetPair | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/recognizer.dart:387` OneSequenceGestureRecognizer | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/recognizer.dart:594` PrimaryPointerGestureRecognizer | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/resampler.dart:36` PointerEventResampler | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/scale.dart:24` kDefaultMouseScrollToScaleFactor | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/scale.dart:31` kDefaultTrackpadScrollToScaleFactor | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/scale.dart:272` ScaleEndDetails | crates/incular-gestures/src/scale.rs:79 | gestures | not checked: widget relevance and behavior pending |
| `gestures/scale.dart:345` ScaleGestureRecognizer | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/scale.dart:97` ScaleStartDetails | crates/incular-gestures/src/scale.rs:42 | gestures | not checked: widget relevance and behavior pending |
| `gestures/scale.dart:157` ScaleUpdateDetails | crates/incular-gestures/src/scale.rs:56 | gestures | not checked: widget relevance and behavior pending |
| `gestures/tap_and_drag.dart:750` BaseTapAndDragGestureRecognizer | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/tap_and_drag.dart:1464` TapAndDragGestureRecognizer | crates/incular-gestures/src/tap_and_drag.rs:105 | gestures | not checked: widget relevance and behavior pending |
| `gestures/tap_and_drag.dart:1402` TapAndHorizontalDragGestureRecognizer | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/tap_and_drag.dart:1439` TapAndPanGestureRecognizer | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/tap_and_drag.dart:83` TapDragDownDetails | crates/incular-gestures/src/tap_and_drag.rs:20 | gestures | not checked: widget relevance and behavior pending |
| `gestures/tap_and_drag.dart:365` TapDragEndDetails | crates/incular-gestures/src/tap_and_drag.rs:61 | gestures | not checked: widget relevance and behavior pending |
| `gestures/tap_and_drag.dart:189` TapDragStartDetails | crates/incular-gestures/src/tap_and_drag.rs:38 | gestures | not checked: widget relevance and behavior pending |
| `gestures/tap_and_drag.dart:250` TapDragUpdateDetails | crates/incular-gestures/src/tap_and_drag.rs:48 | gestures | not checked: widget relevance and behavior pending |
| `gestures/tap_and_drag.dart:136` TapDragUpDetails | crates/incular-gestures/src/tap_and_drag.rs:29 | gestures | not checked: widget relevance and behavior pending |
| `gestures/tap.dart:202` BaseTapGestureRecognizer | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/tap.dart:32` TapDownDetails | crates/incular-gestures/src/details.rs:215 | gestures | not checked: widget relevance and behavior pending |
| `gestures/tap.dart:461` TapGestureRecognizer | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/tap.dart:108` TapMoveDetails | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/tap.dart:77` TapUpDetails | crates/incular-gestures/src/details.rs:223 | gestures | not checked: widget relevance and behavior pending |
| `gestures/team.dart:139` GestureArenaTeam | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/velocity_tracker.dart:295` IOSScrollViewFlingVelocityTracker | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/velocity_tracker.dart:416` MacOSScrollViewFlingVelocityTracker | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/velocity_tracker.dart:15` Velocity | crates/incular-animation/src/controller.rs:896 | gestures | not checked: widget relevance and behavior pending |
| `gestures/velocity_tracker.dart:87` VelocityEstimate | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `gestures/velocity_tracker.dart:138` VelocityTracker | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `painting/_network_image_io.dart:22` NetworkImage | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/_network_image_web.dart:39` debugRestoreHttpRequestFactory | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/_network_image_web.dart:55` debugRestoreImgElementFactory | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/_network_image_web.dart:35` httpRequestFactory | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/_network_image_web.dart:51` imgElementFactory | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/_network_image_web.dart:63` NetworkImage | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/_web_image_info_io.dart:16` WebImageInfo | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/_web_image_info_web.dart:17` WebImageInfo | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/alignment.dart:315` Alignment | crates/incular-config/src/alignment.rs:97 | paint | not checked: widget relevance and behavior pending |
| `painting/alignment.dart:509` AlignmentDirectional | crates/incular-config/src/alignment.rs:239 | paint | not checked: widget relevance and behavior pending |
| `painting/alignment.dart:25` AlignmentGeometry | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/alignment.dart:742` TextAlignVertical | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `painting/basic_types.dart:128` Axis | crates/incular-config/src/alignment.rs:5 | paint | not checked: widget relevance and behavior pending |
| `painting/basic_types.dart:208` AxisDirection | crates/incular-config/src/alignment.rs:57 | paint | not checked: widget relevance and behavior pending |
| `painting/basic_types.dart:324` axisDirectionIsReversed | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/basic_types.dart:283` axisDirectionToAxis | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/basic_types.dart:149` flipAxis | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/basic_types.dart:310` flipAxisDirection | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/basic_types.dart:86` RenderComparison | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/basic_types.dart:294` textDirectionToAxisDirection | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `painting/basic_types.dart:167` VerticalDirection | crates/incular-config/src/alignment.rs:88 | paint | not checked: widget relevance and behavior pending |
| `painting/beveled_rectangle_border.dart:23` BeveledRectangleBorder | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/binding.dart:215` imageCache | crates/incular-image/src/lib.rs:510 | paint | not checked: widget relevance and behavior pending |
| `painting/binding.dart:23` PaintingBinding | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/border_radius.dart:350` BorderRadius | crates/incular-widgets/src/painting_effects/geometry.rs:48 | paint | not checked: widget relevance and behavior pending |
| `painting/border_radius.dart:598` BorderRadiusDirectional | crates/incular-widgets/src/painting_effects/geometry.rs:157 | paint | not checked: widget relevance and behavior pending |
| `painting/border_radius.dart:25` BorderRadiusGeometry | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/borders.dart:65` BorderSide | crates/incular-widgets/src/painting_effects/decoration.rs:22 | paint | not checked: widget relevance and behavior pending |
| `painting/borders.dart:19` BorderStyle | crates/incular-widgets/src/painting_effects/decoration.rs:14 | paint | not checked: widget relevance and behavior pending |
| `painting/borders.dart:657` OutlinedBorder | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/borders.dart:877` paintBorder | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/borders.dart:367` ShapeBorder | crates/incular-widgets/src/painting_effects/shape_borders.rs:15 | paint | not checked: widget relevance and behavior pending |
| `painting/box_border.dart:431` Border | crates/incular-controls/src/styles.rs:763 | paint | not checked: widget relevance and behavior pending |
| `painting/box_border.dart:799` BorderDirectional | crates/incular-widgets/src/painting_effects/decoration.rs:246 | paint | not checked: widget relevance and behavior pending |
| `painting/box_border.dart:67` BoxBorder | crates/incular-widgets/src/painting_effects/decoration.rs:133 | paint | not checked: widget relevance and behavior pending |
| `painting/box_border.dart:26` BoxShape | crates/incular-widgets/src/painting_effects/decoration.rs:305 | paint | not checked: widget relevance and behavior pending |
| `painting/box_decoration.dart:81` BoxDecoration | crates/incular-widgets/src/painting_effects/decoration.rs:451 | paint | not checked: widget relevance and behavior pending |
| `painting/box_fit.dart:146` applyBoxFit | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/box_fit.dart:22` BoxFit | crates/incular-widgets/src/tree/descriptors.rs:21 | paint | not checked: widget relevance and behavior pending |
| `painting/box_fit.dart:83` FittedSizes | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/box_shadow.dart:32` BoxShadow | crates/incular-widgets/src/painting_effects/decoration.rs:326 | paint | not checked: widget relevance and behavior pending |
| `painting/circle_border.dart:37` CircleBorder | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/clip.dart:11` ClipContext | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `painting/colors.dart:491` ColorProperty | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/colors.dart:410` ColorSwatch | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/colors.dart:236` HSLColor | crates/incular-core/src/geometry.rs:211 | paint | not checked: widget relevance and behavior pending |
| `painting/colors.dart:68` HSVColor | crates/incular-core/src/geometry.rs:254 | paint | not checked: widget relevance and behavior pending |
| `painting/continuous_rectangle_border.dart:38` ContinuousRectangleBorder | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/debug.dart:196` debugAssertAllPaintingVarsUnset | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/debug.dart:233` debugCaptureShaderWarmUpImage | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/debug.dart:227` debugCaptureShaderWarmUpPicture | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/debug.dart:246` debugCheckCanResolveTextDirection | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `painting/debug.dart:40` debugDisableShadows | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/debug.dart:183` debugImageOverheadAllowance | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/debug.dart:175` debugInvertOversizedImages | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/debug.dart:56` debugNetworkImageHttpClientProvider | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/debug.dart:150` debugOnPaintImage | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/debug.dart:68` ImageSizeInfo | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/decoration_image.dart:454` debugFlushLastFrameImageSizeInfo | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/decoration_image.dart:48` DecorationImage | crates/incular-widgets/src/painting_effects/decoration.rs:411 | paint | partial scenario and reading: paint-003; other behavior unchecked |
| `painting/decoration_image.dart:281` DecorationImagePainter | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/decoration_image.dart:29` ImageRepeat | crates/incular-widgets/src/tree/descriptors.rs:25 | paint | not checked: widget relevance and behavior pending |
| `painting/decoration_image.dart:526` paintImage | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/decoration.dart:209` BoxPainter | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/decoration.dart:33` Decoration | crates/incular-material/src/app_shell/shell.rs:189 | paint | not checked: widget relevance and behavior pending |
| `painting/edge_insets.dart:390` EdgeInsets | crates/incular-config/src/insets.rs:11 | paint | not checked: widget relevance and behavior pending |
| `painting/edge_insets.dart:742` EdgeInsetsDirectional | crates/incular-config/src/insets.rs:243 | paint | not checked: widget relevance and behavior pending |
| `painting/edge_insets.dart:31` EdgeInsetsGeometry | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/flutter_logo.dart:39` FlutterLogoDecoration | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/flutter_logo.dart:25` FlutterLogoStyle | crates/incular-widgets/src/flutter_logo.rs:22 | paint | not checked: widget relevance and behavior pending |
| `painting/fractional_offset.dart:54` FractionalOffset | crates/incular-config/src/alignment.rs:194 | focus_input | not checked: widget relevance and behavior pending |
| `painting/geometry.dart:41` positionDependentBox | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/gradient.dart:157` Gradient | crates/incular-widgets/src/painting_effects/decoration.rs:515 | paint | not checked: widget relevance and behavior pending |
| `painting/gradient.dart:105` GradientRotation | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/gradient.dart:76` GradientTransform | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/gradient.dart:377` LinearGradient | crates/incular-rendering/src/gradients.rs:189 | paint | not checked: widget relevance and behavior pending |
| `painting/gradient.dart:644` RadialGradient | crates/incular-rendering/src/gradients.rs:197 | paint | not checked: widget relevance and behavior pending |
| `painting/gradient.dart:957` SweepGradient | crates/incular-rendering/src/gradients.rs:211 | paint | not checked: widget relevance and behavior pending |
| `painting/image_cache.dart:86` ImageCache | crates/incular-image/src/lib.rs:510 | paint | not checked: widget relevance and behavior pending |
| `painting/image_cache.dart:537` ImageCacheStatus | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/image_decoder.dart:25` decodeImageFromList | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/image_provider.dart:687` AssetBundleImageKey | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/image_provider.dart:727` AssetBundleImageProvider | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/image_provider.dart:1810` ExactAssetImage | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/image_provider.dart:1589` FileImage | crates/incular-image/src/lib.rs:174 | paint | not checked: widget relevance and behavior pending |
| `painting/image_provider.dart:44` ImageConfiguration | crates/incular-image/src/lib.rs:40 | paint | not checked: widget relevance and behavior pending |
| `painting/image_provider.dart:359` ImageProvider | crates/incular-image/src/lib.rs:117 | paint | not checked: widget relevance and behavior pending |
| `painting/image_provider.dart:1671` MemoryImage | crates/incular-image/src/lib.rs:152 | paint | not checked: widget relevance and behavior pending |
| `painting/image_provider.dart:1535` NetworkImage | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/image_provider.dart:1880` NetworkImageLoadException | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/image_provider.dart:1254` ResizeImage | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/image_provider.dart:796` ResizeImageKey | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/image_provider.dart:834` ResizeImagePolicy | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/image_provider.dart:1495` WebHtmlElementStrategy | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/image_resolution.dart:236` AssetImage | crates/incular-image/src/lib.rs:130 | paint | not checked: widget relevance and behavior pending |
| `painting/image_stream.dart:273` ImageChunkEvent | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/image_stream.dart:41` ImageInfo | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/image_stream.dart:324` ImageStream | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/image_stream.dart:480` ImageStreamCompleter | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/image_stream.dart:450` ImageStreamCompleterHandle | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/image_stream.dart:173` ImageStreamListener | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `painting/image_stream.dart:949` MultiFrameImageStreamCompleter | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/image_stream.dart:883` OneFrameImageStreamCompleter | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/inline_span.dart:28` Accumulator | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/inline_span.dart:129` combineSemanticsInfo | UNRESOLVED | semantics | not checked: widget relevance and behavior pending |
| `painting/inline_span.dart:219` InlineSpan | crates/incular-text/src/spans.rs:65 | text | not checked: widget relevance and behavior pending |
| `painting/inline_span.dart:58` InlineSpanSemanticsInformation | UNRESOLVED | semantics | not checked: widget relevance and behavior pending |
| `painting/linear_border.dart:140` LinearBorder | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/linear_border.dart:33` LinearBorderEdge | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/matrix_utils.dart:614` debugDescribeTransform | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/matrix_utils.dart:14` MatrixUtils | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/matrix_utils.dart:627` TransformProperty | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/notched_shapes.dart:150` AutomaticNotchedShape | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/notched_shapes.dart:42` CircularNotchedRectangle | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/notched_shapes.dart:23` NotchedShape | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/oval_border.dart:29` OvalBorder | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/paint_utilities.dart:23` paintZigZag | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/placeholder_span.dart:39` PlaceholderSpan | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/rounded_rectangle_border.dart:38` RoundedRectangleBorder | crates/incular-widgets/src/painting_effects/shape_borders.rs:134 | paint | not checked: widget relevance and behavior pending |
| `painting/rounded_rectangle_border.dart:226` RoundedSuperellipseBorder | crates/incular-widgets/src/painting_effects/shape_borders.rs:189 | paint | not checked: widget relevance and behavior pending |
| `painting/shader_warm_up.dart:56` ShaderWarmUp | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `painting/shape_decoration.dart:65` ShapeDecoration | crates/incular-widgets/src/painting_effects/shape_decoration.rs:17 | paint | not checked: widget relevance and behavior pending |
| `painting/stadium_border.dart:29` StadiumBorder | crates/incular-widgets/src/painting_effects/shape_borders.rs:77 | paint | not checked: widget relevance and behavior pending |
| `painting/star_border.dart:45` StarBorder | crates/incular-widgets/src/painting_effects/shape_borders.rs:261 | paint | not checked: widget relevance and behavior pending |
| `painting/strut_style.dart:291` StrutStyle | crates/incular-text/src/style/text.rs:129 | paint | not checked: widget relevance and behavior pending |
| `painting/text_painter.dart:41` kDefaultFontSize | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `painting/text_painter.dart:74` PlaceholderDimensions | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `painting/text_painter.dart:47` TextOverflow | crates/incular-text/src/style/text.rs:191 | text | not checked: widget relevance and behavior pending |
| `painting/text_painter.dart:589` TextPainter | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `painting/text_painter.dart:152` TextWidthBasis | crates/incular-text/src/style/text.rs:47 | text | not checked: widget relevance and behavior pending |
| `painting/text_painter.dart:177` WordBoundary | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `painting/text_scaler.dart:20` TextScaler | crates/incular-text/src/style/text.rs:201 | text | not checked: widget relevance and behavior pending |
| `painting/text_span.dart:74` TextSpan | crates/incular-text/src/spans.rs:85 | text | not checked: widget relevance and behavior pending |
| `painting/text_style.dart:1725` lerpFontVariations | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `painting/text_style.dart:466` TextStyle | crates/incular-text/src/style/text_style.rs:12 | text | not checked: widget relevance and behavior pending |
| `physics/clamped_simulation.dart:28` ClampedSimulation | crates/incular-animation/src/physics.rs:355 | physics | partial scenarios and reading: physics; see package coverage and open cases |
| `physics/friction_simulation.dart:172` BoundedFrictionSimulation | crates/incular-animation/src/physics.rs:274 | physics | partial scenarios and reading: physics; see package coverage and open cases |
| `physics/friction_simulation.dart:35` FrictionSimulation | crates/incular-animation/src/physics.rs:236 | physics | partial scenarios and reading: physics; see package coverage and open cases |
| `physics/gravity_simulation.dart:56` GravitySimulation | crates/incular-animation/src/physics.rs:313 | physics | partial scenarios and reading: physics; see package coverage and open cases |
| `physics/simulation.dart:36` Simulation | crates/incular-animation/src/physics.rs:108 | physics | partial scenarios and reading: physics; see package coverage and open cases |
| `physics/spring_simulation.dart:271` ScrollSpringSimulation | crates/incular-animation/src/physics.rs:197 | physics | partial scenarios and reading: physics; see package coverage and open cases |
| `physics/spring_simulation.dart:23` SpringDescription | crates/incular-animation/src/physics.rs:46 | physics | partial scenarios and reading: physics; see package coverage and open cases |
| `physics/spring_simulation.dart:204` SpringSimulation | crates/incular-animation/src/physics.rs:119 | physics | partial scenarios and reading: physics; see package coverage and open cases |
| `physics/spring_simulation.dart:164` SpringType | crates/incular-animation/src/physics.rs:38 | physics | partial scenarios and reading: physics; see package coverage and open cases |
| `physics/tolerance.dart:9` Tolerance | crates/incular-animation/src/physics.rs:7 | physics | partial scenarios and reading: physics; see package coverage and open cases |
| `physics/utils.dart:10` nearEqual | UNRESOLVED | physics | partial scenarios and reading: physics; see package coverage and open cases |
| `physics/utils.dart:21` nearZero | UNRESOLVED | physics | partial scenarios and reading: physics; see package coverage and open cases |
| `rendering/animated_size.dart:62` RenderAnimatedSize | UNRESOLVED | switchers | not checked: widget relevance and behavior pending |
| `rendering/animated_size.dart:15` RenderAnimatedSizeState | UNRESOLVED | switchers | not checked: widget relevance and behavior pending |
| `rendering/binding.dart:712` debugDumpLayerTree | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/binding.dart:760` debugDumpPipelineOwnerTree | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/binding.dart:695` debugDumpRenderTree | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/binding.dart:752` debugDumpSemanticsTree | UNRESOLVED | semantics | not checked: widget relevance and behavior pending |
| `rendering/binding.dart:44` RendererBinding | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/binding.dart:792` RenderingFlutterBinding | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/box.dart:980` BaselineOffset | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/box.dart:100` BoxConstraints | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/box.dart:942` BoxHitTestEntry | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `rendering/box.dart:716` BoxHitTestResult | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `rendering/box.dart:963` BoxParentData | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/box.dart:976` ContainerBoxParentData | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/box.dart:1572` RenderBox | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/box.dart:3287` RenderBoxContainerDefaultsMixin | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/custom_layout.dart:122` MultiChildLayoutDelegate | crates/incular-widgets/src/custom_layout.rs:49 | layout | not checked: widget relevance and behavior pending |
| `rendering/custom_layout.dart:18` MultiChildLayoutParentData | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/custom_layout.dart:313` RenderCustomMultiChildLayoutBox | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/custom_paint.dart:149` CustomPainter | crates/incular-widgets/src/painting_effects/custom_paint.rs:4 | paint | not checked: widget relevance and behavior pending |
| `rendering/custom_paint.dart:307` CustomPainterSemantics | UNRESOLVED | semantics | not checked: widget relevance and behavior pending |
| `rendering/custom_paint.dart:382` RenderCustomPaint | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/debug_overflow_indicator.dart:89` DebugOverflowIndicatorMixin | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:328` debugAssertAllRenderVarsUnset | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:361` debugCheckHasBoundedAxis | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:117` debugCheckIntrinsicSizes | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:79` debugCurrentRepaintColor | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:245` debugDisableClipLayers | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:270` debugDisableOpacityLayers | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:256` debugDisablePhysicalShapeLayers | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:193` debugEnhanceLayoutTimelineArguments | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:214` debugEnhancePaintTimelineArguments | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:233` debugOnProfilePaint | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:39` debugPaintBaselinesEnabled | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:51` debugPaintLayerBordersEnabled | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:294` debugPaintPadding | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:59` debugPaintPointersEnabled | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:36` debugPaintSizeEnabled | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:48` debugPaintTextLayoutBoxes | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:110` debugPrintLayouts | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:87` debugPrintMarkNeedsLayoutStacks | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:90` debugPrintMarkNeedsPaintStacks | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:143` debugProfileLayoutsEnabled | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:172` debugProfilePaintsEnabled | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:67` debugRepaintRainbowEnabled | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/debug.dart:70` debugRepaintTextRainbowEnabled | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/decorated_sliver.dart:14` RenderDecoratedSliver | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/editable.dart:267` RenderEditable | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/editable.dart:2825` RenderEditablePainter | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/editable.dart:49` TextSelectionPoint | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/editable.dart:134` VerticalCaretMovementRun | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/error.dart:32` RenderErrorBox | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/flex.dart:276` CrossAxisAlignment | crates/incular-config/src/alignment.rs:324 | layout | not checked: widget relevance and behavior pending |
| `rendering/flex.dart:112` FlexFit | crates/incular-config/src/alignment.rs:334 | layout | not checked: widget relevance and behavior pending |
| `rendering/flex.dart:126` FlexParentData | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/flex.dart:195` MainAxisAlignment | crates/incular-config/src/alignment.rs:303 | layout | not checked: widget relevance and behavior pending |
| `rendering/flex.dart:162` MainAxisSize | crates/incular-config/src/alignment.rs:296 | layout | not checked: widget relevance and behavior pending |
| `rendering/flex.dart:412` RenderFlex | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/flow.dart:64` FlowDelegate | crates/incular-widgets/src/custom_layout.rs:68 | layout | not checked: widget relevance and behavior pending |
| `rendering/flow.dart:29` FlowPaintingContext | crates/incular-widgets/src/custom_layout.rs:276 | text | not checked: widget relevance and behavior pending |
| `rendering/flow.dart:151` FlowParentData | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/flow.dart:179` RenderFlow | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/image_filter_config.dart:59` ImageFilterConfig | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/image_filter_config.dart:18` ImageFilterContext | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/image.dart:22` RenderImage | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:2927` AnnotatedRegionLayer | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:31` AnnotationEntry | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:57` AnnotationResult | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:2325` BackdropFilterLayer | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:2315` BackdropKey | crates/incular-rendering/src/display_list.rs:20 | paint | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:1871` ClipPathLayer | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:1601` ClipRectLayer | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:1694` ClipRRectLayer | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:1784` ClipRSuperellipseLayer | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:1953` ColorFilterLayer | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:1077` ContainerLayer | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:2603` FollowerLayer | crates/incular-rendering/src/compositor.rs:414 | misc | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:1993` ImageFilterLayer | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:144` Layer | crates/incular-rendering/src/compositor.rs:529 | misc | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:783` LayerHandle | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:2416` LayerLink | crates/incular-rendering/src/compositor.rs:261 | misc | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:2486` LeaderLayer | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:1459` OffsetLayer | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:2138` OpacityLayer | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:1032` PerformanceOverlayLayer | UNRESOLVED | navigation | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:824` PictureLayer | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:1005` PlatformViewLayer | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:2218` ShaderMaskLayer | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:952` TextureLayer | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/layer.dart:2038` TransformLayer | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/layout_helper.dart:30` ChildLayoutHelper | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/list_body.dart:13` ListBodyParentData | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/list_body.dart:27` RenderListBody | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/list_wheel_viewport.dart:27` ListWheelChildManager | UNRESOLVED | scroll | not checked: widget relevance and behavior pending |
| `rendering/list_wheel_viewport.dart:58` ListWheelParentData | UNRESOLVED | scroll | not checked: widget relevance and behavior pending |
| `rendering/list_wheel_viewport.dart:147` RenderListWheelViewport | UNRESOLVED | scroll | not checked: widget relevance and behavior pending |
| `rendering/mouse_tracker.dart:160` MouseTracker | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/object.dart:902` Constraints | crates/incular-config/src/constraints.rs:11 | misc | not checked: widget relevance and behavior pending |
| `rendering/object.dart:4193` ContainerParentDataMixin | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/object.dart:4235` ContainerRenderObjectMixin | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/object.dart:6309` debugDumpRenderObjectSemanticsTree | UNRESOLVED | semantics | not checked: widget relevance and behavior pending |
| `rendering/object.dart:6486` DiagnosticsDebugCreator | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/object.dart:94` PaintingContext | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/object.dart:57` ParentData | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/object.dart:1706` PipelineManifold | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/object.dart:1019` PipelineOwner | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/object.dart:4542` RelayoutWhenSystemFontsChangeMixin | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/object.dart:1864` RenderObject | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/object.dart:4027` RenderObjectWithChildMixin | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/object.dart:4140` RenderObjectWithLayoutCallbackMixin | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/object.dart:4605` SemanticsAnnotationsMixin | UNRESOLVED | semantics | not checked: widget relevance and behavior pending |
| `rendering/paragraph.dart:55` PlaceholderSpanIndexSemanticsTag | UNRESOLVED | semantics | not checked: widget relevance and behavior pending |
| `rendering/paragraph.dart:135` RenderInlineChildrenContainerDefaults | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/paragraph.dart:310` RenderParagraph | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/paragraph.dart:77` TextParentData | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/performance_overlay.dart:16` PerformanceOverlayOption | UNRESOLVED | navigation | not checked: widget relevance and behavior pending |
| `rendering/performance_overlay.dart:65` RenderPerformanceOverlay | UNRESOLVED | navigation | not checked: widget relevance and behavior pending |
| `rendering/platform_view.dart:21` PlatformViewHitTestBehavior | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `rendering/platform_view.dart:672` PlatformViewRenderBox | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/platform_view.dart:76` RenderAndroidView | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/platform_view.dart:468` RenderAppKitView | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/platform_view.dart:284` RenderDarwinPlatformView | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/platform_view.dart:419` RenderUiKitView | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:1389` CustomClipper | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:2377` DecorationPosition | crates/incular-widgets/src/painting_effects/box_painter.rs:22 | paint | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:148` HitTestBehavior | crates/incular-widgets/src/gestures.rs:22 | focus_input | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:3995` RenderAbsorbPointer | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:1107` RenderAnimatedOpacity | UNRESOLVED | implicit | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:981` RenderAnimatedOpacityMixin | UNRESOLVED | implicit | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:4761` RenderAnnotatedRegion | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:450` RenderAspectRatio | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:1201` RenderBackdropFilter | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:4340` RenderBlockSemantics | UNRESOLVED | semantics | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:1897` RenderClipOval | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:1994` RenderClipPath | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:1615` RenderClipRect | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:1684` RenderClipRRect | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:1792` RenderClipRSuperellipse | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:214` RenderConstrainedBox | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:2386` RenderDecoratedBox | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:4399` RenderExcludeSemantics | UNRESOLVED | semantics | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:2798` RenderFittedBox | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:4550` RenderFollowerLayer | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:3079` RenderFractionalTranslation | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:848` RenderIgnoreBaseline | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:3745` RenderIgnorePointer | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:4441` RenderIndexedSemantics | UNRESOLVED | semantics | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:783` RenderIntrinsicHeight | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:624` RenderIntrinsicWidth | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:4475` RenderLeaderLayer | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:344` RenderLimitedBox | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:4379` RenderMergeSemantics | UNRESOLVED | semantics | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:4089` RenderMetaData | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:3318` RenderMouseRegion | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:3834` RenderOffstage | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:872` RenderOpacity | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:2132` RenderPhysicalModel | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:2280` RenderPhysicalShape | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:3210` RenderPointerListener | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:45` RenderProxyBox | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:66` RenderProxyBoxMixin | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:165` RenderProxyBoxWithHitTestBehavior | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:3486` RenderRepaintBoundary | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:4309` RenderSemanticsAnnotations | UNRESOLVED | semantics | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:4107` RenderSemanticsGestureHandler | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:1128` RenderShaderMask | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:2530` RenderTransform | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/proxy_box.dart:1450` ShapeBorderClipper | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `rendering/proxy_sliver.dart:34` RenderProxySliver | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/proxy_sliver.dart:430` RenderSliverAnimatedOpacity | UNRESOLVED | implicit | not checked: widget relevance and behavior pending |
| `rendering/proxy_sliver.dart:449` RenderSliverConstrainedCrossAxis | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/proxy_sliver.dart:227` RenderSliverIgnorePointer | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/proxy_sliver.dart:325` RenderSliverOffstage | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/proxy_sliver.dart:112` RenderSliverOpacity | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/proxy_sliver.dart:486` RenderSliverSemanticsAnnotations | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/rotated_box.dart:24` RenderRotatedBox | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:468` ClearSelectionEvent | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:632` DirectionallyExtendSelectionEvent | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:556` GranularlyExtendSelectionEvent | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:238` Selectable | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:461` SelectAllSelectionEvent | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:200` SelectedContent | crates/incular-widgets/src/selection.rs:59 | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:125` SelectedContentRange | crates/incular-widgets/src/selection.rs:32 | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:516` SelectionEdgeUpdateEvent | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:450` SelectionEvent | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:377` SelectionEventType | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:578` SelectionExtendDirection | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:733` SelectionGeometry | crates/incular-widgets/src/selection.rs:95 | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:76` SelectionHandler | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:844` SelectionPoint | crates/incular-widgets/src/selection.rs:82 | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:260` SelectionRegistrant | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:685` SelectionRegistrar | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:24` SelectionResult | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:704` SelectionStatus | crates/incular-widgets/src/selection.rs:19 | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:313` SelectionUtils | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:488` SelectParagraphSelectionEvent | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:476` SelectWordSelectionEvent | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:421` TextGranularity | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/selection.dart:913` TextSelectionHandleType | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/service_extensions.dart:23` RenderingServiceExtensions | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/shifted_box.dart:593` OverflowBoxFit | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/shifted_box.dart:294` RenderAligningShiftedBox | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/shifted_box.dart:1544` RenderBaseline | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/shifted_box.dart:635` RenderConstrainedOverflowBox | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/shifted_box.dart:835` RenderConstraintsTransformBox | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/shifted_box.dart:1401` RenderCustomSingleChildLayoutBox | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/shifted_box.dart:1142` RenderFractionallySizedOverflowBox | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `rendering/shifted_box.dart:126` RenderPadding | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/shifted_box.dart:397` RenderPositionedBox | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/shifted_box.dart:32` RenderShiftedBox | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/shifted_box.dart:1043` RenderSizedOverflowBox | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/shifted_box.dart:1340` SingleChildLayoutDelegate | crates/incular-widgets/src/custom_layout.rs:23 | layout | not checked: widget relevance and behavior pending |
| `rendering/sliver_fill.dart:144` RenderSliverFillRemaining | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_fill.dart:215` RenderSliverFillRemainingAndOverscroll | UNRESOLVED | scroll | not checked: widget relevance and behavior pending |
| `rendering/sliver_fill.dart:79` RenderSliverFillRemainingWithScrollable | UNRESOLVED | scroll | not checked: widget relevance and behavior pending |
| `rendering/sliver_fill.dart:32` RenderSliverFillViewport | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_fixed_extent_list.dart:42` RenderSliverFixedExtentBoxAdaptor | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_fixed_extent_list.dart:468` RenderSliverFixedExtentList | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_fixed_extent_list.dart:488` RenderSliverVariedExtentList | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_grid.dart:561` RenderSliverGrid | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_grid.dart:294` SliverGridDelegate | crates/incular-widgets/src/scrolling/scroll_views.rs:176 | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_grid.dart:346` SliverGridDelegateWithFixedCrossAxisCount | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_grid.dart:445` SliverGridDelegateWithMaxCrossAxisExtent | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_grid.dart:41` SliverGridGeometry | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_grid.dart:130` SliverGridLayout | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_grid.dart:536` SliverGridParentData | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_grid.dart:171` SliverGridRegularTileLayout | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_group.dart:32` RenderSliverCrossAxisGroup | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_group.dart:211` RenderSliverMainAxisGroup | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_list.dart:40` RenderSliverList | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_multi_box_adaptor.dart:140` KeepAliveParentDataMixin | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_multi_box_adaptor.dart:25` RenderSliverBoxChildManager | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_multi_box_adaptor.dart:201` RenderSliverMultiBoxAdaptor | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_multi_box_adaptor.dart:153` RenderSliverWithKeepAliveMixin | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_multi_box_adaptor.dart:163` SliverMultiBoxAdaptorParentData | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_padding.dart:27` RenderSliverEdgeInsetsPadding | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_padding.dart:305` RenderSliverPadding | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_persistent_header.dart:485` FloatingHeaderSnapConfiguration | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_persistent_header.dart:37` OverScrollHeaderStretchConfiguration | UNRESOLVED | scroll | not checked: widget relevance and behavior pending |
| `rendering/sliver_persistent_header.dart:55` PersistentHeaderShowOnScreenConfiguration | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_persistent_header.dart:508` RenderSliverFloatingPersistentHeader | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_persistent_header.dart:797` RenderSliverFloatingPinnedPersistentHeader | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_persistent_header.dart:120` RenderSliverPersistentHeader | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_persistent_header.dart:404` RenderSliverPinnedPersistentHeader | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_persistent_header.dart:352` RenderSliverScrollingPersistentHeader | UNRESOLVED | scroll | not checked: widget relevance and behavior pending |
| `rendering/sliver_tree.dart:109` RenderTreeSliver | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_tree.dart:65` TreeSliverIndentationType | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver_tree.dart:33` TreeSliverNodeParentData | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver.dart:160` applyGrowthDirectionToAxisDirection | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver.dart:180` applyGrowthDirectionToScrollDirection | UNRESOLVED | scroll | not checked: widget relevance and behavior pending |
| `rendering/sliver.dart:129` GrowthDirection | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver.dart:1310` RenderSliver | crates/incular-widgets/src/scrolling/geometry.rs:97 | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver.dart:1845` RenderSliverHelpers | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver.dart:1942` RenderSliverSingleBoxAdapter | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver.dart:2034` RenderSliverToBoxAdapter | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver.dart:197` SliverConstraints | crates/incular-scroll/src/sliver.rs:13 | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver.dart:641` SliverGeometry | crates/incular-scroll/src/sliver.rs:68 | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver.dart:1036` SliverHitTestEntry | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver.dart:965` SliverHitTestResult | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver.dart:43` SliverLayoutDimensions | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver.dart:1096` SliverLogicalContainerParentData | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver.dart:1076` SliverLogicalParentData | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver.dart:1148` SliverPhysicalContainerParentData | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/sliver.dart:1108` SliverPhysicalParentData | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/stack.dart:28` RelativeRect | crates/incular-widgets/src/animation_transitions.rs:27 | layout | not checked: widget relevance and behavior pending |
| `rendering/stack.dart:766` RenderIndexedStack | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/stack.dart:369` RenderStack | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/stack.dart:301` StackFit | crates/incular-config/src/alignment.rs:369 | layout | not checked: widget relevance and behavior pending |
| `rendering/stack.dart:204` StackParentData | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/table_border.dart:18` TableBorder | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/table.dart:136` FixedColumnWidth | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/table.dart:197` FlexColumnWidth | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/table.dart:161` FractionColumnWidth | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `rendering/table.dart:94` IntrinsicColumnWidth | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/table.dart:235` MaxColumnWidth | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/table.dart:287` MinColumnWidth | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/table.dart:361` RenderTable | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/table.dart:16` TableCellParentData | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/table.dart:333` TableCellVerticalAlignment | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/table.dart:43` TableColumnWidth | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/texture.dart:38` TextureBox | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `rendering/tweens.dart:63` AlignmentGeometryTween | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/tweens.dart:41` AlignmentTween | crates/incular-widgets/src/tweens.rs:9 | layout | not checked: widget relevance and behavior pending |
| `rendering/tweens.dart:18` FractionalOffsetTween | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `rendering/view.dart:134` RenderView | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/view.dart:24` ViewConfiguration | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `rendering/viewport_offset.dart:78` flipScrollDirection | UNRESOLVED | scroll | not checked: widget relevance and behavior pending |
| `rendering/viewport_offset.dart:46` ScrollDirection | UNRESOLVED | scroll | not checked: widget relevance and behavior pending |
| `rendering/viewport_offset.dart:100` ViewportOffset | UNRESOLVED | scroll | not checked: widget relevance and behavior pending |
| `rendering/viewport.dart:27` CacheExtentStyle | crates/incular-widgets/src/advanced_scrolling/two_dimensional.rs:107 | slivers | not checked: widget relevance and behavior pending |
| `rendering/viewport.dart:77` RenderAbstractViewport | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/viewport.dart:1856` RenderShrinkWrappingViewport | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/viewport.dart:1408` RenderViewport | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/viewport.dart:301` RenderViewportBase | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/viewport.dart:188` RevealedOffset | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/viewport.dart:51` SliverPaintOrder | UNRESOLVED | slivers | not checked: widget relevance and behavior pending |
| `rendering/wrap.dart:218` RenderWrap | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `rendering/wrap.dart:62` WrapAlignment | crates/incular-config/src/alignment.rs:341 | layout | not checked: widget relevance and behavior pending |
| `rendering/wrap.dart:130` WrapCrossAlignment | crates/incular-config/src/alignment.rs:352 | layout | not checked: widget relevance and behavior pending |
| `rendering/wrap.dart:202` WrapParentData | UNRESOLVED | layout | not checked: widget relevance and behavior pending |
| `scheduler/binding.dart:1465` defaultSchedulingStrategy | UNRESOLVED | anim_core | not checked: pending audit |
| `scheduler/binding.dart:213` PerformanceModeRequestHandle | UNRESOLVED | anim_core | not checked: pending audit |
| `scheduler/binding.dart:250` SchedulerBinding | UNRESOLVED | anim_core | not checked: pending audit |
| `scheduler/binding.dart:160` SchedulerPhase | UNRESOLVED | anim_core | not checked: pending audit |
| `scheduler/binding.dart:37,45` timeDilation | UNRESOLVED | anim_core | partial diagnostic and reading: anim_core-003 |
| `scheduler/debug.dart:79` debugAssertAllSchedulerVarsUnset | UNRESOLVED | anim_core | not checked: pending audit |
| `scheduler/debug.dart:39` debugPrintBeginFrameBanner | UNRESOLVED | anim_core | not checked: pending audit |
| `scheduler/debug.dart:45` debugPrintEndFrameBanner | UNRESOLVED | anim_core | not checked: pending audit |
| `scheduler/debug.dart:56` debugPrintScheduleFrameStacks | UNRESOLVED | anim_core | not checked: pending audit |
| `scheduler/debug.dart:70` debugTracePostFrameCallbacks | UNRESOLVED | anim_core | not checked: pending audit |
| `scheduler/priority.dart:12` Priority | UNRESOLVED | anim_core | not checked: pending audit |
| `scheduler/service_extensions.dart:16` SchedulerServiceExtensions | UNRESOLVED | anim_core | not checked: pending audit |
| `scheduler/ticker.dart:78` Ticker | crates/incular-animation/src/scheduler.rs:210 | anim_core | partial scenarios and reading: anim_core |
| `scheduler/ticker.dart:528` TickerCanceled | crates/incular-animation/src/scheduler.rs:368 | anim_core | not checked: pending audit |
| `scheduler/ticker.dart:425` TickerFuture | crates/incular-animation/src/scheduler.rs:352 | anim_core | partial scenarios and reading: anim_core |
| `scheduler/ticker.dart:43` TickerProvider | UNRESOLVED | anim_core | not checked: pending audit |
| `services/_background_isolate_binary_messenger_io.dart:14` BackgroundIsolateBinaryMessenger | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/_background_isolate_binary_messenger_web.dart:9` BackgroundIsolateBinaryMessenger | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/asset_bundle.dart:57` AssetBundle | crates/incular-assets/src/bundle.rs:40 | misc | not checked: widget relevance and behavior pending |
| `services/asset_bundle.dart:185` CachingAssetBundle | crates/incular-assets/src/bundle.rs:177 | misc | not checked: widget relevance and behavior pending |
| `services/asset_bundle.dart:146` NetworkAssetBundle | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/asset_bundle.dart:324` PlatformAssetBundle | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/asset_bundle.dart:418` rootBundle | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/asset_manifest.dart:24` AssetManifest | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/asset_manifest.dart:129` AssetMetadata | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/autofill.dart:785` AutofillClient | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/autofill.dart:629` AutofillConfiguration | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/autofill.dart:17` AutofillHints | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/autofill.dart:819` AutofillScope | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/autofill.dart:874` AutofillScopeMixin | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/binary_messenger.dart:22` BinaryMessenger | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/binding.dart:45` ServicesBinding | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/binding.dart:696` SystemContextMenuClient | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/browser_context_menu.dart:18` BrowserContextMenu | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/clipboard.dart:26` Clipboard | crates/incular-platform/src/data_transfer.rs:620 | paint | not checked: widget relevance and behavior pending |
| `services/clipboard.dart:14` ClipboardData | UNRESOLVED | paint | not checked: widget relevance and behavior pending |
| `services/debug.dart:42` debugAssertAllServicesVarsUnset | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/debug.dart:26` debugKeyEventSimulatorTransitModeOverride | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/debug.dart:34` debugPrintKeyboardEvents | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/debug.dart:67` debugProfilePlatformChannels | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/deferred_component.dart:27` DeferredComponent | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/flavor.dart:9` appFlavor | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/flutter_version.dart:15` FlutterVersion | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/font_loader.dart:16` FontLoader | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/haptic_feedback.dart:15` HapticFeedback | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/hardware_keyboard.dart:411` HardwareKeyboard | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/hardware_keyboard.dart:68` KeyboardLockMode | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/hardware_keyboard.dart:729` KeyDataTransitMode | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/hardware_keyboard.dart:259` KeyDownEvent | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/hardware_keyboard.dart:123` KeyEvent | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/hardware_keyboard.dart:937` KeyEventManager | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/hardware_keyboard.dart:821` KeyMessage | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/hardware_keyboard.dart:304` KeyRepeatEvent | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/hardware_keyboard.dart:280` KeyUpEvent | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_inserted_content.dart:15` KeyboardInsertedContent | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_key.g.dart:29` KeyboardKey | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_key.g.dart:70` LogicalKeyboardKey | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_key.g.dart:3533` PhysicalKeyboardKey | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:519` kAndroidNumPadMap | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:17` kAndroidToLogicalKey | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:281` kAndroidToPhysicalKey | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:542` kFuchsiaToLogicalKey | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:780` kFuchsiaToPhysicalKey | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:1728` kGlfwNumpadMap | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:1608` kGlfwToLogicalKey | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:1924` kGtkNumpadMap | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:1747` kGtkToLogicalKey | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:1503` kIosNumPadMap | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:1476` kIosSpecialLogicalMap | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:1531` kIosToLogicalKey | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:1309` kIosToPhysicalKey | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:1953` kLinuxToPhysicalKey | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:1203` kMacOsFunctionKeyMap | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:1181` kMacOsNumPadMap | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:1232` kMacOsToLogicalKey | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:1055` kMacOsToPhysicalKey | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:2741` kWebLocationMap | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:2716` kWebNumPadMap | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:2174` kWebToLogicalKey | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:2479` kWebToPhysicalKey | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:3186` kWindowsNumPadMap | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:2862` kWindowsToLogicalKey | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/keyboard_maps.g.dart:3023` kWindowsToPhysicalKey | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/live_text.dart:18` LiveText | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/message_codec.dart:20` MessageCodec | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/message_codec.dart:35` MethodCall | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/message_codec.dart:66` MethodCodec | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/message_codec.dart:205` MissingPluginException | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/message_codec.dart:108` PlatformException | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/message_codecs.dart:28` BinaryCodec | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/message_codecs.dart:89` JSONMessageCodec | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/message_codecs.dart:118` JSONMethodCodec | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/message_codecs.dart:260` StandardMessageCodec | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/message_codecs.dart:577` StandardMethodCodec | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/message_codecs.dart:44` StringCodec | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/mouse_cursor.dart:188` MouseCursor | crates/incular-gestures/src/details.rs:176 | misc | not checked: widget relevance and behavior pending |
| `services/mouse_cursor.dart:21` MouseCursorManager | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/mouse_cursor.dart:106` MouseCursorSession | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/mouse_cursor.dart:357` SystemMouseCursor | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/mouse_cursor.dart:405` SystemMouseCursors | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/mouse_tracking.dart:49` MouseTrackerAnnotation | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/platform_channel.dart:202` BasicMessageChannel | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/platform_channel.dart:651` EventChannel | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/platform_channel.dart:48` kProfilePlatformChannels | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/platform_channel.dart:292` MethodChannel | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/platform_channel.dart:627` OptionalMethodChannel | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/platform_channel.dart:36` shouldProfilePlatformChannels | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/platform_views.dart:441` AndroidMotionEvent | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/platform_views.dart:361` AndroidPointerCoords | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/platform_views.dart:324` AndroidPointerProperties | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/platform_views.dart:725` AndroidViewController | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/platform_views.dart:1587` AppKitViewController | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/platform_views.dart:1514` DarwinPlatformViewController | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/platform_views.dart:1112` ExpensiveAndroidViewController | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/platform_views.dart:1167` HybridAndroidViewController | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/platform_views.dart:1594` PlatformViewController | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/platform_views.dart:32,38` platformViewsRegistry | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/platform_views.dart:72` PlatformViewsService | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/platform_views.dart:1041` SurfaceAndroidViewController | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/platform_views.dart:1237` TextureAndroidViewController | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/platform_views.dart:1582` UiKitViewController | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/predictive_back_event.dart:26` PredictiveBackEvent | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/predictive_back_event.dart:13` SwipeEdge | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/process_text.dart:90` DefaultProcessTextService | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/process_text.dart:11` ProcessTextAction | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/process_text.dart:35` ProcessTextService | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/raw_keyboard_android.dart:37` RawKeyEventDataAndroid | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/raw_keyboard_fuchsia.dart:32` RawKeyEventDataFuchsia | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/raw_keyboard_ios.dart:32` RawKeyEventDataIos | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/raw_keyboard_linux.dart:257` GLFWKeyHelper | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/raw_keyboard_linux.dart:387` GtkKeyHelper | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/raw_keyboard_linux.dart:196` KeyHelper | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/raw_keyboard_linux.dart:32` RawKeyEventDataLinux | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/raw_keyboard_macos.dart:43` RawKeyEventDataMacOs | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/raw_keyboard_macos.dart:18` runeToLowerCase | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/raw_keyboard_web.dart:37` RawKeyEventDataWeb | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/raw_keyboard_windows.dart:37` RawKeyEventDataWindows | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/raw_keyboard.dart:44` KeyboardSide | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/raw_keyboard.dart:72` ModifierKey | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/raw_keyboard.dart:764` RawKeyboard | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/raw_keyboard.dart:678` RawKeyDownEvent | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/raw_keyboard.dart:369` RawKeyEvent | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/raw_keyboard.dart:147` RawKeyEventData | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/raw_keyboard.dart:699` RawKeyUpEvent | UNRESOLVED | focus_input | not checked: widget relevance and behavior pending |
| `services/restoration.dart:1002` debugIsSerializableForRestoration | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/restoration.dart:504` RestorationBucket | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/restoration.dart:158` RestorationManager | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/scribe.dart:29` Scribe | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/sensitive_content.dart:37` ContentSensitivity | crates/incular-config/src/environment.rs:90 | misc | not checked: widget relevance and behavior pending |
| `services/sensitive_content.dart:86` SensitiveContentService | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/service_extensions.dart:17` ServicesServiceExtensions | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/spell_check.dart:124` DefaultSpellCheckService | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/spell_check.dart:65` SpellCheckResults | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/spell_check.dart:101` SpellCheckService | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/spell_check.dart:27` SuggestionSpan | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/system_channels.dart:33` SystemChannels | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/system_chrome.dart:70` ApplicationSwitcherDescription | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/system_chrome.dart:44` DeviceOrientation | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/system_chrome.dart:442` SystemChrome | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/system_chrome.dart:107` SystemUiMode | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/system_chrome.dart:87` SystemUiOverlay | UNRESOLVED | navigation | not checked: widget relevance and behavior pending |
| `services/system_chrome.dart:235` SystemUiOverlayStyle | UNRESOLVED | navigation | not checked: widget relevance and behavior pending |
| `services/system_navigator.dart:13` SystemNavigator | UNRESOLVED | navigation | not checked: widget relevance and behavior pending |
| `services/system_sound.dart:37` SystemSound | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/system_sound.dart:10` SystemSoundType | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/text_boundary.dart:73` CharacterBoundary | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_boundary.dart:219` DocumentBoundary | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_boundary.dart:131` LineBoundary | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_boundary.dart:146` ParagraphBoundary | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_boundary.dart:26` TextBoundary | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_editing_delta.dart:58` TextEditingDelta | crates/incular-text/src/editing.rs:211 | text | not checked: widget relevance and behavior pending |
| `services/text_editing_delta.dart:359` TextEditingDeltaDeletion | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_editing_delta.dart:299` TextEditingDeltaInsertion | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_editing_delta.dart:476` TextEditingDeltaNonTextUpdate | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_editing_delta.dart:412` TextEditingDeltaReplacement | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_editing.dart:13` TextSelection | crates/incular-semantics/src/lib.rs:88 | text | not checked: widget relevance and behavior pending |
| `services/text_formatter.dart:251` FilteringTextInputFormatter | crates/incular-widgets/src/forms.rs:702 | text | not checked: widget relevance and behavior pending |
| `services/text_formatter.dart:445` LengthLimitingTextInputFormatter | crates/incular-widgets/src/forms.rs:778 | text | not checked: widget relevance and behavior pending |
| `services/text_formatter.dart:55` MaxLengthEnforcement | crates/incular-widgets/src/forms.rs:763 | text | not checked: widget relevance and behavior pending |
| `services/text_formatter.dart:92` TextInputFormatter | crates/incular-widgets/src/forms.rs:691 | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:1487` DeltaTextInputClient | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:912` FloatingCursorDragState | UNRESOLVED | gestures | not checked: widget relevance and behavior pending |
| `services/text_input.dart:2880` IOSSystemContextMenuItemData | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:2923` IOSSystemContextMenuItemDataCopy | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:3118` IOSSystemContextMenuItemDataCustom | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:2941` IOSSystemContextMenuItemDataCut | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:3098` IOSSystemContextMenuItemDataLiveText | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:3000` IOSSystemContextMenuItemDataLookUp | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:2959` IOSSystemContextMenuItemDataPaste | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:3033` IOSSystemContextMenuItemDataSearchWeb | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:2978` IOSSystemContextMenuItemDataSelectAll | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:3065` IOSSystemContextMenuItemDataShare | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:931` RawFloatingCursorPoint | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:1418` ScribbleClient | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:1168` SelectionChangedCause | crates/incular-text/src/editing.rs:241 | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:1439` SelectionRect | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:60` SmartDashesType | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:82` SmartQuotesType | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:2603` SystemContextMenuController | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:470` TextCapitalization | crates/incular-platform/src/lib.rs:538 | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:954` TextEditingValue | crates/incular-text/src/editing.rs:152 | text | partial scenario and reading: text-002; other behavior unchecked |
| `services/text_input.dart:1796` TextInput | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:314` TextInputAction | crates/incular-material/src/inputs/vocabulary.rs:19 | focus_input | not checked: widget relevance and behavior pending |
| `services/text_input.dart:1315` TextInputClient | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:510` TextInputConfiguration | crates/incular-platform/src/lib.rs:587 | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:1529` TextInputConnection | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:2364` TextInputControl | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:104` TextInputType | crates/incular-material/src/inputs/vocabulary.rs:3 | text | not checked: widget relevance and behavior pending |
| `services/text_input.dart:1221` TextSelectionDelegate | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/text_layout_metrics.dart:15` TextLayoutMetrics | UNRESOLVED | text | not checked: widget relevance and behavior pending |
| `services/undo_manager.dart:10` UndoDirection | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/undo_manager.dart:37` UndoManager | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `services/undo_manager.dart:112` UndoManagerClient | UNRESOLVED | misc | not checked: widget relevance and behavior pending |
| `widgets/_platform_selectable_region_context_menu_io.dart:28` PlatformSelectableRegionContextMenu | UNRESOLVED | text | not checked: pending audit |
| `widgets/_platform_selectable_region_context_menu_web.dart:40` PlatformSelectableRegionContextMenu | UNRESOLVED | text | not checked: pending audit |
| `widgets/_web_browser_detection_io.dart:9` WebBrowserDetection | UNRESOLVED | layout | not checked: pending audit |
| `widgets/_web_browser_detection_web.dart:8` WebBrowserDetection | UNRESOLVED | layout | not checked: pending audit |
| `widgets/_web_image_io.dart:10` RawWebImage | UNRESOLVED | paint | not checked: pending audit |
| `widgets/_web_image_web.dart:22` ImgElementPlatformView | UNRESOLVED | paint | not checked: pending audit |
| `widgets/_web_image_web.dart:69` RawWebImage | UNRESOLVED | paint | not checked: pending audit |
| `widgets/_web_image_web.dart:134` RenderWebImage | UNRESOLVED | paint | not checked: pending audit |
| `widgets/_window_io.dart:33` createDefaultOwner | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window_linux.dart:801` DialogWindowControllerLinux | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window_linux.dart:628` RegularWindowControllerLinux | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window_linux.dart:526` WindowingOwnerLinux | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window_macos.dart:328` DialogWindowControllerMacOS | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window_macos.dart:158` RegularWindowControllerMacOS | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window_macos.dart:53` WindowingOwnerMacOS | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window_positioner.dart:283` WindowPositioner | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window_positioner.dart:21` WindowPositionerAnchor | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window_positioner.dart:207` WindowPositionerConstraintAdjustment | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window_web.dart:27` createDefaultOwner | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window_win32.dart:505` DialogWindowControllerWin32 | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window_win32.dart:274` RegularWindowControllerWin32 | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window_win32.dart:93` WindowingOwnerWin32 | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window.dart:66` BaseWindowController | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window.dart:974` createDefaultWindowingOwner | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window.dart:1178` DialogWindow | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window.dart:489` DialogWindowController | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window.dart:390` DialogWindowControllerDelegate | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window.dart:1288` PopupWindow | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window.dart:811` PopupWindowController | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window.dart:775` PopupWindowControllerDelegate | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window.dart:1077` RegularWindow | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window.dart:191` RegularWindowController | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window.dart:111` RegularWindowControllerDelegate | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window.dart:1222` TooltipWindow | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window.dart:695` TooltipWindowController | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window.dart:633` TooltipWindowControllerDelegate | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window.dart:905` WindowingOwner | UNRESOLVED | misc | not checked: pending audit |
| `widgets/_window.dart:1343` WindowScope | UNRESOLVED | misc | not checked: pending audit |
| `widgets/actions.dart:131` Action | crates/incular-android/src/lib.rs:46 | focus_input | not checked: pending audit |
| `widgets/actions.dart:613` ActionDispatcher | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/actions.dart:455` ActionListener | crates/incular-widgets/src/focus_keyboard.rs:300 | focus_input | not checked: pending audit |
| `widgets/actions.dart:706` Actions | crates/incular-gestures/src/keyboard.rs:534 | focus_input | not checked: pending audit |
| `widgets/actions.dart:1531` ActivateAction | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/actions.dart:1502` ActivateIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/actions.dart:1519` ButtonActivateIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/actions.dart:583` CallbackAction | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/actions.dart:509` ContextAction | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/actions.dart:1561` DismissAction | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/actions.dart:1553` DismissIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/actions.dart:1475` DoNothingAction | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/actions.dart:1447` DoNothingAndStopPropagationIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/actions.dart:1422` DoNothingIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/actions.dart:1140` FocusableActionDetector | crates/incular-widgets/src/focus_keyboard.rs:480 | focus_input | not checked: pending audit |
| `widgets/actions.dart:60` Intent | crates/incular-gestures/src/keyboard.rs:115 | focus_input | not checked: pending audit |
| `widgets/actions.dart:1585` PrioritizedAction | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/actions.dart:1567` PrioritizedIntents | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/actions.dart:1543` SelectAction | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/actions.dart:1534` SelectIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/actions.dart:1401` VoidCallbackAction | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/actions.dart:1384` VoidCallbackIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/adapter.dart:23` RenderObjectToWidgetAdapter | UNRESOLVED | misc | not checked: pending audit |
| `widgets/adapter.dart:85` RenderObjectToWidgetElement | UNRESOLVED | misc | not checked: pending audit |
| `widgets/animated_cross_fade.dart:119` AnimatedCrossFade | crates/incular-widgets/src/animated_cross_fade.rs:44 | switchers | partial scenarios and reading: switchers |
| `widgets/animated_cross_fade.dart:26` CrossFadeState | crates/incular-widgets/src/animated_cross_fade.rs:21 | misc | not checked: pending audit |
| `widgets/animated_scroll_view.dart:374` AnimatedGrid | crates/incular-widgets/src/advanced_slivers/animated.rs:1061 | switchers | partial scenarios and reading: switchers |
| `widgets/animated_scroll_view.dart:514` AnimatedGridState | UNRESOLVED | switchers | not checked: pending audit |
| `widgets/animated_scroll_view.dart:75` AnimatedList | crates/incular-widgets/src/advanced_slivers/animated.rs:924 | switchers | partial scenarios and reading: switchers |
| `widgets/animated_scroll_view.dart:303` AnimatedListState | UNRESOLVED | switchers | not checked: pending audit |
| `widgets/animated_scroll_view.dart:1106` SliverAnimatedGrid | crates/incular-widgets/src/advanced_slivers/animated.rs:1211 | switchers | partial scenarios and reading: switchers |
| `widgets/animated_scroll_view.dart:1221` SliverAnimatedGridState | UNRESOLVED | switchers | not checked: pending audit |
| `widgets/animated_scroll_view.dart:960` SliverAnimatedList | crates/incular-widgets/src/scrolling/sliver_lists.rs:959 | switchers | partial scenarios and reading: switchers |
| `widgets/animated_scroll_view.dart:1075` SliverAnimatedListState | UNRESOLVED | switchers | not checked: pending audit |
| `widgets/animated_size.dart:27` AnimatedSize | crates/incular-widgets/src/animated_size.rs:21 | switchers | partial scenarios and reading: switchers |
| `widgets/animated_switcher.dart:110` AnimatedSwitcher | crates/incular-widgets/src/animated_switcher.rs:25 | switchers | partial scenarios and reading: switchers |
| `widgets/annotated_region.dart:15` AnnotatedRegion | crates/incular-widgets/src/compositing.rs:291 | misc | not checked: pending audit |
| `widgets/app_lifecycle_listener.dart:64` AppLifecycleListener | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/app.dart:144` basicLocaleListResolution | UNRESOLVED | misc | not checked: pending audit |
| `widgets/app.dart:286` WidgetsApp | crates/incular-widgets/src/app_shell/app.rs:689 | misc | not checked: pending audit |
| `widgets/async.dart:205` AsyncSnapshot | crates/incular-widgets/src/async_builders.rs:40 | misc | not checked: pending audit |
| `widgets/async.dart:176` ConnectionState | crates/incular-widgets/src/async_builders.rs:25 | misc | not checked: pending audit |
| `widgets/async.dart:536` FutureBuilder | crates/incular-widgets/src/async_builders.rs:226 | misc | not checked: pending audit |
| `widgets/async.dart:406` StreamBuilder | crates/incular-widgets/src/async_builders.rs:613 | misc | not checked: pending audit |
| `widgets/async.dart:44` StreamBuilderBase | crates/incular-widgets/src/async_builders.rs:474 | misc | not checked: pending audit |
| `widgets/autocomplete.dart:731` AutocompleteFirstOptionIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/autocomplete.dart:772` AutocompleteHighlightedOption | crates/incular-widgets/src/forms.rs:578 | misc | not checked: pending audit |
| `widgets/autocomplete.dart:737` AutocompleteLastOptionIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/autocomplete.dart:725` AutocompleteNextOptionIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/autocomplete.dart:744` AutocompleteNextPageOptionIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/autocomplete.dart:719` AutocompletePreviousOptionIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/autocomplete.dart:751` AutocompletePreviousPageOptionIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/autocomplete.dart:99` OptionsViewOpenDirection | UNRESOLVED | misc | not checked: pending audit |
| `widgets/autocomplete.dart:170` RawAutocomplete | crates/incular-widgets/src/forms.rs:527 | misc | partial reading: misc-005; other behavior unchecked |
| `widgets/autofill.dart:16` AutofillContextAction | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/autofill.dart:70` AutofillGroup | crates/incular-widgets/src/forms.rs:601 | text | not checked: pending audit |
| `widgets/autofill.dart:167` AutofillGroupState | UNRESOLVED | text | not checked: pending audit |
| `widgets/automatic_keep_alive.dart:71` AutomaticKeepAlive | crates/incular-widgets/src/advanced_slivers/keep_alive.rs:293 | misc | not checked: pending audit |
| `widgets/automatic_keep_alive.dart:422` AutomaticKeepAliveClientMixin | UNRESOLVED | misc | not checked: pending audit |
| `widgets/automatic_keep_alive.dart:366` KeepAliveHandle | crates/incular-widgets/src/advanced_slivers/keep_alive.rs:10 | misc | not checked: pending audit |
| `widgets/automatic_keep_alive.dart:340` KeepAliveNotification | crates/incular-widgets/src/advanced_slivers/keep_alive.rs:97 | misc | not checked: pending audit |
| `widgets/banner.dart:218` Banner | crates/incular-widgets/src/utilities.rs:216 | misc | not checked: pending audit |
| `widgets/banner.dart:36` BannerLocation | crates/incular-widgets/src/utilities.rs:117 | misc | not checked: pending audit |
| `widgets/banner.dart:61` BannerPainter | UNRESOLVED | paint | not checked: pending audit |
| `widgets/banner.dart:337` CheckedModeBanner | crates/incular-widgets/src/utilities.rs:342 | misc | not checked: pending audit |
| `widgets/basic.dart:7831` AbsorbPointer | crates/incular-widgets/src/gestures.rs:379 | focus_input | not checked: pending audit |
| `widgets/basic.dart:2462` Align | crates/incular-controls/src/overlay.rs:21 | layout | not checked: pending audit |
| `widgets/basic.dart:3707` AspectRatio | crates/incular-layout/src/descriptors.rs:418 | layout | not checked: pending audit |
| `widgets/basic.dart:634` BackdropFilter | crates/incular-widgets/src/compositing.rs:121 | paint | partial reading: paint-001 |
| `widgets/basic.dart:469` BackdropGroup | crates/incular-widgets/src/compositing.rs:235 | paint | not checked: pending audit |
| `widgets/basic.dart:3873` Baseline | crates/incular-layout/src/descriptors.rs:437 | layout | not checked: pending audit |
| `widgets/basic.dart:8141` BlockSemantics | crates/incular-widgets/src/semantics.rs:496 | semantics | partial scenario: semantics__block_previous; other behavior unchecked |
| `widgets/basic.dart:8382` Builder | crates/incular-material/src/app_shell/material_app.rs:196 | misc | not checked: pending audit |
| `widgets/basic.dart:2550` Center | crates/incular-config/src/alignment.rs:107 | layout | not checked: pending audit |
| `widgets/basic.dart:1215` ClipOval | crates/incular-widgets/src/layout/basic/clipping.rs:85 | paint | not checked: pending audit |
| `widgets/basic.dart:1284` ClipPath | crates/incular-widgets/src/layout/basic/clipping.rs:117 | paint | not checked: pending audit |
| `widgets/basic.dart:950` ClipRect | crates/incular-widgets/src/layout/basic/clipping.rs:18 | paint | not checked: pending audit |
| `widgets/basic.dart:1033` ClipRRect | crates/incular-widgets/src/layout/basic/clipping.rs:50 | paint | not checked: pending audit |
| `widgets/basic.dart:1123` ClipRSuperellipse | crates/incular-widgets/src/painting_effects/layers.rs:36 | paint | partial scenario and reading: paint-006; other behavior unchecked |
| `widgets/basic.dart:8474` ColoredBox | crates/incular-widgets/src/layout/basic/painting.rs:14 | misc | not checked: pending audit |
| `widgets/basic.dart:5998` Column | crates/incular-layout/src/descriptors.rs:51 | layout | not checked: pending audit |
| `widgets/basic.dart:1991` CompositedTransformFollower | crates/incular-widgets/src/compositing.rs:364 | layout | not checked: pending audit |
| `widgets/basic.dart:1939` CompositedTransformTarget | crates/incular-widgets/src/compositing.rs:338 | layout | not checked: pending audit |
| `widgets/basic.dart:2836` ConstrainedBox | crates/incular-layout/src/descriptors.rs:372 | layout | not checked: pending audit |
| `widgets/basic.dart:2917` ConstraintsTransformBox | crates/incular-widgets/src/layout/basic/sizing_constraints.rs:531 | layout | not checked: pending audit |
| `widgets/basic.dart:2661` CustomMultiChildLayout | crates/incular-widgets/src/custom_layout.rs:426 | layout | not checked: pending audit |
| `widgets/basic.dart:831` CustomPaint | crates/incular-widgets/src/layout/basic/painting.rs:74 | paint | not checked: pending audit |
| `widgets/basic.dart:2572` CustomSingleChildLayout | crates/incular-widgets/src/custom_layout.rs:396 | layout | not checked: pending audit |
| `widgets/basic.dart:7125` DefaultAssetBundle | crates/incular-widgets/src/asset_bundle.rs:15 | misc | not checked: pending audit |
| `widgets/basic.dart:169` Directionality | crates/incular-widgets/src/environment.rs:201 | misc | not checked: pending audit |
| `widgets/basic.dart:8179` ExcludeSemantics | crates/incular-widgets/src/semantics.rs:457 | semantics | partial scenario: semantics__exclude; other behavior unchecked |
| `widgets/basic.dart:6132` Expanded | crates/incular-controls/src/styles.rs:247 | misc | not checked: pending audit |
| `widgets/basic.dart:2091` FittedBox | crates/incular-widgets/src/layout/basic/baseline_aspect_fractional.rs:116 | layout | not checked: pending audit |
| `widgets/basic.dart:5405` Flex | crates/incular-layout/src/descriptors.rs:15 | layout | not checked: pending audit |
| `widgets/basic.dart:6041` Flexible | crates/incular-layout/src/algorithms.rs:37 | layout | not checked: pending audit |
| `widgets/basic.dart:6467` Flow | crates/incular-widgets/src/custom_layout.rs:489 | layout | not checked: pending audit |
| `widgets/basic.dart:3213` FractionallySizedBox | crates/incular-layout/src/descriptors.rs:453 | focus_input | not checked: pending audit |
| `widgets/basic.dart:2169` FractionalTranslation | crates/incular-widgets/src/layout/basic/transforms.rs:39 | focus_input | not checked: pending audit |
| `widgets/basic.dart:4510` getAxisDirectionFromAxisReverseAndDirectionality | UNRESOLVED | misc | not checked: pending audit |
| `widgets/basic.dart:3903` IgnoreBaseline | crates/incular-widgets/src/custom_layout.rs:639 | layout | not checked: pending audit |
| `widgets/basic.dart:7718` IgnorePointer | crates/incular-widgets/src/gestures.rs:352 | focus_input | not checked: pending audit |
| `widgets/basic.dart:8234` IndexedSemantics | crates/incular-widgets/src/indexed_semantics.rs:11 | semantics | partial scenario: semantics__indexed; other behavior unchecked |
| `widgets/basic.dart:4832` IndexedStack | crates/incular-widgets/src/layout/stack.rs:173 | layout | not checked: pending audit |
| `widgets/basic.dart:3846` IntrinsicHeight | crates/incular-widgets/src/layout/basic/sizing_constraints.rs:474 | layout | not checked: pending audit |
| `widgets/basic.dart:3771` IntrinsicWidth | crates/incular-widgets/src/layout/basic/sizing_constraints.rs:432 | layout | not checked: pending audit |
| `widgets/basic.dart:8260` KeyedSubtree | crates/incular-widgets/src/layout/basic/utility.rs:10 | misc | partial reading: misc-009; other behavior unchecked |
| `widgets/basic.dart:2595` LayoutId | crates/incular-widgets/src/custom_layout.rs:459 | layout | not checked: pending audit |
| `widgets/basic.dart:3324` LimitedBox | crates/incular-widgets/src/layout/basic/sizing_constraints.rs:219 | misc | not checked: pending audit |
| `widgets/basic.dart:4545` ListBody | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:750 | misc | not checked: pending audit |
| `widgets/basic.dart:7241` Listener | crates/incular-widgets/src/raw_input.rs:407 | focus_input | not checked: pending audit |
| `widgets/basic.dart:8121` MergeSemantics | crates/incular-widgets/src/semantics.rs:432 | semantics | partial scenario and reading: semantics-001; other behavior unchecked |
| `widgets/basic.dart:7895` MetaData | crates/incular-widgets/src/raw_input.rs:511 | misc | not checked: pending audit |
| `widgets/basic.dart:7383` MouseRegion | crates/incular-gestures/src/mouse.rs:8 | focus_input | not checked: pending audit |
| `widgets/basic.dart:3582` Offstage | crates/incular-layout/src/descriptors.rs:537 | layout | not checked: pending audit |
| `widgets/basic.dart:334` Opacity | crates/incular-material/src/theme/components.rs:641 | paint | not checked: pending audit |
| `widgets/basic.dart:3385` OverflowBox | crates/incular-widgets/src/layout/basic/sizing_constraints.rs:264 | layout | not checked: pending audit |
| `widgets/basic.dart:2300` Padding | crates/incular-controls/src/styles.rs:775 | layout | not checked: pending audit |
| `widgets/basic.dart:1371` PhysicalModel | crates/incular-widgets/src/painting_effects/layers.rs:75 | paint | partial reading and diagnostic: paint-005; other behavior unchecked |
| `widgets/basic.dart:1471` PhysicalShape | crates/incular-widgets/src/painting_effects/layers.rs:151 | paint | partial scenario and reading: paint-004; other behavior unchecked |
| `widgets/basic.dart:5005` Positioned | crates/incular-layout/src/algorithms.rs:115 | layout | not checked: pending audit |
| `widgets/basic.dart:5253` PositionedDirectional | crates/incular-widgets/src/custom_layout.rs:548 | layout | not checked: pending audit |
| `widgets/basic.dart:6828` RawImage | crates/incular-widgets/src/painting_effects/images.rs:12 | paint | not checked: pending audit |
| `widgets/basic.dart:7639` RepaintBoundary | crates/incular-widgets/src/layout/basic/painting.rs:55 | paint | not checked: pending audit |
| `widgets/basic.dart:6596` RichText | crates/incular-text/src/spans.rs:253 | text | partial scenario and reading: text-001; other behavior unchecked |
| `widgets/basic.dart:2230` RotatedBox | crates/incular-widgets/src/layout/basic/transforms.rs:14 | misc | partial scenario and reading: misc-002; other behavior unchecked |
| `widgets/basic.dart:5806` Row | crates/incular-layout/src/descriptors.rs:46 | layout | not checked: pending audit |
| `widgets/basic.dart:7942` Semantics | crates/incular-widgets/src/semantics.rs:15 | semantics | partial scenarios and reading: semantics-002; other behavior unchecked |
| `widgets/basic.dart:425` ShaderMask | crates/incular-widgets/src/compositing.rs:75 | paint | not checked: pending audit |
| `widgets/basic.dart:2732` SizedBox | crates/incular-layout/src/descriptors.rs:339 | focus_input | partial reading: focus_input-001 |
| `widgets/basic.dart:3494` SizedOverflowBox | crates/incular-widgets/src/layout/basic/sizing_constraints.rs:496 | layout | not checked: pending audit |
| `widgets/basic.dart:3960` SliverPadding | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:200 | slivers | not checked: pending audit |
| `widgets/basic.dart:4357` SliverSemantics | crates/incular-widgets/src/scrolling/sliver_semantics.rs:41 | slivers | not checked: pending audit |
| `widgets/basic.dart:3939` SliverToBoxAdapter | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:45 | slivers | not checked: pending audit |
| `widgets/basic.dart:4699` Stack | crates/incular-layout/src/descriptors.rs:166 | layout | not checked: pending audit |
| `widgets/basic.dart:8450` StatefulBuilder | UNRESOLVED | misc | not checked: pending audit |
| `widgets/basic.dart:1583` Transform | crates/incular-animation/src/curve2d.rs:456 | layout | not checked: pending audit |
| `widgets/basic.dart:3117` UnconstrainedBox | crates/incular-layout/src/descriptors.rs:384 | layout | not checked: pending audit |
| `widgets/basic.dart:7163` WidgetToRenderBoxAdapter | UNRESOLVED | misc | not checked: pending audit |
| `widgets/basic.dart:6191` Wrap | crates/incular-layout/src/descriptors.rs:204 | layout | not checked: pending audit |
| `widgets/binding.dart:1703` debugDumpApp | UNRESOLVED | misc | not checked: pending audit |
| `widgets/binding.dart:1767` RootElement | UNRESOLVED | misc | not checked: pending audit |
| `widgets/binding.dart:1714` RootWidget | UNRESOLVED | misc | not checked: pending audit |
| `widgets/binding.dart:1614` runApp | UNRESOLVED | misc | not checked: pending audit |
| `widgets/binding.dart:1674` runWidget | UNRESOLVED | misc | not checked: pending audit |
| `widgets/binding.dart:454` WidgetsBinding | UNRESOLVED | misc | not checked: pending audit |
| `widgets/binding.dart:85` WidgetsBindingObserver | UNRESOLVED | misc | not checked: pending audit |
| `widgets/binding.dart:1859` WidgetsFlutterBinding | UNRESOLVED | misc | not checked: pending audit |
| `widgets/bottom_navigation_bar_item.dart:30` BottomNavigationBarItem | crates/incular-material/src/navigation/bars.rs:16 | misc | not checked: pending audit |
| `widgets/color_filter.dart:36` ColorFiltered | crates/incular-widgets/src/tree/descriptors.rs:1720 | paint | not checked: pending audit |
| `widgets/constants.dart:19` kMinInteractiveDimension | UNRESOLVED | misc | not checked: pending audit |
| `widgets/container.dart:246` Container | crates/incular-widgets/src/layout/container.rs:25 | misc | not checked: pending audit |
| `widgets/container.dart:63` DecoratedBox | crates/incular-widgets/src/tree/descriptors.rs:445 | paint | not checked: pending audit |
| `widgets/context_menu_button_item.dart:65` ContextMenuButtonItem | crates/incular-widgets/src/context_menu.rs:63 | text | not checked: pending audit |
| `widgets/context_menu_button_item.dart:19` ContextMenuButtonType | crates/incular-widgets/src/context_menu.rs:27 | text | not checked: pending audit |
| `widgets/context_menu_controller.dart:30` ContextMenuController | UNRESOLVED | text | not checked: pending audit |
| `widgets/debug.dart:538` debugAssertAllWidgetVarsUnset | UNRESOLVED | misc | not checked: pending audit |
| `widgets/debug.dart:365` debugCheckHasDirectionality | UNRESOLVED | misc | not checked: pending audit |
| `widgets/debug.dart:302` debugCheckHasMediaQuery | UNRESOLVED | misc | not checked: pending audit |
| `widgets/debug.dart:500` debugCheckHasOverlay | UNRESOLVED | navigation | not checked: pending audit |
| `widgets/debug.dart:270` debugCheckHasTable | UNRESOLVED | layout | not checked: pending audit |
| `widgets/debug.dart:457` debugCheckHasWidgetsLocalizations | UNRESOLVED | misc | not checked: pending audit |
| `widgets/debug.dart:214` debugChildrenHaveDuplicateKeys | UNRESOLVED | misc | not checked: pending audit |
| `widgets/debug.dart:173` debugEnhanceBuildTimelineArguments | UNRESOLVED | misc | not checked: pending audit |
| `widgets/debug.dart:176` debugHighlightDeprecatedWidgets | UNRESOLVED | misc | not checked: pending audit |
| `widgets/debug.dart:241` debugItemsHaveDuplicateKeys | UNRESOLVED | misc | not checked: pending audit |
| `widgets/debug.dart:72` debugOnRebuildDirtyWidget | UNRESOLVED | misc | not checked: pending audit |
| `widgets/debug.dart:88` debugPrintBuildScope | UNRESOLVED | misc | not checked: pending audit |
| `widgets/debug.dart:107` debugPrintGlobalKeyedWidgetLifecycle | UNRESOLVED | misc | not checked: pending audit |
| `widgets/debug.dart:52` debugPrintRebuildDirtyWidgets | UNRESOLVED | misc | not checked: pending audit |
| `widgets/debug.dart:101` debugPrintScheduleBuildForStacks | UNRESOLVED | layout | not checked: pending audit |
| `widgets/debug.dart:135` debugProfileBuildsEnabled | UNRESOLVED | misc | not checked: pending audit |
| `widgets/debug.dart:151` debugProfileBuildsEnabledUserWidgets | UNRESOLVED | misc | not checked: pending audit |
| `widgets/debug.dart:407` debugWidgetBuilderValue | UNRESOLVED | misc | not checked: pending audit |
| `widgets/decorated_sliver.dart:58` DecoratedSliver | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1493 | slivers | not checked: pending audit |
| `widgets/default_selection_style.dart:28` DefaultSelectionStyle | crates/incular-widgets/src/environment.rs:147 | text | not checked: pending audit |
| `widgets/default_text_editing_shortcuts.dart:162` DefaultTextEditingShortcuts | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/default_text_editing_shortcuts.dart:1031` intentForMacOSSelector | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/desktop_text_selection_toolbar_layout_delegate.dart:24` DesktopTextSelectionToolbarLayoutDelegate | crates/incular-widgets/src/context_menu.rs:340 | text | not checked: pending audit |
| `widgets/dismissible.dart:42` DismissDirection | crates/incular-widgets/src/dismissible.rs:40 | gestures | not checked: pending audit |
| `widgets/dismissible.dart:93` Dismissible | crates/incular-navigation/src/presentation.rs:595 | gestures | not checked: pending audit |
| `widgets/dismissible.dart:231` DismissUpdateDetails | crates/incular-widgets/src/dismissible.rs:65 | gestures | not checked: pending audit |
| `widgets/display_feature_sub_screen.dart:54` DisplayFeatureSubScreen | crates/incular-widgets/src/display_feature_sub_screen.rs:15 | misc | not checked: pending audit |
| `widgets/disposable_build_context.dart:25` DisposableBuildContext | UNRESOLVED | text | not checked: pending audit |
| `widgets/drag_boundary.dart:75` DragBoundary | crates/incular-widgets/src/drag_boundary.rs:104 | gestures | not checked: pending audit |
| `widgets/drag_boundary.dart:16` DragBoundaryDelegate | crates/incular-widgets/src/drag_boundary.rs:13 | gestures | not checked: pending audit |
| `widgets/drag_target.dart:108` childDragAnchorStrategy | UNRESOLVED | gestures | not checked: pending audit |
| `widgets/drag_target.dart:174` Draggable | crates/incular-widgets/src/drag_drop.rs:141 | gestures | not checked: pending audit |
| `widgets/drag_target.dart:574` DraggableDetails | crates/incular-widgets/src/drag_drop.rs:124 | gestures | not checked: pending audit |
| `widgets/drag_target.dart:619` DragTarget | crates/incular-widgets/src/drag_drop.rs:447 | gestures | not checked: pending audit |
| `widgets/drag_target.dart:595` DragTargetDetails | UNRESOLVED | gestures | not checked: pending audit |
| `widgets/drag_target.dart:404` LongPressDraggable | crates/incular-widgets/src/drag_drop.rs:580 | gestures | not checked: pending audit |
| `widgets/drag_target.dart:130` pointerDragAnchorStrategy | UNRESOLVED | gestures | not checked: pending audit |
| `widgets/draggable_scrollable_sheet.dart:1030` DraggableScrollableActuator | crates/incular-widgets/src/advanced_scrolling/draggable.rs:240 | gestures | not checked: pending audit |
| `widgets/draggable_scrollable_sheet.dart:64` DraggableScrollableController | crates/incular-widgets/src/advanced_scrolling/draggable.rs:299 | gestures | not checked: pending audit |
| `widgets/draggable_scrollable_sheet.dart:429` DraggableScrollableNotification | crates/incular-widgets/src/advanced_scrolling/draggable.rs:31 | gestures | not checked: pending audit |
| `widgets/draggable_scrollable_sheet.dart:297` DraggableScrollableSheet | crates/incular-widgets/src/advanced_scrolling/draggable.rs:512 | gestures | not checked: pending audit |
| `widgets/dual_transition_builder.dart:30` DualTransitionBuilder | crates/incular-widgets/src/animation_transitions.rs:468 | transitions | not checked: pending audit |
| `widgets/editable_text.dart:453` ContentInsertionConfiguration | UNRESOLVED | text | not checked: pending audit |
| `widgets/editable_text.dart:815` EditableText | crates/incular-text/src/editing.rs:936 | text | partial scenario and reading: text-005 through text-008; other behavior unchecked |
| `widgets/editable_text.dart:2480` EditableTextState | UNRESOLVED | text | not checked: pending audit |
| `widgets/editable_text.dart:118` kDefaultContentInsertionMimeTypes | UNRESOLVED | text | not checked: pending audit |
| `widgets/editable_text.dart:221` TextEditingController | crates/incular-text/src/editing.rs:281 | text | partial scenario and reading: text-002, text-005; other behavior unchecked |
| `widgets/editable_text.dart:401` ToolbarOptions | crates/incular-widgets/src/tree/descriptors.rs:678 | text | not checked: pending audit |
| `widgets/expansible.dart:217` Expansible | crates/incular-widgets/src/utilities.rs:13 | misc | partial reading: misc-001 |
| `widgets/expansible.dart:63` ExpansibleController | UNRESOLVED | misc | not checked: pending audit |
| `widgets/fade_in_image.dart:61` FadeInImage | crates/incular-widgets/src/fade_in_image.rs:23 | paint | not checked: pending audit |
| `widgets/feedback.dart:82` Feedback | crates/incular-material/src/theme/theme_data.rs:420 | misc | not checked: pending audit |
| `widgets/flutter_logo.dart:25` FlutterLogo | crates/incular-widgets/src/flutter_logo.rs:59 | misc | not checked: pending audit |
| `widgets/focus_manager.dart:90` combineKeyEventResults | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_manager.dart:2363` debugDescribeFocusTree | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_manager.dart:2376` debugDumpFocusTree | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_manager.dart:29` debugFocusChanges | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_manager.dart:195` FocusAttachment | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_manager.dart:1546` FocusHighlightMode | crates/incular-gestures/src/focus.rs:274 | focus_input | not checked: pending audit |
| `widgets/focus_manager.dart:1566` FocusHighlightStrategy | crates/incular-gestures/src/focus.rs:282 | focus_input | not checked: pending audit |
| `widgets/focus_manager.dart:1636` FocusManager | crates/incular-gestures/src/focus.rs:856 | focus_input | not checked: pending audit |
| `widgets/focus_manager.dart:448` FocusNode | crates/incular-gestures/src/focus.rs:28 | focus_input | not checked: pending audit |
| `widgets/focus_manager.dart:1346` FocusScopeNode | crates/incular-gestures/src/focus.rs:1020 | focus_input | not checked: pending audit |
| `widgets/focus_manager.dart:65` KeyEventResult | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_manager.dart:2357` primaryFocus | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_manager.dart:280` UnfocusDisposition | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_scope.dart:916` ExcludeFocus | crates/incular-widgets/src/focus_keyboard.rs:774 | focus_input | not checked: pending audit |
| `widgets/focus_scope.dart:123` Focus | crates/incular-runtime/src/simulation.rs:519 | focus_input | not checked: pending audit |
| `widgets/focus_scope.dart:795` FocusScope | crates/incular-widgets/src/focus_keyboard.rs:93 | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:2409` DirectionalFocusAction | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:2380` DirectionalFocusIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:759` DirectionalFocusTraversalPolicyMixin | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:2439` ExcludeFocusTraversal | crates/incular-widgets/src/focus_keyboard.rs:803 | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:1737` FocusOrder | crates/incular-gestures/src/focus.rs:720 | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:2039` FocusTraversalGroup | crates/incular-widgets/src/focus_keyboard.rs:635 | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:1943` FocusTraversalOrder | crates/incular-widgets/src/focus_keyboard.rs:723 | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:185` FocusTraversalPolicy | crates/incular-gestures/src/focus.rs:640 | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:1824` LexicalFocusOrder | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:2318` NextFocusAction | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:2306` NextFocusIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:1787` NumericFocusOrder | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:1882` OrderedTraversalPolicy | crates/incular-gestures/src/focus.rs:832 | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:2353` PreviousFocusAction | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:2340` PreviousFocusIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:1566` ReadingOrderTraversalPolicy | crates/incular-gestures/src/focus.rs:810 | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:2295` RequestFocusAction | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:2254` RequestFocusIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:76` TraversalDirection | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:113` TraversalEdgeBehavior | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/focus_traversal.dart:1377` WidgetOrderTraversalPolicy | crates/incular-gestures/src/focus.rs:774 | focus_input | not checked: pending audit |
| `widgets/form.dart:829` AutovalidateMode | crates/incular-widgets/src/forms.rs:25 | misc | not checked: pending audit |
| `widgets/form.dart:60` Form | crates/incular-controls/src/form.rs:30 | misc | not checked: pending audit |
| `widgets/form.dart:482` FormField | crates/incular-widgets/src/forms.rs:284,452 | misc | partial scenario and reading: misc-007; other behavior unchecked |
| `widgets/form.dart:602` FormFieldState | crates/incular-widgets/src/forms.rs:654 | misc | not checked: pending audit |
| `widgets/form.dart:241` FormState | crates/incular-widgets/src/forms.rs:74 | misc | partial scenario and reading: misc-004; other behavior unchecked |
| `widgets/framework.dart:2306` BuildContext | crates/incular-core/src/context.rs:245 | text | not checked: pending audit |
| `widgets/framework.dart:2901` BuildOwner | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:2684` BuildScope | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:5775` ComponentElement | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:7374` DebugCreator | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:3557` Element | crates/incular-widgets/src/tree.rs:298 | misc | not checked: pending audit |
| `widgets/framework.dart:5613` ErrorWidget | crates/incular-widgets/src/utilities.rs:53 | misc | not checked: pending audit |
| `widgets/framework.dart:159` GlobalKey | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:245` GlobalObjectKey | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:7416` IndexedSlot | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:6252` InheritedElement | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:1853` InheritedWidget | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:203` LabeledGlobalKey | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:7054` LeafRenderObjectElement | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:1938` LeafRenderObjectWidget | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:7162` MultiChildRenderObjectElement | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:1992` MultiChildRenderObjectWidget | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:3474` NotifiableElementMixin | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:89` ObjectKey | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:6172` ParentDataElement | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:1587` ParentDataWidget | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:6135` ProxyElement | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:1522` ProxyWidget | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:6611` RenderObjectElement | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:1891` RenderObjectWidget | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:7312` RenderTreeRootElement | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:7028` RootElementMixin | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:7015` RootRenderObjectElement | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:7092` SingleChildRenderObjectElement | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:1956` SingleChildRenderObjectWidget | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:916` State | crates/incular-controls/src/checkbox.rs:85 | misc | not checked: pending audit |
| `widgets/framework.dart:5900` StatefulElement | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:771` StatefulWidget | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:5884` StatelessElement | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:523` StatelessWidget | UNRESOLVED | misc | not checked: pending audit |
| `widgets/framework.dart:312` Widget | crates/incular-controls/src/icons.rs:35 | misc | not checked: pending audit |
| `widgets/gesture_detector.dart:223` GestureDetector | crates/incular-gestures/src/pointer.rs:171 | gestures | partial scenarios and reading: gestures |
| `widgets/gesture_detector.dart:71` GestureRecognizerFactory | crates/incular-gestures/src/raw.rs:50 | gestures | not checked: pending audit |
| `widgets/gesture_detector.dart:104` GestureRecognizerFactoryWithHandlers | crates/incular-gestures/src/raw.rs:65 | gestures | not checked: pending audit |
| `widgets/gesture_detector.dart:1318` RawGestureDetector | crates/incular-widgets/src/raw_input.rs:561 | gestures | not checked: pending audit |
| `widgets/gesture_detector.dart:1433` RawGestureDetectorState | UNRESOLVED | gestures | not checked: pending audit |
| `widgets/gesture_detector.dart:1665` SemanticsGestureDelegate | UNRESOLVED | gestures | not checked: pending audit |
| `widgets/grid_paper.dart:63` GridPaper | crates/incular-widgets/src/painting_effects/layers.rs:199 | misc | not checked: pending audit |
| `widgets/heroes.dart:170` Hero | crates/incular-widgets/src/hero.rs:159 | navigation | not checked: pending audit |
| `widgets/heroes.dart:810` HeroController | crates/incular-navigation/src/hero_controller.rs:24 | navigation | not checked: pending audit |
| `widgets/heroes.dart:57` HeroFlightDirection | crates/incular-widgets/src/hero.rs:59 | navigation | not checked: pending audit |
| `widgets/heroes.dart:1104` HeroMode | crates/incular-widgets/src/hero.rs:226 | navigation | not checked: pending audit |
| `widgets/icon_data.dart:23` IconData | crates/incular-text/src/style/icons.rs:12 | misc | not checked: pending audit |
| `widgets/icon_data.dart:112` IconDataProperty | UNRESOLVED | misc | not checked: pending audit |
| `widgets/icon_data.dart:156` staticIconProvider | UNRESOLVED | misc | not checked: pending audit |
| `widgets/icon_theme_data.dart:27` IconThemeData | UNRESOLVED | misc | not checked: pending audit |
| `widgets/icon_theme.dart:24` IconTheme | crates/incular-widgets/src/environment.rs:289 | misc | not checked: pending audit |
| `widgets/icon.dart:74` Icon | crates/incular-material/src/buttons/icon_floating.rs:14 | misc | not checked: pending audit |
| `widgets/image_filter.dart:39` ImageFiltered | crates/incular-widgets/src/painting_effects/images.rs:196 | paint | not checked: pending audit |
| `widgets/image_icon.dart:25` ImageIcon | crates/incular-widgets/src/painting_effects/images.rs:140 | paint | not checked: pending audit |
| `widgets/image.dart:67` createLocalImageConfiguration | UNRESOLVED | paint | not checked: pending audit |
| `widgets/image.dart:343` Image | crates/incular-widgets/src/tree/descriptors.rs:34 | paint | partial scenario and reading: paint-002; other behavior unchecked |
| `widgets/image.dart:121` precacheImage | UNRESOLVED | paint | not checked: pending audit |
| `widgets/implicit_animations.dart:994` AnimatedAlign | crates/incular-widgets/src/implicit_animations.rs:519 | implicit | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:604` AnimatedContainer | crates/incular-widgets/src/implicit_animations.rs:246 | implicit | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:2044` AnimatedDefaultTextStyle | crates/incular-widgets/src/implicit_animations.rs:1127 | implicit | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:2332` AnimatedFractionallySizedBox | crates/incular-widgets/src/implicit_animations.rs:619 | implicit | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:1841` AnimatedOpacity | crates/incular-widgets/src/implicit_animations.rs:1079 | implicit | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:902` AnimatedPadding | crates/incular-widgets/src/implicit_animations.rs:471 | implicit | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:2179` AnimatedPhysicalModel | crates/incular-widgets/src/implicit_animations.rs:1172 | implicit | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:1150` AnimatedPositioned | crates/incular-widgets/src/implicit_animations.rs:760 | implicit | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:1310` AnimatedPositionedDirectional | crates/incular-widgets/src/implicit_animations.rs:840 | implicit | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:1615` AnimatedRotation | crates/incular-widgets/src/implicit_animations.rs:977 | implicit | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:1489` AnimatedScale | crates/incular-widgets/src/implicit_animations.rs:921 | implicit | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:1713` AnimatedSlide | crates/incular-widgets/src/implicit_animations.rs:1034 | implicit | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:553` AnimatedWidgetBaseState | UNRESOLVED | transitions | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:138` BorderRadiusTween | crates/incular-widgets/src/tweens.rs:13 | misc | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:156` BorderTween | crates/incular-widgets/src/tweens.rs:15 | misc | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:44` BoxConstraintsTween | crates/incular-widgets/src/tweens.rs:17 | misc | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:73` DecorationTween | crates/incular-widgets/src/tweens.rs:19 | paint | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:120` EdgeInsetsGeometryTween | UNRESOLVED | misc | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:98` EdgeInsetsTween | crates/incular-widgets/src/tweens.rs:21 | misc | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:284` ImplicitlyAnimatedWidget | UNRESOLVED | implicit | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:358` ImplicitlyAnimatedWidgetState | UNRESOLVED | implicit | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:176` Matrix4Tween | crates/incular-animation/src/value.rs:327 | misc | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:1954` SliverAnimatedOpacity | crates/incular-widgets/src/scrolling/sliver_fade.rs:52 | implicit | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/implicit_animations.dart:213` TextStyleTween | crates/incular-widgets/src/tweens.rs:25 | text | partial scenarios and reading: implicit; see package coverage and open cases |
| `widgets/inherited_model.dart:121` InheritedModel | crates/incular-widgets/src/inherited_data.rs:224 | misc | not checked: pending audit |
| `widgets/inherited_model.dart:219` InheritedModelElement | UNRESOLVED | misc | not checked: pending audit |
| `widgets/inherited_notifier.dart:65` InheritedNotifier | crates/incular-widgets/src/inherited_data.rs:286 | misc | not checked: pending audit |
| `widgets/inherited_theme.dart:133` CapturedThemes | crates/incular-widgets/src/inherited_data.rs:323 | misc | not checked: pending audit |
| `widgets/inherited_theme.dart:36` InheritedTheme | UNRESOLVED | misc | not checked: pending audit |
| `widgets/interactive_viewer.dart:67` InteractiveViewer | crates/incular-widgets/src/interactive_viewer.rs:236 | gestures | not checked: pending audit |
| `widgets/interactive_viewer.dart:1305` PanAxis | crates/incular-widgets/src/interactive_viewer.rs:25 | misc | not checked: pending audit |
| `widgets/interactive_viewer.dart:1155` TransformationController | crates/incular-widgets/src/interactive_viewer.rs:41 | layout | not checked: pending audit |
| `widgets/keyboard_listener.dart:31` KeyboardListener | crates/incular-widgets/src/focus_keyboard.rs:152 | focus_input | not checked: pending audit |
| `widgets/layout_builder.dart:40` AbstractLayoutBuilder | UNRESOLVED | layout | not checked: pending audit |
| `widgets/layout_builder.dart:100` ConstrainedLayoutBuilder | UNRESOLVED | layout | not checked: pending audit |
| `widgets/layout_builder.dart:381` LayoutBuilder | crates/incular-widgets/src/layout/basic/sizing_constraints.rs:366 | layout | not checked: pending audit |
| `widgets/layout_builder.dart:309` RenderAbstractLayoutBuilderMixin | UNRESOLVED | layout | not checked: pending audit |
| `widgets/list_wheel_scroll_view.dart:34` ChangeReportingBehavior | crates/incular-widgets/src/advanced_scrolling/wheel.rs:18 | scroll | not checked: pending audit |
| `widgets/list_wheel_scroll_view.dart:329` FixedExtentMetrics | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/list_wheel_scroll_view.dart:229` FixedExtentScrollController | crates/incular-widgets/src/advanced_scrolling/wheel.rs:979 | scroll | not checked: pending audit |
| `widgets/list_wheel_scroll_view.dart:479` FixedExtentScrollPhysics | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/list_wheel_scroll_view.dart:177` ListWheelChildBuilderDelegate | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/list_wheel_scroll_view.dart:57` ListWheelChildDelegate | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/list_wheel_scroll_view.dart:99` ListWheelChildListDelegate | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/list_wheel_scroll_view.dart:143` ListWheelChildLoopingListDelegate | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/list_wheel_scroll_view.dart:834` ListWheelElement | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/list_wheel_scroll_view.dart:577` ListWheelScrollView | crates/incular-widgets/src/advanced_scrolling/wheel.rs:932 | scroll | not checked: pending audit |
| `widgets/list_wheel_scroll_view.dart:1000` ListWheelViewport | crates/incular-widgets/src/advanced_scrolling/wheel.rs:457 | scroll | not checked: pending audit |
| `widgets/localizations.dart:273` DefaultWidgetsLocalizations | UNRESOLVED | misc | not checked: pending audit |
| `widgets/localizations.dart:479` Localizations | crates/incular-widgets/src/environment.rs:68 | misc | not checked: pending audit |
| `widgets/localizations.dart:114` LocalizationsDelegate | UNRESOLVED | misc | not checked: pending audit |
| `widgets/localizations.dart:754` LocalizationsResolver | UNRESOLVED | misc | not checked: pending audit |
| `widgets/localizations.dart:169` WidgetsLocalizations | UNRESOLVED | misc | not checked: pending audit |
| `widgets/lookup_boundary.dart:70` LookupBoundary | crates/incular-widgets/src/environment.rs:631 | misc | not checked: pending audit |
| `widgets/magnifier.dart:150` MagnifierController | crates/incular-widgets/src/magnifier.rs:115 | misc | not checked: pending audit |
| `widgets/magnifier.dart:358` MagnifierDecoration | crates/incular-widgets/src/magnifier.rs:141 | paint | not checked: pending audit |
| `widgets/magnifier.dart:44` MagnifierInfo | crates/incular-widgets/src/magnifier.rs:38 | misc | not checked: pending audit |
| `widgets/magnifier.dart:448` RawMagnifier | crates/incular-widgets/src/magnifier.rs:161 | misc | not checked: pending audit |
| `widgets/magnifier.dart:106` TextMagnifierConfiguration | crates/incular-widgets/src/magnifier.rs:62 | text | not checked: pending audit |
| `widgets/media_query.dart:1141` MediaQuery | crates/incular-widgets/src/environment.rs:22 | misc | partial scenario and reading: misc-003; other behavior unchecked |
| `widgets/media_query.dart:199` MediaQueryData | crates/incular-widgets/src/environment.rs:33 | misc | not checked: pending audit |
| `widgets/media_query.dart:2158` NavigationMode | crates/incular-config/src/environment.rs:73 | misc | not checked: pending audit |
| `widgets/media_query.dart:33` Orientation | crates/incular-controls/src/composite.rs:122 | misc | not checked: pending audit |
| `widgets/media_query.dart:2314` SystemTextScaler | UNRESOLVED | text | not checked: pending audit |
| `widgets/modal_barrier.dart:289` AnimatedModalBarrier | crates/incular-widgets/src/navigation/barrier.rs:17 | switchers | partial scenarios and reading: switchers |
| `widgets/modal_barrier.dart:128` ModalBarrier | crates/incular-navigation/src/presentation.rs:549 | misc | not checked: pending audit |
| `widgets/navigation_toolbar.dart:26` NavigationToolbar | crates/incular-widgets/src/layout/basic/utility.rs:132 | misc | not checked: pending audit |
| `widgets/navigator_pop_handler.dart:40` NavigatorPopHandler | crates/incular-widgets/src/navigation/pop_scopes.rs:292 | navigation | not checked: pending audit |
| `widgets/navigator.dart:1182` DefaultTransitionDelegate | UNRESOLVED | transitions | not checked: pending audit |
| `widgets/navigator.dart:829` HeroControllerScope | UNRESOLVED | navigation | not checked: pending audit |
| `widgets/navigator.dart:1260` kDefaultRouteDirectionalTraversalEdgeBehavior | UNRESOLVED | navigation | not checked: pending audit |
| `widgets/navigator.dart:1255` kDefaultRouteTraversalEdgeBehavior | UNRESOLVED | navigation | not checked: pending audit |
| `widgets/navigator.dart:6422` NavigationNotification | crates/incular-widgets/src/app_shell/router.rs:764 | navigation | not checked: pending audit |
| `widgets/navigator.dart:1561` Navigator | crates/incular-navigation/src/navigator.rs:326 | navigation | not checked: pending audit |
| `widgets/navigator.dart:755` NavigatorObserver | crates/incular-navigation/src/navigator.rs:231 | navigation | not checked: pending audit |
| `widgets/navigator.dart:3696` NavigatorState | UNRESOLVED | navigation | not checked: pending audit |
| `widgets/navigator.dart:685` Page | crates/incular-navigation/src/presentation.rs:27 | navigation | not checked: pending audit |
| `widgets/navigator.dart:6301` RestorableRouteFuture | UNRESOLVED | navigation | not checked: pending audit |
| `widgets/navigator.dart:161` Route | crates/incular-material/src/app_shell/material_app.rs:111 | navigation | not checked: pending audit |
| `widgets/navigator.dart:117` RoutePopDisposition | UNRESOLVED | navigation | not checked: pending audit |
| `widgets/navigator.dart:648` RouteSettings | crates/incular-navigation/src/route_data.rs:356 | navigation | not checked: pending audit |
| `widgets/navigator.dart:901` RouteTransitionRecord | UNRESOLVED | transitions | not checked: pending audit |
| `widgets/navigator.dart:1016` TransitionDelegate | UNRESOLVED | transitions | not checked: pending audit |
| `widgets/nested_scroll_view.dart:183` NestedScrollView | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:596 | scroll | not checked: pending audit |
| `widgets/nested_scroll_view.dart:393` NestedScrollViewState | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/nested_scroll_view.dart:1990` NestedScrollViewViewport | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/nested_scroll_view.dart:2043` RenderNestedScrollViewViewport | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/nested_scroll_view.dart:1720` RenderSliverOverlapAbsorber | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/nested_scroll_view.dart:1869` RenderSliverOverlapInjector | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/nested_scroll_view.dart:1684` SliverOverlapAbsorber | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:2184 | scroll | not checked: pending audit |
| `widgets/nested_scroll_view.dart:1607` SliverOverlapAbsorberHandle | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/nested_scroll_view.dart:1833` SliverOverlapInjector | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:2228 | scroll | not checked: pending audit |
| `widgets/notification_listener.dart:174` LayoutChangedNotification | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/notification_listener.dart:56` Notification | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/notification_listener.dart:100` NotificationListener | crates/incular-widgets/src/scrolling/notifications.rs:12 | focus_input | not checked: pending audit |
| `widgets/orientation_builder.dart:92` DeviceOrientationBuilder | UNRESOLVED | misc | not checked: pending audit |
| `widgets/orientation_builder.dart:27` OrientationBuilder | crates/incular-widgets/src/environment.rs:354 | misc | not checked: pending audit |
| `widgets/overflow_bar.dart:58` OverflowBar | crates/incular-widgets/src/layout/basic/utility.rs:68 | layout | not checked: pending audit |
| `widgets/overflow_bar.dart:20` OverflowBarAlignment | UNRESOLVED | layout | not checked: pending audit |
| `widgets/overlay.dart:478` Overlay | crates/incular-controls/src/overlay.rs:56 | navigation | not checked: pending audit |
| `widgets/overlay.dart:38` OverlayChildLayoutInfo | crates/incular-widgets/src/transient.rs:591 | navigation | not checked: pending audit |
| `widgets/overlay.dart:1712` OverlayChildLocation | UNRESOLVED | navigation | not checked: pending audit |
| `widgets/overlay.dart:108` OverlayEntry | crates/incular-navigation/src/presentation.rs:569 | navigation | not checked: pending audit |
| `widgets/overlay.dart:1802` OverlayPortal | crates/incular-widgets/src/navigation_scopes.rs:17 | navigation | partial scenario, pixel and reading: navigation-007; other behavior unchecked |
| `widgets/overlay.dart:1618` OverlayPortalController | UNRESOLVED | navigation | not checked: pending audit |
| `widgets/overlay.dart:626` OverlayState | UNRESOLVED | navigation | not checked: pending audit |
| `widgets/overscroll_indicator.dart:85` GlowingOverscrollIndicator | crates/incular-widgets/src/overscroll_indicator.rs:621 | scroll | not checked: pending audit |
| `widgets/overscroll_indicator.dart:1052` OverscrollIndicatorNotification | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/overscroll_indicator.dart:662` StretchingOverscrollIndicator | crates/incular-widgets/src/overscroll_indicator.rs:96 | scroll | not checked: pending audit |
| `widgets/page_storage.dart:170` PageStorage | crates/incular-widgets/src/navigation/page_storage.rs:261 | misc | not checked: pending audit |
| `widgets/page_storage.dart:70` PageStorageBucket | crates/incular-widgets/src/navigation/page_storage.rs:130 | misc | not checked: pending audit |
| `widgets/page_storage.dart:35` PageStorageKey | crates/incular-widgets/src/navigation/page_storage.rs:16 | misc | not checked: pending audit |
| `widgets/page_transitions_builder.dart:144` FadeUpwardsPageTransitionsBuilder | UNRESOLVED | transitions | not checked: pending audit |
| `widgets/page_transitions_builder.dart:321` OpenUpwardsPageTransitionsBuilder | UNRESOLVED | transitions | not checked: pending audit |
| `widgets/page_transitions_builder.dart:52` PageTransitionsBuilder | crates/incular-material/src/page_transitions.rs:22 | transitions | not checked: pending audit |
| `widgets/page_view.dart:119` PageController | crates/incular-widgets/src/scrolling/scroll_views.rs:1036 | scroll | partial scenario and reading: scroll-002 |
| `widgets/page_view.dart:286` PageMetrics | UNRESOLVED | misc | not checked: pending audit |
| `widgets/page_view.dart:564` PageScrollPhysics | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/page_view.dart:664` PageView | crates/incular-widgets/src/scrolling/scroll_views.rs:1056 | scroll | partial scenario and reading: scroll |
| `widgets/pages.dart:23` PageRoute | UNRESOLVED | navigation | not checked: pending audit |
| `widgets/pages.dart:89` PageRouteBuilder | crates/incular-navigation/src/presentation.rs:330 | navigation | not checked: pending audit |
| `widgets/performance_overlay.dart:29` PerformanceOverlay | crates/incular-widgets/src/utilities.rs:383 | navigation | not checked: pending audit |
| `widgets/pinned_header_sliver.dart:56` PinnedHeaderSliver | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1772 | slivers | partial scenario: slivers__pinned_header_sliver__child_extent; other behavior unchecked |
| `widgets/placeholder.dart:50` Placeholder | crates/incular-controls/src/text_input.rs:113 | misc | not checked: pending audit |
| `widgets/platform_menu_bar.dart:287` DefaultPlatformMenuDelegate | UNRESOLVED | misc | not checked: pending audit |
| `widgets/platform_menu_bar.dart:177` MenuSerializableShortcut | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/platform_menu_bar.dart:535` PlatformMenu | crates/incular-widgets/src/platform_widgets/menu.rs:262 | misc | not checked: pending audit |
| `widgets/platform_menu_bar.dart:439` PlatformMenuBar | crates/incular-widgets/src/platform_widgets/menu.rs:804 | misc | not checked: pending audit |
| `widgets/platform_menu_bar.dart:212` PlatformMenuDelegate | crates/incular-widgets/src/platform_widgets/menu.rs:409 | misc | not checked: pending audit |
| `widgets/platform_menu_bar.dart:704` PlatformMenuItem | crates/incular-widgets/src/platform_widgets/menu.rs:117 | misc | not checked: pending audit |
| `widgets/platform_menu_bar.dart:648` PlatformMenuItemGroup | crates/incular-widgets/src/platform_widgets/menu.rs:216 | misc | not checked: pending audit |
| `widgets/platform_menu_bar.dart:848` PlatformProvidedMenuItem | UNRESOLVED | misc | not checked: pending audit |
| `widgets/platform_menu_bar.dart:947` PlatformProvidedMenuItemType | UNRESOLVED | misc | not checked: pending audit |
| `widgets/platform_menu_bar.dart:45` ShortcutSerialization | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/platform_view.dart:70` AndroidView | UNRESOLVED | misc | not checked: pending audit |
| `widgets/platform_view.dart:1428` AndroidViewSurface | UNRESOLVED | misc | not checked: pending audit |
| `widgets/platform_view.dart:349` AppKitView | UNRESOLVED | misc | not checked: pending audit |
| `widgets/platform_view.dart:649` HtmlElementView | UNRESOLVED | misc | not checked: pending audit |
| `widgets/platform_view.dart:1108` PlatformViewCreationParams | UNRESOLVED | misc | not checked: pending audit |
| `widgets/platform_view.dart:1189` PlatformViewLink | UNRESOLVED | misc | not checked: pending audit |
| `widgets/platform_view.dart:1331` PlatformViewSurface | UNRESOLVED | misc | not checked: pending audit |
| `widgets/platform_view.dart:313` UiKitView | UNRESOLVED | misc | not checked: pending audit |
| `widgets/pop_scope.dart:83` PopScope | crates/incular-widgets/src/navigation/pop_scopes.rs:90 | navigation | not checked: pending audit |
| `widgets/preferred_size.dart:66` PreferredSize | crates/incular-widgets/src/layout/basic/sizing_constraints.rs:591 | misc | not checked: pending audit |
| `widgets/preferred_size.dart:27` PreferredSizeWidget | UNRESOLVED | misc | not checked: pending audit |
| `widgets/primary_scroll_controller.dart:60` PrimaryScrollController | crates/incular-widgets/src/environment.rs:434 | scroll | not checked: pending audit |
| `widgets/radio_group.dart:294` RadioClient | UNRESOLVED | misc | not checked: pending audit |
| `widgets/radio_group.dart:51` RadioGroup | crates/incular-material/src/selection/checkbox_radio.rs:442 | misc | not checked: pending audit |
| `widgets/radio_group.dart:269` RadioGroupRegistry | UNRESOLVED | misc | not checked: pending audit |
| `widgets/raw_keyboard_listener.dart:41` RawKeyboardListener | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/raw_menu_anchor.dart:1069` DismissMenuAction | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/raw_menu_anchor.dart:980` MenuController | crates/incular-material/src/menus/controls.rs:24 | misc | not checked: pending audit |
| `widgets/raw_menu_anchor.dart:221` RawMenuAnchor | crates/incular-widgets/src/raw_menu_anchor.rs:346 | misc | not checked: pending audit |
| `widgets/raw_menu_anchor.dart:867` RawMenuAnchorGroup | crates/incular-widgets/src/raw_menu_anchor.rs:539 | misc | not checked: pending audit |
| `widgets/raw_menu_anchor.dart:45` RawMenuOverlayInfo | crates/incular-widgets/src/raw_menu_anchor.rs:22 | navigation | not checked: pending audit |
| `widgets/raw_radio.dart:44` RawRadio | crates/incular-widgets/src/radio_selection.rs:46 | misc | not checked: pending audit |
| `widgets/raw_tooltip.dart:231` RawTooltip | crates/incular-widgets/src/raw_tooltip.rs:777 | misc | not checked: pending audit |
| `widgets/raw_tooltip.dart:521` RawTooltipState | UNRESOLVED | misc | not checked: pending audit |
| `widgets/raw_tooltip.dart:65` TooltipPositionContext | UNRESOLVED | text | not checked: pending audit |
| `widgets/raw_tooltip.dart:134` TooltipTriggerMode | crates/incular-material/src/feedback/transient.rs:426 | misc | not checked: pending audit |
| `widgets/reorderable_list.dart:1378` ReorderableDelayedDragStartListener | crates/incular-widgets/src/scrolling/reorderable.rs:272 | gestures | not checked: pending audit |
| `widgets/reorderable_list.dart:1309` ReorderableDragStartListener | crates/incular-widgets/src/scrolling/reorderable.rs:213 | gestures | not checked: pending audit |
| `widgets/reorderable_list.dart:145` ReorderableList | crates/incular-widgets/src/scrolling/reorderable.rs:330 | gestures | not checked: pending audit |
| `widgets/reorderable_list.dart:402` ReorderableListState | UNRESOLVED | gestures | not checked: pending audit |
| `widgets/reorderable_list.dart:501` SliverReorderableList | crates/incular-widgets/src/scrolling/sliver_lists.rs:455 | gestures | not checked: pending audit |
| `widgets/reorderable_list.dart:670` SliverReorderableListState | UNRESOLVED | gestures | not checked: pending audit |
| `widgets/repeating_animation_builder.dart:44` RepeatingAnimationBuilder | crates/incular-widgets/src/repeating_animation_builder.rs:33 | misc | not checked: pending audit |
| `widgets/repeating_animation_builder.dart:13` RepeatMode | crates/incular-widgets/src/repeating_animation_builder.rs:18 | misc | not checked: pending audit |
| `widgets/restoration_properties.dart:241` RestorableBool | crates/incular-runtime/src/restoration.rs:1105 | misc | not checked: pending audit |
| `widgets/restoration_properties.dart:256` RestorableBoolN | crates/incular-runtime/src/restoration.rs:1107 | misc | not checked: pending audit |
| `widgets/restoration_properties.dart:441` RestorableChangeNotifier | UNRESOLVED | misc | not checked: pending audit |
| `widgets/restoration_properties.dart:330` RestorableDateTime | UNRESOLVED | misc | not checked: pending audit |
| `widgets/restoration_properties.dart:358` RestorableDateTimeN | UNRESOLVED | misc | not checked: pending audit |
| `widgets/restoration_properties.dart:199` RestorableDouble | crates/incular-runtime/src/restoration.rs:1101 | misc | not checked: pending audit |
| `widgets/restoration_properties.dart:290` RestorableDoubleN | crates/incular-runtime/src/restoration.rs:1103 | misc | not checked: pending audit |
| `widgets/restoration_properties.dart:615` RestorableEnum | UNRESOLVED | misc | not checked: pending audit |
| `widgets/restoration_properties.dart:521` RestorableEnumN | UNRESOLVED | misc | not checked: pending audit |
| `widgets/restoration_properties.dart:213` RestorableInt | crates/incular-runtime/src/restoration.rs:1097 | misc | not checked: pending audit |
| `widgets/restoration_properties.dart:305` RestorableIntN | crates/incular-runtime/src/restoration.rs:1099 | misc | not checked: pending audit |
| `widgets/restoration_properties.dart:394` RestorableListenable | UNRESOLVED | misc | not checked: pending audit |
| `widgets/restoration_properties.dart:182` RestorableNum | UNRESOLVED | misc | not checked: pending audit |
| `widgets/restoration_properties.dart:275` RestorableNumN | UNRESOLVED | misc | not checked: pending audit |
| `widgets/restoration_properties.dart:227` RestorableString | crates/incular-runtime/src/restoration.rs:1109 | misc | not checked: pending audit |
| `widgets/restoration_properties.dart:320` RestorableStringN | crates/incular-runtime/src/restoration.rs:1111 | misc | not checked: pending audit |
| `widgets/restoration_properties.dart:470` RestorableTextEditingController | UNRESOLVED | text | not checked: pending audit |
| `widgets/restoration_properties.dart:74` RestorableValue | UNRESOLVED | misc | not checked: pending audit |
| `widgets/restoration.dart:438` RestorableProperty | UNRESOLVED | misc | not checked: pending audit |
| `widgets/restoration.dart:647` RestorationMixin | UNRESOLVED | misc | not checked: pending audit |
| `widgets/restoration.dart:56` RestorationScope | crates/incular-core/src/restoration.rs:128 | misc | not checked: pending audit |
| `widgets/restoration.dart:262` RootRestorationScope | crates/incular-widgets/src/navigation/restoration.rs:95 | misc | not checked: pending audit |
| `widgets/restoration.dart:194` UnmanagedRestorationScope | crates/incular-widgets/src/navigation/restoration.rs:25 | misc | not checked: pending audit |
| `widgets/router.dart:972` BackButtonDispatcher | crates/incular-widgets/src/app_shell/router.rs:635 | navigation | partial reading: navigation-006; other behavior unchecked |
| `widgets/router.dart:1179` BackButtonListener | crates/incular-widgets/src/navigation/pop_scopes.rs:414 | navigation | not checked: pending audit |
| `widgets/router.dart:1125` ChildBackButtonDispatcher | crates/incular-widgets/src/navigation/back_dispatch.rs:413 (separate API; Router integration absent) | navigation | partial reading: navigation-006; other behavior unchecked |
| `widgets/router.dart:1503` PlatformRouteInformationProvider | UNRESOLVED | navigation | not checked: pending audit |
| `widgets/router.dart:1603` PopNavigatorRouterDelegateMixin | UNRESOLVED | navigation | not checked: pending audit |
| `widgets/router.dart:1092` RootBackButtonDispatcher | crates/incular-widgets/src/app_shell/router.rs:714 | navigation | not checked: pending audit |
| `widgets/router.dart:49` RouteInformation | crates/incular-widgets/src/app_shell/router.rs:54 | navigation | not checked: pending audit |
| `widgets/router.dart:1246` RouteInformationParser | crates/incular-widgets/src/app_shell/router.rs:349 | navigation | not checked: pending audit |
| `widgets/router.dart:1466` RouteInformationProvider | crates/incular-widgets/src/app_shell/router.rs:203 | navigation | not checked: pending audit |
| `widgets/router.dart:590` RouteInformationReportingType | crates/incular-widgets/src/app_shell/router.rs:151 | navigation | not checked: pending audit |
| `widgets/router.dart:358` Router | crates/incular-widgets/src/app_shell/router.rs:900 | navigation | partial scenario and reading: navigation__router__provider_route_change and navigation-006; other behavior unchecked |
| `widgets/router.dart:135` RouterConfig | crates/incular-widgets/src/app_shell/router.rs:786 | navigation | not checked: pending audit |
| `widgets/router.dart:1341` RouterDelegate | crates/incular-widgets/src/app_shell/router.rs:435 | navigation | not checked: pending audit |
| `widgets/routes.dart:708` LocalHistoryEntry | UNRESOLVED | misc | not checked: pending audit |
| `widgets/routes.dart:747` LocalHistoryRoute | UNRESOLVED | misc | not checked: pending audit |
| `widgets/routes.dart:1269` ModalRoute | crates/incular-navigation/src/route_observer.rs:22 | navigation | partial reading: navigation-001 |
| `widgets/routes.dart:55` OverlayRoute | UNRESOLVED | navigation | not checked: pending audit |
| `widgets/routes.dart:2819` PopEntry | UNRESOLVED | misc | not checked: pending audit |
| `widgets/routes.dart:2383` PopupRoute | UNRESOLVED | misc | not checked: pending audit |
| `widgets/routes.dart:654` PredictiveBackRoute | UNRESOLVED | misc | not checked: pending audit |
| `widgets/routes.dart:2580` RawDialogRoute | UNRESOLVED | misc | not checked: pending audit |
| `widgets/routes.dart:2520` RouteAware | crates/incular-navigation/src/route_observer.rs:112 | navigation | not checked: pending audit |
| `widgets/routes.dart:2436` RouteObserver | crates/incular-navigation/src/route_observer.rs:233 | navigation | not checked: pending audit |
| `widgets/routes.dart:2746` showGeneralDialog | UNRESOLVED | misc | not checked: pending audit |
| `widgets/routes.dart:111` TransitionRoute | UNRESOLVED | transitions | not checked: pending audit |
| `widgets/safe_area.dart:61` SafeArea | crates/incular-widgets/src/safe_area.rs:17 | misc | partial scenario and reading: misc-008; other behavior unchecked |
| `widgets/safe_area.dart:169` SliverSafeArea | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1687 | slivers | not checked: pending audit |
| `widgets/scroll_activity.dart:584` BallisticScrollActivity | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_activity.dart:476` DragScrollActivity | UNRESOLVED | gestures | not checked: pending audit |
| `widgets/scroll_activity.dart:695` DrivenScrollActivity | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_activity.dart:221` HoldScrollActivity | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_activity.dart:181` IdleScrollActivity | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_activity.dart:67` ScrollActivity | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_activity.dart:35` ScrollActivityDelegate | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_activity.dart:256` ScrollDragController | UNRESOLVED | gestures | not checked: pending audit |
| `widgets/scroll_activity.dart:205` ScrollHoldController | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_aware_image_provider.dart:48` ScrollAwareImageProvider | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_configuration.dart:40` AndroidOverscrollIndicator | crates/incular-widgets/src/scroll_behavior.rs:36 | scroll | not checked: pending audit |
| `widgets/scroll_configuration.dart:73` ScrollBehavior | crates/incular-widgets/src/scroll_behavior.rs:46 | scroll | not checked: pending audit |
| `widgets/scroll_configuration.dart:397` ScrollConfiguration | crates/incular-widgets/src/environment.rs:391 | scroll | not checked: pending audit |
| `widgets/scroll_context.dart:30` ScrollContext | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_controller.dart:72` ScrollController | crates/incular-scroll/src/controller.rs:27 | scroll | partial scenarios and reading: scroll-001, scroll-003 |
| `widgets/scroll_controller.dart:383` TrackingScrollController | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_delegate.dart:352` SliverChildBuilderDelegate | crates/incular-widgets/src/scrolling/child_delegates.rs:119 | scroll | not checked: pending audit |
| `widgets/scroll_delegate.dart:135` SliverChildDelegate | crates/incular-widgets/src/scrolling/child_delegates.rs:13 | scroll | not checked: pending audit |
| `widgets/scroll_delegate.dart:633` SliverChildListDelegate | crates/incular-widgets/src/scrolling/child_delegates.rs:165 | scroll | not checked: pending audit |
| `widgets/scroll_delegate.dart:993` TwoDimensionalChildBuilderDelegate | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_delegate.dart:946` TwoDimensionalChildDelegate | crates/incular-widgets/src/advanced_scrolling/two_dimensional.rs:37 | scroll | not checked: pending audit |
| `widgets/scroll_delegate.dart:1165` TwoDimensionalChildListDelegate | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_metrics.dart:172` FixedScrollMetrics | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_metrics.dart:50` ScrollMetrics | crates/incular-scroll/src/metrics.rs:3 | scroll | not checked: pending audit |
| `widgets/scroll_notification_observer.dart:87` ScrollNotificationObserver | crates/incular-widgets/src/scrolling/notifications.rs:66 | scroll | not checked: pending audit |
| `widgets/scroll_notification_observer.dart:155` ScrollNotificationObserverState | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_notification.dart:372` defaultScrollNotificationPredicate | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_notification.dart:226` OverscrollNotification | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_notification.dart:298` ScrollEndNotification | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_notification.dart:129` ScrollNotification | crates/incular-scroll/src/notifications.rs:29 | scroll | not checked: pending audit |
| `widgets/scroll_notification.dart:156` ScrollStartNotification | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_notification.dart:183` ScrollUpdateNotification | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_notification.dart:339` UserScrollNotification | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_notification.dart:52` ViewportElementMixin | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_notification.dart:28` ViewportNotificationMixin | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_physics.dart:946` AlwaysScrollableScrollPhysics | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_physics.dart:678` BouncingScrollPhysics | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_physics.dart:836` ClampingScrollPhysics | crates/incular-scroll/src/physics.rs:103 | scroll | not checked: pending audit |
| `widgets/scroll_physics.dart:969` NeverScrollableScrollPhysics | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_physics.dart:567` RangeMaintainingScrollPhysics | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_physics.dart:33` ScrollDecelerationRate | crates/incular-scroll/src/physics.rs:125 | scroll | not checked: pending audit |
| `widgets/scroll_physics.dart:88` ScrollPhysics | crates/incular-scroll/src/physics.rs:189 | scroll | not checked: pending audit |
| `widgets/scroll_position_with_single_context.dart:46` ScrollPositionWithSingleContext | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_position.dart:1157` ScrollMetricsNotification | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_position.dart:189` ScrollPosition | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_position.dart:38` ScrollPositionAlignmentPolicy | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_simulation.dart:18` BouncingScrollSimulation | UNRESOLVED | physics | partial scenarios and reading: physics; see package coverage and open cases |
| `widgets/scroll_simulation.dart:164` ClampingScrollSimulation | UNRESOLVED | physics | partial scenarios and reading: physics; see package coverage and open cases |
| `widgets/scroll_view.dart:840` BoxScrollView | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scroll_view.dart:696` CustomScrollView | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:381 | scroll | not checked: pending audit |
| `widgets/scroll_view.dart:1912` GridView | crates/incular-widgets/src/scrolling/scroll_views.rs:742 | scroll | not checked: pending audit |
| `widgets/scroll_view.dart:1256` ListView | crates/incular-widgets/src/scrolling/scroll_views.rs:362 | scroll | not checked: pending audit |
| `widgets/scroll_view.dart:95` ScrollView | crates/incular-widgets/src/tree/descriptors.rs:1857 | scroll | not checked: pending audit |
| `widgets/scroll_view.dart:40` ScrollViewKeyboardDismissBehavior | crates/incular-scroll/src/activity.rs:375 | scroll | not checked: pending audit |
| `widgets/scrollable_helpers.dart:156` EdgeDraggingAutoScroller | UNRESOLVED | gestures | not checked: pending audit |
| `widgets/scrollable_helpers.dart:36` ScrollableDetails | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scrollable_helpers.dart:401` ScrollAction | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scrollable_helpers.dart:358` ScrollIncrementDetails | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scrollable_helpers.dart:335` ScrollIncrementType | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scrollable_helpers.dart:377` ScrollIntent | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scrollable.dart:1795` DiagonalDragBehavior | crates/incular-widgets/src/advanced_scrolling/two_dimensional.rs:117 | gestures | not checked: pending audit |
| `widgets/scrollable.dart:121` Scrollable | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:563 | scroll | partial reading: scroll-004 and semantics-003; other behavior unchecked |
| `widgets/scrollable.dart:560` ScrollableState | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scrollable.dart:1842` TwoDimensionalScrollable | crates/incular-widgets/src/advanced_scrolling/two_dimensional.rs:140 | scroll | not checked: pending audit |
| `widgets/scrollable.dart:2004` TwoDimensionalScrollableState | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scrollbar.dart:980` RawScrollbar | crates/incular-widgets/src/advanced_scrolling/raw_scrollbar.rs:344 | scroll | not checked: pending audit |
| `widgets/scrollbar.dart:1353` RawScrollbarState | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scrollbar.dart:47` ScrollbarOrientation | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/scrollbar.dart:92` ScrollbarPainter | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/selectable_region.dart:2358` MultiSelectableSelectionContainerDelegate | UNRESOLVED | text | not checked: pending audit |
| `widgets/selectable_region.dart:236` SelectableRegion | crates/incular-widgets/src/radio_selection.rs:84 | layout | not checked: pending audit |
| `widgets/selectable_region.dart:3388` SelectableRegionSelectionStatus | UNRESOLVED | text | not checked: pending audit |
| `widgets/selectable_region.dart:3452` SelectableRegionSelectionStatusScope | UNRESOLVED | text | not checked: pending audit |
| `widgets/selectable_region.dart:345` SelectableRegionState | UNRESOLVED | layout | not checked: pending audit |
| `widgets/selectable_region.dart:3605` SelectionDetails | crates/incular-widgets/src/selection.rs:112 | text | not checked: pending audit |
| `widgets/selectable_region.dart:3514` SelectionListener | crates/incular-widgets/src/selection_listener.rs:9 | focus_input | not checked: pending audit |
| `widgets/selectable_region.dart:3619` SelectionListenerNotifier | crates/incular-widgets/src/selection.rs:432 | focus_input | not checked: pending audit |
| `widgets/selectable_region.dart:2089` StaticSelectionContainerDelegate | UNRESOLVED | text | not checked: pending audit |
| `widgets/selection_container.dart:44` SelectionContainer | crates/incular-widgets/src/selection_container.rs:11 | text | not checked: pending audit |
| `widgets/selection_container.dart:272` SelectionContainerDelegate | crates/incular-widgets/src/selection.rs:340 | text | not checked: pending audit |
| `widgets/selection_container.dart:247` SelectionRegistrarScope | UNRESOLVED | text | not checked: pending audit |
| `widgets/semantics_debugger.dart:22` SemanticsDebugger | crates/incular-widgets/src/semantics_debugger.rs:16 | semantics | not checked: pending audit |
| `widgets/sensitive_content.dart:331` SensitiveContent | crates/incular-widgets/src/environment.rs:665 | misc | not checked: pending audit |
| `widgets/sensitive_content.dart:126` SensitiveContentHost | crates/incular-widgets/src/environment.rs:523 | misc | not checked: pending audit |
| `widgets/service_extensions.dart:126` WidgetInspectorServiceExtensions | UNRESOLVED | misc | not checked: pending audit |
| `widgets/service_extensions.dart:24` WidgetsServiceExtensions | UNRESOLVED | misc | not checked: pending audit |
| `widgets/shared_app_data.dart:72` SharedAppData | crates/incular-widgets/src/inherited_data.rs:113 | misc | not checked: pending audit |
| `widgets/shortcuts.dart:1185` CallbackShortcuts | crates/incular-widgets/src/focus_keyboard.rs:377 | focus_input | not checked: pending audit |
| `widgets/shortcuts.dart:685` CharacterActivator | crates/incular-widgets/src/shortcut_registry.rs:18 | focus_input | not checked: pending audit |
| `widgets/shortcuts.dart:51` KeySet | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/shortcuts.dart:154` LockState | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/shortcuts.dart:291` LogicalKeySet | crates/incular-gestures/src/keyboard.rs:837 | focus_input | not checked: pending audit |
| `widgets/shortcuts.dart:191` ShortcutActivator | crates/incular-gestures/src/keyboard.rs:754 | focus_input | not checked: pending audit |
| `widgets/shortcuts.dart:827` ShortcutManager | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/shortcuts.dart:383` ShortcutMapProperty | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/shortcuts.dart:1507` ShortcutRegistrar | crates/incular-widgets/src/shortcut_registry.rs:282 | focus_input | not checked: pending audit |
| `widgets/shortcuts.dart:1291` ShortcutRegistry | crates/incular-widgets/src/shortcut_registry.rs:132 | focus_input | not checked: pending audit |
| `widgets/shortcuts.dart:1247` ShortcutRegistryEntry | crates/incular-widgets/src/shortcut_registry.rs:251 | focus_input | not checked: pending audit |
| `widgets/shortcuts.dart:1007` Shortcuts | crates/incular-gestures/src/keyboard.rs:990 | focus_input | not checked: pending audit |
| `widgets/shortcuts.dart:436` SingleActivator | crates/incular-gestures/src/keyboard.rs:701 | focus_input | not checked: pending audit |
| `widgets/single_child_scroll_view.dart:147` SingleChildScrollView | crates/incular-widgets/src/scrolling/scroll_views.rs:6 | scroll | not checked: pending audit |
| `widgets/size_changed_layout_notifier.dart:34` SizeChangedLayoutNotification | crates/incular-widgets/src/size_changed.rs:19 | layout | not checked: pending audit |
| `widgets/size_changed_layout_notifier.dart:58` SizeChangedLayoutNotifier | crates/incular-widgets/src/size_changed.rs:25 | layout | not checked: pending audit |
| `widgets/sliver_fill.dart:258` SliverFillRemaining | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:969 | slivers | not checked: pending audit |
| `widgets/sliver_fill.dart:35` SliverFillViewport | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1019 | slivers | not checked: pending audit |
| `widgets/sliver_floating_header.dart:31` FloatingHeaderSnapMode | UNRESOLVED | slivers | not checked: pending audit |
| `widgets/sliver_floating_header.dart:64` SliverFloatingHeader | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1828 | slivers | partial scenario and reading: slivers-002; other behavior unchecked |
| `widgets/sliver_layout_builder.dart:27` SliverLayoutBuilder | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1136 | slivers | not checked: pending audit |
| `widgets/sliver_persistent_header.dart:125` SliverPersistentHeader | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:239 | slivers | not checked: pending audit |
| `widgets/sliver_persistent_header.dart:20` SliverPersistentHeaderDelegate | UNRESOLVED | slivers | not checked: pending audit |
| `widgets/sliver_prototype_extent_list.dart:42` SliverPrototypeExtentList | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:904 | slivers | not checked: pending audit |
| `widgets/sliver_resizing_header.dart:54` SliverResizingHeader | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1935 | slivers | not checked: pending audit |
| `widgets/sliver_tree.dart:398` TreeSliver | crates/incular-widgets/src/advanced_slivers/tree_sliver.rs:520 | slivers | not checked: pending audit |
| `widgets/sliver_tree.dart:171` TreeSliverController | crates/incular-widgets/src/advanced_slivers/tree_sliver.rs:376 | slivers | not checked: pending audit |
| `widgets/sliver_tree.dart:31` TreeSliverNode | crates/incular-widgets/src/advanced_slivers/tree_sliver.rs:15 | slivers | not checked: pending audit |
| `widgets/sliver_tree.dart:118` TreeSliverStateMixin | UNRESOLVED | slivers | not checked: pending audit |
| `widgets/sliver.dart:1577` KeepAlive | crates/incular-widgets/src/advanced_slivers/keep_alive.rs:242 | slivers | not checked: pending audit |
| `widgets/sliver.dart:1646` SliverConstrainedCrossAxis | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1434 | slivers | not checked: pending audit |
| `widgets/sliver.dart:1720` SliverCrossAxisExpanded | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1392 | slivers | not checked: pending audit |
| `widgets/sliver.dart:1798` SliverCrossAxisGroup | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1232 | slivers | partial scenario: slivers__cross_axis_group__lanes; other behavior unchecked |
| `widgets/sliver.dart:1912` SliverEnsureSemantics | crates/incular-widgets/src/scrolling/sliver_semantics.rs:158 | slivers | not checked: pending audit |
| `widgets/sliver.dart:435` SliverFixedExtentList | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:791 | slivers | not checked: pending audit |
| `widgets/sliver.dart:733` SliverGrid | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:134 | slivers | not checked: pending audit |
| `widgets/sliver.dart:1414` SliverIgnorePointer | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1642 | slivers | not checked: pending audit |
| `widgets/sliver.dart:167` SliverList | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:79 | slivers | not checked: pending audit |
| `widgets/sliver.dart:1853` SliverMainAxisGroup | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1180 | slivers | partial scenario: slivers__pinned_header_and_main_axis_group__scroll; other behavior unchecked |
| `widgets/sliver.dart:923` SliverMultiBoxAdaptorElement | UNRESOLVED | slivers | not checked: pending audit |
| `widgets/sliver.dart:56` SliverMultiBoxAdaptorWidget | UNRESOLVED | slivers | not checked: pending audit |
| `widgets/sliver.dart:1482` SliverOffstage | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1595 | slivers | not checked: pending audit |
| `widgets/sliver.dart:1337` SliverOpacity | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1553 | slivers | not checked: pending audit |
| `widgets/sliver.dart:597` SliverVariedExtentList | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:843 | slivers | scenario and reading: slivers-001 |
| `widgets/sliver.dart:41` SliverWithKeepAliveWidget | UNRESOLVED | slivers | not checked: pending audit |
| `widgets/slotted_render_object_widget.dart:129` SlottedContainerRenderObjectMixin | UNRESOLVED | misc | not checked: pending audit |
| `widgets/slotted_render_object_widget.dart:52` SlottedMultiChildRenderObjectWidget | UNRESOLVED | misc | not checked: pending audit |
| `widgets/slotted_render_object_widget.dart:70` SlottedMultiChildRenderObjectWidgetMixin | UNRESOLVED | misc | not checked: pending audit |
| `widgets/slotted_render_object_widget.dart:232` SlottedRenderObjectElement | UNRESOLVED | misc | not checked: pending audit |
| `widgets/snapshot_widget.dart:50` SnapshotController | UNRESOLVED | misc | not checked: pending audit |
| `widgets/snapshot_widget.dart:19` SnapshotMode | UNRESOLVED | misc | not checked: pending audit |
| `widgets/snapshot_widget.dart:396` SnapshotPainter | UNRESOLVED | paint | not checked: pending audit |
| `widgets/snapshot_widget.dart:104` SnapshotWidget | crates/incular-widgets/src/painting_effects/layers.rs:14 | misc | not checked: pending audit |
| `widgets/spacer.dart:41` Spacer | crates/incular-widgets/src/layout/flex.rs:372 | misc | not checked: pending audit |
| `widgets/spell_check.dart:206` buildTextSpanWithSpellCheckSuggestions | UNRESOLVED | text | not checked: pending audit |
| `widgets/spell_check.dart:23` SpellCheckConfiguration | UNRESOLVED | misc | not checked: pending audit |
| `widgets/standard_component_type.dart:38` StandardComponentType | UNRESOLVED | misc | not checked: pending audit |
| `widgets/status_transitions.dart:9` StatusTransitionWidget | UNRESOLVED | transitions | not checked: pending audit |
| `widgets/stretch_effect.dart:20` StretchEffect | crates/incular-widgets/src/overscroll_indicator.rs:28 | misc | not checked: pending audit |
| `widgets/system_context_menu.dart:240` IOSSystemContextMenuItem | UNRESOLVED | text | not checked: pending audit |
| `widgets/system_context_menu.dart:280` IOSSystemContextMenuItemCopy | UNRESOLVED | text | not checked: pending audit |
| `widgets/system_context_menu.dart:510` IOSSystemContextMenuItemCustom | UNRESOLVED | text | not checked: pending audit |
| `widgets/system_context_menu.dart:303` IOSSystemContextMenuItemCut | UNRESOLVED | text | not checked: pending audit |
| `widgets/system_context_menu.dart:480` IOSSystemContextMenuItemLiveText | UNRESOLVED | text | not checked: pending audit |
| `widgets/system_context_menu.dart:376` IOSSystemContextMenuItemLookUp | UNRESOLVED | text | not checked: pending audit |
| `widgets/system_context_menu.dart:326` IOSSystemContextMenuItemPaste | UNRESOLVED | text | not checked: pending audit |
| `widgets/system_context_menu.dart:412` IOSSystemContextMenuItemSearchWeb | UNRESOLVED | text | not checked: pending audit |
| `widgets/system_context_menu.dart:349` IOSSystemContextMenuItemSelectAll | UNRESOLVED | text | not checked: pending audit |
| `widgets/system_context_menu.dart:450` IOSSystemContextMenuItemShare | UNRESOLVED | text | not checked: pending audit |
| `widgets/system_context_menu.dart:57` SystemContextMenu | UNRESOLVED | text | not checked: pending audit |
| `widgets/table.dart:118` Table | crates/incular-layout/src/descriptors.rs:242 | layout | not checked: pending audit |
| `widgets/table.dart:439` TableCell | crates/incular-widgets/src/layout/basic/utility.rs:37 | layout | not checked: pending audit |
| `widgets/table.dart:38` TableRow | UNRESOLVED | layout | not checked: pending audit |
| `widgets/tap_region.dart:558` RenderTapRegion | UNRESOLVED | misc | not checked: pending audit |
| `widgets/tap_region.dart:197` RenderTapRegionSurface | UNRESOLVED | misc | not checked: pending audit |
| `widgets/tap_region.dart:370` TapRegion | crates/incular-widgets/src/raw_input.rs:734 | misc | not checked: pending audit |
| `widgets/tap_region.dart:58` TapRegionRegistry | UNRESOLVED | misc | not checked: pending audit |
| `widgets/tap_region.dart:136` TapRegionSurface | crates/incular-widgets/src/raw_input.rs:835 | misc | not checked: pending audit |
| `widgets/tap_region.dart:745` TextFieldTapRegion | crates/incular-widgets/src/raw_input.rs:860 | text | not checked: pending audit |
| `widgets/text_editing_intents.dart:290` CopySelectionTextIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:48` DeleteCharacterIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:62` DeleteToLineBreakIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:55` DeleteToNextWordBoundaryIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:69` DirectionalCaretMovementIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:27` DirectionalTextEditingIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:20` DoNothingAndStopPropagationTextIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:400` EditableTextTapOutsideIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:436` EditableTextTapUpOutsideIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:157` ExpandSelectionToDocumentBoundaryIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:174` ExpandSelectionToLineBreakIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:112` ExtendSelectionByCharacterIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:272` ExtendSelectionByPageIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:255` ExtendSelectionToDocumentBoundaryIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:187` ExtendSelectionToLineBreakIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:223` ExtendSelectionToNextParagraphBoundaryIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:241` ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:121` ExtendSelectionToNextWordBoundaryIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:140` ExtendSelectionToNextWordBoundaryOrCaretLocationIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:201` ExtendSelectionVerticallyToAdjacentLineIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:212` ExtendSelectionVerticallyToAdjacentPageIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:313` PasteTextIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:323` RedoTextIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:333` ReplaceTextIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:265` ScrollToDocumentBoundaryIntent | UNRESOLVED | scroll | not checked: pending audit |
| `widgets/text_editing_intents.dart:278` SelectAllTextIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:383` TransposeCharactersIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:357` UndoTextIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_editing_intents.dart:367` UpdateSelectionIntent | UNRESOLVED | focus_input | not checked: pending audit |
| `widgets/text_selection_toolbar_anchors.dart:21` TextSelectionToolbarAnchors | crates/incular-widgets/src/context_menu.rs:136 | text | not checked: pending audit |
| `widgets/text_selection_toolbar_layout_delegate.dart:26` TextSelectionToolbarLayoutDelegate | UNRESOLVED | text | not checked: pending audit |
| `widgets/text_selection.dart:3849` ClipboardStatus | UNRESOLVED | text | not checked: pending audit |
| `widgets/text_selection.dart:3765` ClipboardStatusNotifier | UNRESOLVED | text | not checked: pending audit |
| `widgets/text_selection.dart:282,322` EmptyTextSelectionControls | UNRESOLVED | text | not checked: pending audit |
| `widgets/text_selection.dart:3960` LiveTextInputStatus | UNRESOLVED | text | not checked: pending audit |
| `widgets/text_selection.dart:3871` LiveTextInputStatusNotifier | UNRESOLVED | text | not checked: pending audit |
| `widgets/text_selection.dart:1047` SelectionOverlay | UNRESOLVED | navigation | not checked: pending audit |
| `widgets/text_selection.dart:97` TextSelectionControls | UNRESOLVED | text | not checked: pending audit |
| `widgets/text_selection.dart:3413` TextSelectionGestureDetector | UNRESOLVED | gestures | not checked: pending audit |
| `widgets/text_selection.dart:2192` TextSelectionGestureDetectorBuilder | UNRESOLVED | gestures | not checked: pending audit |
| `widgets/text_selection.dart:2156` TextSelectionGestureDetectorBuilderDelegate | UNRESOLVED | gestures | not checked: pending audit |
| `widgets/text_selection.dart:3978` TextSelectionHandleControls | UNRESOLVED | text | not checked: pending audit |
| `widgets/text_selection.dart:330` TextSelectionOverlay | UNRESOLVED | navigation | not checked: pending audit |
| `widgets/text_selection.dart:56` ToolbarItemsParentData | UNRESOLVED | text | not checked: pending audit |
| `widgets/text.dart:276` DefaultTextHeightBehavior | crates/incular-widgets/src/environment.rs:260 | text | not checked: pending audit |
| `widgets/text.dart:55` DefaultTextStyle | crates/incular-widgets/src/environment.rs:230 | text | partial reading and diagnostic: text-004; other behavior unchecked |
| `widgets/text.dart:497` Text | crates/incular-material/src/catalog/data_table.rs:49 | text | not checked: pending audit |
| `widgets/texture.dart:35` Texture | UNRESOLVED | text | not checked: pending audit |
| `widgets/ticker_provider.dart:317` SingleTickerProviderStateMixin | UNRESOLVED | misc | not checked: pending audit |
| `widgets/ticker_provider.dart:25` TickerMode | crates/incular-widgets/src/environment.rs:469 | anim_core | partial reading: anim_core-005 |
| `widgets/ticker_provider.dart:607` TickerModeData | UNRESOLVED | misc | not checked: pending audit |
| `widgets/ticker_provider.dart:442` TickerProviderStateMixin | UNRESOLVED | misc | not checked: pending audit |
| `widgets/title.dart:12` Title | crates/incular-controls/src/popup.rs:218 | misc | not checked: pending audit |
| `widgets/toggleable.dart:409` ToggleablePainter | UNRESOLVED | paint | not checked: pending audit |
| `widgets/toggleable.dart:37` ToggleableStateMixin | UNRESOLVED | misc | not checked: pending audit |
| `widgets/transitions.dart:939` AlignTransition | crates/incular-widgets/src/animation_transitions.rs:137 | transitions | partial scenarios and reading: transitions; see package coverage and open cases |
| `widgets/transitions.dart:1199` AnimatedBuilder | crates/incular-widgets/src/explicit_animations.rs:127 | transitions | partial scenarios and reading: transitions; see package coverage and open cases |
| `widgets/transitions.dart:80` AnimatedWidget | UNRESOLVED | transitions | partial scenarios and reading: transitions; see package coverage and open cases |
| `widgets/transitions.dart:878` DecoratedBoxTransition | crates/incular-widgets/src/animation_transitions.rs:333 | transitions | partial scenarios and reading: transitions; see package coverage and open cases |
| `widgets/transitions.dart:995` DefaultTextStyleTransition | crates/incular-widgets/src/animation_transitions.rs:375 | transitions | partial scenarios and reading: transitions; see package coverage and open cases |
| `widgets/transitions.dart:565` FadeTransition | crates/incular-widgets/src/tree/descriptors.rs:1330 | transitions | partial scenarios and reading: transitions; see package coverage and open cases |
| `widgets/transitions.dart:1108` ListenableBuilder | crates/incular-widgets/src/explicit_animations.rs:225 | transitions | partial scenarios and reading: transitions; see package coverage and open cases |
| `widgets/transitions.dart:272` MatrixTransition | crates/incular-widgets/src/animation_transitions.rs:402 | transitions | partial scenarios and reading: transitions; see package coverage and open cases |
| `widgets/transitions.dart:763` PositionedTransition | crates/incular-widgets/src/animation_transitions.rs:261 | transitions | partial scenarios and reading: transitions; see package coverage and open cases |
| `widgets/transitions.dart:814` RelativePositionedTransition | crates/incular-widgets/src/animation_transitions.rs:294 | transitions | partial scenarios and reading: transitions; see package coverage and open cases |
| `widgets/transitions.dart:720` RelativeRectTween | UNRESOLVED | transitions | partial scenarios and reading: transitions; see package coverage and open cases |
| `widgets/transitions.dart:392` RotationTransition | crates/incular-widgets/src/tree/descriptors.rs:1486 | transitions | partial scenarios and reading: transitions; see package coverage and open cases |
| `widgets/transitions.dart:351` ScaleTransition | crates/incular-widgets/src/tree/descriptors.rs:1464 | transitions | partial scenarios and reading: transitions; see package coverage and open cases |
| `widgets/transitions.dart:446` SizeTransition | crates/incular-widgets/src/animation_transitions.rs:188 | transitions | partial scenarios and reading: transitions; see package coverage and open cases |
| `widgets/transitions.dart:191` SlideTransition | crates/incular-widgets/src/tree/descriptors.rs:1352 | transitions | partial scenarios and reading: transitions; see package coverage and open cases |
| `widgets/transitions.dart:658` SliverFadeTransition | crates/incular-widgets/src/scrolling/sliver_fade.rs:16 | transitions | partial scenarios and reading: transitions; see package coverage and open cases |
| `widgets/tween_animation_builder.dart:105` TweenAnimationBuilder | crates/incular-widgets/src/implicit_animations.rs:1292 | implicit | not checked: pending audit |
| `widgets/two_dimensional_scroll_view.dart:50` TwoDimensionalScrollView | crates/incular-widgets/src/advanced_scrolling/two_dimensional.rs:1031 | scroll | not checked: pending audit |
| `widgets/two_dimensional_viewport.dart:1794` ChildVicinity | crates/incular-widgets/src/advanced_scrolling/two_dimensional.rs:21 | slivers | not checked: pending audit |
| `widgets/two_dimensional_viewport.dart:516` RenderTwoDimensionalViewport | UNRESOLVED | slivers | not checked: pending audit |
| `widgets/two_dimensional_viewport.dart:1777` TwoDimensionalChildManager | UNRESOLVED | slivers | not checked: pending audit |
| `widgets/two_dimensional_viewport.dart:140` TwoDimensionalViewport | crates/incular-widgets/src/advanced_scrolling/two_dimensional.rs:434 | slivers | not checked: pending audit |
| `widgets/two_dimensional_viewport.dart:412` TwoDimensionalViewportParentData | UNRESOLVED | slivers | not checked: pending audit |
| `widgets/undo_history.dart:33` UndoHistory | crates/incular-widgets/src/forms.rs:622 | misc | not checked: pending audit |
| `widgets/undo_history.dart:361` UndoHistoryController | crates/incular-runtime/src/undo.rs:236 | misc | not checked: pending audit |
| `widgets/undo_history.dart:96` UndoHistoryState | crates/incular-runtime/src/undo.rs:38 | misc | not checked: pending audit |
| `widgets/undo_history.dart:308` UndoHistoryValue | UNRESOLVED | misc | not checked: pending audit |
| `widgets/unique_widget.dart:20` UniqueWidget | UNRESOLVED | misc | not checked: pending audit |
| `widgets/value_listenable_builder.dart:71` ValueListenableBuilder | UNRESOLVED | transitions | not checked: pending audit |
| `widgets/view.dart:329` RawView | UNRESOLVED | misc | not checked: pending audit |
| `widgets/view.dart:75` View | crates/incular-widgets/src/app_shell/view.rs:439 | misc | not checked: pending audit |
| `widgets/view.dart:702` ViewAnchor | crates/incular-widgets/src/app_shell/view.rs:887 | misc | not checked: pending audit |
| `widgets/view.dart:665` ViewCollection | UNRESOLVED | misc | not checked: pending audit |
| `widgets/viewport.dart:357` ShrinkWrappingViewport | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:700 | slivers | not checked: pending audit |
| `widgets/viewport.dart:55` Viewport | crates/incular-controls/src/alert_dialog.rs:93 | slivers | not checked: pending audit |
| `widgets/visibility.dart:356` SliverVisibility | crates/incular-widgets/src/scrolling/sliver_descriptors.rs:1725 | slivers | not checked: pending audit |
| `widgets/visibility.dart:50` Visibility | crates/incular-layout/src/descriptors.rs:513 | layout | not checked: pending audit |
| `widgets/widget_inspector.dart:4263` debugIsLocalCreationLocation | UNRESOLVED | misc | not checked: pending audit |
| `widgets/widget_inspector.dart:4279` debugIsWidgetLocalCreation | UNRESOLVED | misc | not checked: pending audit |
| `widgets/widget_inspector.dart:4115` debugTransformDebugCreator | UNRESOLVED | layout | not checked: pending audit |
| `widgets/widget_inspector.dart:4245` DevToolsDeepLinkProperty | UNRESOLVED | misc | not checked: pending audit |
| `widgets/widget_inspector.dart:3128` DisableWidgetInspectorScope | UNRESOLVED | misc | not checked: pending audit |
| `widgets/widget_inspector.dart:3100` EnableWidgetInspectorScope | UNRESOLVED | misc | not checked: pending audit |
| `widgets/widget_inspector.dart:3165` InspectorButton | UNRESOLVED | misc | not checked: pending audit |
| `widgets/widget_inspector.dart:3146` InspectorButtonVariant | UNRESOLVED | misc | not checked: pending audit |
| `widgets/widget_inspector.dart:724` InspectorReferenceData | UNRESOLVED | misc | not checked: pending audit |
| `widgets/widget_inspector.dart:3270` InspectorSelection | UNRESOLVED | text | not checked: pending audit |
| `widgets/widget_inspector.dart:4358` InspectorSerializationDelegate | UNRESOLVED | misc | not checked: pending audit |
| `widgets/widget_inspector.dart:4565` WeakMap | UNRESOLVED | misc | not checked: pending audit |
| `widgets/widget_inspector.dart:4561` widgetFactory | UNRESOLVED | misc | not checked: pending audit |
| `widgets/widget_inspector.dart:2838` WidgetInspector | UNRESOLVED | misc | not checked: pending audit |
| `widgets/widget_inspector.dart:778` WidgetInspectorService | UNRESOLVED | misc | not checked: pending audit |
| `widgets/widget_span.dart:71` WidgetSpan | crates/incular-text/src/spans.rs:13 | text | partial scenario and reading: text-003; other behavior unchecked |
| `widgets/widget_state.dart:180` WidgetState | crates/incular-material/src/theme/state.rs:8 | misc | not checked: pending audit |
| `widgets/widget_state.dart:513` WidgetStateBorderSide | UNRESOLVED | misc | not checked: pending audit |
| `widgets/widget_state.dart:286` WidgetStateColor | UNRESOLVED | misc | not checked: pending audit |
| `widgets/widget_state.dart:1012` WidgetStateMapper | UNRESOLVED | misc | not checked: pending audit |
| `widgets/widget_state.dart:374` WidgetStateMouseCursor | UNRESOLVED | misc | not checked: pending audit |
| `widgets/widget_state.dart:674` WidgetStateOutlinedBorder | UNRESOLVED | misc | not checked: pending audit |
| `widgets/widget_state.dart:833` WidgetStateProperty | crates/incular-material/src/theme/state.rs:290 | misc | not checked: pending audit |
| `widgets/widget_state.dart:1089` WidgetStatePropertyAll | crates/incular-material/src/theme/state.rs:293 | misc | not checked: pending audit |
| `widgets/widget_state.dart:27` WidgetStatesConstraint | UNRESOLVED | misc | not checked: pending audit |
| `widgets/widget_state.dart:1152` WidgetStatesController | crates/incular-material/src/theme/state.rs:307 | misc | not checked: pending audit |
| `widgets/widget_state.dart:742` WidgetStateTextStyle | UNRESOLVED | text | not checked: pending audit |
| `widgets/will_pop_scope.dart:25` WillPopScope | UNRESOLVED | navigation | not checked: pending audit |
