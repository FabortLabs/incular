import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class RememberedPage extends StatefulWidget {
  const RememberedPage({required super.key, required this.item});
  final String item;

  @override
  State<RememberedPage> createState() => _RememberedPageState();
}

class _RememberedPageState extends State<RememberedPage>
    with AutomaticKeepAliveClientMixin<RememberedPage> {
  @override
  bool get wantKeepAlive => true;

  @override
  Widget build(BuildContext context) {
    super.build(context);
    return Text(widget.item);
  }
}

void main() {
  testWidgets('a kept page follows its key to a different offscreen index', (
    tester,
  ) async {
    final controller = PageController();
    addTearDown(controller.dispose);
    final resolved = <String>[];
    Widget view(List<String> items) => Directionality(
      textDirection: TextDirection.ltr,
      child: PageView.builder(
        controller: controller,
        itemCount: items.length,
        itemBuilder: (_, index) => RememberedPage(
          key: ValueKey(items[index]),
          item: items[index],
        ),
        findChildIndexCallback: (key) {
          final item = (key as ValueKey<String>).value;
          resolved.add(item);
          final index = items.indexOf(item);
          return index == -1 ? null : index;
        },
      ),
    );
    const keyA = ValueKey('a');
    await tester.pumpWidget(view(['a', 'b', 'c']));
    final first = tester.state(find.byKey(keyA));
    expect(resolved, isEmpty);

    controller.jumpToPage(1);
    await tester.pump();
    expect(first.mounted, isTrue);
    expect(resolved, isEmpty);

    await tester.pumpWidget(view(['c', 'b', 'a']));
    expect(resolved, ['a', 'b']);
    expect(controller.page, 1);
    expect(first.mounted, isTrue);
    controller.jumpToPage(2);
    await tester.pump();
    expect(tester.state(find.byKey(keyA)), same(first));

    // Null means no new index; removal releases the parked child.
    controller.jumpToPage(1);
    await tester.pump();
    await tester.pumpWidget(view(['c', 'b']));
    expect(first.mounted, isFalse);
  });
}
