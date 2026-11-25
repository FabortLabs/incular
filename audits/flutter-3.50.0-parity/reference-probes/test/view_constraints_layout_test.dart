import 'dart:ui' show ViewConstraints;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Converted view bounds constrain both axes without remounting', (
    WidgetTester tester,
  ) async {
    const key = ValueKey<String>('sized child');
    Widget build(ViewConstraints bounds) => Directionality(
      textDirection: TextDirection.ltr,
      child: Align(
        alignment: Alignment.topLeft,
        child: ConstrainedBox(
          constraints: BoxConstraints.fromViewConstraints(bounds),
          child: const SizedBox(key: key, width: 10, height: 1),
        ),
      ),
    );

    await tester.pumpWidget(
      build(
        const ViewConstraints(
          minWidth: 1,
          maxWidth: 2,
          minHeight: 3,
          maxHeight: 4,
        ),
      ),
    );
    final finder = find.byKey(key);
    final original = tester.element(finder);
    expect(tester.getSize(finder), const Size(2, 3));

    await tester.pumpWidget(build(ViewConstraints.tight(const Size(5, 6))));
    expect(tester.element(finder), same(original));
    expect(tester.getSize(finder), const Size(5, 6));
  });
}
