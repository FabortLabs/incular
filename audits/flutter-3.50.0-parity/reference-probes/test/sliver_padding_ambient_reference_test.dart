import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('retained nested padding resolves each directional edge once', (
    WidgetTester tester,
  ) async {
    const childKey = ValueKey<String>('child');
    final ViewportOffset position = ViewportOffset.zero();
    addTearDown(position.dispose);
    final viewport = Viewport(
      offset: position,
      slivers: const <Widget>[
        SliverPadding(
          padding: EdgeInsetsDirectional.fromSTEB(11, 8, 7, 2),
          sliver: SliverPadding(
            padding: EdgeInsetsDirectional.fromSTEB(13, 3, 3, 9),
            sliver: SliverToBoxAdapter(
              child: SizedBox(key: childKey, width: 100, height: 50),
            ),
          ),
        ),
      ],
    );
    Widget scene(TextDirection direction) => Directionality(
      textDirection: direction,
      child: Align(
        alignment: Alignment.topLeft,
        child: SizedBox(width: 100, height: 80, child: viewport),
      ),
    );
    await tester.pumpWidget(scene(TextDirection.ltr));
    final Finder finder = find.byKey(childKey);
    final RenderBox child = tester.renderObject<RenderBox>(finder);
    for (final (TextDirection direction, double left) in [
      (TextDirection.ltr, 24.0),
      (TextDirection.rtl, 10.0),
      (TextDirection.ltr, 24.0),
    ]) {
      await tester.pumpWidget(scene(direction));
      expect(tester.renderObject<RenderBox>(finder), same(child));
      expect(tester.getRect(finder), Rect.fromLTWH(left, 11, 66, 50));
      expect(position.pixels, 0);
    }
  });
}
