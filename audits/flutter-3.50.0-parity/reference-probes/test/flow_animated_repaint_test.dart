import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class RecordingFlow extends FlowDelegate {
  RecordingFlow(this.animation, this.values, this.onMeasure)
    : super(repaint: animation);
  final Animation<double> animation;
  final List<double> values;
  final VoidCallback onMeasure;

  @override
  BoxConstraints getConstraintsForChild(int index, BoxConstraints constraints) {
    onMeasure();
    return constraints.loosen();
  }

  @override
  void paintChildren(FlowPaintingContext context) {
    values.add(animation.value);
    for (var i = 0; i < context.childCount; i++) {
      context.paintChild(
        i,
        transform: Matrix4.translationValues(
          0,
          animation.value * 100 + 40 * i,
          0,
        ),
      );
    }
  }

  @override
  bool shouldRepaint(RecordingFlow oldDelegate) => false;
}

void main() {
  testWidgets(
    'Flow repaints on the first and later animation ticks without laying out',
    (WidgetTester tester) async {
      final animation = AnimationController(
        duration: const Duration(milliseconds: 100),
        vsync: tester,
      );
      addTearDown(animation.dispose);
      final values = <double>[];
      final hits = <int>[];
      var measures = 0;
      await tester.pumpWidget(
        Flow(
          delegate: RecordingFlow(animation, values, () => measures++),
          children: [
            for (var i = 0; i < 2; i++)
              GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTap: () => hits.add(i),
                child: const SizedBox(width: 80, height: 60),
              ),
          ],
        ),
      );
      expect(values, [0]);
      final beforeMeasures = measures;
      animation.forward();
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 30));
      expect(measures, beforeMeasures);
      expect(values, [0, 0, closeTo(.3, .00001)]);
      await tester.tapAt(const Offset(20, 50));
      expect(hits, [0]);
    animation.stop();
    },
  );
}
