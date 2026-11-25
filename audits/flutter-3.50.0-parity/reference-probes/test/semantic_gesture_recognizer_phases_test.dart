import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Semantic directional drag drives the axis and pan phases', (
    WidgetTester tester,
  ) async {
    final semantics = tester.ensureSemantics();
    try {
      final key = GlobalKey();
      final phases = <String>[];

      GestureDragDownCallback down(String phase) => (DragDownDetails details) {
        expect(details.localPosition, const Offset(100, 50));
        expect(details.globalPosition, const Offset(120, 80));
        phases.add(phase);
      };
      GestureDragStartCallback start(String phase) =>
          (DragStartDetails details) {
            expect(details.localPosition, const Offset(100, 50));
            expect(details.globalPosition, const Offset(120, 80));
            phases.add(phase);
          };
      GestureDragUpdateCallback update(String phase) =>
          (DragUpdateDetails details) {
            expect(details.delta, const Offset(-160, 0));
            expect(details.primaryDelta, -160);
            expect(details.localPosition, const Offset(120, 80));
            phases.add(phase);
          };
      GestureDragEndCallback end(String phase, double? velocity) =>
          (DragEndDetails details) {
            expect(details.primaryVelocity, velocity);
            expect(details.velocity, Velocity.zero);
            expect(details.localPosition, const Offset(-60, 50));
            expect(details.globalPosition, const Offset(-40, 80));
            phases.add(phase);
          };

      await tester.pumpWidget(
        Align(
          alignment: Alignment.topLeft,
          child: Transform.translate(
            offset: const Offset(20, 30),
            child: GestureDetector(
              key: key,
              onHorizontalDragDown: down('horizontal.down'),
              onHorizontalDragStart: start('horizontal.start'),
              onHorizontalDragUpdate: update('horizontal.update'),
              onHorizontalDragEnd: end('horizontal.end', 0),
              onPanDown: down('pan.down'),
              onPanStart: start('pan.start'),
              onPanUpdate: update('pan.update'),
              onPanEnd: end('pan.end', null),
              child: const SizedBox(width: 200, height: 100),
            ),
          ),
        ),
      );
      final gesture = key.currentContext!.findRenderObject()!;
      tester.binding.pipelineOwner.semanticsOwner!.performAction(
        gesture.debugSemantics!.id,
        SemanticsAction.scrollLeft,
      );
      expect(phases, [
        'horizontal.down',
        'horizontal.start',
        'horizontal.update',
        'horizontal.end',
        'pan.down',
        'pan.start',
        'pan.update',
        'pan.end',
      ]);
    } finally {
      semantics.dispose();
    }
  });
}
