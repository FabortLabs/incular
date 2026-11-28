import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Delegate implements TextSelectionGestureDetectorBuilderDelegate {
  _Delegate(this.forcePressEnabled);
  @override
  final editableTextKey = GlobalKey<EditableTextState>();
  @override
  final bool forcePressEnabled;
  @override
  bool get selectionEnabled => true;
}

void main() {
  for (final bool enabled in [true, false]) {
    testWidgets(
      'editor force press uses the last pressure coordinate: enabled=$enabled',
      (tester) async {
        final delegate = _Delegate(enabled);
        final builder = TextSelectionGestureDetectorBuilder(delegate: delegate);
        final controller = TextEditingController(text: 'alpha beta gamma');
        final focus = FocusNode();
        final changes = <(TextSelection, SelectionChangedCause?)>[];
        addTearDown(controller.dispose);
        addTearDown(focus.dispose);
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Overlay(
              initialEntries: <OverlayEntry>[
                OverlayEntry(
                  builder: (_) => Align(
                    alignment: Alignment.topLeft,
                    child: builder.buildGestureDetector(
                      behavior: HitTestBehavior.translucent,
                      child: EditableText(
                        key: delegate.editableTextKey,
                        rendererIgnoresPointer: true,
                        focusNode: focus,
                        controller: controller,
                        style: const TextStyle(fontSize: 14),
                        cursorColor: const Color(0xffffffff),
                        backgroundCursorColor: const Color(0xff222222),
                        onSelectionChanged: (selection, cause) =>
                            changes.add((selection, cause)),
                        contextMenuBuilder: (_, _) => const SizedBox(
                          key: ValueKey<String>('force menu'),
                          width: 12,
                          height: 12,
                        ),
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
        );
        final RenderEditable render =
            delegate.editableTextKey.currentState!.renderEditable;
        Offset at(int index) => render.localToGlobal(
          render.getLocalRectForCaret(TextPosition(offset: index)).center,
        );
        final point = at(2);
        final pointer = tester.nextPointer;
        final gesture = await tester.createGesture(pointer: pointer);
        await gesture.downWithCustomEvent(
          point,
          PointerDownEvent(
            pointer: pointer,
            position: point,
            pressure: 3,
            pressureMin: 0,
            pressureMax: 6,
          ),
        );
        await tester.pump();
        if (enabled) {
          expect(
            controller.selection,
            const TextSelection(baseOffset: 0, extentOffset: 5),
          );
          expect(
            find.byKey(const ValueKey<String>('force menu')),
            findsOneWidget,
          );
          expect(changes.single.$2, SelectionChangedCause.forcePress);
          final beta = at(8);
          await gesture.updateWithCustomEvent(
            PointerMoveEvent(
              pointer: pointer,
              position: beta,
              delta: beta - point,
              pressure: 3.4,
              pressureMin: 0,
              pressureMax: 6,
            ),
          );
          await tester.pump();
          expect(
            controller.selection,
            const TextSelection(baseOffset: 0, extentOffset: 5),
          );
          await gesture.updateWithCustomEvent(
            PointerUpEvent(
              pointer: pointer,
              position: at(13),
              pressureMin: 0,
              pressureMax: 6,
            ),
          );
          await tester.pump();
          expect(
            controller.selection,
            const TextSelection(baseOffset: 6, extentOffset: 10),
          );
          expect(changes.map((entry) => entry.$2), [
            SelectionChangedCause.forcePress,
            SelectionChangedCause.forcePress,
          ]);
        } else {
          expect(changes, isEmpty);
          await gesture.up();
          await tester.pump();
          expect(
            controller.selection,
            const TextSelection.collapsed(offset: 2),
          );
          expect(changes.single.$2, SelectionChangedCause.tap);
          expect(
            find.byKey(const ValueKey<String>('force menu')),
            findsNothing,
          );
        }
      },
    );
  }
}
