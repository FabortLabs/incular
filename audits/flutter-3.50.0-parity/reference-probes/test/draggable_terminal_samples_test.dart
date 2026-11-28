import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final cancel in [false, true]) {
    testWidgets('terminal pointer sample is not drag movement (cancel: $cancel)', (
      WidgetTester tester,
    ) async {
      final deltas = <Offset>[];
      final offered = <String>[];
      final dropped = <String>[];
      final ended = <DraggableDetails>[];

      Widget target(String name, double left) => Positioned(
        left: left,
        top: 50,
        width: 100,
        height: 80,
        child: DragTarget<int>(
          onWillAcceptWithDetails: (_) {
            offered.add(name);
            return true;
          },
          onAcceptWithDetails: (_) => dropped.add(name),
          builder: (_, _, _) => const SizedBox.expand(),
        ),
      );

      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Overlay(
            initialEntries: [
              OverlayEntry(
                builder: (_) => Stack(
                  children: [
                    Positioned(
                      left: 10,
                      top: 50,
                      child: Draggable<int>(
                        data: 1,
                        axis: Axis.horizontal,
                        onDragUpdate: (details) => deltas.add(details.delta),
                        onDragEnd: ended.add,
                        feedback: const SizedBox(width: 20, height: 20),
                        child: const ColoredBox(
                          color: Color(0xff000000),
                          child: SizedBox(width: 40, height: 40),
                        ),
                      ),
                    ),
                    target('first', 110),
                    target('second', 220),
                  ],
                ),
              ),
            ],
          ),
        ),
      );

      final pointer = TestPointer(7);
      await tester.sendEventToBinding(pointer.down(const Offset(20, 60)));
      await tester.pump();
      await tester.sendEventToBinding(pointer.move(const Offset(145, 75)));
      await tester.pump();
      expect(deltas, [const Offset(125, 15)]);
      expect(offered, ['first']);

      const terminal = Offset(260, 80);
      if (cancel) {
        final original = pointer.cancel();
        await tester.sendEventToBinding(PointerCancelEvent(
          pointer: original.pointer, kind: original.kind, device: original.device, position: terminal,
        ));
      } else {
        final original = pointer.up();
        await tester.sendEventToBinding(PointerUpEvent(
          pointer: original.pointer, kind: original.kind, device: original.device, position: terminal,
        ));
      }
      await tester.pump();
      expect(deltas, [const Offset(125, 15)]);
      expect(offered, ['first']);
      expect(dropped, cancel ? isEmpty : ['first']);
      expect(ended.single.wasAccepted, !cancel);
      expect(ended.single.offset, const Offset(135, 50));
    });
  }
}
