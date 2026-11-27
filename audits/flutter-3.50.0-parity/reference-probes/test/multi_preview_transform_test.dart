import 'package:flutter/widget_previews.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const Color outer = Color(0xFF0000FF);
const Color inner = Color(0xFFFF0000);
const Color leaf = Color(0xFF00FF00);
final List<String> trace = <String>[];

base class Member extends Preview {
  const Member({required super.name, required super.brightness});

  @override
  Preview transform() {
    trace.add(name!);
    return (super.transform().toBuilder()..addWrapper(
          (Widget child) => ColoredBox(color: inner, child: child),
        ))
        .build();
  }
}

base class Pair extends MultiPreview {
  const Pair();

  @override
  List<Preview> get previews => const <Preview>[
    Member(name: 'Light', brightness: Brightness.light),
    Member(name: 'Dark', brightness: Brightness.dark),
  ];

  List<Preview> transformMembers() => super.transform();

  @override
  List<Preview> transform() => transformMembers().map((Preview preview) {
    return (preview.toBuilder()
          ..name = 'Pair ${preview.name}'
          ..addWrapper(
            (Widget child) => ColoredBox(color: outer, child: child),
          ))
        .build();
  }).toList();
}

void main() {
  testWidgets(
    'multi preview transforms in order and adds the group outer wrapper',
    (WidgetTester tester) async {
      trace.clear();
      const Pair pair = Pair();
      final List<Preview> result = pair.transform();
      expect(trace, <String>['Light', 'Dark']);
      expect(result.map((Preview preview) => preview.name), <String>[
        'Pair Light',
        'Pair Dark',
      ]);
      expect(result.map((Preview preview) => preview.brightness), <Brightness>[
        Brightness.light,
        Brightness.dark,
      ]);
      const Widget content = ColoredBox(
        color: leaf,
        child: SizedBox(width: 40, height: 20),
      );
      List<Color> colors() => tester
          .widgetList<ColoredBox>(find.byType(ColoredBox))
          .map((ColoredBox box) => box.color)
          .toList();
      for (final Preview preview in result) {
        await tester.pumpWidget(Center(child: preview.wrapper!(content)));
        expect(colors(), <Color>[outer, inner, leaf]);
      }
      final List<Preview> fresh = pair.transformMembers();
      expect(trace, <String>['Light', 'Dark', 'Light', 'Dark']);
      await tester.pumpWidget(Center(child: fresh.first.wrapper!(content)));
      expect(colors(), <Color>[inner, leaf]);
      expect(fresh.first.name, 'Light');
      expect(result.first.name, 'Pair Light');
    },
  );
}
