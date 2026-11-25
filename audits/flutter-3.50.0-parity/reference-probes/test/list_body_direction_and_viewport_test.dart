import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const _first = ValueKey('first');
const _body = ValueKey('body');
const _children = [
  SizedBox(key: _first, width: 200, height: 150),
  SizedBox(width: 200, height: 150),
  SizedBox(width: 200, height: 150),
  SizedBox(width: 200, height: 150),
];

void main() {
  testWidgets(
    'reversing a vertical body moves but does not replace the first child',
    (tester) async {
      Widget frame(bool reverse) => Column(
        children: [ListBody(reverse: reverse, children: _children)],
      );
      await tester.pumpWidget(frame(false));
      final first = find.byKey(_first);
      final element = tester.element(first);
      expect(tester.getTopLeft(first), Offset.zero);
      await tester.pumpWidget(frame(true));
      expect(tester.element(first), same(element));
      expect(tester.getRect(first), const Rect.fromLTWH(0, 450, 800, 150));
    },
  );

  testWidgets('horizontal reverse combines with changing inherited direction', (
    tester,
  ) async {
    Widget frame(TextDirection direction) => Row(
      textDirection: TextDirection.ltr,
      children: [
        Directionality(
          textDirection: direction,
          child: const ListBody(
            mainAxis: Axis.horizontal,
            reverse: true,
            children: _children,
          ),
        ),
      ],
    );
    await tester.pumpWidget(frame(TextDirection.ltr));
    final first = find.byKey(_first);
    expect(tester.getRect(first), const Rect.fromLTWH(600, 0, 200, 600));
    await tester.pumpWidget(frame(TextDirection.rtl));
    expect(tester.getRect(first), const Rect.fromLTWH(0, 0, 200, 600));
  });

  testWidgets(
    'a matching viewport stretches width and retains content extent',
    (tester) async {
      final controller = ScrollController();
      addTearDown(controller.dispose);
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: 320,
              height: 250,
              child: SingleChildScrollView(
                controller: controller,
                child: const ListBody(key: _body, children: _children),
              ),
            ),
          ),
        ),
      );
      expect(tester.getSize(find.byKey(_body)), const Size(320, 600));
      expect(
        tester.getRect(find.byKey(_first)),
        const Rect.fromLTWH(0, 0, 320, 150),
      );
      expect(controller.position.maxScrollExtent, 350);
    },
  );

  testWidgets('an empty vertical body still fills the cross axis', (
    tester,
  ) async {
    await tester.pumpWidget(
      const Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 320,
          child: Column(children: [ListBody(key: _body)]),
        ),
      ),
    );
    expect(tester.getSize(find.byKey(_body)), const Size(320, 0));
  });

  testWidgets('intrinsic wrappers supply an unbounded cross axis', (
    tester,
  ) async {
    const shapes = [
      SizedBox(width: 40, height: 20),
      SizedBox(width: 60, height: 30),
    ];
    await tester.pumpWidget(
      const UnconstrainedBox(
        child: IntrinsicWidth(
          child: ListBody(key: _body, children: shapes),
        ),
      ),
    );
    expect(tester.getSize(find.byKey(_body)), const Size(60, 50));
    await tester.pumpWidget(
      const Directionality(
        textDirection: TextDirection.ltr,
        child: UnconstrainedBox(
          child: IntrinsicHeight(
            child: ListBody(
              key: _body,
              mainAxis: Axis.horizontal,
              children: shapes,
            ),
          ),
        ),
      ),
    );
    expect(tester.getSize(find.byKey(_body)), const Size(100, 50));
  });
}
