import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a partly filled final row is not silently accepted', (
    tester,
  ) async {
    Object? error;
    try {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Table(
            children: const [
              TableRow(
                children: const [
                  SizedBox.square(dimension: 20),
                  SizedBox.square(dimension: 20),
                ],
              ),
              TableRow(children: const [SizedBox.square(dimension: 20)]),
            ],
          ),
        ),
      );
    } catch (caught) {
      error = caught;
    }
    error ??= tester.takeException();
    expect(error, isFlutterError);
    expect(error.toString(), contains('irregular row lengths'));
  });
}
