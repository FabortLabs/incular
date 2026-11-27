import 'dart:ui' show PointerDeviceKind;

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _SelectionDelegate implements TextSelectionGestureDetectorBuilderDelegate {
  @override
  final GlobalKey<EditableTextState> editableTextKey = GlobalKey<EditableTextState>();

  @override
  bool get forcePressEnabled => false;

  @override
  bool get selectionEnabled => true;
}

void main() {
  testWidgets('pointer selection distinguishes tap, double tap, drag and long press', (
    WidgetTester tester,
  ) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.linux;
    addTearDown(() => debugDefaultTargetPlatformOverride = null);
    final TextEditingController controller = TextEditingController(text: 'alpha beta gamma');
    final FocusNode focus = FocusNode();
    final _SelectionDelegate delegate = _SelectionDelegate();
    final List<(TextSelection, SelectionChangedCause?)> changes = [];
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    controller.selection = const TextSelection.collapsed(offset: 0);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: TextSelectionGestureDetectorBuilder(delegate: delegate).buildGestureDetector(
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
              onSelectionChanged: (selection, cause) => changes.add((selection, cause)),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    changes.clear();

    final Offset origin = tester.getTopLeft(find.byType(EditableText));
    final Offset word = origin + const Offset(30, 7);
    final TestGesture tap = await tester.startGesture(word);
    await tester.pump(const Duration(milliseconds: 50));
    await tap.up();
    await tester.pump();
    expect(controller.selection.baseOffset, 2);
    expect(changes.last.$2, SelectionChangedCause.tap);

    await tap.down(word);
    await tester.pump(const Duration(milliseconds: 50));
    await tap.up();
    await tester.pump();
    expect((controller.selection.baseOffset, controller.selection.extentOffset), (0, 5));
    expect(changes.last.$2, SelectionChangedCause.doubleTap);

    await tester.pump(const Duration(milliseconds: 600));
    final TestGesture drag = await tester.startGesture(
      origin + const Offset(16, 7),
      kind: PointerDeviceKind.mouse,
    );
    await drag.moveTo(origin + const Offset(48, 7));
    await drag.moveTo(origin + const Offset(114, 7));
    await drag.up();
    await tester.pump();
    expect((controller.selection.baseOffset, controller.selection.extentOffset), (1, 8));
    expect(changes.last.$2, SelectionChangedCause.drag);

    await tester.pump(const Duration(milliseconds: 600));
    await tester.longPressAt(origin + const Offset(100, 7), kind: PointerDeviceKind.touch);
    await tester.pump();
    expect((controller.selection.baseOffset, controller.selection.extentOffset), (6, 10));
    expect(changes.last.$2, SelectionChangedCause.longPress);
  });
}
