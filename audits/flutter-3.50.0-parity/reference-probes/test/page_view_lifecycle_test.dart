import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const page0Key = ValueKey<String>('page0');
const page1Key = ValueKey<String>('page1');

class PlainPage extends StatefulWidget {
  const PlainPage({super.key});

  @override
  State<PlainPage> createState() => _PlainPageState();
}

class _PlainPageState extends State<PlainPage> {
  @override
  Widget build(BuildContext context) =>
      const ColoredBox(color: Color(0xff2196f3));
}

void main() {
  for (final partial in [false, true]) {
    testWidgets('settled PageView disposes old page (partial=$partial)', (
      tester,
    ) async {
      final controller = PageController();
      addTearDown(controller.dispose);
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: PageView(
            controller: controller,
            children: const [
              PlainPage(key: page0Key),
              ColoredBox(key: page1Key, color: Color(0xffff5722)),
              ColoredBox(color: Color(0xff4caf50)),
            ],
          ),
        ),
      );
      final firstState = tester.state(find.byKey(page0Key));
      if (partial) {
        controller.jumpTo(controller.position.viewportDimension / 2);
        await tester.pump();
        expect(controller.page, 0.5);
        expect(find.byKey(page0Key), findsOneWidget);
        expect(find.byKey(page1Key), findsOneWidget);
      }
      controller.jumpToPage(1);
      await tester.pump();
      expect(controller.page, 1.0);
      expect(find.byKey(page1Key), findsOneWidget);
      expect(find.byKey(page0Key, skipOffstage: false), findsNothing);
      expect(firstState.mounted, isFalse);
      controller.jumpToPage(0);
      await tester.pump();
      expect(tester.state(find.byKey(page0Key)), isNot(same(firstState)));
    });
  }
  for (final reverse in [false, true]) {
    testWidgets(
      'positive cache evicts exactly at its edge (reverse=$reverse)',
      (tester) async {
        final controller = PageController();
        addTearDown(controller.dispose);
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: PageView(
              controller: controller,
              reverse: reverse,
              pageSnapping: false,
              allowImplicitScrolling: true,
              scrollCacheExtent: const ScrollCacheExtent.pixels(40),
              children: const [
                PlainPage(key: page0Key),
                ColoredBox(key: page1Key, color: Color(0xffff5722)),
                ColoredBox(color: Color(0xff4caf50)),
              ],
            ),
          ),
        );
        final first = find.byKey(page0Key, skipOffstage: false);
        final firstState = tester.state(first);
        for (final double offset in [800, 839]) {
          controller.jumpTo(offset);
          await tester.pump();
          expect(tester.state(first), same(firstState));
        }
        controller.jumpTo(840);
        await tester.pump();
        expect(first, findsNothing);
        expect(firstState.mounted, isFalse);
        controller.jumpTo(839);
        await tester.pump();
        expect(tester.state(first), isNot(same(firstState)));
      },
    );
  }
}
