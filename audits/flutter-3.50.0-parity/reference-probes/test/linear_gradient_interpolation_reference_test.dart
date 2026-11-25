import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'aligned animated box interpolates stops and late resolves direction',
    () {
      const a = BoxDecoration(
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [Color(0x22222222), Color(0x66666666)],
          tileMode: TileMode.repeated,
        ),
      );
      const b = BoxDecoration(
        gradient: LinearGradient(
          begin: AlignmentDirectional.topStart,
          end: AlignmentDirectional.bottomEnd,
          colors: [Color(0x44444444), Color(0x66666666), Color(0x88888888)],
          tileMode: TileMode.mirror,
        ),
      );
      final middle = BoxDecoration.lerp(a, b, 0.5)!.gradient! as LinearGradient;
      const rect = Rect.fromLTWH(0, 0, 100, 60);
      expect(
        middle.begin.resolve(TextDirection.ltr).withinRect(rect),
        Offset.zero,
      );
      expect(
        middle.begin.resolve(TextDirection.rtl).withinRect(rect),
        const Offset(50, 0),
      );
      expect(middle.tileMode, TileMode.mirror);
      expect(middle.stops, [0, 0.5, 1]);
      expect(middle.colors.map((color) => color.toARGB32()), [
        0x33333333,
        0x55555555,
        0x77777777,
      ]);
    },
  );
}
