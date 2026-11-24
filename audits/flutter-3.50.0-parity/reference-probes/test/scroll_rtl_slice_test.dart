import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('horizontal offset zero follows direction and reverse', (
    t,
  ) async {
    const viewport = ValueKey('viewport'), child = ValueKey('child');
    for (final (dir, reverse, want) in [
      (TextDirection.ltr, false, 0.0),
      (TextDirection.ltr, true, -100.0),
      (TextDirection.rtl, true, 0.0),
      (TextDirection.rtl, false, -100.0),
    ]) {
      await t.pumpWidget(
        Directionality(
          textDirection: dir,
          child: Center(
            child: SizedBox(
              width: 100,
              height: 60,
              child: SingleChildScrollView(
                key: viewport,
                scrollDirection: Axis.horizontal,
                reverse: reverse,
                child: const SizedBox(key: child, width: 200, height: 60),
              ),
            ),
          ),
        ),
      );
      expect(
        t.getTopLeft(find.byKey(child)).dx -
            t.getTopLeft(find.byKey(viewport)).dx,
        want,
        reason: '$dir reverse=$reverse',
      );
    }
  });
}
