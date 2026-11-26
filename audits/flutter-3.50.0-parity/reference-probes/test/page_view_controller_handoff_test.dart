import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Changing controllers keeps the live unsnapped page', (
    WidgetTester tester,
  ) async {
    final first = PageController(viewportFraction: 0.875);
    final second = PageController(initialPage: 4, viewportFraction: 0.75);
    addTearDown(first.dispose);
    addTearDown(second.dispose);
    Widget build(PageController controller) => Directionality(
      textDirection: TextDirection.ltr,
      child: PageView(
        controller: controller,
        pageSnapping: false,
        children: List<Widget>.generate(6, (int i) => Text('Page $i')),
      ),
    );

    await tester.pumpWidget(build(first));
    first.jumpTo(1260);
    await tester.pump();
    expect(first.page, closeTo(1.8, 1e-12));

    await tester.pumpWidget(build(second));
    expect(first.hasClients, isFalse);
    expect(second.hasClients, isTrue);
    expect(second.page, closeTo(1.8, 1e-12));
    expect(second.position.pixels, closeTo(1080, 1e-12));
    await tester.pump();
    expect(second.position.pixels, closeTo(1080, 1e-12));
  });

  testWidgets(
    'Replacing a PageController transfers the current position and detaches the old controller',
    (tester) async {
      final first = PageController();
      final second = PageController();
      addTearDown(first.dispose);
      addTearDown(second.dispose);
      Widget view(PageController controller) => Directionality(
        textDirection: TextDirection.ltr,
        child: PageView(
          controller: controller,
          children: const [Text('First page'), Text('Second page')],
        ),
      );
      await tester.pumpWidget(view(first));
      first.jumpToPage(1);
      await tester.pump();
      final retained = first.position;
      expect(first.page, 1.0);
      await tester.pumpWidget(view(second));
      expect(first.hasClients, isFalse);
      expect(second.position, same(retained));
      expect(second.page, 1.0);
      expect(find.text('Second page'), findsOneWidget);
      second.jumpToPage(0);
      await tester.pump();
      expect(find.text('First page'), findsOneWidget);
    },
  );
}
