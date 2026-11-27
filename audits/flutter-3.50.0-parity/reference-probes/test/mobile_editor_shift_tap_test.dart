import 'dart:ui' show PointerDeviceKind;

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _SelectionDelegate
    implements TextSelectionGestureDetectorBuilderDelegate {
  @override
  final GlobalKey<EditableTextState> editableTextKey =
      GlobalKey<EditableTextState>();

  @override
  bool get forcePressEnabled => false;

  @override
  bool get selectionEnabled => true;
}

class _Editor {
  final controller = TextEditingController(text: 'alpha beta gamma');
  final focus = FocusNode();
  final delegate = _SelectionDelegate();
  final changes = <SelectionChangedCause?>[];
  static const initialSelection = TextSelection(baseOffset: 2, extentOffset: 5);

  Future<Offset> mount(WidgetTester tester, TargetPlatform platform) async {
    debugDefaultTargetPlatformOverride = platform;
    addTearDown(() => debugDefaultTargetPlatformOverride = null);
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    controller.selection = initialSelection;
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: TextSelectionGestureDetectorBuilder(delegate: delegate)
              .buildGestureDetector(
                behavior: HitTestBehavior.translucent,
                child: EditableText(
                  key: delegate.editableTextKey,
                  controller: controller,
                  focusNode: focus,
                  autofocus: true,
                  rendererIgnoresPointer: true,
                  style: const TextStyle(fontSize: 14),
                  cursorColor: const Color(0xffffffff),
                  backgroundCursorColor: const Color(0xff333333),
                  onSelectionChanged: (_, cause) => changes.add(cause),
                ),
              ),
        ),
      ),
    );
    await tester.pump();
    changes.clear();
    return tester.getTopLeft(find.byType(EditableText));
  }
}

void main() {
  for (final (platform, base) in [
    (TargetPlatform.android, 2),
    (TargetPlatform.iOS, 5),
  ]) {
    testWidgets('$platform samples Shift for the confirmed finger tap', (
      tester,
    ) async {
      final editor = _Editor();
      final origin = await editor.mount(tester, platform);
      await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      final finger = await tester.startGesture(origin + const Offset(16, 7));
      await tester.pump(const Duration(milliseconds: 60));
      expect(editor.controller.selection, _Editor.initialSelection);
      expect(editor.changes, isEmpty);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await finger.up();
      await tester.pump();
      expect(
        editor.controller.selection,
        TextSelection(baseOffset: base, extentOffset: 1),
      );
      expect(editor.changes, [SelectionChangedCause.tap]);
      expect(editor.focus.hasPrimaryFocus, isTrue);
    });
  }

  testWidgets(
    'iOS mouse waits for a committed tap rather than a canceled press',
    (tester) async {
      final editor = _Editor();
      final origin = await editor.mount(tester, TargetPlatform.iOS);
      final point = origin + const Offset(58, 7);
      final canceled = await tester.startGesture(
        point,
        kind: PointerDeviceKind.mouse,
      );
      await tester.pump(const Duration(milliseconds: 100));
      expect(editor.controller.selection, _Editor.initialSelection);
      expect(editor.changes, isEmpty);
      await canceled.cancel();
      await tester.pump(const Duration(milliseconds: 600));
      expect(editor.controller.selection, _Editor.initialSelection);
      expect(editor.changes, isEmpty);

      final completed = await tester.startGesture(
        point,
        kind: PointerDeviceKind.mouse,
      );
      await completed.up();
      await tester.pump();
      expect(
        editor.controller.selection,
        const TextSelection.collapsed(offset: 4),
      );
      expect(editor.changes, [SelectionChangedCause.tap]);
    },
  );
}
