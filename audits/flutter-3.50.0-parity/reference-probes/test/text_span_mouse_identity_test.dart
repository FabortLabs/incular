import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final equivalent in [false, true]) {
    testWidgets('Mouse tracking across equivalent=$equivalent spans', (
      WidgetTester tester,
    ) async {
      final events = <String>[];
      void enter(PointerEnterEvent _) => events.add('enter');
      void exit(PointerExitEvent _) => events.add('exit');

      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: Text.rich(
              TextSpan(
                children: [
                  TextSpan(text: 'xxxxx', onEnter: enter, onExit: exit),
                  TextSpan(
                    text: equivalent ? 'xxxxx' : 'yyyyy',
                    onEnter: enter,
                    onExit: exit,
                  ),
                ],
              ),
              style: const TextStyle(fontSize: 10),
            ),
          ),
        ),
      );
      final center = tester.getCenter(find.byType(RichText));
      final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
      await mouse.addPointer();
      await mouse.moveTo(center - const Offset(20, 0));
      expect(events, ['enter']);
      await mouse.moveTo(center + const Offset(20, 0));
      expect(events, equivalent ? ['enter'] : ['enter', 'exit', 'enter']);
      await mouse.removePointer();
    });
  }
}
