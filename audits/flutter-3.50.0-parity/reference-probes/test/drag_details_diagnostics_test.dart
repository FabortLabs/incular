import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';

String description(Object details) =>
    details.toString().replaceFirst(RegExp(r'#[a-f0-9]{5}(?=\()'), '');

void main() {
  test('default drag details have ordered one-line descriptions', () {
    const position = 'globalPosition: Offset(0.0, 0.0), localPosition: Offset(0.0, 0.0)';
    expect(description(DragDownDetails()), 'DragDownDetails($position)');
    expect(
      description(DragStartDetails()),
      'DragStartDetails($position, sourceTimeStamp: null, kind: null)',
    );
    expect(
      description(DragUpdateDetails(globalPosition: Offset.zero)),
      'DragUpdateDetails($position, sourceTimeStamp: null, delta: Offset(0.0, 0.0), primaryDelta: null)',
    );
    expect(
      description(DragEndDetails()),
      'DragEndDetails($position, velocity: Velocity(0.0, 0.0), primaryVelocity: null)',
    );
  });
}
