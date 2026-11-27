import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Replacing one factory key leaves one active recognizer', (
    WidgetTester tester,
  ) async {
    final calls = <String>[];
    GestureRecognizerFactoryWithHandlers<TapGestureRecognizer> factory(
      String tag,
    ) {
      return GestureRecognizerFactoryWithHandlers<TapGestureRecognizer>(
        TapGestureRecognizer.new,
        (TapGestureRecognizer tap) => tap.onTap = () => calls.add(tag),
      );
    }

    final factories = <Type, GestureRecognizerFactory>{
      TapGestureRecognizer: factory('old'),
    };
    factories[TapGestureRecognizer] = factory('replacement');
    await tester.pumpWidget(
      Center(
        child: RawGestureDetector(
          gestures: factories,
          behavior: HitTestBehavior.opaque,
          child: const SizedBox(width: 100, height: 100),
        ),
      ),
    );
    await tester.tap(find.byType(RawGestureDetector));
    expect(calls, ['replacement']);
  });
}
