import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('signed duration rounding and native integer limits', () {
    const low = -9223372036854775808;
    const high = 9223372036854775807;
    int sample(int a, int b, double t) {
      return lerpDuration(
        Duration(microseconds: a),
        Duration(microseconds: b),
        t,
      ).inMicroseconds;
    }

    expect(sample(0, 1, 0.5), 1);
    expect(sample(0, -1, 0.5), -1);
    expect(sample(low, high, 0.5), low);
    expect(sample(0, 1, 1e30), high);
    expect(sample(0, -1, 1e30), low);
    for (final t in [double.nan, double.infinity, double.negativeInfinity]) {
      expect(() => sample(0, 1, t), throwsUnsupportedError);
    }
  });
}
