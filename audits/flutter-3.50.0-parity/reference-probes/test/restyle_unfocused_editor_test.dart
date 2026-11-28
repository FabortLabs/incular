import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('changing a previously focused editor relayouts only its authored style', (
    WidgetTester tester,
  ) async {
    final nodes = [FocusNode(), FocusNode()];
    final controllers = List.generate(4, (_) => TextEditingController());
    addTearDown(() {
      for (final node in nodes) {
        node.dispose();
      }
      for (final controller in controllers) {
        controller.dispose();
      }
    });
    const first = ValueKey('old editor');
    const second = ValueKey('current editor');
    Widget build({required bool update}) => Directionality(
      textDirection: TextDirection.ltr,
      child: Column(children: [
        EditableText(
          key: first,
          controller: controllers[update ? 2 : 0],
          focusNode: nodes[0],
          style: TextStyle(fontSize: update ? 20 : 9, fontFamily: 'FlutterTest'),
          cursorColor: const Color(0xff0000ff),
          backgroundCursorColor: const Color(0xffaaaaaa),
        ),
        EditableText(
          key: second,
          controller: controllers[update ? 3 : 1],
          focusNode: nodes[1],
          style: const TextStyle(fontSize: 9, fontFamily: 'FlutterTest'),
          cursorColor: const Color(0xff0000ff),
          backgroundCursorColor: const Color(0xffaaaaaa),
        ),
      ]),
    );
    await tester.pumpWidget(build(update: false));
    expect(tester.getSize(find.byKey(first)).height, 9);
    await tester.tap(find.byKey(first));
    await tester.showKeyboard(find.byKey(first));
    controllers[0].text = 'test';
    await tester.pump();
    expect(nodes[0].hasPrimaryFocus, isTrue);
    await tester.tap(find.byKey(second));
    await tester.showKeyboard(find.byKey(second));
    expect(nodes[1].hasPrimaryFocus, isTrue);

    await tester.pumpWidget(build(update: true));
    expect(tester.getSize(find.byKey(first)).height, 20);
    expect(tester.getSize(find.byKey(second)).height, 9);
    expect(nodes[1].hasPrimaryFocus, isTrue);
    expect(nodes[0].hasFocus, isFalse);
  });
}
