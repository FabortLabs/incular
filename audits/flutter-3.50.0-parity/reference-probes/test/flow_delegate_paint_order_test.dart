import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class OrderedFlow extends FlowDelegate {
  OrderedFlow(this.order);
  final List<int> order;

  @override
  BoxConstraints getConstraintsForChild(int index, BoxConstraints constraints) {
    return constraints.loosen();
  }

  @override
  void paintChildren(FlowPaintingContext context) {
    for (final index in order) {
      context.paintChild(index);
    }
  }

  @override
  bool shouldRepaint(OrderedFlow oldDelegate) => true;
}

void main() {
  testWidgets('overlap hits the last painted child, not last index', (
    tester,
  ) async {
    for (final order in [
      [0, 1],
      [1, 0],
    ]) {
      final hits = <int>[];
      await tester.pumpWidget(
        Flow(
          key: UniqueKey(),
          delegate: OrderedFlow(order),
          children: [
            for (final index in [0, 1])
              GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTap: () => hits.add(index),
                child: const SizedBox(width: 100, height: 100),
              ),
          ],
        ),
      );
      await tester.tapAt(const Offset(20, 20));
      expect(hits, [order.last]);
    }
  });
}
