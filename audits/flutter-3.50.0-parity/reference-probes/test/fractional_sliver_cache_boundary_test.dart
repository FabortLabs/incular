import 'package:flutter/rendering.dart' show ScrollCacheExtent;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('fixed extent uses half-open fractional cache bounds', (tester) async {
    for (final (extent, start, end, expected) in [
      (20.0, 0.0, 60.0, [0, 1, 2]),
      (20.3, 0.0, 60.9, [0, 1, 2]),
      (20.3, 0.0, 60.899, [0, 1, 2]),
      (20.3, 0.0, 60.901, [0, 1, 2, 3]),
      (1.1, 7.7, 8.05, [7]),
      (1.1, 7.699, 8.05, [6, 7]),
      (1.1, 7.701, 8.05, [7]),
    ]) {
      final seen = <int>[];
      final controller = ScrollController(initialScrollOffset: start);
      addTearDown(controller.dispose);
      await tester.pumpWidget(Directionality(
        textDirection: TextDirection.ltr,
        child: Align(alignment: Alignment.topLeft, child: SizedBox(
          width: 40,
          height: end - start,
          child: CustomScrollView(
            controller: controller,
            scrollCacheExtent: const ScrollCacheExtent.pixels(0),
            slivers: [
              SliverFixedExtentList.builder(
                itemExtent: extent,
                itemCount: 10,
                itemBuilder: (context, index) {
                  seen.add(index);
                  return const SizedBox.expand();
                },
              ),
            ],
          ),
        )),
      ));
      expect(seen, expected, reason: 'extent=$extent and cache=[$start,$end)');
      await tester.pumpWidget(const SizedBox.shrink());
    }
  });
}
