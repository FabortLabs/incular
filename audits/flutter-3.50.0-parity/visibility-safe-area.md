# Paired slice: visibility and safe area

Flutter `ab2912035e` (3.50.0) and Incular implementation `c647cb85`, tested
from audit-only HEAD `46116823`. Windows headless tests, not physical-device
verification.

| Case | Flutter | Incular | Disposition |
| --- | --- | --- | --- |
| C07: a visible neutral editor has focus, then its retained Visibility is hidden with only maintainState enabled | Initially focused, then no focus | Initially focused; after hiding, controller received x and changed a to ax | Paired mismatch |
| C08: outer maintained SafeArea plus ordinary inner SafeArea, bottom safe padding and view padding both 20, viewport 200 high | Child height 180 | Child height 160 | Paired mismatch |
| C08 control: neither SafeArea has maintainBottomViewPadding | Child height 180 | Child height 180 | Paired match for this exact case |

For C07, the Flutter assertion covers the active focus node; the Incular
assertion verifies the downstream edit, not just the focus flag. This is the
default retain-state case, not Flutter's intentional Visibility.maintain or
Offstage behavior. Upstream adjacent control: `packages/flutter/test/widgets/visibility_test.dart:462–493`.
Flutter excludes focus when hidden by default
(`widgets/indexed_stack.dart:417–421,451–473`); the Incular retained focus
policy is visible in `crates/incular-widgets/src/layout/basic/visibility.rs:9–41,118–145`
and `tree/focus.rs:27–57`.

For C08, ordinary SafeArea lowering consumes inherited padding. Flutter does
the same even when the outer maintains bottom view padding
(`widgets/safe_area.dart:113–135`); Incular's retained maintained arm does not
publish reduced insets (`crates/incular-widgets/src/safe_area.rs:203–251`).
The older context helper test asserts accumulation for two maintained helpers;
that is a distinct path and not proof of Flutter parity
(`crates/incular-runtime/tests/system_environment.rs:394–415`).

## Reproduction

Run Flutter only in the scratch package created for the paired comparisons,
with the source below as `test/visibility_safe_area_slice_test.dart`.
Use the pinned SDK and `flutter test --no-pub test/visibility_safe_area_slice_test.dart`.
All three Flutter assertions passed. Rust used two temporary test targets with
`cargo test --locked --no-fail-fast -p incular-runtime --test audit_safe_area_slice_tmp --test audit_visibility_slice_tmp slice_`.
Both parity assertions failed; the ordinary SafeArea control ran before its
variant and passed. Temporary targets were removed. The scratch path is
`.tmp/flutter350-full-audit/`.

```dart
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
void main() {
  for (final maintain in [false,true]) {
    testWidgets('Nested SafeArea consumes bottom padding: maintain=$maintain',(t) async{
      const key=ValueKey('probe');
      await t.pumpWidget(Directionality(textDirection:TextDirection.ltr,child:Center(
        child:SizedBox(width:200,height:200,child:MediaQuery(
          data:const MediaQueryData(padding:EdgeInsets.only(bottom:20),viewPadding:EdgeInsets.only(bottom:20)),
          child:SafeArea(maintainBottomViewPadding:maintain,child:const SafeArea(child:SizedBox.expand(key:key))))))));
      expect(t.getSize(find.byKey(key)).height,180);
    });
  }
  testWidgets('Hidden retained editor stops being the active input client',(t) async{
    final focus=FocusNode();
    final c=TextEditingController(text:'a');
    Widget scene(bool visible)=>Directionality(textDirection:TextDirection.ltr,child:Visibility(
      visible:visible,maintainState:true,child:EditableText(controller:c,focusNode:focus,
        style:const TextStyle(fontSize:14),cursorColor:const Color(0xff000000),backgroundCursorColor:const Color(0xff888888))));
    try {
      await t.pumpWidget(scene(true));
      await t.showKeyboard(find.byType(EditableText));
      expect(focus.hasFocus,isTrue);
      await t.pumpWidget(scene(false));
      await t.pump();
      expect(focus.hasFocus,isFalse);
      expect(c.text,'a');
    } finally {
      await t.pumpWidget(const SizedBox());
      focus.dispose();
      c.dispose();
    }
  });
}
```

Append to a temporary copy of `crates/incular-runtime/tests/system_environment.rs`:

```rust
#[test]
fn slice_nested_safe_area_consumes_bottom_padding() {
    fn measure(maintain: bool) -> f32 {
        let inner=SafeArea::new(Widget::box_(Size::new(50.,50.), Color::WHITE));
        let mut rt=Runtime::new(SafeArea::new(inner).maintain_bottom_view_padding(maintain).into()).unwrap();
        let _=rt.set_environment(keyboard_hidden());
        let (list,_)=rt.run_frame(Constraints::tight(Size::new(200.,200.))).unwrap();
        painted_sizes(&list)[0].height
    }
    assert_eq!(measure(false),180.,"ordinary nested SafeArea control");
    assert_eq!(measure(true),180.,"outer maintained inset must be consumed for inner normal SafeArea");
}
```

Append to a temporary copy of `crates/incular-runtime/tests/text_field_touch_focus.rs`:

```rust
#[test]
fn slice_hidden_retained_editor_excludes_focus() {
    use incular_widgets::Visibility;
    use incular_runtime::Signal;
    let controller=TextEditingController::with_text("a");
    let visible=Signal::new(true);
    let s=visible.clone();
    let c=controller.clone();
    let build=move || -> Widget {
        Column::new([Widget::from(Visibility::new(EditableText::new(c.clone())
            .text_direction(TextDirection::Ltr).size(Size::new(400.,40.)))
            .maintain_state(true).visible(s.get())),
            SizedBox::new().width(400.).height(100.).into()]).into()
    };
    let mut runtime=Runtime::new(build()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    let mut now=Instant::now();
    runtime.run_frame_at(bounds(),now).unwrap();
    let root=runtime.tree().root().unwrap();
    runtime.register_builder(root,build).unwrap();
    now+=Duration::from_millis(16);
    runtime.run_frame_at(bounds(),now).unwrap();
    tap(&mut runtime,Offset::new(100.,20.),&mut now);
    assert!(runtime.focused_element().is_some(),"visible control did not focus");
    visible.set(false);
    now+=Duration::from_millis(16);
    runtime.run_frame_at(bounds(),now).unwrap();
    type_text(&mut runtime,now+Duration::from_millis(16));
    assert_eq!(controller.text(),"a","hidden editor should not receive input");
}
```

Remaining adjacent cases: explicit maintainFocusability/offstage/ancestor
traversal, focused route restore, active composition when hiding, maintained
bottom with changing viewInsets, disabled sides/minimums, and nested mixed
policies. These original focused fixtures do not mark upstream registrations
as exhaustively reviewed.
