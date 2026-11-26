import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('Editable minimum width excludes the caret and uses word boundaries', () {
    const style = TextStyle(fontFamily: 'Ahem', fontSize: 10, height: 1);
    final editable = RenderEditable(
      text: const TextSpan(text: '12345', style: style),
      startHandleLayerLink: LayerLink(),
      endHandleLayerLink: LayerLink(),
      textDirection: TextDirection.ltr,
      offset: ViewportOffset.zero(),
      textSelectionDelegate: _Delegate(),
    );
    addTearDown(editable.dispose);
    expect(editable.getMinIntrinsicWidth(double.infinity), closeTo(50, 0.001));
    expect(editable.getMaxIntrinsicWidth(double.infinity), closeTo(52, 0.001));

    editable.maxLines = null;
    editable.text = const TextSpan(text: 'Hi World', style: style);
    expect(editable.getMinIntrinsicWidth(double.infinity), closeTo(50, 0.001));
    expect(editable.getMaxIntrinsicWidth(double.infinity), closeTo(82, 0.001));
    editable.textScaler = const TextScaler.linear(2);
    expect(editable.getMinIntrinsicWidth(double.infinity), closeTo(100, 0.001));
    expect(editable.getMaxIntrinsicWidth(double.infinity), closeTo(162, 0.001));
  });
}

class _Delegate with TextSelectionDelegate {
  @override
  TextEditingValue textEditingValue = TextEditingValue.empty;

  @override
  void hideToolbar([bool hideHandles = true]) {}

  @override
  void userUpdateTextEditingValue(TextEditingValue value, SelectionChangedCause cause) {}

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
