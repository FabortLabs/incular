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

class _ToolbarEditor extends EditableText {
  _ToolbarEditor({
    required super.key,
    required super.controller,
    required super.focusNode,
    required super.onSelectionChanged,
  }) : super(
         autofocus: true,
         rendererIgnoresPointer: true,
         style: const TextStyle(fontSize: 14),
         cursorColor: const Color(0xffffffff),
         backgroundCursorColor: const Color(0xff333333),
       );

  @override
  EditableTextState createState() => _ToolbarEditorState();
}

class _ToolbarEditorState extends EditableTextState {
  bool toolbarVisible = false;

  @override
  void toggleToolbar([bool hideHandles = true]) {
    assert(!hideHandles);
    toolbarVisible = !toolbarVisible;
  }

  @override
  void hideToolbar([bool hideHandles = true]) {
    toolbarVisible = false;
    super.hideToolbar(hideHandles);
  }
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

  testWidgets('a focused Android editor waits for a completed finger tap', (WidgetTester tester) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.android;
    addTearDown(() => debugDefaultTargetPlatformOverride = null);
    final TextEditingController controller = TextEditingController(text: 'alpha beta gamma');
    final FocusNode focus = FocusNode();
    final _SelectionDelegate delegate = _SelectionDelegate();
    final List<SelectionChangedCause?> causes = [];
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    controller.selection = const TextSelection.collapsed(offset: 1);
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
              onSelectionChanged: (selection, cause) => causes.add(cause),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    causes.clear();
    final point = tester.getTopLeft(find.byType(EditableText)) + const Offset(58, 7);

    final TestGesture canceled = await tester.startGesture(point);
    await tester.pump(const Duration(milliseconds: 100));
    expect(controller.selection.baseOffset, 1);
    expect(causes, isEmpty);
    await canceled.cancel();
    await tester.pump();
    expect(controller.selection.baseOffset, 1);
    expect(causes, isEmpty);
    expect(focus.hasPrimaryFocus, isTrue);

    await tester.pump(const Duration(milliseconds: 600));
    final TestGesture complete = await tester.startGesture(point);
    await tester.pump(const Duration(milliseconds: 100));
    expect(controller.selection.baseOffset, 1);
    expect(causes, isEmpty);
    await complete.up();
    await tester.pump();
    expect(controller.selection.baseOffset, 4);
    expect(causes, [SelectionChangedCause.tap]);
    expect(focus.hasPrimaryFocus, isTrue);
  });

  testWidgets(
    'iOS touch taps preserve a selected range or move to a word edge',
    (WidgetTester tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      addTearDown(() => debugDefaultTargetPlatformOverride = null);
      final TextEditingController controller = TextEditingController(
        text: 'alpha beta gamma',
      );
      final FocusNode focus = FocusNode();
      final _SelectionDelegate delegate = _SelectionDelegate();
      final List<SelectionChangedCause?> causes = [];
      const TextSelection selected = TextSelection(
        baseOffset: 1,
        extentOffset: 4,
      );
      addTearDown(controller.dispose);
      addTearDown(focus.dispose);
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Align(
            alignment: Alignment.topLeft,
            child: TextSelectionGestureDetectorBuilder(delegate: delegate)
                .buildGestureDetector(
                  behavior: HitTestBehavior.translucent,
                  child: _ToolbarEditor(
                    key: delegate.editableTextKey,
                    controller: controller,
                    focusNode: focus,
                    onSelectionChanged: (selection, cause) => causes.add(cause),
                  ),
                ),
          ),
        ),
      );
      final state = delegate.editableTextKey.currentState! as _ToolbarEditorState;
      await tester.pump();
      controller.selection = selected;
      await tester.pump();
      causes.clear();
      final Offset origin = tester.getTopLeft(find.byType(_ToolbarEditor));
      Future<void> tap(double x, int ms) async {
        final TestGesture gesture = await tester.createGesture();
        await gesture.down(
          origin + Offset(x, 7),
          timeStamp: Duration(milliseconds: ms),
        );
        await gesture.up(timeStamp: Duration(milliseconds: ms + 50));
      }

      await tap(30, 100);
      await tester.pump();
      expect(controller.selection, selected);
      expect(state.toolbarVisible, isTrue);
      expect(causes, isEmpty);

      await tester.pump(const Duration(milliseconds: 650));
      await tap(30, 700);
      await tester.pump();
      expect(controller.selection, selected);
      expect(state.toolbarVisible, isFalse);

      controller.selection = const TextSelection.collapsed(offset: 0);
      await tester.pump(const Duration(milliseconds: 650));
      await tap(100, 1400);
      await tester.pump();
      expect(
        controller.selection,
        const TextSelection.collapsed(
          offset: 10,
          affinity: TextAffinity.upstream,
        ),
      );
      expect(causes, [SelectionChangedCause.tap]);
      expect(focus.hasPrimaryFocus, isTrue);
    },
  );
}
