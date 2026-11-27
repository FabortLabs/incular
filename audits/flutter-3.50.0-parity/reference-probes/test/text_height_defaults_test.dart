import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  const TextHeightBehavior trimmed = TextHeightBehavior(
    applyHeightToFirstAscent: false,
    applyHeightToLastDescent: false,
  );
  const TextHeightBehavior full = TextHeightBehavior();
  const TextStyle style = TextStyle(fontSize: 10, height: 3);
  const Text text = Text('Hg', style: style, textDirection: TextDirection.ltr);

  testWidgets('Equal inherited height values stay quiet and a changed value relayouts', (
    WidgetTester tester,
  ) async {
    var builds = 0;
    final Builder listener = Builder(
      builder: (BuildContext context) {
        builds++;
        DefaultTextHeightBehavior.maybeOf(context);
        return text;
      },
    );
    Widget make(TextHeightBehavior value) => Center(
      child: DefaultTextHeightBehavior(textHeightBehavior: value, child: listener),
    );
    await tester.pumpWidget(make(trimmed));
    final Finder paragraph = find.byType(RichText);
    final double initialHeight = tester.getSize(paragraph).height;
    expect(initialHeight, lessThan(30));
    expect(builds, 1);

    await tester.pumpWidget(make(trimmed));
    expect(builds, 1);
    expect(tester.getSize(paragraph).height, initialHeight);

    await tester.pumpWidget(make(full));
    expect(builds, 2);
    expect(tester.getSize(paragraph).height, 30);
  });

  testWidgets('Nearest DefaultTextStyle can leave the standalone height behavior in control', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(const Center(
      child: DefaultTextHeightBehavior(
        textHeightBehavior: full,
        child: DefaultTextStyle(
          style: style,
          textHeightBehavior: trimmed,
          child: DefaultTextStyle(style: style, child: text),
        ),
      ),
    ));
    final Finder paragraph = find.byType(RichText);
    expect(tester.widget<RichText>(paragraph).textHeightBehavior, full);
    expect(tester.getSize(paragraph).height, 30);
  });

  testWidgets('AnimatedDefaultTextStyle applies height behavior immediately', (
    WidgetTester tester,
  ) async {
    Widget make(TextHeightBehavior behavior, double spacing) => Center(
      child: AnimatedDefaultTextStyle(
        duration: const Duration(milliseconds: 200),
        style: TextStyle(fontSize: 10, height: 3, letterSpacing: spacing),
        textHeightBehavior: behavior,
        child: const Text('Hg', textDirection: TextDirection.ltr),
      ),
    );
    await tester.pumpWidget(make(trimmed, 0));
    final Finder paragraph = find.byType(RichText);
    expect(tester.getSize(paragraph).height, lessThan(30));
    final double startingWidth = tester.getSize(paragraph).width;
    await tester.pumpWidget(make(full, 10));
    expect(tester.getSize(paragraph).height, 30);
    expect(tester.getSize(paragraph).width, startingWidth);
    await tester.pump(const Duration(milliseconds: 100));
    expect(tester.getSize(paragraph).width, greaterThan(startingWidth));
  });
}
