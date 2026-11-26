import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final axis in Axis.values) {
    for (final decorated in [false, true]) {
      testWidgets(
        'parallel intrinsics exclude spacing: $axis, decorated=$decorated',
        (tester) async {
          final horizontal = axis == Axis.horizontal;
          final first = GlobalKey();
          final second = GlobalKey();
          final wrapKey = GlobalKey();
          final wrap = Wrap(
            key: wrapKey,
            direction: axis,
            spacing: 7,
            runSpacing: 4,
            children: [
              SizedBox(
                key: first,
                width: horizontal ? 20 : 5,
                height: horizontal ? 5 : 20,
              ),
              SizedBox(
                key: second,
                width: horizontal ? 30 : 9,
                height: horizontal ? 9 : 30,
              ),
            ],
          );
          final measured = decorated
              ? ColoredBox(color: const Color(0xff000000), child: wrap)
              : wrap;
          await tester.pumpWidget(
            Directionality(
              textDirection: TextDirection.ltr,
              child: Center(
                child: horizontal
                    ? IntrinsicWidth(child: measured)
                    : IntrinsicHeight(child: measured),
              ),
            ),
          );
          final origin = tester.getTopLeft(find.byKey(wrapKey));
          expect(
            tester.getSize(find.byKey(wrapKey)),
            horizontal ? const Size(50, 18) : const Size(18, 50),
          );
          expect(tester.getTopLeft(find.byKey(first)) - origin, Offset.zero);
          expect(
            tester.getTopLeft(find.byKey(second)) - origin,
            horizontal ? const Offset(0, 9) : const Offset(9, 0),
          );
        },
      );
    }
  }
}
