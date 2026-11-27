import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('placeholder preserves hairline and subpixel strokes', (
    tester,
  ) async {
    for (final width in <double>[0, 0.03125]) {
      await tester.pumpWidget(Placeholder(strokeWidth: width));
      expect(
        tester.renderObject(find.byType(Placeholder)),
        paints..path(strokeWidth: width),
      );
    }
  });
}
