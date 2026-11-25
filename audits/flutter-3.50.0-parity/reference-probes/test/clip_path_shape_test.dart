import 'package:flutter/painting.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _RecordingStadium extends StadiumBorder {
  _RecordingStadium(this.bounds);

  final List<Rect> bounds;

  @override
  Path getOuterPath(Rect rect, {TextDirection? textDirection}) {
    bounds.add(rect);
    return super.getOuterPath(rect, textDirection: textDirection);
  }
}

void main() {
  testWidgets('ClipPath.shape uses its child size under loose constraints', (
    WidgetTester tester,
  ) async {
    final bounds = <Rect>[];
    await tester.pumpWidget(
      Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 300, maxHeight: 300),
          child: ClipPath.shape(
            shape: _RecordingStadium(bounds),
            child: const SizedBox(width: 100, height: 40, child: Placeholder()),
          ),
        ),
      ),
    );
    expect(tester.getSize(find.byType(ClipPath)), const Size(100, 40));
    expect(bounds, <Rect>[const Rect.fromLTWH(0, 0, 100, 40)]);
  });
}
