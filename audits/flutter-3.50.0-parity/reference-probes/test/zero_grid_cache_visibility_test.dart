import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('zero-area grid tiles may remain cached but do not appear', (
    tester,
  ) async {
    Future<void> layout(double size) async {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: SizedBox.square(
              dimension: size,
              child: GridView.count(
                crossAxisCount: 4,
                children: List<Widget>.generate(20, (i) => Text('$i')),
              ),
            ),
          ),
        ),
      );
    }

    await layout(0);
    expect(find.text('0'), findsNothing);
    expect(find.text('1'), findsNothing);
    expect(find.text('0', skipOffstage: false), findsOneWidget);
    expect(find.text('1', skipOffstage: false), findsNothing);

    await layout(300);
    expect(tester.getSize(find.byType(GridView)), const Size(300, 300));
    expect(find.text('0'), findsOneWidget);
  });
}
