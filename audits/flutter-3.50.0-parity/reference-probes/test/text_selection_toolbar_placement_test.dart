import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('toolbar clamps to the horizontal edge of the selected anchor', (
    tester,
  ) async {
    const toolbar = ValueKey<String>('selection toolbar');

    Future<void> check(Offset above, bool? fitsAbove, Offset expected) async {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: CustomSingleChildLayout(
            delegate: TextSelectionToolbarLayoutDelegate(
              anchorAbove: above,
              anchorBelow: const Offset(400, 500),
              fitsAbove: fitsAbove,
            ),
            child: const SizedBox(key: toolbar, width: 200, height: 43),
          ),
        ),
      );
      final Finder child = find.byKey(toolbar);
      expect(tester.getSize(child), const Size(200, 43));
      expect(tester.getTopLeft(child), expected);
    }

    await check(const Offset(775, 80), null, const Offset(600, 37));
    await check(const Offset(775, 80), false, const Offset(300, 500));
    await check(const Offset(20, 20), true, Offset.zero);
  });
}
