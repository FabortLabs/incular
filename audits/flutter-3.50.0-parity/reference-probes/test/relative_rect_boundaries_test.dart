import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('fromRect and toRect account separately for the container origin', () {
    const Rect container = Rect.fromLTRB(7, 9, 107, 89);
    const Rect item = Rect.fromLTRB(1, 20, 57, 99);
    final RelativeRect relative = RelativeRect.fromRect(item, container);
    expect(relative, const RelativeRect.fromLTRB(-6, 11, 50, -10));
    expect(relative.toRect(container), const Rect.fromLTRB(-6, 11, 50, 90));
    expect(relative.toRect(container).shift(container.topLeft), item);
    expect(relative.toSize(container.size), const Size(56, 79));
  });

  test('disjoint intersections retain signed edge distances', () {
    const RelativeRect near = RelativeRect.fromLTRB(10, 10, 70, 70);
    const RelativeRect far = RelativeRect.fromLTRB(70, 70, 10, 10);
    final RelativeRect disjoint = near.intersect(far);
    const Rect container = Rect.fromLTWH(17, 19, 100, 100);
    expect(disjoint, const RelativeRect.fromLTRB(70, 70, 70, 70));
    expect(disjoint.toRect(container), const Rect.fromLTRB(70, 70, 30, 30));
    expect(disjoint.toSize(container.size), const Size(-40, -40));
  });

  test('one-sided nullable interpolation has an asymmetric pinned failure', () {
    const RelativeRect rect = RelativeRect.fromLTRB(4, 8, 12, 16);
    expect(
      RelativeRect.lerp(null, rect, 0.25),
      const RelativeRect.fromLTRB(1, 2, 3, 4),
    );
    expect(
      () => RelativeRect.lerp(rect, null, 0.25),
      throwsA(isA<TypeError>()),
    );
  });
}
