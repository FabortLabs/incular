import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final equalOrders in [false, true]) {
    for (final useWidgetOrder in [false, true]) {
      testWidgets('secondary order resolves equal or absent orders ($equalOrders, $useWidgetOrder)', (tester) async {
        final first = FocusNode();
        final second = FocusNode();
        addTearDown(first.dispose);
        addTearDown(second.dispose);
        Widget item(FocusNode node, double left) {
          final focus = Focus(focusNode: node, child: const SizedBox(width: 20, height: 20));
          return Positioned(
            left: left,
            top: 0,
            child: equalOrders ? FocusTraversalOrder(order: const NumericFocusOrder(1), child: focus) : focus,
          );
        }

        await tester.pumpWidget(Directionality(
          textDirection: TextDirection.rtl,
          child: FocusTraversalGroup(
            policy: OrderedTraversalPolicy(
              secondary: useWidgetOrder ? WidgetOrderTraversalPolicy() : ReadingOrderTraversalPolicy(),
            ),
            child: Stack(alignment: Alignment.topLeft, children: [item(first, 10), item(second, 100)]),
          ),
        ));
        expect(first.nextFocus(), isTrue);
        await tester.pump();
        expect(first.hasPrimaryFocus, useWidgetOrder);
        expect(second.hasPrimaryFocus, !useWidgetOrder);
      });
    }
  }
}
