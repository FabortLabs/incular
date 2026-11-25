import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Pixel cache extent does not scale with viewport width', (
    WidgetTester tester,
  ) async {
    Widget build(double width, ScrollCacheExtent cache) => Directionality(
      textDirection: TextDirection.ltr,
      child: Center(
        child: SizedBox(
          width: width,
          height: 200,
          child: PageView(
            allowImplicitScrolling: true,
            scrollCacheExtent: cache,
            children: List<Widget>.generate(4, (page) => Text('Page $page')),
          ),
        ),
      ),
    );

    const pixels = ScrollCacheExtent.pixels(300);
    const relative = ScrollCacheExtent.viewport(1);
    for (final cache in [pixels, relative]) {
      Finder page(int i) => find.text('Page $i', skipOffstage: false);
      await tester.pumpWidget(build(400, cache));
      expect(page(0), findsOneWidget);
      expect(page(1), findsOneWidget);
      expect(page(2), findsNothing);
      await tester.pumpWidget(build(200, cache));
      expect(page(0), findsOneWidget);
      expect(page(1), findsOneWidget);
      expect(page(2), cache == pixels ? findsOneWidget : findsNothing);
      expect(page(3), findsNothing);
    }
  });
}
