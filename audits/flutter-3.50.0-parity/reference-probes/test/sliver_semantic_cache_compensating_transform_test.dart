import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'A roleless off-cache wrapper does not hide a child translated back into view',
    (WidgetTester tester) async {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: CustomScrollView(
            slivers: <Widget>[
              SliverToBoxAdapter(
                child: Transform.translate(
                  offset: const Offset(0, -500),
                  child: Padding(
                    padding: const EdgeInsets.all(2),
                    child: Transform.translate(
                      offset: const Offset(0, 500),
                      child: Semantics(
                        label: 'compensated',
                        child: const SizedBox(height: 20),
                      ),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      );
      final Finder label = find.bySemanticsLabel('compensated');
      expect(label, findsOneWidget);
      final data = tester.getSemantics(label).getSemanticsData();
      expect(data.label, 'compensated');
      expect(tester.getTopLeft(label).dy, 2);
    },
  );
}
