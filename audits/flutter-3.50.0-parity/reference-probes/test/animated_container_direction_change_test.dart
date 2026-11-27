import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final bool pivot in <bool>[false, true]) {
    final String property = pivot ? 'pivot' : 'alignment';
    testWidgets('Direction changes resolve $property without restarting', (WidgetTester tester) async {
      final Key target = UniqueKey();
      var completions = 0;

      Widget build(TextDirection direction, AlignmentDirectional value) {
        final child = SizedBox(key: target, width: 100, height: 200);
        return Directionality(
          textDirection: direction,
          child: pivot
              ? Center(
                  child: AnimatedContainer(
                    duration: const Duration(milliseconds: 200),
                    transform: Matrix4.diagonal3Values(0.5, 0.5, 1),
                    transformAlignment: value,
                    onEnd: () => completions++,
                    child: child,
                  ),
                )
              : AnimatedContainer(
                  duration: const Duration(milliseconds: 200),
                  alignment: value,
                  onEnd: () => completions++,
                  child: child,
                ),
        );
      }

      double left() => tester.getTopLeft(find.byKey(target)).dx;

      await tester.pumpWidget(build(TextDirection.ltr, AlignmentDirectional.centerStart));
      expect(left(), pivot ? 350.0 : 0.0);

      await tester.pumpWidget(build(TextDirection.ltr, AlignmentDirectional.centerEnd));
      await tester.pump(const Duration(milliseconds: 50));
      expect(left(), pivot ? 362.5 : 175.0);

      await tester.pumpWidget(build(TextDirection.rtl, AlignmentDirectional.centerEnd));
      expect(left(), pivot ? 387.5 : 525.0);
      expect(completions, 0);

      await tester.pump(const Duration(milliseconds: 50));
      expect(left(), pivot ? 375.0 : 350.0);
      await tester.pump(const Duration(milliseconds: 100));
      expect(left(), pivot ? 350.0 : 0.0);
      expect(completions, 0);
      await tester.pump(const Duration(milliseconds: 1));
      expect(completions, 1);
      await tester.pump(const Duration(milliseconds: 100));
      expect(completions, 1);
    });
  }
}
