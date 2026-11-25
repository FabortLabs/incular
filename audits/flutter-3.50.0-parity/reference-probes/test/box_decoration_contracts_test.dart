import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('BoxDecoration self-lerp preserves its background blend mode', () {
    const from = BoxDecoration(
      color: Color(0xff7f1fff),
      backgroundBlendMode: BlendMode.multiply,
    );
    final to = from.copyWith(backgroundBlendMode: BlendMode.difference);
    expect(from, isNot(to));
    expect(BoxDecoration.lerp(from, to, 0), from);
    expect(BoxDecoration.lerp(from, to, 1), to);
    expect(BoxDecoration.lerp(from, to, 0.5)!.backgroundBlendMode, isNull);
    expect(BoxDecoration.lerp(from, from, 0.5), from);
  });

  test('BoxDecoration only skips interpolation for identical instances', () {
    const from = BoxDecoration(
      color: Color(0xff000000),
      backgroundBlendMode: BlendMode.multiply,
    );
    final to = from.copyWith();
    expect(to, from);
    expect(identical(from, to), isFalse);
    expect(BoxDecoration.lerp(from, to, 0.5)!.backgroundBlendMode, isNull);
  });

  test('BoxDecoration switches shape at the midpoint', () {
    const rectangle = BoxDecoration(color: Color(0xff000000));
    final circle = rectangle.copyWith(shape: BoxShape.circle);
    expect(
      BoxDecoration.lerp(rectangle, circle, 0.25)!.shape,
      BoxShape.rectangle,
    );
    expect(BoxDecoration.lerp(rectangle, circle, 0.5)!.shape, BoxShape.circle);
    expect(BoxDecoration.lerp(rectangle, circle, 0.75)!.shape, BoxShape.circle);
  });

  test('BoxDecoration image alone cannot enable a background blend', () {
    const picture = DecorationImage(image: AssetImage('unused.png'));
    expect(BoxDecoration(image: picture).debugAssertIsValid(), isTrue);
    expect(
      BoxDecoration(
        image: picture,
        color: const Color(0xff000000),
        backgroundBlendMode: BlendMode.multiply,
      ).debugAssertIsValid(),
      isTrue,
    );
    expect(
      () => BoxDecoration(backgroundBlendMode: BlendMode.multiply),
      throwsAssertionError,
    );
    expect(
      () => BoxDecoration(
        image: picture,
        backgroundBlendMode: BlendMode.multiply,
      ),
      throwsAssertionError,
    );
  });
}
