import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('The default reveal works without an advertised action bit', (
    WidgetTester tester,
  ) async {
    const Key target = ValueKey<String>('reveal');
    final ScrollController controller = ScrollController();
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: ListView.builder(
          controller: controller,
          itemExtent: 50,
          itemCount: 30,
          itemBuilder: (BuildContext context, int index) => Semantics(
            key: index == 14 ? target : null,
            label: 'row $index',
            child: const SizedBox(height: 50),
          ),
        ),
      ),
    );
    final SemanticsNode hidden = tester.getSemantics(
      find.byKey(target, skipOffstage: false),
    );
    final SemanticsOwner owner = tester.binding.pipelineOwner.semanticsOwner!;
    final int id = hidden.id;
    expect(controller.offset, 0);
    expect(hidden.getSemanticsData().flagsCollection.isHidden, isTrue);
    expect(
      hidden.getSemanticsData().hasAction(SemanticsAction.showOnScreen),
      isFalse,
    );
    owner.performAction(id, SemanticsAction.showOnScreen);
    await tester.pumpAndSettle();
    final SemanticsNode revealed = tester.getSemantics(find.byKey(target));
    expect(revealed.id, id);
    expect(controller.offset, 150);
    expect(revealed.getSemanticsData().flagsCollection.isHidden, isFalse);
    expect(
      revealed.getSemanticsData().hasAction(SemanticsAction.showOnScreen),
      isFalse,
    );
  });
}
