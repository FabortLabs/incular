import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Delegate implements TextSelectionGestureDetectorBuilderDelegate {
  @override
  final editableTextKey = GlobalKey<EditableTextState>();
  @override
  bool get forcePressEnabled => false;
  @override
  bool get selectionEnabled => true;
}

class _Builder extends TextSelectionGestureDetectorBuilder {
  _Builder(_Delegate delegateArgument, this.events)
    : super(delegate: delegateArgument);
  final List<String> events;

  @override
  void onTapDown(TapDragDownDetails details) {
    events.add('down:${details.consecutiveTapCount}');
    super.onTapDown(details);
  }

  @override
  void onSingleTapUp(TapDragUpDetails details) {
    events.add('up:${details.consecutiveTapCount}');
    super.onSingleTapUp(details);
  }

  @override
  void onDragSelectionStart(TapDragStartDetails details) {
    events.add('start:${details.consecutiveTapCount}');
    super.onDragSelectionStart(details);
  }
}

void main() {
  for (final bool ignorePointer in [false, true]) {
    testWidgets(
      'editor and outer selection arena: ignorePointer=$ignorePointer',
      (tester) async {
        final TextEditingController controller = TextEditingController(
          text: 'I love flutter!',
        );
        final FocusNode focus = FocusNode();
        addTearDown(controller.dispose);
        addTearDown(focus.dispose);
        final delegate = _Delegate();
        final events = <String>[];
        final builder = _Builder(delegate, events);
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: builder.buildGestureDetector(
              behavior: HitTestBehavior.translucent,
              child: EditableText(
                key: delegate.editableTextKey,
                focusNode: focus,
                controller: controller,
                rendererIgnoresPointer: ignorePointer,
                style: const TextStyle(fontSize: 14),
                cursorColor: const Color(0xffffffff),
                backgroundCursorColor: const Color(0xff222222),
              ),
            ),
          ),
        );
        final RenderEditable render =
            delegate.editableTextKey.currentState!.renderEditable;
        Offset at(int index) => render.localToGlobal(
          render.getLocalRectForCaret(TextPosition(offset: index)).center,
        );
        final first = await tester.startGesture(at(4));
        await tester.pump(const Duration(milliseconds: 50));
        await first.up();
        await tester.pump();
        expect(controller.selection, const TextSelection.collapsed(offset: 4));
        final next = await tester.startGesture(at(4));
        await tester.pump(const Duration(milliseconds: 16));
        await next.moveTo(at(7));
        await tester.pump(const Duration(milliseconds: 16));
        await next.moveTo(at(10));
        await tester.pump(const Duration(milliseconds: 16));
        await next.up();
        await tester.pump();
        expect(
          controller.selection,
          ignorePointer
              ? const TextSelection(baseOffset: 2, extentOffset: 14)
              : const TextSelection.collapsed(offset: 10),
          reason: events.join(', '),
        );
        expect(events, contains(ignorePointer ? 'start:2' : 'start:1'));
      },
    );
  }
}
