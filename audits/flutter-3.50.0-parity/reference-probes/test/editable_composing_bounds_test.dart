import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'Composing bounds use local paint offsets and include newline boxes',
    () {
      final editable = RenderEditable(
        text: const TextSpan(
          text: 'AAA\nB\nCC\nDDDD',
          style: TextStyle(fontSize: 10, height: 1),
        ),
        maxLines: null,
        startHandleLayerLink: LayerLink(),
        endHandleLayerLink: LayerLink(),
        textDirection: TextDirection.ltr,
        offset: ViewportOffset.fixed(20),
        textSelectionDelegate: _Delegate(),
      );
      addTearDown(editable.dispose);
      editable.layout(BoxConstraints.tight(const Size(120, 20)));
      Rect? rect(int start, int end) =>
          editable.getRectForComposingRange(TextRange(start: start, end: end));
      void expectRect(Rect? actual, Rect expected) {
        expect(actual, isNotNull);
        expect(actual!.left, closeTo(expected.left, 0.001));
        expect(actual.top, closeTo(expected.top, 0.001));
        expect(actual.width, closeTo(expected.width, 0.001));
        expect(actual.height, closeTo(expected.height, 0.001));
      }

      expectRect(rect(0, 3), const Rect.fromLTWH(0, -20, 30, 10));
      expectRect(rect(3, 4), const Rect.fromLTWH(30, -20, 0, 10));
      expectRect(rect(6, 8), const Rect.fromLTWH(0, 0, 20, 10));
    },
  );

  test(
    'A composing range does not partially select a combined glyph',
    () async {
      const family = 'Audit Composing Inter';
      var directory = Directory.current;
      while (!File('${directory.path}/harness/flutter/fonts/Inter-Regular.ttf')
          .existsSync()) {
        if (directory.parent.path == directory.path) {
          throw StateError('Missing Incular OFL harness font');
        }
        directory = directory.parent;
      }
      final file = File(
        '${directory.path}/harness/flutter/fonts/Inter-Regular.ttf',
      );
      final data = ByteData.sublistView(await file.readAsBytes());
      await (FontLoader(family)..addFont(Future.value(data))).load();
      final editable = RenderEditable(
        text: const TextSpan(
          text: 'a\u0301',
          style: TextStyle(fontFamily: family, fontSize: 20),
        ),
        startHandleLayerLink: LayerLink(),
        endHandleLayerLink: LayerLink(),
        textDirection: TextDirection.ltr,
        offset: ViewportOffset.zero(),
        textSelectionDelegate: _Delegate(),
      );
      addTearDown(editable.dispose);
      editable.layout(BoxConstraints.tight(const Size(120, 30)));
      Rect? rect(int start, int end) =>
          editable.getRectForComposingRange(TextRange(start: start, end: end));
      expect(rect(0, 1), isNull);
      expect(rect(1, 2), isNull);
      expect(rect(0, 2)!.width, greaterThan(0));
    },
  );
}

class _Delegate with TextSelectionDelegate {
  @override
  TextEditingValue textEditingValue = TextEditingValue.empty;

  @override
  void hideToolbar([bool hideHandles = true]) {}

  @override
  void userUpdateTextEditingValue(
    TextEditingValue value,
    SelectionChangedCause cause,
  ) {}

  @override
  void bringIntoView(TextPosition position) {}

  @override
  void cutSelection(SelectionChangedCause cause) {}

  @override
  Future<void> pasteText(SelectionChangedCause cause) async {}

  @override
  void selectAll(SelectionChangedCause cause) {}

  @override
  void copySelection(SelectionChangedCause cause) {}
}
