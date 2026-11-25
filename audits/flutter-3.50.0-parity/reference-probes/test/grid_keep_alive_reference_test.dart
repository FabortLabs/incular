import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Tile extends StatefulWidget {
  const _Tile(this.index, {super.key});

  final int index;

  @override
  State<_Tile> createState() => _TileState();
}

class _TileState extends State<_Tile> {
  @override
  Widget build(BuildContext context) => Semantics(
    container: true,
    label: 'tile ${widget.index}',
    child: const SizedBox.expand(),
  );
}

void main() {
  testWidgets('only the tile with a keep-alive request survives its row', (
    WidgetTester tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    final sibling = GlobalKey<_TileState>();
    final target = GlobalKey<_TileState>();
    Widget scene(bool keep) => Directionality(
      textDirection: TextDirection.ltr,
      child: Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 100,
          height: 100,
          child: GridView(
            controller: controller,
            gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
              crossAxisCount: 2,
              mainAxisExtent: 50,
            ),
            cacheExtent: 0,
            addAutomaticKeepAlives: false,
            addRepaintBoundaries: false,
            addSemanticIndexes: false,
            children: List<Widget>.generate(
              20,
              (i) => KeepAlive(
                keepAlive: i == 1 && keep,
                child: _Tile(
                  i,
                  key: i == 0
                      ? sibling
                      : i == 1
                      ? target
                      : null,
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpWidget(scene(true));
    final firstTarget = target.currentState!;
    final firstSibling = sibling.currentState!;
    controller.jumpTo(300);
    await tester.pump();
    expect(target.currentState, same(firstTarget));
    expect(firstSibling.mounted, isFalse);
    expect(find.bySemanticsLabel('tile 1'), findsNothing);
    await tester.pumpWidget(scene(true));
    expect(target.currentState, same(firstTarget));
    controller.jumpTo(0);
    await tester.pump();
    expect(target.currentState, same(firstTarget));
    expect(sibling.currentState, isNot(same(firstSibling)));
    controller.jumpTo(300);
    await tester.pump();
    await tester.pumpWidget(scene(false));
    expect(firstTarget.mounted, isFalse);
  });
}
