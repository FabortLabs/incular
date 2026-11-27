import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Childless animated padding has its own interpolated extent', (WidgetTester tester) async {
    Widget build(double padding) => Directionality(
      textDirection: TextDirection.rtl,
      child: Center(
        child: AnimatedPadding(
          padding: EdgeInsets.all(padding),
          duration: const Duration(milliseconds: 200),
        ),
      ),
    );

    await tester.pumpWidget(build(5));
    expect(tester.getSize(find.byType(AnimatedPadding)), const Size(10, 10));
    await tester.pumpWidget(build(15));
    expect(tester.getSize(find.byType(AnimatedPadding)), const Size(10, 10));
    await tester.pump(const Duration(milliseconds: 100));
    expect(tester.getSize(find.byType(AnimatedPadding)), const Size(20, 20));
    await tester.pump(const Duration(milliseconds: 100));
    expect(tester.getSize(find.byType(AnimatedPadding)), const Size(30, 30));
  });

  test('Invalid negative animated padding fails synchronously in debug', () {
    expect(
      () => AnimatedPadding(
        padding: const EdgeInsets.only(right: -1),
        duration: const Duration(milliseconds: 200),
      ),
      throwsAssertionError,
    );
  });
}
