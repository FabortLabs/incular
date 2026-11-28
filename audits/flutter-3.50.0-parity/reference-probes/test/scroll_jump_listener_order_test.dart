import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('jumping publishes the position before scroll notifications', (
    tester,
  ) async {
    final scroll = ScrollController();
    addTearDown(scroll.dispose);
    final log = <String>[];
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: NotificationListener<ScrollNotification>(
          onNotification: (notification) {
            log.add(notification.runtimeType.toString());
            return true;
          },
          child: ListView.builder(
            controller: scroll,
            itemExtent: 100,
            itemCount: 200,
            itemBuilder: (_, i) => const SizedBox(),
          ),
        ),
      ),
    );
    scroll.position.addListener(() {
      log.add('position');
    });
    scroll.jumpTo(100);
    expect(log, [
      'position',
      'ScrollStartNotification',
      'ScrollUpdateNotification',
      'ScrollEndNotification',
    ]);
    scroll.jumpTo(100);
    expect(log, hasLength(4));
    await tester.pump();
    expect(log, hasLength(4));
  });
}
