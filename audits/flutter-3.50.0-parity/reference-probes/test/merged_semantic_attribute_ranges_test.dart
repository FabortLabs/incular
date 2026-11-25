import 'dart:ui';

import 'package:flutter/semantics.dart' show AttributedString;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('merged spans follow UTF-16 separators and directional markers', (
    WidgetTester tester,
  ) async {
    final handle = tester.ensureSemantics();
    const key = ValueKey('merged');
    Widget frame(TextDirection innerDirection) => Directionality(
      textDirection: TextDirection.ltr,
      child: MergeSemantics(
        key: key,
        child: Semantics(
          label: '😀',
          textDirection: TextDirection.ltr,
          child: Semantics(
            container: true,
            textDirection: innerDirection,
            attributedLabel: AttributedString(
              'مرحبا',
              attributes: [
                LocaleStringAttribute(
                  range: const TextRange(start: 0, end: 2),
                  locale: const Locale('ar'),
                ),
              ],
            ),
            child: const SizedBox.square(dimension: 10),
          ),
        ),
      ),
    );
    await tester.pumpWidget(frame(TextDirection.rtl));
    final int id = tester.getSemantics(find.byKey(key)).id;
    AttributedString result = tester
        .getSemantics(find.byKey(key))
        .getSemanticsData()
        .attributedLabel;
    expect(result.string, '😀\n\u202Bمرحبا\u202C');
    expect(result.attributes.single.range, const TextRange(start: 4, end: 6));
    expect(
      (result.attributes.single as LocaleStringAttribute).locale,
      const Locale('ar'),
    );

    await tester.pumpWidget(frame(TextDirection.ltr));
    expect(tester.getSemantics(find.byKey(key)).id, id);
    result = tester
        .getSemantics(find.byKey(key))
        .getSemanticsData()
        .attributedLabel;
    expect(result.string, '😀\nمرحبا');
    expect(result.attributes.single.range, const TextRange(start: 3, end: 5));
    handle.dispose();
  });
}
