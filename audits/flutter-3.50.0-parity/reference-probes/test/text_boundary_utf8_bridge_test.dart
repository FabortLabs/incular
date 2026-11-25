import 'package:flutter/widgets.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('paragraph boundaries include CRLF after non-ASCII text', () {
    const boundary = ParagraphBoundary('é\r\nβ');
    expect(boundary.getTextBoundaryAt(2), const TextRange(start: 0, end: 3));
    expect(boundary.getTextBoundaryAt(3), const TextRange(start: 3, end: 4));
  });

  test('negative positions preserve the endpoint on an empty document', () {
    const character = CharacterBoundary('');
    const document = DocumentBoundary('');
    const paragraph = ParagraphBoundary('');
    for (final TextBoundary boundary in [character, document]) {
      expect(
        boundary.getTextBoundaryAt(-1),
        const TextRange(start: -1, end: 0),
      );
      expect(boundary.getTextBoundaryAt(0), const TextRange(start: 0, end: -1));
    }
    expect(paragraph.getTextBoundaryAt(-1), TextRange.empty);
  });

  testWidgets('line boundary excludes a hard line break in real layout', (
    tester,
  ) async {
    final controller = TextEditingController(text: 'abcdefghij\nk');
    final focus = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(focus.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: EditableText(
          controller: controller,
          focusNode: focus,
          style: const TextStyle(fontSize: 14),
          cursorColor: const Color(0xFF000000),
          backgroundCursorColor: const Color(0xFFFFFFFF),
          maxLines: null,
        ),
      ),
    );
    final state = tester.state<EditableTextState>(find.byType(EditableText));
    final LineBoundary boundary = LineBoundary(state.renderEditable);
    expect(boundary.getTextBoundaryAt(3), const TextRange(start: 0, end: 10));
    expect(boundary.getTextBoundaryAt(10), const TextRange(start: 0, end: 10));
    expect(boundary.getTextBoundaryAt(11), const TextRange(start: 11, end: 12));
  });
}
