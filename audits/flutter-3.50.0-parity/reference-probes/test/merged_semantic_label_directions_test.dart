import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'a merged node embeds an opposing descendant in its own direction',
    (WidgetTester tester) async {
      final semantics = tester.ensureSemantics();
      const key = ValueKey('merged');

      Widget frame(String outerLabel, TextDirection innerDirection) =>
          Directionality(
            textDirection: TextDirection.ltr,
            child: MergeSemantics(
              key: key,
              child: Semantics(
                label: outerLabel,
                textDirection: TextDirection.ltr,
                child: Semantics(
                  container: true,
                  label: 'مرحبا',
                  textDirection: innerDirection,
                  child: const SizedBox.square(dimension: 10),
                ),
              ),
            ),
          );

      await tester.pumpWidget(frame('Welcome', TextDirection.rtl));
      final int id = tester.getSemantics(find.byKey(key)).id;
      expect(
        tester.getSemantics(find.byKey(key)).getSemanticsData().label,
        'Welcome\n\u202Bمرحبا\u202C',
      );
      expect(
        tester.getSemantics(find.byKey(key)).getSemanticsData().textDirection,
        TextDirection.ltr,
      );

      await tester.pumpWidget(frame('Welcome', TextDirection.ltr));
      expect(tester.getSemantics(find.byKey(key)).id, id);
      expect(
        tester.getSemantics(find.byKey(key)).getSemanticsData().label,
        'Welcome\nمرحبا',
      );

      await tester.pumpWidget(frame('', TextDirection.rtl));
      expect(tester.getSemantics(find.byKey(key)).id, id);
      expect(
        tester.getSemantics(find.byKey(key)).getSemanticsData().label,
        '\u202Bمرحبا\u202C',
      );
      semantics.dispose();
    },
  );
}
