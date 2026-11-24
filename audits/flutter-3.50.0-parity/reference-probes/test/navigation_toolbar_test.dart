import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const _leading = ValueKey<String>('leading');
const _middle = ValueKey<String>('middle');
const _trailing = ValueKey<String>('trailing');

Future<List<Rect>> _measure(
  WidgetTester tester, {
  double leading = 80,
  double middle = 40,
  double trailing = 20,
  double? spacing,
  bool centered = true,
  TextDirection direction = TextDirection.ltr,
}) async {
  final a = SizedBox(key: _leading, width: leading, height: 24);
  final b = SizedBox(key: _middle, width: middle, height: 20);
  final c = SizedBox(key: _trailing, width: trailing, height: 24);
  final toolbar = spacing == null
      ? NavigationToolbar(
          leading: a,
          middle: b,
          trailing: c,
          centerMiddle: centered,
        )
      : NavigationToolbar(
          leading: a,
          middle: b,
          trailing: c,
          centerMiddle: centered,
          middleSpacing: spacing,
        );
  await tester.pumpWidget(
    Directionality(
      textDirection: direction,
      child: Align(
        alignment: Alignment.topLeft,
        child: SizedBox(width: 300, height: 60, child: toolbar),
      ),
    ),
  );
  return [
    _leading,
    _middle,
    _trailing,
  ].map((key) => tester.getRect(find.byKey(key))).toList();
}

void main() {
  testWidgets(
    'asymmetric centered slots use the whole bar and stretch leading',
    (tester) async {
      expect(await _measure(tester), const [
        Rect.fromLTWH(0, 0, 80, 60),
        Rect.fromLTWH(130, 20, 40, 20),
        Rect.fromLTWH(280, 18, 20, 24),
      ]);
    },
  );
  testWidgets('equal side widths keep the middle at the toolbar center', (
    tester,
  ) async {
    expect(
      (await _measure(tester, leading: 50, trailing: 50))[1],
      const Rect.fromLTWH(130, 20, 40, 20),
    );
  });
  testWidgets(
    'default and explicit uncentered gaps start from the leading edge',
    (tester) async {
      expect(
        (await _measure(tester, centered: false))[1],
        const Rect.fromLTWH(96, 20, 40, 20),
      );
      expect(
        (await _measure(tester, centered: false, spacing: 24))[1],
        const Rect.fromLTWH(104, 20, 40, 20),
      );
    },
  );
  testWidgets('uncentered rtl mirrors every slot through the toolbar', (
    tester,
  ) async {
    expect(
      await _measure(tester, centered: false, direction: TextDirection.rtl),
      const [
        Rect.fromLTWH(220, 0, 80, 60),
        Rect.fromLTWH(164, 20, 40, 20),
        Rect.fromLTWH(0, 18, 20, 24),
      ],
    );
  });
  testWidgets('middle fits between leading and trailing reservations', (
    tester,
  ) async {
    expect(
      (await _measure(tester, leading: 200, middle: 100))[1],
      const Rect.fromLTWH(216, 20, 48, 20),
    );
    expect(
      (await _measure(tester, leading: 20, middle: 100, trailing: 140))[1],
      const Rect.fromLTWH(44, 20, 100, 20),
    );
    expect(
      (await _measure(tester, leading: 180, middle: 100, trailing: 100))[1],
      const Rect.fromLTWH(196, 20, 0, 20),
    );
  });
}
