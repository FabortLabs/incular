import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('changing only an absorbed child refreshes controls', (
    tester,
  ) async {
    final controller = GlobalKey();
    final value = ValueNotifier<Set<String>>(<String>{'abc', 'def'});
    addTearDown(value.dispose);
    await tester.pumpWidget(
      Semantics(
        key: controller,
        controlsNodes: const <String>{'abc', 'ghi'},
        child: ValueListenableBuilder<Set<String>>(
          valueListenable: value,
          builder: (_, names, _) =>
              Semantics(controlsNodes: names, child: const Placeholder()),
        ),
      ),
    );
    final before = tester.getSemantics(find.byKey(controller));
    expect(before.getSemanticsData().controlsNodes, <String>{
      'abc',
      'ghi',
      'def',
    });

    value.value = <String>{'ghi', 'new'};
    await tester.pump();
    final after = tester.getSemantics(find.byKey(controller));
    expect(after.id, before.id);
    expect(after.getSemanticsData().controlsNodes, <String>{
      'abc',
      'ghi',
      'new',
    });
  });
}
