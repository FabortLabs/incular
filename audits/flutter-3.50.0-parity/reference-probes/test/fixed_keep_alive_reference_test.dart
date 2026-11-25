import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Marker extends StatefulWidget {
  const _Marker({super.key});

  @override
  State<_Marker> createState() => _MarkerState();
}

class _MarkerState extends State<_Marker> {
  @override
  Widget build(BuildContext context) => Semantics(
    container: true,
    label: 'tile 0',
    child: const SizedBox.expand(),
  );
}

void main() {
  for (final page in [false, true]) {
    testWidgets(
      'fixed views keep state and honor request withdrawal page=$page',
      (WidgetTester tester) async {
        final ScrollController controller = page
            ? PageController()
            : ScrollController();
        addTearDown(controller.dispose);
        final marker = GlobalKey<_MarkerState>();
        Widget scene(bool keep) {
          final children = <Widget>[
            KeepAlive(
              keepAlive: keep,
              child: _Marker(key: marker),
            ),
            ...List<Widget>.generate(11, (_) => const SizedBox.expand()),
          ];
          final view = page
              ? PageView.custom(
                  pageSnapping: false,
                  controller: controller as PageController,
                  childrenDelegate: SliverChildListDelegate(
                    children,
                    addAutomaticKeepAlives: false,
                    addRepaintBoundaries: false,
                    addSemanticIndexes: false,
                  ),
                )
              : ListView(
                  controller: controller,
                  itemExtent: 50,
                  cacheExtent: 0,
                  addAutomaticKeepAlives: false,
                  addRepaintBoundaries: false,
                  addSemanticIndexes: false,
                  children: children,
                );
          return Directionality(
            textDirection: TextDirection.ltr,
            child: Align(
              alignment: Alignment.topLeft,
              child: SizedBox(width: 100, height: 100, child: view),
            ),
          );
        }

        final semantics = find.bySemanticsLabel('tile 0');
        await tester.pumpWidget(scene(true));
        final first = marker.currentState!;
        expect(semantics, findsOneWidget);
        controller.jumpTo(300);
        await tester.pump();
        expect(marker.currentState, same(first));
        expect(semantics, findsNothing);
        await tester.pumpWidget(scene(true));
        expect(marker.currentState, same(first));
        expect(semantics, findsNothing);
        await tester.pumpWidget(scene(false));
        expect(first.mounted, isFalse);
        controller.jumpTo(0);
        await tester.pump();
        expect(marker.currentState, isNot(same(first)));
        expect(semantics, findsOneWidget);
      },
    );
  }
}
