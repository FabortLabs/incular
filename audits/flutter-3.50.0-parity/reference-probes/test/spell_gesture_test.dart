import 'dart:ui' show PointerDeviceKind;

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _SelectionDelegate
    implements TextSelectionGestureDetectorBuilderDelegate {
  @override
  final GlobalKey<EditableTextState> editableTextKey = GlobalKey();
  @override
  bool get forcePressEnabled => false;
  @override
  bool get selectionEnabled => true;
}

class _Checker implements SpellCheckService {
  @override
  Future<List<SuggestionSpan>?> fetchSpellCheckSuggestions(
    Locale locale,
    String text,
  ) async => null;
}

Future<EditableTextState> mount(
  WidgetTester tester,
  TextEditingController controller,
  FocusNode focus,
) async {
  final selection = _SelectionDelegate();
  final gestures = TextSelectionGestureDetectorBuilder(delegate: selection);
  addTearDown(controller.dispose);
  addTearDown(focus.dispose);
  final configuration = SpellCheckConfiguration(
    spellCheckService: _Checker(),
    misspelledTextStyle: const TextStyle(),
    spellCheckSuggestionsToolbarBuilder: (_, _) => const IgnorePointer(
      child: Align(alignment: Alignment.topRight, child: Text('Suggestions')),
    ),
  );
  await tester.pumpWidget(
    Directionality(
      textDirection: TextDirection.ltr,
      child: MediaQuery(
        data: const MediaQueryData(),
        child: Overlay(
          initialEntries: [
            OverlayEntry(
              builder: (_) => gestures.buildGestureDetector(
                behavior: HitTestBehavior.translucent,
                child: EditableText(
                  key: selection.editableTextKey,
                  controller: controller,
                  focusNode: focus,
                  style: const TextStyle(fontSize: 18),
                  cursorColor: const Color(0xFF000000),
                  backgroundCursorColor: const Color(0xFF000000),
                  locale: const Locale('en'),
                  rendererIgnoresPointer: true,
                  contextMenuBuilder: (_, _) => const IgnorePointer(
                    child: Align(
                      alignment: Alignment.topRight,
                      child: Text('Selection menu'),
                    ),
                  ),
                  spellCheckConfiguration: configuration,
                ),
              ),
            ),
          ],
        ),
      ),
    ),
  );
  return selection.editableTextKey.currentState!;
}

Offset caret(EditableTextState state, int offset) {
  final render = state.renderEditable;
  return render.localToGlobal(
    render.getLocalRectForCaret(TextPosition(offset: offset)).center,
  );
}

void main() {
  testWidgets(
    'iOS touch selects a misspelling, precise taps only place the caret',
    (WidgetTester tester) async {
      final text = TextEditingController(text: 'tset here');
      final focus = FocusNode();
      final state = await mount(tester, text, focus);
      focus.requestFocus();
      await tester.pump();
      text.selection = const TextSelection.collapsed(offset: 7);
      state.spellCheckResults = const SpellCheckResults('tset here', [
        SuggestionSpan(TextRange(start: 0, end: 4), ['test']),
      ]);
      await tester.pump();
      final wrong = caret(state, 2);

      await tester.tapAt(wrong);
      expect(text.selection.baseOffset, 0);
      expect(text.selection.extentOffset, 4);
      expect(state.selectionOverlay!.spellCheckToolbarIsVisible, isTrue);

      await tester.pump(const Duration(milliseconds: 400));
      // Reopen explicitly after any frame's caret-reveal work. This pins the
      // same-selection toggle independently of that asynchronous animation.
      if (!state.selectionOverlay!.spellCheckToolbarIsVisible) {
        expect(state.showSpellCheckSuggestionsToolbar(), isTrue);
      }
      await tester.tapAt(wrong);
      expect(text.selection.baseOffset, 0);
      expect(text.selection.extentOffset, 4);
      expect(state.selectionOverlay!.spellCheckToolbarIsVisible, isFalse);
      expect(state.selectionOverlay!.toolbarIsVisible, isFalse);

      await tester.pump(const Duration(milliseconds: 400));
      final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
      await mouse.down(wrong);
      await mouse.up();
      await tester.pumpAndSettle();
      expect(text.selection.isCollapsed, isTrue);
      expect(text.selection.extentOffset, inInclusiveRange(1, 3));
      expect(find.text('Suggestions'), findsNothing);
    },
    variant: const TargetPlatformVariant({TargetPlatform.iOS}),
  );

  testWidgets('an unfocused Android drag does not focus an editor', (
    WidgetTester tester,
  ) async {
    final text = TextEditingController(text: 'one two three four five');
    final focus = FocusNode();
    final state = await mount(tester, text, focus);
    final from = caret(state, 1);
    final to = caret(state, 15);
    await tester.dragFrom(from, to - from);
    await tester.pumpAndSettle();
    expect(focus.hasFocus, isFalse);
    expect(text.selection.isCollapsed, isTrue);

    await tester.tapAt(from);
    await tester.pumpAndSettle();
    expect(focus.hasFocus, isTrue);
    await tester.pump(const Duration(milliseconds: 500));
    final gesture = await tester.startGesture(from);
    await gesture.moveBy((to - from) / 2);
    await tester.pump(const Duration(milliseconds: 30));
    await gesture.moveTo(to);
    await tester.pump(const Duration(milliseconds: 30));
    await gesture.up();
    await tester.pumpAndSettle();
    expect(text.selection.isCollapsed, isTrue);
    expect(text.selection.extentOffset, 15);
  }, variant: const TargetPlatformVariant({TargetPlatform.android}));
}
