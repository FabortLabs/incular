import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Tile extends StatefulWidget {
  const _Tile(this.index, {super.key});

  final int index;

  @override
  State<_Tile> createState() => _TileState();
}

class _TileState extends State<_Tile> {
  bool _keep = false;

  void keep(bool value) => setState(() => _keep = value);

  @override
  Widget build(BuildContext context) => KeepAlive(
    keepAlive: _keep,
    child: Semantics(
      container: true,
      label: 'tile ${widget.index}',
      child: const SizedBox(height: 50),
    ),
  );
}

void main() {
  for (final (fixed, grid) in [(false, false), (true, false), (false, true)]) {
    testWidgets('child locally updates keep-alive: fixed=$fixed grid=$grid', (
      WidgetTester tester,
    ) async {
      final key = GlobalKey<_TileState>();
      final controller = ScrollController();
      addTearDown(controller.dispose);
      Widget item(BuildContext context, int i) =>
          _Tile(i, key: i == 0 ? key : null);
      final view = grid
          ? GridView.builder(
              controller: controller,
              gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
                crossAxisCount: 2,
                mainAxisExtent: 50,
              ),
              cacheExtent: 0,
              addAutomaticKeepAlives: false,
              addRepaintBoundaries: false,
              addSemanticIndexes: false,
              itemCount: 40,
              itemBuilder: item,
            )
          : ListView.builder(
              controller: controller,
              itemExtent: fixed ? 50 : null,
              cacheExtent: 0,
              addAutomaticKeepAlives: false,
              addRepaintBoundaries: false,
              addSemanticIndexes: false,
              itemCount: 20,
              itemBuilder: item,
            );
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(width: 100, height: 100, child: view),
          ),
        ),
      );
      final target = key.currentState!;
      target.keep(true);
      await tester.pump();
      controller.jumpTo(300);
      await tester.pump();
      expect(key.currentState, same(target));
      expect(find.bySemanticsLabel('tile 0'), findsNothing);

      final away = controller.offset;
      target.keep(false);
      await tester.pump();
      expect(target.mounted, isFalse);
      expect(controller.offset, away);
      controller.jumpTo(0);
      await tester.pump();
      expect(key.currentState, isNot(same(target)));
    });
  }
}
