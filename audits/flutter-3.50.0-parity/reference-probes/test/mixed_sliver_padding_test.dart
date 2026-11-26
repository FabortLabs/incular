import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('mixed sliver padding follows retained Directionality', (
    tester,
  ) async {
    final view = GlobalKey();
    final leaf = GlobalKey();
    final padding = const EdgeInsets.fromLTRB(
      3,
      5,
      7,
      11,
    ).add(const EdgeInsetsDirectional.fromSTEB(13, 17, 19, 23));
    Element? original;
    for (final (direction, left) in [
      (TextDirection.ltr, 16.0),
      (TextDirection.rtl, 22.0),
      (TextDirection.ltr, 16.0),
    ]) {
      await tester.pumpWidget(
        Directionality(
          textDirection: direction,
          child: Center(
            child: SizedBox(
              key: view,
              width: 100,
              height: 120,
              child: CustomScrollView(
                slivers: [
                  SliverPadding(
                    padding: padding,
                    sliver: SliverToBoxAdapter(
                      child: SizedBox(key: leaf, width: 100, height: 50),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      );
      final child = find.byKey(leaf);
      original ??= tester.element(child);
      expect(tester.element(child), same(original));
      expect(tester.getSize(child), const Size(58, 50));
      expect(
        tester.getTopLeft(child) - tester.getTopLeft(find.byKey(view)),
        Offset(left, 22),
      );
    }
  });
}
