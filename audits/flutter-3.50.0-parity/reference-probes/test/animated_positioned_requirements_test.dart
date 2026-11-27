import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Animated directional edges require the inherited Directionality', (WidgetTester tester) async {
    await tester.pumpWidget(
      Stack(
        // An explicit direction enables Stack, not its animated descendant.
        textDirection: TextDirection.ltr,
        children: const <Widget>[
          AnimatedPositionedDirectional(
            start: 10,
            duration: Duration(milliseconds: 200),
            child: SizedBox(width: 10, height: 10),
          ),
        ],
      ),
    );
    final Object? error = tester.takeException();
    expect(error, isA<FlutterError>());
    expect(error.toString(), contains('No Directionality widget found'));
    expect(error.toString(), contains('AnimatedPositionedDirectional'));
  });

  test('Each animated axis accepts at most two of its three variables', () {
    expect(
      () => AnimatedPositioned(
        duration: Duration.zero,
        left: 1,
        right: 2,
        width: 3,
        child: const SizedBox(),
      ),
      throwsAssertionError,
    );
    expect(
      () => AnimatedPositionedDirectional(
        duration: Duration.zero,
        top: 1,
        bottom: 2,
        height: 3,
        child: const SizedBox(),
      ),
      throwsAssertionError,
    );
  });
}
