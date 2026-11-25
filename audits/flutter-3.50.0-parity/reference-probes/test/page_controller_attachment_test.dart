import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('controller hooks follow the mounted position, not each build', (
    tester,
  ) async {
    final log = <String>[];
    PageController create(String name) => PageController(
      onAttach: (_) => log.add('attach $name'),
      onDetach: (_) => log.add('detach $name'),
    );
    final a = create('a');
    final b = create('b');
    addTearDown(a.dispose);
    addTearDown(b.dispose);

    Widget host(PageController controller) => Directionality(
      textDirection: TextDirection.ltr,
      child: PageView(
        controller: controller,
        children: const [Text('0'), Text('1'), Text('2')],
      ),
    );

    await tester.pumpWidget(host(a));
    expect(log, ['attach a']);
    await tester.pumpWidget(host(a));
    expect(log, ['attach a']);
    a.jumpToPage(1);
    await tester.pumpAndSettle();
    expect(log, ['attach a']);
    await tester.pumpWidget(host(b));
    expect(log, ['attach a', 'detach a', 'attach b']);
    expect(b.page, 1);
    await tester.pumpWidget(const SizedBox());
    expect(log, ['attach a', 'detach a', 'attach b', 'detach b']);
    await tester.pumpWidget(host(a));
    expect(log, ['attach a', 'detach a', 'attach b', 'detach b', 'attach a']);
    expect(a.page, 0);
  });
}
