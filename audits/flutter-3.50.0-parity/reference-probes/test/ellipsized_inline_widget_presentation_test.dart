import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('An ellipsized inline widget is not painted, hit or announced', (
    WidgetTester tester,
  ) async {
    final key = GlobalKey();
    const color = Color(0xff17bb40);
    var taps = 0;

    Future<void> mount(double width) async {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: width,
              child: RichText(
                overflow: TextOverflow.ellipsis,
                maxLines: 1,
                text: TextSpan(
                  text: 'a long line to be truncated.',
                  style: const TextStyle(fontSize: 16, height: 1),
                  children: [
                    WidgetSpan(
                      child: Semantics(
                        label: 'truncated',
                        container: true,
                        child: GestureDetector(
                          onTap: () => taps += 1,
                          child: ColoredBox(
                            key: key,
                            color: color,
                            child: const SizedBox(width: 64, height: 16),
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      );
    }

    await mount(800);
    expect(find.bySemanticsLabel('truncated'), findsOneWidget);
    expect(find.byType(RichText), paints..rect(color: color));
    final location = tester.getCenter(find.byKey(key));
    await tester.tapAt(location);
    expect(taps, 1);

    await mount(100);
    expect(find.byKey(key), findsOneWidget);
    final root = tester.binding.pipelineOwner.semanticsOwner!.rootSemanticsNode!;
    final attached = <String>[];
    bool visit(SemanticsNode node) {
      if (node.label.isNotEmpty) {
        attached.add(node.label);
      }
      node.visitChildren(visit);
      return true;
    }
    visit(root);
    expect(attached, ['a long line to be truncated.']);
    expect(find.byType(RichText), isNot(paints..rect(color: color)));
    await tester.tapAt(const Offset(1, 1));
    expect(taps, 1);

    await mount(800);
    expect(find.bySemanticsLabel('truncated'), findsOneWidget);
    expect(find.byType(RichText), paints..rect(color: color));
    await tester.tapAt(location);
    expect(taps, 2);
  });
}
