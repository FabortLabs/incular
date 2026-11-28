import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('regular tile geometry uses exact row boundaries and mirrors columns', () {
    const delegate = SliverGridDelegateWithFixedCrossAxisCount(
      crossAxisCount: 3,
      mainAxisExtent: 20,
      mainAxisSpacing: 5,
      crossAxisSpacing: 4,
    );
    const constraints = SliverConstraints(
      axisDirection: AxisDirection.down,
      growthDirection: GrowthDirection.forward,
      userScrollDirection: ScrollDirection.idle,
      scrollOffset: 100,
      precedingScrollExtent: 0,
      overlap: 0,
      remainingPaintExtent: 0,
      crossAxisExtent: 104,
      crossAxisDirection: AxisDirection.left,
      viewportMainAxisExtent: 100,
      remainingCacheExtent: 0,
      cacheOrigin: 0,
    );
    final layout = delegate.getLayout(constraints);
    expect(layout.getMinChildIndexForScrollOffset(25), 3);
    expect(layout.getMaxChildIndexForScrollOffset(25), 2);
    expect(layout.getMaxChildIndexForScrollOffset(25.125), 5);
    expect(layout.getMaxChildIndexForScrollOffset(0), 0);
    final tile = layout.getGeometryForChildIndex(3);
    expect(tile.scrollOffset, 25);
    expect(tile.crossAxisOffset, 72);
    expect(tile.trailingScrollOffset, 45);
    expect(tile.getBoxConstraints(constraints), const BoxConstraints.tightFor(width: 32, height: 20));
    expect(layout.computeMaxScrollOffset(0), 0);
    expect(layout.computeMaxScrollOffset(7), 70);
  });
}
