import 'package:flutter/foundation.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Handles extends TextSelectionControls with TextSelectionHandleControls {
  @override
  Size getHandleSize(double lineHeight) => const Size(30, 30);

  @override
  Offset getHandleAnchor(TextSelectionHandleType type, double lineHeight) =>
      const Offset(15, 0);

  @override
  Widget buildHandle(
    BuildContext context,
    TextSelectionHandleType type,
    double lineHeight, [
    VoidCallback? onTap,
  ]) => const SizedBox.square(
    dimension: 30,
    child: ColoredBox(color: Color(0xff0000ff)),
  );
}

void main() {
  testWidgets('a held handle snaps at the current scaled row distance', (
    tester,
  ) async {
    final controller = TextEditingController(
      text: '0123456789\n0123456789\n0123456789',
    );
    final focus = FocusNode();
    final handles = _Handles();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    var scale = 1.0;
    var pivot = 0.0;
    late StateSetter change;
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Overlay(
          initialEntries: [
            OverlayEntry(
              builder: (_) => Center(
                child: StatefulBuilder(
                  builder: (_, setState) {
                    change = setState;
                    return Transform(
                      transform: Matrix4.diagonal3Values(1, scale, 1)
                        ..setEntry(1, 3, (1 - scale) * pivot),
                      child: SizedBox(
                        width: 300,
                        height: 200,
                        child: EditableText(
                          controller: controller,
                          focusNode: focus,
                          maxLines: null,
                          style: const TextStyle(fontSize: 14, height: 1),
                          cursorColor: const Color(0xffff0000),
                          backgroundCursorColor: const Color(0xffcccccc),
                          showSelectionHandles: true,
                          selectionControls: handles,
                        ),
                      ),
                    );
                  },
                ),
              ),
            ),
          ],
        ),
      ),
    );
    focus.requestFocus();
    await tester.pump();
    controller.selection = const TextSelection(baseOffset: 2, extentOffset: 7);
    await tester.pumpAndSettle();
    final editable = tester
        .state<EditableTextState>(find.byType(EditableText))
        .renderEditable;
    final row = editable.preferredLineHeight;
    expect(row, greaterThan(0));
    final end = editable
        .getEndpointsForSelection(controller.selection)
        .last
        .point;
    final handle = editable.localToGlobal(end) + const Offset(0, 10);
    final gesture = await tester.startGesture(handle);
    await gesture.moveBy(const Offset(20, 0));
    await tester.pump();

    pivot = end.dy;
    change(() => scale = 2);
    await tester.pump();
    await gesture.moveTo(handle + Offset(0, 1.4 * row));
    await tester.pump();
    final first = controller.selection;
    expect(first.baseOffset, 2);
    expect(first.extentOffset, inInclusiveRange(3, 10));
    await gesture.moveTo(handle + Offset(0, 2.1 * row));
    await tester.pump();
    final second = controller.selection;
    expect(second.baseOffset, 2);
    expect(second.extentOffset, first.extentOffset + 11);
    await gesture.up();
    expect(focus.hasFocus, isTrue);
  }, variant: const TargetPlatformVariant({TargetPlatform.android}));
}
