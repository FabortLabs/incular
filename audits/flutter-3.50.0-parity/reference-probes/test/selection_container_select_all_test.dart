import 'package:flutter/rendering.dart' show SelectedContent;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'Select all visits enabled child containers but skips disabled subtrees',
    (WidgetTester tester) async {
      final key = GlobalKey<SelectableRegionState>();
      final child = StaticSelectionContainerDelegate();
      addTearDown(child.dispose);
      SelectedContent? content;
      await tester.pumpWidget(
        TestWidgetsApp(
          home: SelectableRegion(
            key: key,
            selectionControls: emptyTextSelectionControls,
            onSelectionChanged: (value) => content = value,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                const Text('first'),
                SelectionContainer(
                  delegate: child,
                  child: const Text('second'),
                ),
                const SelectionContainer.disabled(child: Text('ignored')),
                const Text('third'),
              ],
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      key.currentState!.selectAll();
      await tester.pump();
      expect(content?.plainText, 'firstsecondthird');
      expect(child.getSelectedContent()?.plainText, 'second');
    },
  );
}
