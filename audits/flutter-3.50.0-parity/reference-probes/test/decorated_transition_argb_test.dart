import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('compact decoration colors round the continuous Flutter sample', () {
    final tween = DecorationTween(
      begin: BoxDecoration(
        color: const Color(0xFFFFFFFF),
        border: Border.all(width: 4),
      ),
      end: BoxDecoration(
        color: const Color(0xFF000000),
        border: Border.all(color: const Color(0xFF202020)),
      ),
    );
    final linear = tween.transform(0.5) as BoxDecoration;
    final eased =
        tween.transform(Curves.easeOut.transform(0.5)) as BoxDecoration;
    final linearBorder = (linear.border! as Border).left.color.toARGB32();
    final easedBorder = (eased.border! as Border).left.color.toARGB32();
    expect(
      [
        linear.color!.toARGB32(),
        linearBorder,
        eased.color!.toARGB32(),
        easedBorder,
      ],
      [0xFF808080, 0xFF101010, 0xFF515151, 0xFF161616],
    );
  });
}
